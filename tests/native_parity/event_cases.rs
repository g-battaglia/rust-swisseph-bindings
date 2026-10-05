//! Small bounded event recipes. Searches remain below the declared per-backend
//! cap; quiet instants and one-try misses are data, not omitted test branches.
use super::cases::{Case, one, rejected, step};
use super::registry::Op;
use swisseph_bindings::*;

/// Input-only global/local events and their unsafe Rust-only boundaries.
pub fn smoke() -> Vec<Case> {
    use Op::*;
    let mut cases = Vec::new();
    for op in [SolEclipseWhenGlob, LunEclipseWhen] {
        for backward in [0, 1] {
            cases.push(one(
                format!("ECLIPSE_GLOBAL_{}_{backward}", op.entry().name),
                "eclipses",
                step(op, &[FLG_MOSEPH, 0, backward], &[2451545.0], &[]),
            ));
        }
    }
    cases.push(one(
        "ECLIPSE_FILTER_NATIVE_ERROR".into(),
        "eclipses",
        step(
            SolEclipseWhenGlob,
            &[FLG_MOSEPH, ECL_CENTRAL | ECL_PARTIAL, 0],
            &[2451545.0],
            &[],
        ),
    ));
    for (instant, jd) in [2451545.0, 2460409.26].into_iter().enumerate() {
        cases.push(one(
            format!("SOLAR_GEOMETRY_{instant}"),
            "eclipses",
            step(SolEclipseWhere, &[FLG_MOSEPH], &[jd], &[]),
        ));
        cases.push(one(
            format!("SOLAR_CIRCUMSTANCES_{instant}"),
            "eclipses",
            step(SolEclipseHow, &[FLG_MOSEPH], &[jd, -96.8, 32.78, 0.0], &[]),
        ));
        for present in [0, 1] {
            cases.push(Case {
                key: format!("LUNAR_CIRCUMSTANCES_{instant}_{present}"),
                family: "eclipses",
                steps: vec![
                    step(SetTopo, &[], &[2.35, 48.85, 1000.0], &[]),
                    step(
                        LunEclipseHow,
                        &[FLG_MOSEPH, present],
                        &[jd, -96.8, 32.78, 0.0],
                        &[],
                    ),
                    step(CalcUt, &[MOON, FLG_MOSEPH | FLG_TOPOCTR], &[jd], &[]),
                ],
            });
        }
    }
    for op in [SolEclipseWhenLoc, LunEclipseWhenLoc] {
        for backward in [0, 1] {
            cases.push(one(
                format!("ECLIPSE_LOCAL_{}_{backward}", op.entry().name),
                "eclipses",
                step(
                    op,
                    &[FLG_MOSEPH, backward],
                    &[2451545.0, 0.0, 51.5, 0.0],
                    &[],
                ),
            ));
        }
    }
    for op in [
        LunOccultWhenGlob,
        LunOccultWhenGlobWithOptions,
        LunOccultWhenLoc,
        LunOccultWhenLocWithOptions,
    ] {
        let local = matches!(op, LunOccultWhenLoc | LunOccultWhenLocWithOptions);
        let options = matches!(
            op,
            LunOccultWhenGlobWithOptions | LunOccultWhenLocWithOptions
        );
        for backward in [0, 1] {
            for one_try in 0..=i32::from(options) {
                let mut ints = vec![VENUS, FLG_MOSEPH];
                if !local {
                    ints.push(0);
                }
                ints.push(backward);
                if options {
                    ints.push(one_try);
                }
                let mut floats = vec![2451545.0];
                if local {
                    floats.extend([0.0, 51.5, 0.0]);
                }
                cases.push(one(
                    format!("OCCULT_{}_{backward}_{one_try}", op.entry().name),
                    "occultations",
                    step(op, &ints, &floats, &[None]),
                ));
                ints[0] = MOON;
                cases.push(one(
                    format!(
                        "SELF_OCCULT_REJECTED_{}_{backward}_{one_try}",
                        op.entry().name
                    ),
                    "occultations",
                    rejected(op, &ints, &floats, &[Some("")]),
                ));
            }
        }
    }
    cases.push(one(
        "OCCULT_ONE_TRY_HIT_INPUT".into(),
        "occultations",
        step(
            LunOccultWhenGlobWithOptions,
            &[VENUS, FLG_MOSEPH, 0, 0, 1],
            &[2451600.0],
            &[None],
        ),
    ));
    cases.push(one(
        "OCCULT_GEOMETRY_QUIET".into(),
        "occultations",
        step(LunOccultWhere, &[VENUS, FLG_MOSEPH], &[2451545.0], &[None]),
    ));
    cases.push(one(
        "OCCULT_SELF_GEOMETRY_REJECTED".into(),
        "occultations",
        rejected(LunOccultWhere, &[MOON, FLG_MOSEPH], &[2451545.0], &[None]),
    ));

    let mut defaults = vec![2451545.0, 0.0, 51.5, 0.0];
    defaults.extend([0.0; 10]);
    for op in [HeliacalUt, HeliacalPhenoUt, VisLimitMag] {
        for (object, name) in ["Venus", "Sun", "__parity_unknown_object__"]
            .into_iter()
            .enumerate()
        {
            let ints = if op == VisLimitMag {
                vec![FLG_MOSEPH]
            } else {
                vec![HELIACAL_RISING, FLG_MOSEPH]
            };
            cases.push(one(
                format!("HELIACAL_{}_{object}", op.entry().name),
                "heliacal",
                step(op, &ints, &defaults, &[Some(name)]),
            ));
        }
    }
    for (instant, offset) in [0.0, 0.25, 0.5, 0.75].into_iter().enumerate() {
        let mut inputs = defaults.clone();
        inputs[0] += offset;
        cases.push(one(
            format!("VISIBILITY_DAY_{instant}"),
            "heliacal",
            step(VisLimitMag, &[FLG_MOSEPH], &inputs, &[Some("Venus")]),
        ));
    }
    for unsafe_flag in [
        HELFLAG_AVKIND_VR,
        HELFLAG_AVKIND_PTO,
        HELFLAG_AVKIND_MIN7,
        HELFLAG_AVKIND_MIN9,
    ] {
        cases.push(Case {
            key: format!("HELIACAL_UNSAFE_SEARCH_REJECTED_{unsafe_flag}"),
            family: "heliacal",
            steps: vec![
                rejected(
                    HeliacalUt,
                    &[HELIACAL_RISING, FLG_MOSEPH | unsafe_flag],
                    &defaults,
                    &[Some("Venus")],
                ),
                step(Version, &[], &[], &[]),
            ],
        });
    }
    cases
}
