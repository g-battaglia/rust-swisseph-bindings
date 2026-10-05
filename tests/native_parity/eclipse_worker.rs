//! Safe eclipse/occultation calls. Complete contact arrays, attributes and
//! getters are packed separately from the independent native slot projections.
use super::event_pack as pack;
use super::protocol::{Record, Request};
use super::registry::Op;
use swisseph_bindings::*;

/// Explicit direction/one-try bits, not inferred from a returned event.
fn search(r: &Request, backward: usize, one_try: usize) -> OccultSearchOptions {
    OccultSearchOptions {
        backward: r.i(backward) != 0,
        one_try: r.i(one_try) != 0,
    }
}

/// Dispatch exact shapes. Epochs/contacts are UT days; geography is degrees
/// east/north plus elevation in metres. Zero event types remain successful data.
pub fn call(op: Op, r: &Request) -> Result<Record, Error> {
    use Op::*;
    match op {
        SolEclipseWhenGlob => {
            sol_eclipse_when_glob(r.f(0), r.i(0), r.i(1), r.i(2) != 0).map(pack::solar_global)
        }
        LunEclipseWhen => {
            lun_eclipse_when(r.f(0), r.i(0), r.i(1), r.i(2) != 0).map(pack::lunar_global)
        }
        SolEclipseWhere => sol_eclipse_where(r.f(0), r.i(0)).map(pack::solar_geometry),
        SolEclipseHow => {
            sol_eclipse_how(r.f(0), r.i(0), r.f(1), r.f(2), r.f(3)).map(pack::solar_how)
        }
        LunEclipseHow => lun_eclipse_how(
            r.f(0),
            r.i(0),
            if r.i(1) == 0 {
                None
            } else {
                Some((r.f(1), r.f(2), r.f(3)))
            },
        )
        .map(pack::lunar_how),
        SolEclipseWhenLoc => {
            sol_eclipse_when_loc(r.f(0), r.i(0), r.f(1), r.f(2), r.f(3), r.i(1) != 0)
                .map(pack::solar_local)
        }
        LunEclipseWhenLoc => {
            lun_eclipse_when_loc(r.f(0), r.i(0), r.f(1), r.f(2), r.f(3), r.i(1) != 0)
                .map(pack::lunar_local)
        }
        LunOccultWhenGlob | LunOccultWhenGlobWithOptions => if op == LunOccultWhenGlob {
            lun_occult_when_glob(
                r.f(0),
                r.i(0),
                r.optional_text(0),
                r.i(1),
                r.i(2),
                r.i(3) != 0,
            )
        } else {
            lun_occult_when_glob_with_options(
                r.f(0),
                r.i(0),
                r.optional_text(0),
                r.i(1),
                r.i(2),
                search(r, 3, 4),
            )
        }
        .map(pack::occult_global),
        LunOccultWhenLoc | LunOccultWhenLocWithOptions => if op == LunOccultWhenLoc {
            lun_occult_when_loc(
                r.f(0),
                r.i(0),
                r.optional_text(0),
                r.i(1),
                r.f(1),
                r.f(2),
                r.f(3),
                r.i(2) != 0,
            )
        } else {
            lun_occult_when_loc_with_options(
                r.f(0),
                r.i(0),
                r.optional_text(0),
                r.i(1),
                r.f(1),
                r.f(2),
                r.f(3),
                search(r, 2, 3),
            )
        }
        .map(pack::occult_local),
        LunOccultWhere => {
            lun_occult_where(r.f(0), r.i(0), r.optional_text(0), r.i(1)).map(pack::occult_geometry)
        }
        _ => unreachable!("validated eclipse operation"),
    }
}
