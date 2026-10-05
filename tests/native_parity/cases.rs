//! Owned input recipes only: no native output vectors or residuals.
//!
//! Cheap matrices use known-safe calendars, bodies and flag recipes. Dangerous
//! conversion/search inputs are explicitly Rust-only steps, never C requests.
use super::protocol::Request;
use super::registry::Op;
use swisseph_bindings::*;

/// J2000 is an input epoch, not a captured reference result.
const J2000: f64 = 2451545.0;

/// Meaning of a step; safety rejections cannot accidentally invoke the oracle.
#[derive(Clone, Copy)]
pub enum Expectation {
    /// Compare the complete public result to the direct native result.
    Native,
    /// Call only the safe Rust boundary and require InvalidInput.
    Rejected,
    /// Validate each executable's own pathname, not cross-process equality.
    ExecutablePath,
    /// Rust-only source policy on the preceding C-checked real position.
    /// Native primary/component evidence must match the recipe first.
    SourcePolicy {
        accepted: bool,
        primary: i32,
        moshier_component: bool,
    },
}

/// One input step in a sequenced, process-global state recipe.
pub struct Step {
    /// Canonical public operation, also identifying its C symbol/projector.
    pub op: Op,
    /// Explicit arguments in the independently documented wire schema.
    pub request: Request,
    /// Native comparison or a named safe-boundary/process exception.
    pub expectation: Expectation,
}

/// Input-only case with a stable, filterable identifier and stateful steps.
pub struct Case {
    /// Reproduction key; it contains no derived native value.
    pub key: String,
    /// Family used by targeted local runs.
    pub family: &'static str,
    /// Ordered operations; both workers retain the same effective native state.
    pub steps: Vec<Step>,
}

/// Build an explicitly typed transport step; schema validation precedes use.
pub(super) fn step(op: Op, ints: &[i32], floats: &[f64], texts: &[Option<&str>]) -> Step {
    Step {
        op,
        request: Request {
            op: op as u32,
            ints: ints.to_vec(),
            floats: floats.to_vec(),
            texts: texts
                .iter()
                .map(|t| t.map(|t| t.as_bytes().to_vec()))
                .collect(),
            ..Request::default()
        },
        expectation: Expectation::Native,
    }
}
/// A hazardous step belongs only to the safe API, not to native parity.
pub(super) fn rejected(op: Op, ints: &[i32], floats: &[f64], texts: &[Option<&str>]) -> Step {
    Step {
        expectation: Expectation::Rejected,
        ..step(op, ints, floats, texts)
    }
}
/// Create a one-call matrix case with an input-recipe key.
pub(super) fn one(key: String, family: &'static str, value: Step) -> Case {
    Case {
        key,
        family,
        steps: vec![value],
    }
}

/// Data-independent foundation manifest. Additional families remain in the
/// coverage ledger until their independent callers and cases are verified.
pub fn smoke(empty_path: &str) -> Vec<Case> {
    use Op::*;
    let mut cases = vec![Case {
        key: "NATIVE_IDENTITY".into(),
        family: "identity",
        steps: vec![
            step(Version, &[], &[], &[]),
            Step {
                expectation: Expectation::ExecutablePath,
                ..step(LibraryPath, &[], &[], &[])
            },
        ],
    }];
    // A non-empty explicit empty path prevents checkout/catalog discovery.
    cases.push(Case {
        key: "EMPTY_DATA_CONFIGURATION".into(),
        family: "configuration",
        steps: vec![step(SetEphePath, &[], &[], &[Some(empty_path)])],
    });
    for body in [SUN, MOON, MARS, CUPIDO, CERES, AST_OFFSET + 1, 999] {
        cases.push(one(
            format!("BODY_NAME_{body}"),
            "identity",
            step(GetPlanetName, &[body], &[], &[]),
        ));
    }
    for mode in 0..47 {
        cases.push(one(
            format!("SIDEREAL_NAME_{mode}"),
            "identity",
            step(GetAyanamsaName, &[mode], &[], &[]),
        ));
    }
    cases.push(one(
        "SIDEREAL_NAME_ABSENT".into(),
        "identity",
        step(GetAyanamsaName, &[SIDM_USER], &[], &[]),
    ));
    for system in *b"PKORCEWIGpkwZ" {
        cases.push(one(
            format!("HOUSE_NAME_{system}"),
            "identity",
            step(HouseName, &[i32::from(system)], &[], &[]),
        ));
    }

    for calendar in [JUL_CAL, GREG_CAL] {
        for (year, month, day) in [
            (-4712, 1, 1),
            (-1, 12, 31),
            (0, 1, 1),
            (1582, 10, 15),
            (1900, 2, 28),
            (2000, 2, 29),
            (2024, 12, 31),
        ] {
            for (index, hour) in [0.0, 12.0, 23.99999].into_iter().enumerate() {
                cases.push(one(
                    format!("JULDAY_{calendar}_{year}_{month}_{day}_{index}"),
                    "time",
                    step(Julday, &[year, month, day, calendar], &[hour], &[]),
                ));
            }
        }
        for (index, jd) in [-1_000_000.5, -0.5, 0.0, J2000, J2000 + 0.99999]
            .into_iter()
            .enumerate()
        {
            cases.push(one(
                format!("REVJUL_{calendar}_{index}"),
                "time",
                step(Revjul, &[calendar], &[jd], &[]),
            ));
            for op in [JdetToUtc, Jdut1ToUtc] {
                cases.push(one(
                    format!("UTC_INVERSE_{}_{calendar}_{index}", op.entry().name),
                    "time",
                    step(op, &[calendar], &[jd], &[]),
                ));
            }
        }
        for (index, (year, month, day, hour, minute, second)) in [
            (1900, 1, 1, 12, 0, 0.5),
            (2000, 1, 1, 12, 0, 0.0),
            (2016, 12, 31, 23, 59, 59.5),
            (2016, 12, 31, 23, 59, 60.5),
            (2017, 1, 1, 0, 0, 0.0),
            (2000, 2, 30, 12, 0, 0.0),
        ]
        .into_iter()
        .enumerate()
        {
            cases.push(one(
                format!("UTC_CALENDAR_{calendar}_{index}"),
                "time",
                step(
                    UtcToJd,
                    &[year, month, day, hour, minute, calendar],
                    &[second],
                    &[],
                ),
            ));
            cases.push(one(
                format!("DATE_CONVERSION_{calendar}_{index}"),
                "time",
                step(DateConversion, &[year, month, day, calendar], &[12.25], &[]),
            ));
        }
    }
    for (index, zone) in [-12.0, -5.5, 0.0, 5.75, 14.0].into_iter().enumerate() {
        cases.push(one(
            format!("TIMEZONE_{index}"),
            "time",
            step(UtcTimeZone, &[2000, 1, 1, 0, 15], &[0.5, zone], &[]),
        ));
    }
    for (index, jd) in [2415020.0, J2000, 2460310.5].into_iter().enumerate() {
        for op in [DayOfWeek, Deltat, Sidtime, TimeEqu] {
            cases.push(one(
                format!("TIME_SCALAR_{}_{index}", op.entry().name),
                "time",
                step(op, &[], &[jd], &[]),
            ));
        }
        for flags in [FLG_MOSEPH, FLG_SWIEPH] {
            cases.push(one(
                format!("DELTAT_SOURCE_{index}_{flags}"),
                "time",
                step(DeltatEx, &[flags], &[jd], &[]),
            ));
        }
        cases.push(one(
            format!("SIDTIME_FRAME_{index}"),
            "time",
            step(Sidtime0, &[], &[jd, 23.44, 0.001], &[]),
        ));
        for op in [LmtToLat, LatToLmt] {
            cases.push(one(
                format!("LOCAL_TIME_{}_{index}", op.entry().name),
                "time",
                step(op, &[], &[jd, 12.5], &[]),
            ));
        }
    }

    // 504 positions: 12 epochs × 7 bodies × 3 output recipes × UT/ET.
    // These are comparisons with each corresponding native symbol, not an
    // assertion that UT and ET with the same numeric label should agree.
    for epoch in 0..12 {
        let jd = J2000 + (epoch as f64 - 6.0) * 365.25;
        for body in [SUN, MOON, MERCURY, VENUS, MARS, JUPITER, TRUE_NODE] {
            for recipe in [0, FLG_SPEED, FLG_SPEED | FLG_EQUATORIAL | FLG_XYZ] {
                for op in [Calc, CalcUt] {
                    cases.push(one(
                        format!("POSITION_{}_{epoch}_{body}_{recipe}", op.entry().name),
                        "positions",
                        step(op, &[body, FLG_MOSEPH | recipe], &[jd], &[]),
                    ));
                }
            }
        }
    }
    cases.push(Case {
        key: "POSITION_NATIVE_OUTCOMES".into(),
        family: "positions",
        steps: vec![
            step(Calc, &[SUN, FLG_MOSEPH | FLG_HELCTR], &[J2000], &[]),
            step(CalcUt, &[999, FLG_MOSEPH], &[J2000], &[]),
            step(Calc, &[MARS, FLG_MOSEPH | FLG_BARYCTR], &[J2000], &[]),
            step(CalcPctr, &[MOON, MARS, FLG_MOSEPH], &[J2000], &[]),
            step(CalcUt, &[SUN, FLG_SWIEPH | FLG_SPEED], &[J2000], &[]),
            step(
                SetJplFile,
                &[],
                &[],
                &[Some("missing-native-parity-jpl.eph")],
            ),
            step(CalcUt, &[SUN, FLG_JPLEPH | FLG_SPEED], &[J2000], &[]),
            step(SetJplFile, &[], &[], &[Some("")]),
        ],
    });
    cases.push(Case {
        key: "CONFIGURATION_STATE_AND_REJECTION".into(),
        family: "configuration",
        steps: vec![
            step(SetTopo, &[], &[12.5, 41.9, 50.0], &[]),
            step(
                CalcUt,
                &[MOON, FLG_MOSEPH | FLG_TOPOCTR | FLG_SPEED],
                &[J2000],
                &[],
            ),
            rejected(SetTopo, &[], &[f64::NAN, 0.0, 0.0], &[]),
            step(
                CalcUt,
                &[MOON, FLG_MOSEPH | FLG_TOPOCTR | FLG_SPEED],
                &[J2000],
                &[],
            ),
            step(SetSidMode, &[SIDM_USER], &[J2000, 24.0], &[]),
            rejected(SetSidMode, &[SIDM_USER], &[J2000, f64::NAN], &[]),
            step(GetAyanamsa, &[], &[J2000], &[]),
            step(GetAyanamsaUt, &[], &[J2000], &[]),
            step(GetAyanamsaEx, &[FLG_MOSEPH], &[J2000], &[]),
            step(GetAyanamsaExUt, &[FLG_MOSEPH], &[J2000], &[]),
            step(SetTidAcc, &[], &[-26.0], &[]),
            step(GetTidAcc, &[], &[], &[]),
            step(SetDeltaTUserdef, &[1], &[0.0008], &[]),
            step(Deltat, &[], &[J2000], &[]),
            step(CalcUt, &[SUN, FLG_MOSEPH | FLG_SIDEREAL], &[J2000], &[]),
            step(SetDeltaTUserdef, &[0], &[0.0], &[]),
            step(SetLapseRate, &[], &[0.0065], &[]),
            step(SetTopo, &[], &[0.0, 0.0, 0.0], &[]),
            step(SetSidMode, &[SIDM_FAGAN_BRADLEY], &[0.0, 0.0], &[]),
            step(SetTidAcc, &[], &[SE_TIDAL_AUTOMATIC], &[]),
            step(Close, &[], &[], &[]),
            step(SetEphePath, &[], &[], &[Some(empty_path)]),
            step(CalcUt, &[SUN, FLG_MOSEPH], &[J2000], &[]),
        ],
    });
    for slot in [-1, 0, 1, 2, 3, 4, 5] {
        cases.push(one(
            format!("EMPTY_FILE_SLOT_{slot}"),
            "configuration",
            step(GetCurrentFileData, &[slot], &[], &[]),
        ));
    }

    for (index, latitude) in [-89.9, -51.5, 0.0, 51.5, 89.9].into_iter().enumerate() {
        for system in *b"PKORCEWI" {
            let system = i32::from(system);
            for op in [Houses, HousesEx, HousesEx2] {
                let ints = if op == Houses {
                    vec![system]
                } else {
                    vec![0, system]
                };
                cases.push(one(
                    format!("HOUSES_{}_{index}_{system}", op.entry().name),
                    "houses",
                    step(op, &ints, &[J2000, latitude, -0.12], &[]),
                ));
            }
        }
    }
    for system in *b"PKORI" {
        let system = i32::from(system);
        cases.push(one(
            format!("ARMC_{system}"),
            "houses",
            step(HousesArmc, &[system], &[123.0, 51.5, 23.44], &[]),
        ));
        cases.push(one(
            format!("ARMC_SPEEDS_{system}"),
            "houses",
            step(HousesArmcEx2, &[system], &[123.0, 51.5, 23.44, -10.0], &[]),
        ));
        cases.push(one(
            format!("HOUSE_POINT_{system}"),
            "houses",
            step(HousePos, &[system], &[123.0, 51.5, 23.44, 45.0, 2.0], &[]),
        ));
    }
    cases.push(one(
        "GAUQUELIN_WIDE_BUFFERS".into(),
        "houses",
        step(HousesGauquelin, &[0], &[J2000, 51.5, -0.12], &[]),
    ));

    for (index, angle) in [
        -720.0,
        -180.0,
        -0.0,
        0.0,
        359.9999999,
        360.0,
        720.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ]
    .into_iter()
    .enumerate()
    {
        for op in [Degnorm, Radnorm] {
            cases.push(one(
                format!("NORMALIZE_{}_{index}", op.entry().name),
                "angles",
                step(op, &[], &[angle], &[]),
            ));
        }
        for op in [Difdegn, Difdeg2n, Difrad2n, DegMidp, RadMidp] {
            cases.push(one(
                format!("ANGLE_PAIR_{}_{index}", op.entry().name),
                "angles",
                step(op, &[], &[angle, 10.0], &[]),
            ));
        }
    }
    for (index, angle) in [-1.5, -0.5, -0.0, 0.0, 0.5, 1.5, 2147483647.0, -2147483647.0]
        .into_iter()
        .enumerate()
    {
        cases.push(one(
            format!("ROUND_INTEGER_{index}"),
            "angles",
            step(D2l, &[], &[angle], &[]),
        ));
    }
    for (index, angle) in [0.0, 45.0, 359.9].into_iter().enumerate() {
        cases.push(one(
            format!("ROTATION_{index}"),
            "angles",
            step(Cotrans, &[], &[angle, 10.0, 1.5, 23.44], &[]),
        ));
        cases.push(one(
            format!("ROTATION_SPEEDS_{index}"),
            "angles",
            step(
                CotransSp,
                &[],
                &[angle, 10.0, 1.5, -1.0, 0.01, -0.002, -23.44],
                &[],
            ),
        ));
    }
    for cs in [
        -129600001,
        -64800000,
        -1,
        0,
        1,
        10799950,
        129600000,
        i32::MAX - 100,
    ] {
        for op in [Csnorm, Csroundsec, Cs2degstr] {
            cases.push(one(
                format!("CENTISECONDS_{}_{cs}", op.entry().name),
                "angles",
                step(op, &[cs], &[], &[]),
            ));
        }
        for op in [Difcsn, Difcs2n] {
            cases.push(one(
                format!("CENTISECOND_PAIR_{}_{cs}", op.entry().name),
                "angles",
                step(op, &[cs, 0], &[], &[]),
            ));
        }
        cases.push(one(
            format!("TIME_FORMAT_{cs}"),
            "angles",
            step(Cs2timestr, &[cs, i32::from(b':'), 0], &[], &[]),
        ));
        cases.push(one(
            format!("DIRECTION_FORMAT_{cs}"),
            "angles",
            step(
                Cs2lonlatstr,
                &[cs, i32::from(b'E'), i32::from(b'W')],
                &[],
                &[],
            ),
        ));
    }
    for (index, angle) in [-360.0, -29.999999, -0.0, 0.0, 29.999999, 360.0]
        .into_iter()
        .enumerate()
    {
        for flags in [
            0,
            SPLIT_DEG_ROUND_SEC,
            SPLIT_DEG_ZODIACAL,
            SPLIT_DEG_NAKSHATRA,
            SPLIT_DEG_ROUND_DEG | SPLIT_DEG_KEEP_DEG,
        ] {
            cases.push(one(
                format!("SPLIT_{index}_{flags}"),
                "angles",
                step(SplitDeg, &[flags], &[angle], &[]),
            ));
        }
    }
    cases.push(Case {
        key: "SAFE_FFI_ARITHMETIC_REGRESSIONS".into(),
        family: "safety",
        steps: vec![
            rejected(Revjul, &[GREG_CAL], &[f64::NAN], &[]),
            rejected(SplitDeg, &[0], &[1e100], &[]),
            rejected(Difcsn, &[i32::MAX, i32::MIN], &[], &[]),
            rejected(Csroundsec, &[i32::MAX], &[], &[]),
            rejected(
                Cs2lonlatstr,
                &[i32::MIN, i32::from(b'E'), i32::from(b'W')],
                &[],
                &[],
            ),
            rejected(SetEphePath, &[], &[], &[Some("invalid\0path")]),
            rejected(CalcUt, &[SUN, FLG_MOSEPH], &[f64::NAN], &[]),
            step(Version, &[], &[], &[]),
            step(CalcUt, &[SUN, FLG_MOSEPH], &[J2000], &[]),
        ],
    });
    for calendar in [JUL_CAL, GREG_CAL] {
        cases.push(Case {
            key: format!("SAFE_UTC_DIAGNOSTIC_SECONDS_{calendar}"),
            family: "safety",
            steps: [f64::MAX, -f64::MAX, 1e100, -1.0, 61.0]
                .into_iter()
                .map(|second| rejected(UtcToJd, &[2000, 1, 1, 12, 0, calendar], &[second], &[]))
                .chain(std::iter::once(step(
                    UtcToJd,
                    &[2000, 1, 1, 12, 0, calendar],
                    &[0.0],
                    &[],
                )))
                .collect(),
        });
    }
    cases.push(Case {
        key: "SESSION_BASELINE_SETUP".into(),
        family: "sessions",
        steps: vec![step(SetEphePath, &[], &[], &[Some(empty_path)])],
    });
    cases.extend(super::physical_cases::smoke());
    cases.extend(super::crossing_cases::smoke());
    cases.extend(super::event_cases::smoke());
    cases
}
