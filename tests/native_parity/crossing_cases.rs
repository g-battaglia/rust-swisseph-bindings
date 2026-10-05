//! Bounded, replayable search inputs. No event epochs from another calculation
//! are stored. Solar heliocentric searches are known hazards and Rust-only.
use super::cases::{Case, one, rejected, step};
use super::registry::Op;
use swisseph_bindings::*;

/// Exercise each ET/UT symbol, seams, both heliocentric directions, node
/// coordinates, supported source fallbacks and native failure/safety outcomes.
pub fn smoke() -> Vec<Case> {
    use Op::*;
    let mut cases = Vec::new();
    for op in [Solcross, SolcrossUt, Mooncross, MooncrossUt] {
        for (target, longitude) in [0.0, 359.99999].into_iter().enumerate() {
            cases.push(one(
                format!("CROSS_{}_{target}", op.entry().name),
                "crossings",
                step(op, &[FLG_MOSEPH], &[longitude, 2451545.0], &[]),
            ));
        }
        // A source/center combination with a defined, quick native failure.
        cases.push(one(
            format!("CROSS_NATIVE_ERROR_{}", op.entry().name),
            "crossings",
            step(op, &[FLG_MOSEPH | FLG_BARYCTR], &[0.0, 2451545.0], &[]),
        ));
    }
    for op in [MooncrossNode, MooncrossNodeUt] {
        cases.push(one(
            format!("NODE_CROSS_{}", op.entry().name),
            "crossings",
            step(op, &[FLG_MOSEPH], &[2451545.0], &[]),
        ));
        cases.push(one(
            format!("NODE_CROSS_ERROR_{}", op.entry().name),
            "crossings",
            step(op, &[FLG_MOSEPH | FLG_BARYCTR], &[2451545.0], &[]),
        ));
    }
    for op in [HelioCross, HelioCrossUt] {
        for direction in [-1, 1] {
            cases.push(one(
                format!("HELIO_CROSS_{}_{direction}", op.entry().name),
                "crossings",
                step(op, &[MARS, FLG_MOSEPH, direction], &[0.0, 2451545.0], &[]),
            ));
        }
        cases.push(one(
            format!("HELIO_CROSS_ERROR_{}", op.entry().name),
            "crossings",
            step(op, &[999, FLG_MOSEPH, 1], &[0.0, 2451545.0], &[]),
        ));
    }
    for op in [Solcross, SolcrossUt] {
        cases.push(Case {
            key: format!("SOLAR_HELIOCENTRIC_REJECTED_{}", op.entry().name),
            family: "crossings",
            steps: vec![
                rejected(op, &[FLG_MOSEPH | FLG_HELCTR], &[0.0, 2451545.0], &[]),
                step(Version, &[], &[], &[]),
                step(op, &[FLG_MOSEPH], &[0.0, 2451545.0], &[]),
            ],
        });
    }
    cases
}
