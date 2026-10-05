//! Rust-only hazardous Session requests and retained-snapshot checks. The
//! independent C caller never receives these non-finite/out-of-domain inputs.
use super::protocol::{Record, Request};
use super::registry::Op;
use super::rust_worker::INVALID_INPUT;
use super::session_probe::State;
use swisseph_bindings::*;

/// Private rejection recipe; its input epoch is never a native C request.
pub const REJECT: u32 = 2002;

/// Check every computation method's actual safe error and return its public
/// classification/message, not successful or uninitialized result components.
pub fn run(r: &Request, state: &State) -> Result<Record, String> {
    if r.ints.len() != 2 || r.floats.len() != 1 || !r.texts.is_empty() {
        return Err("session rejection shape".into());
    }
    let op = Op::from_wire(r.i(1) as u32).ok_or("session rejection operation")?;
    let session = state.get(r.i(0))?;
    let jd = r.f(0);
    let result = match op {
        Op::Calc => session.calc(jd, MOON, FLG_MOSEPH).map(|_| ()),
        Op::CalcUt => session.calc_ut(jd, MOON, FLG_MOSEPH).map(|_| ()),
        Op::Houses => session.houses(jd, 48.85, 2.35, b'P').map(|_| ()),
        Op::HousesEx => session.houses_ex(jd, 0, 48.85, 2.35, b'P').map(|_| ()),
        Op::GetAyanamsaUt => session.get_ayanamsa_ut(jd).map(|_| ()),
        Op::Deltat => session.deltat(jd).map(|_| ()),
        Op::RiseTrans => session
            .rise_trans(
                jd, SUN, None, FLG_MOSEPH, CALC_RISE, 12.5, 41.9, 50.0, 1013.25, 15.0,
            )
            .map(|_| ()),
        _ => return Err("unsupported session rejection operation".into()),
    };
    let error = result
        .err()
        .ok_or("unsafe session recipe unexpectedly accepted")?;
    if error.kind() != ErrorKind::InvalidInput {
        return Err("session rejection classification".into());
    }
    Ok(Record {
        code: INVALID_INPUT,
        texts: vec![error.message().as_bytes().to_vec()],
        ..Record::default()
    })
}
