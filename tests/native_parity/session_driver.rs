//! Separate serial C baselines for actual public Session methods. C remains
//! entirely outside the Rust process/mutex, including during thread contention.
use super::cases::step;
use super::process::Worker;
use super::protocol::{Record, Request};
use super::registry::Op;
use super::session_probe::{PROBE, PROVENANCE, configuration};
use swisseph_bindings::*;

/// Exact executed Session totals, kept separate from free-function inventory.
#[derive(Default)]
pub struct Summary {
    /// Actual successful serial/concurrent computation results checked against C.
    pub computations: usize,
    /// Rust-only hazardous epochs rejected by actual public Session methods.
    pub rejections: usize,
    /// All exposed computation/provenance/rejection/status fields checked.
    pub fields: usize,
}

/// Send a C request with a unique step ID; failures disclose only categories.
fn native(c: &mut Worker, id: &mut u32, mut request: Request) -> Result<Record, String> {
    request.id = *id;
    *id += 1;
    c.request(&request)
}

/// Install every declared knob in public-ABI order, independently of Session's
/// private apply helper. No Rust free setter is used to create the baseline.
fn configure(c: &mut Worker, id: &mut u32, which: usize, path: &str) -> Result<(), String> {
    use Op::*;
    let config = configuration(which);
    let writes = [
        step(SetEphePath, &[], &[], &[Some(path)]),
        step(SetJplFile, &[], &[], &[Some("__parity_missing_jpl__.eph")]),
        step(SetTopo, &[], &config.topo, &[]),
        step(SetSidMode, &[config.sidereal], &config.reference, &[]),
        step(SetTidAcc, &[], &[config.tidal], &[]),
        step(SetDeltaTUserdef, &[1], &[config.delta_t], &[]),
        step(SetLapseRate, &[], &[config.lapse], &[]),
    ];
    for write in writes {
        let record = native(c, id, write.request)?;
        if record.code != 0 || record.ints.len() + record.floats.len() + record.texts.len() != 0 {
            return Err("C session configuration status/layout".into());
        }
    }
    Ok(())
}

/// Original operation inputs, reused for the corresponding public Session
/// computation. Flags do not conceal the actual returned primary source.
pub(super) fn input(op: Op, source: i32) -> Request {
    use Op::*;
    let jd = 2451545.0;
    match op {
        Calc | CalcUt => {
            step(
                op,
                &[MOON, source | FLG_SPEED | FLG_TOPOCTR | FLG_SIDEREAL],
                &[jd],
                &[],
            )
            .request
        }
        Houses => step(op, &[i32::from(b'P')], &[jd, 48.85, 2.35], &[]).request,
        HousesEx => {
            step(
                op,
                &[FLG_SIDEREAL, i32::from(b'P')],
                &[jd, 48.85, 2.35],
                &[],
            )
            .request
        }
        GetAyanamsaUt | Deltat => step(op, &[], &[jd], &[]).request,
        RiseTrans => {
            step(
                op,
                &[SUN, source, CALC_RISE],
                &[jd, 12.5, 41.9, 50.0, 1013.25, 15.0],
                &[None],
            )
            .request
        }
        _ => unreachable!("seven declared session methods"),
    }
}

/// Freeze serial native results/snapshots in memory, then validate serial,
/// concurrent and post-close ownership. Returns only pass/field counts.
pub fn run(
    c: &mut Worker,
    rust: &mut Worker,
    paths: [&str; 2],
    source: i32,
) -> Result<Summary, String> {
    use Op::*;
    let mut id = 50000;
    let mut summary = Summary::default();
    for op in [
        Calc,
        CalcUt,
        Houses,
        HousesEx,
        GetAyanamsaUt,
        Deltat,
        RiseTrans,
    ] {
        let mut baseline = Vec::new();
        let mut snapshots = Vec::new();
        for (which, path) in paths.iter().enumerate() {
            configure(c, &mut id, which, path)?;
            let mut record = native(c, &mut id, input(op, source))?;
            if record.code < 0 {
                return Err(format!("C session baseline failure: {}", op.entry().name));
            }
            // Capture before applying B or allowing unrelated native work.
            let mut history = Vec::new();
            for slot in 0..5 {
                history.push(native(
                    c,
                    &mut id,
                    step(GetCurrentFileData, &[slot], &[], &[]).request,
                )?);
            }
            snapshots.push(history);
            match op {
                Calc | CalcUt => super::compare::project_accessors(&mut record, 6, 3)?,
                Houses | HousesEx => super::compare::project_houses(&mut record, 12, false)?,
                _ => (),
            }
            baseline.push(record);
        }
        if op != Houses && super::compare::same(&baseline[0], &baseline[1], false).is_ok() {
            return Err(format!(
                "session A/B fixtures lack distinct C baselines: {}",
                op.entry().name
            ));
        }
        for threads in [4, 8] {
            let repetitions = if op == RiseTrans { 1 } else { 8 };
            let mut request = Request {
                id,
                op: PROBE,
                ints: vec![
                    op as i32,
                    source,
                    baseline[0].code,
                    baseline[1].code,
                    threads,
                    repetitions,
                ],
                floats: baseline[0]
                    .floats
                    .iter()
                    .chain(&baseline[1].floats)
                    .copied()
                    .collect(),
                texts: vec![
                    Some(baseline[0].texts.first().cloned().unwrap_or_default()),
                    Some(baseline[1].texts.first().cloned().unwrap_or_default()),
                    Some(paths[0].as_bytes().to_vec()),
                    Some(paths[1].as_bytes().to_vec()),
                ],
            };
            id += 1;
            let count = rust.request(&request).map_err(|e| {
                format!("session method={} threads={threads}: {e}", op.entry().name)
            })?;
            if count.code != 0
                || count.ints.len() != 2
                || !count.floats.is_empty()
                || !count.texts.is_empty()
                || count.ints[0] != 3 + threads * repetitions
                || count.ints[1] <= 0
            {
                return Err("session probe summary layout".into());
            }
            summary.computations += count.ints[0] as usize;
            summary.fields += count.ints[1] as usize;
            // Free close cannot replace either retained session's owned history.
            request = step(Close, &[], &[], &[]).request;
            request.id = id;
            id += 1;
            if rust.request(&request)?.code != 0 {
                return Err("post-session close failed".into());
            }
            for (which, history) in snapshots.iter().enumerate() {
                // Every actual method must reject a non-finite epoch without
                // replacing this owner's previously captured C-matched history.
                for rejected_op in [
                    Calc,
                    CalcUt,
                    Houses,
                    HousesEx,
                    GetAyanamsaUt,
                    Deltat,
                    RiseTrans,
                ] {
                    let unsafe_request = Request {
                        id,
                        op: super::session_rejection::REJECT,
                        ints: vec![which as i32, rejected_op as i32],
                        floats: vec![f64::NAN],
                        ..Request::default()
                    };
                    id += 1;
                    let rejected = rust.request(&unsafe_request)?;
                    summary.fields += super::compare::compare(rejected_op, None, &rejected, true)?;
                    summary.rejections += 1;
                }
                for slot in [-1, 5, i32::MAX] {
                    let query = Request {
                        id,
                        op: PROVENANCE,
                        ints: vec![which as i32, slot],
                        ..Request::default()
                    };
                    id += 1;
                    let mut record = rust.request(&query)?;
                    record.id = 0;
                    record.op = 0;
                    summary.fields += super::compare::same(
                        &Record {
                            ints: vec![0],
                            ..Record::default()
                        },
                        &record,
                        false,
                    )?;
                }
                for (slot, expected) in history.iter().enumerate() {
                    let query = Request {
                        id,
                        op: PROVENANCE,
                        ints: vec![which as i32, slot as i32],
                        ..Request::default()
                    };
                    id += 1;
                    let mut got = rust.request(&query)?;
                    got.id = expected.id;
                    got.op = expected.op;
                    summary.fields += super::compare::same(expected, &got, false).map_err(|e| {
                        format!(
                            "session snapshot {} owner={which} slot={slot}: {e}",
                            op.entry().name
                        )
                    })?;
                }
            }
        }
    }
    Ok(summary)
}
