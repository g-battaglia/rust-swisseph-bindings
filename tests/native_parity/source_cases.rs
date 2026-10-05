//! Selected Swiss/Moshier/JPL and partial-file source recipes. All policy
//! checks consume a genuine public Position already compared completely with C.
//! Real JPL coverage needs `SWISSEPH_JPL_DIR` pointing at an explicitly selected
//! directory with MD5-verified `de440.eph`; without it the JPL-success
//! recipes report BLOCKED (no cases) unless `--require-jpl` hard-fails.
use super::cases::{Case, Expectation, Step, step};
use super::registry::Op;
use swisseph_bindings::*;

/// JPL archive selected for genuine `FLG_JPLEPH` coverage. DE440 is the
/// selected MD5-verified file with modern coverage.
const JPL_FILE: &str = "de440.eph";
/// JPL coverage window: inside DE440-LE440 (1849–2150) with margin, so file
/// bounds never turn success recipes into out-of-range Native errors.
const JPL_DATES: [f64; 4] = [2415020.0, 2451545.0, 2460000.5, 2500000.0];

/// One helper expectation with independently specified native evidence.
fn policy(allowed: i32, accepted: bool, primary: i32, moshier_component: bool) -> Step {
    Step {
        expectation: Expectation::SourcePolicy {
            accepted,
            primary,
            moshier_component,
        },
        ..step(Op::RequireSourceFlags, &[allowed], &[], &[])
    }
}

/// Source/fallback pilot. Directories are owned or explicitly selected inputs;
/// no native output vector is used to construct an expected result.
/// Without `SWISSEPH_JPL_DIR` the JPL-success block is BLOCKED (no cases)
/// unless `require_jpl` hard-fails. Data comes only from the
/// explicit directory selected with `SWISSEPH_JPL_DIR`.
pub fn pilot(
    data: &str,
    empty: &str,
    planet_only: &str,
    moon_only: &str,
    require_jpl: bool,
) -> Result<Vec<Case>, String> {
    use Op::*;
    let jd = 2451545.0;
    let mut cases = Vec::new();
    for (directory, path) in [
        ("complete", data),
        ("empty", empty),
        ("planet_only", planet_only),
        ("moon_only", moon_only),
    ] {
        for body in [SUN, MOON, MERCURY, MARS, JUPITER, CERES] {
            for op in [Calc, CalcUt] {
                cases.push(Case {
                    key: format!("SOURCE_{directory}_{}_{body}", op.entry().name),
                    family: "sources",
                    steps: vec![
                        step(SetEphePath, &[], &[], &[Some(path)]),
                        step(op, &[body, FLG_SWIEPH | FLG_SPEED], &[jd], &[]),
                        step(GetCurrentFileData, &[0], &[], &[]),
                        step(GetCurrentFileData, &[1], &[], &[]),
                        step(GetCurrentFileData, &[2], &[], &[]),
                    ],
                });
            }
        }
    }
    for (body, center) in [(MARS, SUN), (MERCURY, EARTH), (MOON, MARS)] {
        for frame in [0, FLG_XYZ | FLG_EQUATORIAL] {
            cases.push(Case {
                key: format!("SOURCE_PCTR_SWISS_{body}_{center}_{frame}"),
                family: "sources",
                steps: vec![
                    step(SetEphePath, &[], &[], &[Some(data)]),
                    step(
                        CalcPctr,
                        &[body, center, FLG_SWIEPH | FLG_SPEED | frame],
                        &[jd],
                        &[],
                    ),
                    step(GetCurrentFileData, &[0], &[], &[]),
                    step(GetCurrentFileData, &[1], &[], &[]),
                ],
            });
        }
    }
    cases.push(Case {
        key: "SOURCE_SWISS_STRICT_SUCCESS".into(),
        family: "sources",
        steps: vec![
            step(SetEphePath, &[], &[], &[Some(data)]),
            step(CalcUt, &[MARS, FLG_SWIEPH | FLG_SPEED], &[jd], &[]),
            policy(FLG_SWIEPH, true, FLG_SWIEPH, false),
            policy(0, true, FLG_SWIEPH, false),
            policy(FLG_MOSEPH, false, FLG_SWIEPH, false),
        ],
    });
    cases.push(Case {
        key: "SOURCE_COMPONENT_FALLBACK".into(),
        family: "sources",
        steps: vec![
            step(SetEphePath, &[], &[], &[Some(planet_only)]),
            step(CalcUt, &[MARS, FLG_SWIEPH | FLG_SPEED], &[jd], &[]),
            policy(FLG_SWIEPH, false, FLG_SWIEPH, true),
            policy(FLG_SWIEPH | FLG_MOSEPH, true, FLG_SWIEPH, true),
            policy(0, true, FLG_SWIEPH, true),
        ],
    });
    cases.push(Case {
        key: "SOURCE_MOSHIER_FALLBACK".into(),
        family: "sources",
        steps: vec![
            step(SetEphePath, &[], &[], &[Some(empty)]),
            step(CalcUt, &[MARS, FLG_SWIEPH | FLG_SPEED], &[jd], &[]),
            policy(FLG_SWIEPH, false, FLG_MOSEPH, false),
            policy(FLG_MOSEPH, true, FLG_MOSEPH, false),
            policy(FLG_SWIEPH | FLG_MOSEPH, true, FLG_MOSEPH, false),
        ],
    });
    for (directory, path, primary) in [("complete", data, FLG_SWIEPH), ("empty", empty, FLG_MOSEPH)]
    {
        cases.push(Case {
            key: format!("SOURCE_JPL_MISSING_{directory}"),
            family: "sources",
            steps: vec![
                step(SetEphePath, &[], &[], &[Some(path)]),
                step(SetJplFile, &[], &[], &[Some("__parity_missing_jpl__.eph")]),
                step(CalcUt, &[MARS, FLG_JPLEPH | FLG_SPEED], &[jd], &[]),
                policy(FLG_JPLEPH, false, primary, false),
                policy(FLG_SWIEPH | FLG_MOSEPH, true, primary, false),
            ],
        });
    }
    // Swiss-only regressions must precede the optional-JPL return below.
    // Former matrix seed/index 2112: ECL_NUT in a planet-centric request has
    // no history-independent native result (the engine recomputes nutation
    // internally; first-call positions/speeds differ from repeats whether it
    // appears as target or centre). The matrix no longer generates such
    // requests. The no-speed target neighbour is stable and stays parity.
    cases.push(Case {
        key: "SOURCE_PCTR_NUTATION_HISTORY_DEPENDENCE".into(),
        family: "sources",
        steps: vec![
            step(SetEphePath, &[], &[], &[Some(data)]),
            step(SetJplFile, &[], &[], &[Some("__parity_missing_jpl__.eph")]),
            step(SetTopo, &[], &[-93.052, 40.040000000000006, 2467.0], &[]),
            step(SetSidMode, &[17], &[0.0, 0.0], &[]),
            step(SetTidAcc, &[], &[SE_TIDAL_AUTOMATIC], &[]),
            step(SetDeltaTUserdef, &[0], &[0.0], &[]),
            step(SetLapseRate, &[], &[0.0065], &[]),
            // Without speeds the planet-centric pseudo-body result is stable
            // and compares bit-exactly against the independent C caller.
            step(CalcPctr, &[ECL_NUT, MARS, FLG_TOPOCTR], &[2443912.658], &[]),
            step(CalcPctr, &[ECL_NUT, MARS, FLG_TOPOCTR], &[2443912.658], &[]),
        ],
    });
    // Genuine JPL success: the only recipes that select a real JPL file.
    // The directory must come from SWISSEPH_JPL_DIR (never the checkout).
    // Absence is BLOCKED, not fallback double-coverage, unless --require-jpl.
    let jpl = match std::env::var("SWISSEPH_JPL_DIR") {
        Ok(dir) => dir,
        Err(_) if require_jpl => {
            return Err("real-JPL recipes require an explicit SWISSEPH_JPL_DIR".into());
        }
        Err(_) => {
            println!(
                "BLOCKED JPL-success: set SWISSEPH_JPL_DIR to MD5-verified de440.eph (or pass --require-jpl to fail)"
            );
            return Ok(cases);
        }
    };
    let jpl = std::path::Path::new(&jpl)
        .canonicalize()
        .map_err(|e| format!("JPL data directory: {e}"))?;
    let archive = jpl.join(JPL_FILE);
    // MD5 is the checksum the mirror publishes for this archive; it guards
    // the download here, never a native result.
    let digest = std::process::Command::new("md5")
        .arg(&archive)
        .output()
        .map_err(|e| format!("JPL archive {JPL_FILE}: {e}"))?;
    let digest = String::from_utf8(digest.stdout).map_err(|e| e.to_string())?;
    let digest = digest.split('=').nth(1).ok_or("JPL md5 layout")?.trim();
    if digest != "8a1c6e63ce2b0ab4716e8c07e45f7d5a" {
        return Err(format!("JPL archive {JPL_FILE} digest mismatch"));
    }
    let jpl = jpl.to_str().ok_or("JPL path encoding")?;
    for date in JPL_DATES {
        for body in [
            SUN, MOON, MERCURY, VENUS, MARS, JUPITER, SATURN, TRUE_NODE, MEAN_APOG,
        ] {
            for op in [Calc, CalcUt] {
                cases.push(Case {
                    key: format!("SOURCE_JPL_{}_{body}_{date}", op.entry().name),
                    family: "sources",
                    steps: vec![
                        step(SetEphePath, &[], &[], &[Some(jpl)]),
                        step(SetJplFile, &[], &[], &[Some(JPL_FILE)]),
                        step(op, &[body, FLG_JPLEPH | FLG_SPEED], &[date], &[]),
                        policy(FLG_JPLEPH, true, FLG_JPLEPH, false),
                        policy(0, true, FLG_JPLEPH, false),
                        policy(FLG_SWIEPH, false, FLG_JPLEPH, false),
                    ],
                });
            }
        }
    }
    // JPL planet-centric success plus strict-source gating on a JPL result.
    for (body, center) in [(MOON, MARS), (MARS, SUN)] {
        cases.push(Case {
            key: format!("SOURCE_JPL_PCTR_{body}_{center}"),
            family: "sources",
            steps: vec![
                step(SetEphePath, &[], &[], &[Some(jpl)]),
                step(SetJplFile, &[], &[], &[Some(JPL_FILE)]),
                step(
                    CalcPctr,
                    &[body, center, FLG_JPLEPH | FLG_SPEED],
                    &[2451545.0],
                    &[],
                ),
                policy(FLG_JPLEPH, true, FLG_JPLEPH, false),
            ],
        });
    }
    Ok(cases)
}
