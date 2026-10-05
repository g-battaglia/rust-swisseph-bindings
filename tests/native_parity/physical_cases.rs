//! Input-only star and observer recipes. Missing catalogs use the documented
//! legacy preflight; no hazardous extended lookup is sent directly to C.
use super::cases::{Case, one, rejected, step};
use super::registry::Op;
use swisseph_bindings::*;

/// File-free observer results, native absence/errors and safe boundary checks.
/// Positions following observer-taking calls also check their native side effect.
pub fn smoke() -> Vec<Case> {
    use Op::*;
    let jd = 2451545.0;
    let epoch = [jd];
    let mut cases = Vec::new();
    for op in [
        Fixstar,
        FixstarUt,
        Fixstar2,
        Fixstar2Ut,
        FixstarMag,
        Fixstar2Mag,
    ] {
        let magnitude = matches!(op, FixstarMag | Fixstar2Mag);
        cases.push(one(
            format!("STAR_MISSING_{}", op.entry().name),
            "stars",
            step(
                op,
                if magnitude { &[] } else { &[FLG_MOSEPH] },
                if magnitude { &[] } else { &epoch },
                &[Some("Sirius")],
            ),
        ));
        cases.push(one(
            format!("STAR_NUL_{}", op.entry().name),
            "stars",
            rejected(
                op,
                if magnitude { &[] } else { &[FLG_MOSEPH] },
                if magnitude { &[] } else { &epoch },
                &[Some("Sir\0ius")],
            ),
        ));
    }
    for op in [NodAps, NodApsUt] {
        for body in [MOON, MARS, JUPITER, 999] {
            for method in [
                NODBIT_MEAN,
                NODBIT_OSCU,
                NODBIT_OSCU_BAR,
                NODBIT_OSCU | NODBIT_FOPOINT,
            ] {
                cases.push(one(
                    format!("NODES_{}_{body}_{method}", op.entry().name),
                    "observer",
                    step(op, &[body, FLG_MOSEPH | FLG_SPEED, method], &[jd], &[]),
                ));
            }
        }
    }
    for frame in [ECL2HOR, EQU2HOR] {
        for (observer, latitude) in [0.0, 48.85, -33.9].into_iter().enumerate() {
            for (point, (a, b)) in [(0.0, 0.0), (200.0, 60.0), (359.99, -60.0)]
                .into_iter()
                .enumerate()
            {
                cases.push(one(
                    format!("HORIZONTAL_{frame}_{observer}_{point}"),
                    "observer",
                    step(
                        Azalt,
                        &[frame],
                        &[jd, 2.35, latitude, 1000.0, 1013.25, 15.0, a, b],
                        &[],
                    ),
                ));
                cases.push(one(
                    format!("CELESTIAL_{frame}_{observer}_{point}"),
                    "observer",
                    step(AzaltRev, &[frame], &[jd, 2.35, latitude, 1000.0, a, b], &[]),
                ));
            }
        }
    }
    for direction in [TRUE_TO_APP, APP_TO_TRUE] {
        for (index, altitude) in [-10.0, 0.0, 10.0, 90.0].into_iter().enumerate() {
            for (atmosphere, pressure) in [0.0, 1013.25].into_iter().enumerate() {
                cases.push(one(
                    format!("REFRAC_{direction}_{index}_{atmosphere}"),
                    "observer",
                    step(Refrac, &[direction], &[altitude, pressure, 15.0], &[]),
                ));
                for (height, elevation) in [0.0, 1000.0].into_iter().enumerate() {
                    cases.push(one(
                        format!("REFRAC_EXTENDED_{direction}_{index}_{atmosphere}_{height}"),
                        "observer",
                        step(
                            RefracExtended,
                            &[direction],
                            &[altitude, elevation, pressure, 15.0, 0.0065],
                            &[],
                        ),
                    ));
                }
            }
        }
        // Finite inputs causing non-finite native output are safe C calls;
        // the v1.0.0 public contract deliberately returns a Native error.
        cases.push(one(
            format!("REFRAC_NONFINITE_{direction}"),
            "observer",
            step(Refrac, &[direction], &[10.0, 1013.25, -273.0], &[]),
        ));
        cases.push(one(
            format!("REFRAC_DIP_NONFINITE_{direction}"),
            "observer",
            step(
                RefracExtended,
                &[direction],
                &[10.0, -430.0, 1013.25, 15.0, 0.0065],
                &[],
            ),
        ));
    }
    for op in [Pheno, PhenoUt, GetOrbitalElements, OrbitMaxMinTrueDistance] {
        for body in [SUN, MOON, MERCURY, MARS, JUPITER, SATURN, 999] {
            cases.push(one(
                format!("PHYSICAL_{}_{body}", op.entry().name),
                "observer",
                step(op, &[body, FLG_MOSEPH], &[jd], &[]),
            ));
        }
    }
    for op in [RiseTrans, RiseTransTrueHor] {
        for (observer, latitude) in [48.85, -33.9, 89.0].into_iter().enumerate() {
            for mode in [CALC_RISE, CALC_SET, CALC_MTRANSIT, CALC_ITRANSIT] {
                let mut floats = vec![jd, 2.35, latitude, 0.0, 1013.25, 15.0];
                if op == RiseTransTrueHor {
                    floats.push(1.0);
                }
                cases.push(Case {
                    key: format!("RISE_{}_{observer}_{mode}", op.entry().name),
                    family: "observer",
                    steps: vec![
                        step(op, &[SUN, FLG_MOSEPH, mode], &floats, &[None]),
                        step(
                            CalcUt,
                            &[MOON, FLG_MOSEPH | FLG_TOPOCTR | FLG_SPEED],
                            &[jd],
                            &[],
                        ),
                    ],
                });
            }
        }
    }
    for method in 0..=5 {
        cases.push(one(
            format!("GAUQUELIN_METHOD_{method}"),
            "observer",
            step(
                GauquelinSector,
                &[MARS, FLG_MOSEPH, method],
                &[jd, 2.35, 48.85, 0.0, 1013.25, 15.0],
                &[None],
            ),
        ));
    }
    cases.push(one(
        "OBSERVER_HEIGHT_REJECTED".into(),
        "observer",
        rejected(
            Azalt,
            &[EQU2HOR],
            &[jd, 2.35, 48.85, 1e100, 1013.25, 15.0, 200.0, 60.0],
            &[],
        ),
    ));
    cases
}

/// Explicit modern catalog success, rewritten names, unknown-query errors,
/// and all star output layouts. This manifest requires a selected data path.
pub fn catalog(data_path: &str) -> Vec<Case> {
    use Op::*;
    let mut cases = vec![Case {
        key: "CATALOG_CONFIGURATION".into(),
        family: "stars",
        steps: vec![step(SetEphePath, &[], &[], &[Some(data_path)])],
    }];
    for op in [
        Fixstar,
        FixstarUt,
        Fixstar2,
        Fixstar2Ut,
        FixstarMag,
        Fixstar2Mag,
    ] {
        let magnitude = matches!(op, FixstarMag | Fixstar2Mag);
        for (query, name) in [
            "Sirius",
            ",alCMa",
            "1",
            "",
            "__parity_unknown_star__",
            "Siri%",
            "Siri*",
        ]
        .into_iter()
        .enumerate()
        {
            for (recipe, flags) in [
                FLG_MOSEPH,
                FLG_MOSEPH | FLG_SPEED | FLG_EQUATORIAL,
                FLG_MOSEPH | FLG_XYZ,
                FLG_MOSEPH | FLG_RADIANS,
            ]
            .into_iter()
            .enumerate()
            .take(if magnitude { 1 } else { 4 })
            {
                let flag_argument = [flags];
                // Every filtered case carries its own path prerequisite.
                cases.push(Case {
                    key: format!("CATALOG_{}_{query}_{recipe}", op.entry().name),
                    family: "stars",
                    steps: vec![
                        step(SetEphePath, &[], &[], &[Some(data_path)]),
                        step(
                            op,
                            if magnitude { &[] } else { &flag_argument },
                            if magnitude { &[] } else { &[2451545.0] },
                            &[Some(name)],
                        ),
                    ],
                });
            }
        }
    }
    cases
}
