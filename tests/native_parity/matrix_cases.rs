//! Version-one bounded extended matrix, measured as `matrix-pilot` until branch
//! acceptance is reconciled. Fixed integer generation selects only declared
//! input recipes, never arbitrary native flags/search targets. No astronomy,
//! expected vectors, residuals or upstream fixtures are generated here.
use super::cases::{Case, step};
use super::registry::Op;
use swisseph_bindings::*;

/// Replay seed for the owned xorshift64* input generator (not native results).
pub const SEED: u64 = 0x7377_6973_735f_7631;
/// Fixed generated position cap for the first measured extended matrix.
pub const POSITION_CASES: usize = 32_000;
/// Recipe schema version; changing inputs requires a new input identity and measurement.
/// Version 2 maps the nutation pseudo-body away from planet-centric requests
/// entirely (target or centre): native `swe_calc_pctr` recomputes `ECL_NUT`
/// internally and first-call positions/speeds differ from repeats, so
/// identical recipes cannot compare bit-exactly there.
pub const VERSION: u32 = 2;

/// Deterministic unsigned generator. Explicit wrapping makes replay independent
/// of debug overflow settings; only bounded integer selection feeds the cases.
/// The sweep profile reuses this generator with its own seed/version/cap.
pub struct Generator(u64);
impl Generator {
    /// Create a generator from an explicit seed; zero is never a stream state.
    pub fn seeded(seed: u64) -> Self {
        Self(if seed == 0 { 1 } else { seed })
    }
    /// One xorshift64* step; zero is never used as an initial state.
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    /// Uniform-recipe selector modulo an explicit nonzero finite cap.
    pub fn index(&mut self, cap: usize) -> usize {
        (self.next() % cap as u64) as usize
    }
}

/// Sweep soak seed: distinct from the matrix seed so the soak never replays
/// the accepted matrix prefix as new coverage.
pub const SWEEP_SEED: u64 = 0x0073_7765_705f_7331;
/// Fixed sweep position cap; `--samples` selects an explicit smaller prefix.
pub const SWEEP_CASES: usize = 8_000;
/// Sweep recipe version; changing inputs requires a new input identity.
pub const SWEEP_VERSION: u32 = 1;

/// Replayable sweep soak over valid dates/bodies/flag recipes/observers.
/// Cheap operations only: no searches, no generated event targets, no
/// arbitrary flag bits — every recipe uses the declared supported inputs.
/// Nutation planet-centric requests are excluded like in the matrix (native
/// history dependence); the mapping keeps the sweep reproducible.
pub fn sweep(paths: [&str; 4], samples: usize) -> Vec<Case> {
    use Op::*;
    let bodies = [
        SUN,
        MOON,
        MERCURY,
        VENUS,
        MARS,
        JUPITER,
        SATURN,
        URANUS,
        NEPTUNE,
        PLUTO,
        MEAN_NODE,
        TRUE_NODE,
        MEAN_APOG,
        OSCU_APOG,
        EARTH,
        CHIRON,
        CERES,
        VESTA,
        CUPIDO,
        HADES,
        AST_OFFSET + 1,
    ];
    let sources = [0, FLG_SWIEPH, FLG_MOSEPH];
    let recipes = flags();
    let observers: [[f64; 3]; 6] = [
        [2.35, 48.85, 50.0],
        [-0.12, 51.5, 25.0],
        [11.34, 48.14, 520.0],
        [151.21, -33.87, 50.0],
        [-74.0, 40.7, 10.0],
        [139.7, 35.7, 40.0],
    ];
    let mut rng = Generator::seeded(SWEEP_SEED);
    let mut cases = Vec::with_capacity(samples);
    for index in 0..samples {
        let directory = rng.index(paths.len());
        let source = sources[rng.index(sources.len())];
        let body = bodies[rng.index(bodies.len())];
        let recipe = rng.index(recipes.len());
        let mode = rng.index(47) as i32;
        let observer = observers[rng.index(observers.len())];
        // ±60 years about J2000, wider than the matrix but still modern.
        let jd = 2451545.0 + (rng.index(43_830_001) as f64 - 21_915_000.0) / 1000.0;
        let mut steps = setup(paths[directory], mode, 0.0, 0.0, observer);
        if index % 8 == 0 {
            let center = [SUN, EARTH, MARS][rng.index(3)];
            if body == ECL_NUT || center == ECL_NUT {
                steps.push(step(CalcUt, &[MARS, source | recipes[recipe]], &[jd], &[]));
            } else {
                steps.push(step(
                    CalcPctr,
                    &[body, center, source | recipes[recipe]],
                    &[jd],
                    &[],
                ));
            }
        } else {
            let op = if index % 2 == 0 { Calc } else { CalcUt };
            steps.push(step(op, &[body, source | recipes[recipe]], &[jd], &[]));
        }
        cases.push(Case {
            key: format!("SWEEP_V{SWEEP_VERSION}_{SWEEP_SEED:016x}_{index:05}"),
            family: "sweep-positions",
            steps,
        });
    }
    cases
}

/// Explicit supported position flag recipes. Native restrictions (e.g. Moshier
/// barycentre) remain compared errors, not silently removed or forced successes.
fn flags() -> [i32; 20] {
    [
        0,
        FLG_SPEED,
        FLG_SPEED3,
        FLG_SPEED | FLG_EQUATORIAL,
        FLG_SPEED | FLG_XYZ,
        FLG_SPEED | FLG_XYZ | FLG_EQUATORIAL,
        FLG_SPEED | FLG_RADIANS,
        FLG_SPEED | FLG_RADIANS | FLG_EQUATORIAL,
        FLG_TRUEPOS,
        FLG_NOGDEFL | FLG_NOABERR,
        FLG_SPEED | FLG_J2000,
        FLG_SPEED | FLG_NONUT,
        FLG_SPEED | FLG_ICRS,
        FLG_SPEED | FLG_HELCTR,
        FLG_SPEED | FLG_BARYCTR,
        FLG_SPEED | FLG_TOPOCTR,
        FLG_SPEED | FLG_SIDEREAL,
        FLG_SPEED | FLG_SIDEREAL | FLG_XYZ,
        FLG_SPEED | FLG_EQUATORIAL | FLG_J2000 | FLG_ICRS,
        FLG_SPEED | FLG_XYZ | FLG_HELCTR,
    ]
}

/// Every case declares all seven knobs so an exact case filter has the same
/// configuration as a combined run. All dates/heights are conservative safe
/// modern inputs. Path choices distinguish complete/empty/partial real files.
fn setup(
    path: &str,
    mode: i32,
    epoch: f64,
    offset: f64,
    topo: [f64; 3],
) -> Vec<super::cases::Step> {
    use Op::*;
    vec![
        step(SetEphePath, &[], &[], &[Some(path)]),
        step(SetJplFile, &[], &[], &[Some("__parity_missing_jpl__.eph")]),
        step(SetTopo, &[], &topo, &[]),
        step(SetSidMode, &[mode], &[epoch, offset], &[]),
        step(SetTidAcc, &[], &[SE_TIDAL_AUTOMATIC], &[]),
        step(SetDeltaTUserdef, &[0], &[0.0], &[]),
        step(SetLapseRate, &[], &[0.0065], &[]),
    ]
}

/// Thirty-two thousand replayable six-component position comparisons, independently
/// ET/UT and occasional planet-centred calls. Inputs include every standard body,
/// nutation, fictitious representatives, an asteroid alias and native missing-body
/// errors. Source selections are default/Swiss/Moshier/missing-JPL, never hidden.
pub fn positions(paths: [&str; 4], samples: usize) -> Vec<Case> {
    use Op::*;
    let bodies = [
        ECL_NUT,
        SUN,
        MOON,
        MERCURY,
        VENUS,
        MARS,
        JUPITER,
        SATURN,
        URANUS,
        NEPTUNE,
        PLUTO,
        MEAN_NODE,
        TRUE_NODE,
        MEAN_APOG,
        OSCU_APOG,
        EARTH,
        CHIRON,
        PHOLUS,
        CERES,
        PALLAS,
        JUNO,
        VESTA,
        INTP_APOG,
        INTP_PERG,
        CUPIDO,
        HADES,
        ZEUS,
        KRONOS,
        APOLLON,
        ADMETOS,
        VULKANUS,
        POSEIDON,
        AST_OFFSET + 1,
        999,
    ];
    let sources = [0, FLG_SWIEPH, FLG_MOSEPH, FLG_JPLEPH];
    let recipes = flags();
    let mut rng = Generator(SEED);
    let mut cases = Vec::with_capacity(samples);
    for index in 0..samples {
        let directory = rng.index(paths.len());
        let source = sources[rng.index(sources.len())];
        let body = bodies[rng.index(bodies.len())];
        let recipe = rng.index(recipes.len());
        let mode = rng.index(47) as i32;
        // ±30 years about J2000, preserving exact encoded input bits.
        let jd = 2451545.0 + (rng.index(21_915_001) as f64 - 10_957_500.0) / 1000.0;
        let longitude = rng.index(360_001) as f64 / 1000.0 - 180.0;
        let latitude = rng.index(132_001) as f64 / 1000.0 - 66.0;
        let altitude = rng.index(3631) as f64 - 430.0;
        let mut steps = setup(
            paths[directory],
            mode,
            0.0,
            0.0,
            [longitude, latitude, altitude],
        );
        if index % 16 == 0 {
            let center = [SUN, EARTH, MARS][rng.index(3)];
            // ECL_NUT is excluded from planet-centric requests entirely:
            // native history dependence (see VERSION) makes exact parity
            // unachievable there, as target or as centre. Every other
            // recipe stays bit-identical to version 1.
            if body == ECL_NUT || center == ECL_NUT {
                steps.push(step(CalcUt, &[MARS, source | recipes[recipe]], &[jd], &[]));
            } else {
                steps.push(step(
                    CalcPctr,
                    &[body, center, source | recipes[recipe]],
                    &[jd],
                    &[],
                ));
            }
        } else {
            let op = if index % 2 == 0 { Calc } else { CalcUt };
            steps.push(step(op, &[body, source | recipes[recipe]], &[jd], &[]));
        }
        cases.push(Case {
            key: format!("MATRIX_V{VERSION}_{SEED:016x}_{index:05}"),
            family: "matrix-positions",
            steps,
        });
    }
    cases
}

/// Every predefined sidereal mode plus separately parameterized USER ET/UT
/// options, scalar provenance APIs, positions and sidereal house outputs.
/// Experimental option bits are public-header input metadata, not new exports.
pub fn sidereal(path: &str) -> Vec<Case> {
    use Op::*;
    let mut cases = Vec::new();
    for mode in 0..47 {
        for option in [0, 256, 512, 2048, 4096 | 8192] {
            for (epoch, jd) in [2415020.0, 2451545.0, 2460310.5].into_iter().enumerate() {
                let mut steps = setup(path, mode | option, 0.0, 0.0, [2.35, 48.85, 50.0]);
                for op in [GetAyanamsa, GetAyanamsaUt] {
                    steps.push(step(op, &[], &[jd], &[]));
                }
                for op in [GetAyanamsaEx, GetAyanamsaExUt] {
                    steps.push(step(op, &[FLG_SWIEPH], &[jd], &[]));
                }
                steps.push(step(
                    CalcUt,
                    &[MARS, FLG_SWIEPH | FLG_SPEED | FLG_SIDEREAL],
                    &[jd],
                    &[],
                ));
                steps.push(step(
                    HousesEx2,
                    &[FLG_SIDEREAL, i32::from(b'P')],
                    &[jd, 48.85, 2.35],
                    &[],
                ));
                cases.push(Case {
                    key: format!("SIDEREAL_MODE_{mode}_{option}_{epoch}"),
                    family: "matrix-sidereal",
                    steps,
                });
            }
        }
    }
    for ut in [0, 1024] {
        for (epoch, reference) in [2415020.0, 2451545.0, 2460310.5].into_iter().enumerate() {
            for (shift, offset) in [-17.5, 0.0, 29.999999].into_iter().enumerate() {
                for pinned in [0, 1] {
                    let mut steps =
                        setup(path, SIDM_USER | ut, reference, offset, [2.35, 48.85, 50.0]);
                    steps.push(step(SetDeltaTUserdef, &[pinned], &[0.0008], &[]));
                    for op in [GetAyanamsa, GetAyanamsaUt] {
                        steps.push(step(op, &[], &[2451545.0], &[]));
                    }
                    for op in [GetAyanamsaEx, GetAyanamsaExUt] {
                        steps.push(step(op, &[FLG_SWIEPH], &[2451545.0], &[]));
                    }
                    steps.push(step(
                        CalcUt,
                        &[MOON, FLG_SWIEPH | FLG_SIDEREAL | FLG_SPEED],
                        &[2451545.0],
                        &[],
                    ));
                    steps.push(step(
                        HousesEx,
                        &[FLG_SIDEREAL, i32::from(b'P')],
                        &[2451545.0, 48.85, 2.35],
                        &[],
                    ));
                    cases.push(Case {
                        key: format!("SIDEREAL_USER_{ut}_{epoch}_{shift}_{pinned}"),
                        family: "matrix-sidereal",
                        steps,
                    });
                }
            }
        }
    }
    cases
}

/// Every admitted ASCII letter/lowercase native fallback and broad standard
/// systems, including polar errors and all ARMC/Sunshine/sector output widths.
pub fn houses(path: &str) -> Vec<Case> {
    use Op::*;
    let mut cases = Vec::new();
    for system in (b'A'..=b'Z')
        .chain(b'a'..=b'z')
        .filter(|letter| !matches!(letter, b'G' | b'g'))
    {
        for (place, latitude) in [-89.9, -51.5, 0.0, 51.5, 89.9].into_iter().enumerate() {
            for (seam, longitude) in [-180.0, -0.0, 0.0, 180.0].into_iter().enumerate() {
                let mut steps = setup(
                    path,
                    SIDM_FAGAN_BRADLEY,
                    0.0,
                    0.0,
                    [longitude, latitude, 0.0],
                );
                for op in [Houses, HousesEx, HousesEx2] {
                    let ints = if op == Houses {
                        vec![i32::from(system)]
                    } else {
                        vec![0, i32::from(system)]
                    };
                    steps.push(step(op, &ints, &[2451545.0, latitude, longitude], &[]));
                }
                steps.push(step(
                    HousesArmc,
                    &[i32::from(system)],
                    &[123.0, latitude, 23.44],
                    &[],
                ));
                steps.push(step(
                    HousesArmcEx2,
                    &[i32::from(system)],
                    &[123.0, latitude, 23.44, -10.0],
                    &[],
                ));
                steps.push(step(
                    HousePos,
                    &[i32::from(system)],
                    &[123.0, latitude, 23.44, 359.999999, -2.0],
                    &[],
                ));
                cases.push(Case {
                    key: format!("HOUSE_ASCII_{system}_{place}_{seam}"),
                    family: "matrix-houses",
                    steps,
                });
            }
        }
    }
    for (epoch, jd) in [2415020.0, 2451545.0, 2460310.5].into_iter().enumerate() {
        for latitude in [-51.5, 0.0, 51.5] {
            for frame in [0, FLG_RADIANS, FLG_SIDEREAL] {
                cases.push(Case {
                    key: format!("HOUSE_SECTORS_{epoch}_{latitude}_{frame}"),
                    family: "matrix-houses",
                    steps: setup(path, SIDM_LAHIRI, 0.0, 0.0, [0.0, latitude, 0.0])
                        .into_iter()
                        .chain([step(HousesGauquelin, &[frame], &[jd, latitude, 0.0], &[])])
                        .collect(),
                });
            }
        }
    }
    cases
}

/// Small independent synthetic replay test; no native fixture or generated
/// expected result is used. Repeated seeds must give identical integer streams.
pub fn self_test() -> Result<(), String> {
    let mut first = Generator(SEED);
    let mut second = Generator(SEED);
    for _ in 0..1024 {
        if first.next() != second.next() {
            return Err("matrix PRNG replay".into());
        }
    }
    if first.index(47) >= 47 {
        return Err("matrix PRNG bound".into());
    }
    Ok(())
}
