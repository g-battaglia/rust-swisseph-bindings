//! All eight safe crossing entry points, with their own ET/UT scales.
//! The native engine supplies the epochs and coordinates; no search/solver is
//! implemented here. Invalid/nonterminating paths are rejected in Rust only.
use super::protocol::{Record, Request};
use super::registry::Op;
use swisseph_bindings::*;

/// Preserve a crossing's exact time and complete diagnostic.
fn crossing(value: LongitudeCrossing) -> Record {
    Record {
        floats: vec![value.time],
        texts: vec![value.diagnostic.into_bytes()],
        ..Record::default()
    }
}

/// Copy all node components in time/longitude/latitude order.
fn node(value: NodeCrossing) -> Record {
    Record {
        floats: vec![value.time, value.longitude, value.latitude],
        texts: vec![value.diagnostic.into_bytes()],
        ..Record::default()
    }
}

/// Dispatch validated crossing inputs; targets are degrees and times are days.
pub fn call(op: Op, r: &Request) -> Result<Record, Error> {
    use Op::*;
    match op {
        Solcross => solcross(r.f(0), r.f(1), r.i(0)).map(crossing),
        SolcrossUt => solcross_ut(r.f(0), r.f(1), r.i(0)).map(crossing),
        Mooncross => mooncross(r.f(0), r.f(1), r.i(0)).map(crossing),
        MooncrossUt => mooncross_ut(r.f(0), r.f(1), r.i(0)).map(crossing),
        MooncrossNode => mooncross_node(r.f(0), r.i(0)).map(node),
        MooncrossNodeUt => mooncross_node_ut(r.f(0), r.i(0)).map(node),
        HelioCross => helio_cross(r.i(0), r.f(0), r.f(1), r.i(1), r.i(2)).map(crossing),
        HelioCrossUt => helio_cross_ut(r.i(0), r.f(0), r.f(1), r.i(1), r.i(2)).map(crossing),
        _ => unreachable!("validated crossing operation"),
    }
}
