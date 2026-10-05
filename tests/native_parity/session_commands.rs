//! Bounded commands for real persistent public builders/sessions. These private
//! controls have no C counterparts: the supervisor separately applies only the
//! declared knobs through public C setters and compares the dependent operation.
//! A masked-off knob means inheritance, never a fabricated native default.
use super::protocol::{Record, Request};
use super::registry::Op;
use super::session_probe::State;
use swisseph_bindings::*;

/// Private builder command; exact shape is five integers/eight doubles/two texts.
pub const BUILD: u32 = 2010;
/// Private session operation; integers begin with slot and original operation ID.
pub const COMPUTE: u32 = 2011;
/// Clone an actual Session from source slot into destination slot.
pub const CLONE: u32 = 2012;
/// Drop exactly one retained Session without calling native close.
pub const DROP: u32 = 2013;
/// Ephemeris path bit; selected None restores the native compiled default.
pub const PATH: i32 = 1;
/// JPL bit; selected None uses the public builder's close-file operation.
pub const JPL: i32 = 2;
/// Observer longitude/latitude/elevation bit.
pub const TOPO: i32 = 4;
/// Sidereal mode/reference epoch/offset bit.
pub const SIDEREAL: i32 = 8;
/// Tidal acceleration bit.
pub const TIDAL: i32 = 16;
/// Delta-T bit; automatic versus pinned is a separate boolean argument.
pub const DELTA: i32 = 32;
/// Lapse-rate bit.
pub const LAPSE: i32 = 64;
/// All seven writable knobs, not a scientific result or native flag.
pub const ALL: i32 = 127;

/// Input-only recipe shared with the C script, not validation/packing logic.
#[derive(Clone, Default)]
pub struct Recipe {
    /// Selected knobs. Unset knobs are inherited at computation time.
    pub mask: i32,
    /// Native sidereal mode/options for a selected sidereal knob.
    pub mode: i32,
    /// Clear a selected Delta-T override instead of installing `numbers[6]`.
    pub automatic_delta: bool,
    /// Constructor recipe: 0 new, 1 Default, 2 cloned builder before build.
    pub style: i32,
    /// Observer triple, reference epoch/offset, tide, Delta-T days, lapse K/m.
    pub numbers: [f64; 8],
    /// Owned optional path; None differs from masked-off inheritance.
    pub path: Option<String>,
    /// Owned optional JPL name; None differs from masked-off inheritance.
    pub jpl: Option<String>,
}

impl Recipe {
    /// Encode only input knobs. Unsafe strings/numerics remain Rust-only inputs.
    pub fn request(&self, slot: i32) -> Request {
        Request {
            op: BUILD,
            ints: vec![
                slot,
                self.mask,
                self.mode,
                i32::from(self.automatic_delta),
                self.style,
            ],
            floats: self.numbers.to_vec(),
            texts: vec![
                self.path.as_ref().map(|s| s.as_bytes().to_vec()),
                self.jpl.as_ref().map(|s| s.as_bytes().to_vec()),
            ],
            ..Request::default()
        }
    }
}

/// Preserve actual public error kind and message without success-shaped fields.
fn result(value: Result<Record, Error>) -> Record {
    value.unwrap_or_else(super::rust_worker::error)
}

/// Validate the private shape before any indexing, then invoke public builder
/// methods. Store only a successfully built, owned Session. Failed replacement
/// leaves the previous slot/history intact, a supervisor-owned control invariant.
fn build(r: &Request, state: &mut State) -> Result<Record, String> {
    if r.ints.len() != 5
        || r.floats.len() != 8
        || r.texts.len() != 2
        || !(0..16).contains(&r.i(0))
        || r.i(1) & !ALL != 0
        || !matches!(r.i(3), 0 | 1)
        || !matches!(r.i(4), 0..=2)
    {
        return Err("persistent builder command shape/mask/slot".into());
    }
    let mut builder = if r.i(4) == 1 {
        SessionBuilder::default()
    } else {
        SessionBuilder::new()
    };
    let mask = r.i(1);
    if mask & PATH != 0 {
        match r.optional_text(0) {
            Some(path) => {
                builder.ephe_path(path);
            }
            None => {
                builder.default_ephe_path();
            }
        }
    }
    if mask & JPL != 0 {
        match r.optional_text(1) {
            Some(name) => {
                builder.jpl_file(name);
            }
            None => {
                builder.close_jpl_file();
            }
        }
    }
    if mask & TOPO != 0 {
        builder.topo(r.f(0), r.f(1), r.f(2));
    }
    if mask & SIDEREAL != 0 {
        builder.sid_mode(r.i(2), r.f(3), r.f(4));
    }
    if mask & TIDAL != 0 {
        builder.tid_acc(r.f(5));
    }
    if mask & DELTA != 0 {
        if r.i(3) != 0 {
            builder.clear_delta_t_override();
        } else {
            builder.delta_t_override(r.f(6));
        }
    }
    if mask & LAPSE != 0 {
        builder.lapse_rate(r.f(7));
    }
    if r.i(4) == 2 {
        builder = builder.clone();
    }
    Ok(match builder.build() {
        Ok(session) => {
            state.put(r.i(0), session)?;
            Record::default()
        }
        Err(error) => super::rust_worker::error(error),
    })
}

/// Invoke an actual Session method using the ordinary operation's independently
/// specified schema. Return public values/errors; no raw C/private binding call.
fn compute(r: &Request, state: &State) -> Result<Record, String> {
    if r.ints.len() < 2 {
        return Err("persistent session command shape".into());
    }
    let op = Op::from_wire(r.i(1) as u32).ok_or("persistent session operation")?;
    let input = Request {
        op: op as u32,
        ints: r.ints[2..].to_vec(),
        floats: r.floats.clone(),
        texts: r.texts.clone(),
        ..Request::default()
    };
    super::schema::validate(op, &input)?;
    let s = state.get(r.i(0))?;
    use Op::*;
    Ok(result(match op {
        Calc | CalcUt => {
            let position = if op == Calc {
                s.calc(input.f(0), input.i(0), input.i(1))
            } else {
                s.calc_ut(input.f(0), input.i(0), input.i(1))
            };
            position.map(|value| super::rust_worker::position(value, &mut None))
        }
        Houses => s
            .houses(input.f(0), input.f(1), input.f(2), input.i(0) as u8)
            .map(super::rust_worker::house),
        HousesEx => s
            .houses_ex(
                input.f(0),
                input.i(0),
                input.f(1),
                input.f(2),
                input.i(1) as u8,
            )
            .map(super::rust_worker::house),
        GetAyanamsaUt => s
            .get_ayanamsa_ut(input.f(0))
            .map(super::rust_worker::scalar),
        Deltat => s.deltat(input.f(0)).map(super::rust_worker::scalar),
        RiseTrans => s
            .rise_trans(
                input.f(0),
                input.i(0),
                input.optional_text(0),
                input.i(1),
                input.i(2),
                input.f(1),
                input.f(2),
                input.f(3),
                input.f(4),
                input.f(5),
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
        _ => return Err("unsupported persistent session method".into()),
    }))
}

/// Dispatch bounded ownership controls. Cloning shares the real Session's
/// history; separate builds do not. Drop never calls the binding's global close.
pub fn dispatch(r: &Request, state: &mut State) -> Result<Record, String> {
    match r.op {
        BUILD => build(r, state),
        COMPUTE => compute(r, state),
        CLONE if r.ints.len() == 2 && r.floats.is_empty() && r.texts.is_empty() => {
            let session = state.get(r.i(1))?.clone();
            state.put(r.i(0), session)?;
            Ok(Record::default())
        }
        DROP if r.ints.len() == 1 && r.floats.is_empty() && r.texts.is_empty() => {
            state.remove(r.i(0))?;
            Ok(Record::default())
        }
        _ => Err("persistent session control shape".into()),
    }
}
