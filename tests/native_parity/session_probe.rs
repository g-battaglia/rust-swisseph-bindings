//! C-baseline session and concurrency probes, executed in the Rust worker.
//!
//! The driver obtains expected fields from the separate C worker, projects only
//! public slots, and transports them in memory. No Rust calculation supplies a
//! reference. Fully specified A/B configurations permit concurrent comparison;
//! inherited unspecified knobs deliberately receive no isolation promise.
use super::protocol::{Record, Request};
use super::registry::Op;
use std::sync::{Arc, Barrier};
use swisseph_bindings::*;

/// Private request ID for one family of session computations and its two C
/// baselines. It is not a public binding operation or a fictitious C symbol.
pub const PROBE: u32 = 2000;
/// Private query of one retained session snapshot; does not reapply config.
pub const PROVENANCE: u32 = 2001;

/// Actual public sessions retained between probe and ownership/history queries.
#[derive(Default)]
pub struct State {
    /// A/B instances; threaded clones share each instance's actual history.
    sessions: Vec<Option<Session>>,
}

impl State {
    /// Select an existing session without applying any native configuration.
    pub(super) fn get(&self, which: i32) -> Result<&Session, String> {
        self.sessions
            .get(usize::try_from(which).map_err(|_| "negative session ID")?)
            .and_then(Option::as_ref)
            .ok_or_else(|| "session ID not built".into())
    }

    /// Replace a bounded slot with an owned public Session, dropping no other
    /// slot and performing no native close/configuration operation.
    pub(super) fn put(&mut self, which: i32, session: Session) -> Result<(), String> {
        if !(0..16).contains(&which) {
            return Err("session slot capacity".into());
        }
        self.sessions.resize_with(16, || None);
        self.sessions[which as usize] = Some(session);
        Ok(())
    }

    /// Remove a built slot; its clones, other histories and native state live on.
    pub(super) fn remove(&mut self, which: i32) -> Result<(), String> {
        self.get(which)?;
        self.sessions[which as usize] = None;
        Ok(())
    }
}

/// Fully explicit configuration shared only as input recipes, not as a native
/// validator or result projector. Degrees/metres, days and K/m follow the API.
pub struct Configuration {
    /// Observer longitude/latitude/elevation.
    pub topo: [f64; 3],
    /// Sidereal mode, including the USER-UT option in configuration B.
    pub sidereal: i32,
    /// Sidereal reference epoch and offset, days/degrees.
    pub reference: [f64; 2],
    /// Tidal acceleration, arcsec/cy².
    pub tidal: f64,
    /// Explicit TT minus UT, days.
    pub delta_t: f64,
    /// Atmospheric temperature lapse rate, K/m.
    pub lapse: f64,
}

/// Distinct configuration inputs; neither numerical results nor fitted values.
pub fn configuration(which: usize) -> Configuration {
    if which == 0 {
        Configuration {
            topo: [11.34, 48.14, 520.0],
            sidereal: SIDM_LAHIRI,
            reference: [0.0, 0.0],
            tidal: -25.8,
            delta_t: 0.0008,
            lapse: 0.0065,
        }
    } else {
        Configuration {
            topo: [151.21, -33.87, 50.0],
            sidereal: SIDM_USER | 1024,
            reference: [2451545.0, 17.5],
            tidal: -22.0,
            delta_t: 0.0012,
            lapse: 0.005,
        }
    }
}

/// Independent Rust builder route: all seven knobs are explicit, including the
/// missing JPL selection. Building alone must not apply the configuration.
fn session(which: usize, path: &str) -> Result<Session, Error> {
    let config = configuration(which);
    SessionBuilder::new()
        .ephe_path(path)
        .jpl_file("__parity_missing_jpl__.eph")
        .topo(config.topo[0], config.topo[1], config.topo[2])
        .sid_mode(config.sidereal, config.reference[0], config.reference[1])
        .tid_acc(config.tidal)
        .delta_t_override(config.delta_t)
        .lapse_rate(config.lapse)
        .build()
}

/// Compute through one of the seven actual session methods, packing the same
/// complete public components/accessors as the corresponding free operation.
fn compute(session: &Session, op: Op, source: i32) -> Result<Record, Error> {
    let jd = 2451545.0;
    let flags = source | FLG_SPEED | FLG_TOPOCTR | FLG_SIDEREAL;
    match op {
        Op::Calc | Op::CalcUt => {
            let value = if op == Op::Calc {
                session.calc(jd, MOON, flags)
            } else {
                session.calc_ut(jd, MOON, flags)
            }?;
            Ok(Record {
                code: value.returned_flags,
                floats: value
                    .values
                    .into_iter()
                    .chain([value.longitude(), value.latitude(), value.distance()])
                    .collect(),
                texts: vec![value.diagnostic.into_bytes()],
                ..Record::default()
            })
        }
        Op::Houses | Op::HousesEx => {
            let value = if op == Op::Houses {
                session.houses(jd, 48.85, 2.35, b'P')
            } else {
                session.houses_ex(jd, FLG_SIDEREAL, 48.85, 2.35, b'P')
            }?;
            Ok(Record {
                floats: value
                    .cusps
                    .into_iter()
                    .chain(value.angles)
                    .chain([value.ascendant(), value.mc()])
                    .collect(),
                ..Record::default()
            })
        }
        Op::GetAyanamsaUt => session.get_ayanamsa_ut(jd).map(|value| Record {
            floats: vec![value],
            ..Record::default()
        }),
        Op::Deltat => session.deltat(jd).map(|value| Record {
            floats: vec![value],
            ..Record::default()
        }),
        Op::RiseTrans => session
            .rise_trans(
                jd, SUN, None, source, CALC_RISE, 12.5, 41.9, 50.0, 1013.25, 15.0,
            )
            .map(|value| match value {
                RiseTransitOutcome::Event {
                    time_ut,
                    diagnostic,
                } => Record {
                    floats: vec![time_ut],
                    texts: vec![diagnostic.into_bytes()],
                    ..Record::default()
                },
                RiseTransitOutcome::Circumpolar { diagnostic } => Record {
                    code: -2,
                    texts: vec![diagnostic.into_bytes()],
                    ..Record::default()
                },
            }),
        _ => unreachable!("validated session method"),
    }
}

/// Validate the bounded probe shape, run serial A/B/A and four/eight-thread
/// contention, then return checked result/field counts, never reference vectors.
pub fn run(r: &Request, state: &mut State) -> Result<Record, String> {
    if r.ints.len() != 6 || r.texts.len() != 4 || r.texts.iter().any(Option::is_none) {
        return Err("session probe shape".into());
    }
    let op = Op::from_wire(r.i(0) as u32).ok_or("session probe operation")?;
    let width = match op {
        Op::Calc | Op::CalcUt => 9,
        Op::Houses | Op::HousesEx => 22,
        Op::GetAyanamsaUt | Op::Deltat | Op::RiseTrans => 1,
        _ => return Err("unsupported session probe operation".into()),
    };
    if r.floats.len() != width * 2
        || !matches!(r.i(4), 4 | 8)
        || !(1..=32).contains(&r.i(5))
        || (op == Op::RiseTrans && r.i(5) != 1)
    {
        return Err("session probe width/thread/repetition cap".into());
    }
    let sessions = [session(0, r.text(2)), session(1, r.text(3))]
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let expected = (0..2)
        .map(|which| Record {
            code: r.i(which + 2),
            floats: r.floats[which * width..(which + 1) * width].to_vec(),
            texts: if matches!(
                op,
                Op::Houses | Op::HousesEx | Op::GetAyanamsaUt | Op::Deltat
            ) {
                Vec::new()
            } else {
                vec![r.text(which).as_bytes().to_vec()]
            },
            ..Record::default()
        })
        .collect::<Vec<_>>();
    let mut fields = 0;
    for which in [0, 1, 0] {
        let result =
            compute(&sessions[which], op, r.i(1)).map_err(|_| "serial session native failure")?;
        fields += super::compare::same(&expected[which], &result, false)
            .map_err(|e| format!("serial session {which}: {e}"))?;
    }
    let barrier = Arc::new(Barrier::new(r.i(4) as usize));
    let mut children = Vec::new();
    for index in 0..r.i(4) {
        let which = index as usize % 2;
        let session = sessions[which].clone();
        let expected = expected[which].clone();
        let barrier = Arc::clone(&barrier);
        let source = r.i(1);
        let repetitions = r.i(5);
        children.push(std::thread::spawn(move || -> Result<usize, String> {
            barrier.wait();
            let mut fields = 0;
            for iteration in 0..repetitions {
                // Competing free calls/close use the same binding boundary;
                // the fully explicit session must reapply its own state.
                if iteration % 4 == 0 {
                    close().map_err(|_| "competing close failed")?;
                }
                if iteration % 4 == 1 {
                    set_topo(-77.0, 38.9, 0.0).map_err(|_| "competing free setter failed")?;
                }
                let result = compute(&session, op, source)
                    .map_err(|_| "concurrent session native failure")?;
                fields += super::compare::same(&expected, &result, false)
                    .map_err(|e| format!("concurrent session {which}: {e}"))?;
            }
            Ok(fields)
        }));
    }
    // Collect every thread, even after a failure; no detached background work.
    let mut failure = None;
    for child in children {
        match child.join() {
            Ok(Ok(count)) => fields += count,
            Ok(Err(error)) => {
                failure.get_or_insert(error);
            }
            Err(_) => {
                failure.get_or_insert("session probe thread panic".into());
            }
        }
    }
    if let Some(error) = failure {
        return Err(error);
    }
    state.sessions = sessions.into_iter().map(Some).collect();
    Ok(Record {
        ints: vec![3 + r.i(4) * r.i(5), fields as i32],
        ..Record::default()
    })
}

/// Return every public provenance field from the actual retained session,
/// after intervening computations/free calls/close. No native getter is used.
pub fn provenance(r: &Request, state: &State) -> Result<Record, String> {
    if r.ints.len() != 2 || !r.floats.is_empty() || !r.texts.is_empty() {
        return Err("session provenance shape".into());
    }
    let session = state.get(r.i(0))?;
    let value = session
        .get_current_file_data(r.i(1))
        .map_err(|_| "session provenance failure")?;
    Ok(match value {
        None => Record {
            ints: vec![0],
            ..Record::default()
        },
        Some(v) => Record {
            ints: vec![1, v.denum],
            floats: vec![v.tfstart, v.tfend],
            texts: vec![v.path.into_bytes()],
            ..Record::default()
        },
    })
}
