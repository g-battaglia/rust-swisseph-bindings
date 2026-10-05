//! Integration checks: horizontal coordinates/refraction  and
//! phenomena/orbital data.
//!
//! Native configuration is process-global, so every state-dependent step
//! runs inside the single sequenced `stateful_physical` test. Standalone
//! tests use pure-Rust input validation or explicit, state-independent
//! refraction calls. Assertions are structural (ranges, round trips, method effects,
//! error classification, same-engine consistency), never stored reference
//! vectors.

use swisseph_bindings::{
    APP_TO_TRUE, ECL2HOR, EQU2HOR, ErrorKind, FLG_MOSEPH, HOR2ECL, HOR2EQU, MARS, MOON, SUN,
    TRUE_TO_APP, azalt, azalt_rev, get_orbital_elements, orbit_max_min_true_distance, pheno,
    pheno_ut, refrac, refrac_extended, set_ephe_path, set_lapse_rate,
};

/// J2000.0 as a Julian Day label.
const J2000: f64 = 2451545.0;
/// Rome observer (east-positive longitude, north latitude, sea level).
const ROME_LON: f64 = 12.5;
const ROME_LAT: f64 = 41.9;
const SEA_LEVEL: f64 = 0.0;
/// Standard atmosphere for refraction.
const PRESSURE: f64 = 1013.25;
const TEMP: f64 = 15.0;
const LAPSE: f64 = 0.0065;

#[test]
fn physical_constants_match_pinned_header() {
    assert_eq!(ECL2HOR, 0);
    assert_eq!(EQU2HOR, 1);
    assert_eq!(HOR2ECL, 0);
    assert_eq!(HOR2EQU, 1);
    assert_eq!(TRUE_TO_APP, 0);
    assert_eq!(APP_TO_TRUE, 1);
}

#[test]
fn horizontal_rejects_non_finite_input() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for result in [
            azalt(
                bad, EQU2HOR, ROME_LON, ROME_LAT, SEA_LEVEL, PRESSURE, TEMP, 200.0, 60.0,
            )
            .map(|_| ()),
            azalt(
                J2000, EQU2HOR, bad, ROME_LAT, SEA_LEVEL, PRESSURE, TEMP, 200.0, 60.0,
            )
            .map(|_| ()),
            azalt(
                J2000, EQU2HOR, ROME_LON, ROME_LAT, SEA_LEVEL, PRESSURE, TEMP, bad, 60.0,
            )
            .map(|_| ()),
            azalt_rev(bad, HOR2EQU, ROME_LON, ROME_LAT, SEA_LEVEL, 140.0, 30.0).map(|_| ()),
            azalt_rev(J2000, HOR2EQU, ROME_LON, ROME_LAT, SEA_LEVEL, bad, 30.0).map(|_| ()),
        ] {
            let err = result.expect_err("non-finite input must be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
        }
    }
}

#[test]
fn refraction_rejects_non_finite_input() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for result in [
            refrac(bad, PRESSURE, TEMP, TRUE_TO_APP).map(|_| ()),
            refrac(10.0, bad, TEMP, TRUE_TO_APP).map(|_| ()),
            refrac_extended(10.0, SEA_LEVEL, PRESSURE, TEMP, bad, TRUE_TO_APP).map(|_| ()),
            set_lapse_rate(bad).map(|_| ()),
        ] {
            let err = result.expect_err("non-finite input must be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
        }
    }
}

#[test]
fn refraction_classifies_non_finite_native_output() {
    let err = refrac(10.0, PRESSURE, -273.0, TRUE_TO_APP)
        .expect_err("finite inputs must not expose a non-finite converted altitude");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(err.message().contains("refrac:"));
    assert!(err.message().contains("non-finite"));

    // Include a below-sea-level observer with ordinary atmosphere: only
    // the native dip is non-finite, so checking the scalar return is not enough.
    for (height, pressure, temperature, lapse) in [
        (-1000.0, PRESSURE, TEMP, LAPSE),
        (SEA_LEVEL, PRESSURE, -273.15, LAPSE),
        (SEA_LEVEL, PRESSURE, -273.0, LAPSE),
        (SEA_LEVEL, PRESSURE, -300.0, LAPSE),
        (1000.0, PRESSURE, TEMP, 1e100),
        (1000.0, 1e308, TEMP, LAPSE),
    ] {
        for direction in [TRUE_TO_APP, APP_TO_TRUE] {
            let err = refrac_extended(10.0, height, pressure, temperature, lapse, direction)
                .expect_err("no non-finite native detail may become a successful result");
            assert_eq!(err.kind(), ErrorKind::Native);
            assert!(err.message().contains("refrac_extended:"));
            assert!(err.message().contains("non-finite"));
        }
    }

    // Errors do not poison native access; retain zero-pressure identity
    // and all finite details from normal calls without clamping or rewriting.
    for direction in [TRUE_TO_APP, APP_TO_TRUE] {
        assert_eq!(refrac(10.0, 0.0, -273.0, direction).unwrap(), 10.0);
        assert!(refrac(10.0, PRESSURE, TEMP, direction).unwrap().is_finite());
        for height in [SEA_LEVEL, 1000.0] {
            let result = refrac_extended(10.0, height, PRESSURE, TEMP, LAPSE, direction).unwrap();
            assert!(
                [
                    result.converted,
                    result.true_altitude,
                    result.apparent_altitude,
                    result.refraction,
                    result.dip,
                ]
                .iter()
                .all(|value| value.is_finite())
            );
        }
    }
}

#[test]
fn physical_rejects_non_finite_time() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for result in [
            pheno(bad, MARS, FLG_MOSEPH).map(|_| ()),
            pheno_ut(bad, MARS, FLG_MOSEPH).map(|_| ()),
            get_orbital_elements(bad, MARS, FLG_MOSEPH).map(|_| ()),
            orbit_max_min_true_distance(bad, MARS, FLG_MOSEPH).map(|_| ()),
        ] {
            let err = result.expect_err("non-finite date must be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
        }
    }
}

#[test]
fn stateful_physical() {
    // Moshier-only sequencing: no data files needed, so no path setup.
    // Every step below shares the process-global native state in order.

    // --- azalt: equatorial frame structural checks ---
    let h = azalt(
        J2000, EQU2HOR, ROME_LON, ROME_LAT, SEA_LEVEL, PRESSURE, TEMP, 200.0, 60.0,
    )
    .expect("equatorial azalt must succeed");
    assert!((0.0..360.0).contains(&h.azimuth), "az={}", h.azimuth);
    assert!(
        (-90.0..=90.0).contains(&h.true_altitude),
        "true={}",
        h.true_altitude
    );
    // Refraction lifts the apparent altitude for objects above the horizon.
    assert!(
        h.apparent_altitude >= h.true_altitude,
        "app={} true={}",
        h.apparent_altitude,
        h.true_altitude
    );

    // --- azalt: ecliptic frame differs from equatorial for same numbers ---
    let he = azalt(
        J2000, ECL2HOR, ROME_LON, ROME_LAT, SEA_LEVEL, PRESSURE, TEMP, 200.0, 60.0,
    )
    .expect("ecliptic azalt must succeed");
    assert!(
        (he.azimuth - h.azimuth).abs() > 1.0,
        "frames must differ: equ={} ecl={}",
        h.azimuth,
        he.azimuth
    );

    // --- azalt_rev round trip reproduces the input pair ---
    let c = azalt_rev(
        J2000,
        HOR2EQU,
        ROME_LON,
        ROME_LAT,
        SEA_LEVEL,
        h.azimuth,
        h.true_altitude,
    )
    .expect("reverse conversion must succeed");
    assert!(
        (c.longitude_or_ra - 200.0).abs() < 1e-6,
        "ra={}",
        c.longitude_or_ra
    );
    assert!(
        (c.latitude_or_dec - 60.0).abs() < 1e-6,
        "dec={}",
        c.latitude_or_dec
    );

    // --- azalt_rev ecliptic output differs from equatorial ---
    let ce = azalt_rev(
        J2000,
        HOR2ECL,
        ROME_LON,
        ROME_LAT,
        SEA_LEVEL,
        h.azimuth,
        h.true_altitude,
    )
    .expect("ecliptic reverse must succeed");
    assert!(
        (ce.longitude_or_ra - c.longitude_or_ra).abs() > 1.0,
        "frames must differ"
    );

    // --- refrac: forward correction band + zero-pressure identity ---
    let app = refrac(10.0, PRESSURE, TEMP, TRUE_TO_APP).expect("refrac must succeed");
    assert!((app - 10.0888).abs() < 1e-3, "refraction at 10 deg: {app}");
    let unchanged = refrac(10.0, 0.0, TEMP, TRUE_TO_APP).expect("zero pressure must succeed");
    assert_eq!(unchanged, 10.0, "zero pressure disables refraction");
    // Reverse direction lowers the altitude back.
    let back = refrac(app, PRESSURE, TEMP, APP_TO_TRUE).expect("reverse must succeed");
    assert!(
        (back - 10.0).abs() < 0.01,
        "forward/reverse near-inverse: {back}"
    );

    // --- refrac_extended: sea-level details + elevated dip ---
    let r = refrac_extended(10.0, SEA_LEVEL, PRESSURE, TEMP, LAPSE, TRUE_TO_APP)
        .expect("extended refrac must succeed");
    assert!((r.refraction - 0.0867).abs() < 1e-3, "ref={}", r.refraction);
    assert_eq!(r.dip, 0.0, "no dip at sea level");
    assert_eq!(r.true_altitude, 10.0);
    assert!(
        (r.converted - r.apparent_altitude).abs() < 1e-12,
        "forward return is the apparent altitude"
    );
    let hi = refrac_extended(10.0, 1000.0, PRESSURE, TEMP, LAPSE, TRUE_TO_APP)
        .expect("elevated refrac must succeed");
    assert!(hi.dip < -0.8 && hi.dip > -1.0, "dip at 1000 m: {}", hi.dip);
    // Lapse rate feeds the dip: a steeper lapse gives a less negative dip.
    let steep = refrac_extended(10.0, 1000.0, PRESSURE, TEMP, 0.0100, TRUE_TO_APP)
        .expect("steep lapse must succeed");
    assert!(
        steep.dip > hi.dip,
        "lapse effect: {} vs {}",
        steep.dip,
        hi.dip
    );

    // --- set_lapse_rate: accepted, finite-only, state survives ---
    set_lapse_rate(0.0100).expect("lapse override must succeed");
    set_lapse_rate(LAPSE).expect("lapse restore must succeed");
    // Conversions still work after touching the global.
    let _ = azalt(
        J2000, EQU2HOR, ROME_LON, ROME_LAT, SEA_LEVEL, PRESSURE, TEMP, 200.0, 60.0,
    )
    .expect("azalt after lapse restore must succeed");

    // --- pheno: Mars structural bands + flag provenance ---
    let ph = pheno(J2000, MARS, FLG_MOSEPH).expect("mars pheno must succeed");
    assert!(
        (0.0..180.0).contains(&ph.phase_angle()),
        "phase={}",
        ph.phase_angle()
    );
    assert!(
        (0.0..=1.0).contains(&ph.illuminated_fraction()),
        "illum={}",
        ph.illuminated_fraction()
    );
    assert!((0.0..180.0).contains(&ph.elongation()));
    assert!(
        ph.apparent_diameter() > 0.0,
        "diam={}",
        ph.apparent_diameter()
    );
    assert_eq!(ph.returned_flags & FLG_MOSEPH, FLG_MOSEPH);
    assert_eq!(ph.values[5..], [0.0; 15], "reserved tail is zero");

    // --- pheno_ut agrees with pheno up to the Delta-T epoch shift ---
    let phu = pheno_ut(J2000, MARS, FLG_MOSEPH).expect("mars pheno_ut must succeed");
    assert_eq!(phu.returned_flags, ph.returned_flags);
    assert!(
        (phu.phase_angle() - ph.phase_angle()).abs() < 0.01,
        "UT/ET split: {} vs {}",
        phu.phase_angle(),
        ph.phase_angle()
    );
    // Repeat calls are exactly reproducible.
    let repeat = pheno(J2000, MARS, FLG_MOSEPH).expect("repeat must succeed");
    assert_eq!(ph.values, repeat.values);

    // --- pheno: Sun reports the native degenerate row verbatim ---
    let sun = pheno(J2000, SUN, FLG_MOSEPH).expect("sun pheno must succeed");
    assert_eq!(sun.phase_angle(), 0.0);
    assert_eq!(sun.elongation(), 0.0);
    assert!(sun.magnitude() < -20.0, "sun mag={}", sun.magnitude());

    // --- pheno: unknown body fails natively with diagnostic ---
    let err = pheno(J2000, 999, FLG_MOSEPH).expect_err("body 999 must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(!err.to_string().is_empty(), "diagnostic kept");

    // --- orbital elements: Mars bands + reserved tail ---
    let el = get_orbital_elements(J2000, MARS, FLG_MOSEPH).expect("mars elements must succeed");
    assert!(
        (el.semi_major_axis_au() - 1.524).abs() < 0.01,
        "a={}",
        el.semi_major_axis_au()
    );
    assert!(
        (el.eccentricity() - 0.093).abs() < 0.01,
        "e={}",
        el.eccentricity()
    );
    assert!(
        (el.inclination_deg() - 1.85).abs() < 0.05,
        "i={}",
        el.inclination_deg()
    );
    assert!(
        el.perihelion_distance_au() < el.aphelion_distance_au(),
        "q={} Q={}",
        el.perihelion_distance_au(),
        el.aphelion_distance_au()
    );
    assert_eq!(el.values[17..], [0.0; 33], "reserved tail is zero");
    assert!(el.diagnostic.is_empty());

    // --- orbital elements: Moon is geocentric, Sun is a native error ---
    let moon = get_orbital_elements(J2000, MOON, FLG_MOSEPH).expect("moon elements must succeed");
    assert!(
        moon.semi_major_axis_au() < 0.01,
        "geocentric moon a={}",
        moon.semi_major_axis_au()
    );
    let err = get_orbital_elements(J2000, SUN, FLG_MOSEPH).expect_err("sun has no elements");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.to_string().contains("not valid"),
        "native verdict kept: {err}"
    );

    // --- distance extremes: ordering + current inside the range ---
    let d =
        orbit_max_min_true_distance(J2000, MARS, FLG_MOSEPH).expect("mars extremes must succeed");
    assert!(
        d.min_distance < d.true_distance && d.true_distance < d.max_distance,
        "min={} true={} max={}",
        d.min_distance,
        d.true_distance,
        d.max_distance
    );
    assert!(d.diagnostic.is_empty());
    // Repeat calls are exactly reproducible.
    let d2 = orbit_max_min_true_distance(J2000, MARS, FLG_MOSEPH).expect("repeat must succeed");
    assert_eq!(
        (d.max_distance, d.min_distance, d.true_distance),
        (d2.max_distance, d2.min_distance, d2.true_distance)
    );

    // --- extremes: unknown body fails natively ---
    let err = orbit_max_min_true_distance(J2000, 999, FLG_MOSEPH).expect_err("body 999 must fail");
    assert_eq!(err.kind(), ErrorKind::Native);

    // Restore the shipped data path for later suites that expect it.
    set_ephe_path(Some("swisseph/ephe")).expect("path restore must succeed");
}
