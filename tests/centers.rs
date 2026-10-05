//! Integration checks: planet-centric positions, the center/flag matrix
//! for `calc`, and the remaining model configuration (`set_jpl_file`,
//! tidal acceleration, user Delta T).
//!
//! Native configuration is process-global, so every state-mutating step
//! runs inside the single sequenced `stateful_centers_and_models` test.
//! Standalone tests use Ephemeris-Time `calc`/`calc_pctr` with the Moshier
//! model only, which depends on no shared configuration. Assertions are
//! structural (ranges, flag provenance, error classification, same-engine
//! consistency), never stored reference vectors.

use swisseph_bindings::{
    Calendar, ErrorKind, FLG_ASTROMETRIC, FLG_BARYCTR, FLG_EQUATORIAL, FLG_HELCTR, FLG_J2000,
    FLG_JPLEPH, FLG_MOSEPH, FLG_NOABERR, FLG_NOGDEFL, FLG_NONUT, FLG_RADIANS, FLG_SPEED,
    FLG_SPEED3, FLG_SWIEPH, FLG_TRUEPOS, FLG_XYZ, MARS, MOON, SE_TIDAL_AUTOMATIC, SE_TIDAL_DE421,
    SE_TIDAL_DE431, SE_TIDAL_DEFAULT, SUN, calc, calc_pctr, deltat, deltat_ex, get_tid_acc, julday,
    set_delta_t_userdef, set_ephe_path, set_jpl_file, set_tid_acc,
};

#[path = "data_dir.rs"]
mod test_data;

/// J2000.0 as an Ephemeris-Time label (TT noon, no UT conversion inside `calc`).
const J2000_ET: f64 = 2451545.0;

#[test]
fn pctr_rejects_unknown_bodies() {
    let err = calc_pctr(J2000_ET, 999_999, MARS, FLG_MOSEPH).expect_err("unknown target must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(!err.message().is_empty(), "native diagnostic must be kept");
    let err = calc_pctr(J2000_ET, MOON, 999_999, FLG_MOSEPH).expect_err("unknown center must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
}

#[test]
fn moshier_barycentric_gap_is_a_native_error() {
    // The native engine does not support barycentric Moshier positions,
    // which covers planet-centric `calc_pctr` too. The binding preserves
    // this capability gap as a classified native error with the engine
    // diagnostic, never as invented data.
    let err = calc_pctr(J2000_ET, MOON, MARS, FLG_MOSEPH | FLG_SPEED)
        .expect_err("moshier pctr must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(!err.message().is_empty(), "native diagnostic must be kept");
    let err = calc(J2000_ET, SUN, FLG_MOSEPH | FLG_SPEED | FLG_BARYCTR)
        .expect_err("barycentric moshier must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(!err.message().is_empty(), "native diagnostic must be kept");
}

#[test]
fn heliocentric_sun_is_valid_zero_data() {
    // The Sun seen from the Sun is the origin: a valid zero position must
    // come back as data (`Ok`), never as an error.
    let sun = calc(J2000_ET, SUN, FLG_MOSEPH | FLG_HELCTR | FLG_SPEED).expect("helio sun");
    assert!(
        sun.distance().abs() < 0.01,
        "helio sun distance, got {}",
        sun.distance()
    );
    for value in sun.values {
        assert!(value.is_finite(), "non-finite component in {sun:?}");
    }
}

#[test]
fn equatorial_output_uses_ra_dec_ranges() {
    let sun = calc(J2000_ET, SUN, FLG_MOSEPH | FLG_SPEED | FLG_EQUATORIAL).expect("equatorial sun");
    let (ra, dec) = (sun.values[0], sun.values[1]);
    assert!((0.0..360.0).contains(&ra), "right ascension, got {ra}");
    assert!(dec.abs() <= 30.0, "declination, got {dec}");
}

#[test]
fn cartesian_norm_matches_polar_distance() {
    let polar = calc(J2000_ET, SUN, FLG_MOSEPH | FLG_SPEED).expect("polar sun");
    let xyz = calc(J2000_ET, SUN, FLG_MOSEPH | FLG_SPEED | FLG_XYZ).expect("cartesian sun");
    let (x, y, z) = (xyz.values[0], xyz.values[1], xyz.values[2]);
    let norm = (x * x + y * y + z * z).sqrt();
    assert!(
        (norm - polar.distance()).abs() < 1e-9,
        "cartesian norm {norm} vs polar distance {}",
        polar.distance()
    );
}

#[test]
fn radians_output_scales_degrees() {
    let deg = calc(J2000_ET, SUN, FLG_MOSEPH | FLG_SPEED).expect("degree sun");
    let rad = calc(J2000_ET, SUN, FLG_MOSEPH | FLG_SPEED | FLG_RADIANS).expect("radian sun");
    let factor = std::f64::consts::PI / 180.0;
    assert!(
        (rad.values[0] - deg.values[0] * factor).abs() < 1e-12,
        "radian/degree mismatch: {} vs {}",
        rad.values[0],
        deg.values[0]
    );
}

#[test]
fn speeds_are_zero_without_speed_flags() {
    // The wrapper zeroes its buffer before the call, so components the
    // engine does not write stay an explicit zero, never garbage.
    let slow = calc(J2000_ET, SUN, FLG_MOSEPH).expect("sun without speeds");
    assert_eq!(slow.values[3..6], [0.0, 0.0, 0.0]);
    let fast = calc(J2000_ET, SUN, FLG_MOSEPH | FLG_SPEED).expect("sun with speeds");
    assert!(
        fast.values[3] > 0.9 && fast.values[3] < 1.1,
        "sun daily motion, got {}",
        fast.values[3]
    );
}

#[test]
fn astrometric_flag_aliases_its_components() {
    // `FLG_ASTROMETRIC` is defined as `FLG_NOABERR | FLG_NOGDEFL`; both
    // spellings must take the same native code path bit-for-bit.
    let aliased = calc(J2000_ET, SUN, FLG_MOSEPH | FLG_SPEED | FLG_ASTROMETRIC).expect("aliased");
    let explicit = calc(
        J2000_ET,
        SUN,
        FLG_MOSEPH | FLG_SPEED | FLG_NOABERR | FLG_NOGDEFL,
    )
    .expect("explicit");
    assert_eq!(aliased.values, explicit.values);
    assert_eq!(aliased.returned_flags, explicit.returned_flags);
}

#[test]
fn geometric_and_frame_variants_succeed() {
    // True geometric, J2000-equinox and no-nutation requests must all be
    // accepted with finite output; exact offsets are engine internals,
    // not binding assertions.
    for flags in [
        FLG_MOSEPH | FLG_SPEED | FLG_TRUEPOS,
        FLG_MOSEPH | FLG_SPEED | FLG_J2000,
        FLG_MOSEPH | FLG_SPEED | FLG_NONUT,
    ] {
        let pos = calc(J2000_ET, SUN, flags).expect("frame variant");
        assert!(pos.values.iter().all(|v| v.is_finite()), "flags {flags:#X}");
    }
    // Barycentric Moshier output is a native capability gap (see
    // `moshier_barycentric_gap_is_a_native_error`); the file-based check
    // lives in the stateful test below.
}

#[test]
fn three_point_speeds_agree_with_default_speeds() {
    let precise = calc(J2000_ET, SUN, FLG_MOSEPH | FLG_SPEED).expect("precise speeds");
    let approx = calc(J2000_ET, SUN, FLG_MOSEPH | FLG_SPEED3).expect("3-point speeds");
    assert!(
        (approx.values[3] - precise.values[3]).abs() < 0.01,
        "3-point {} vs precise {}",
        approx.values[3],
        precise.values[3]
    );
}

#[test]
fn jpl_and_config_validation_never_reaches_native() {
    let nul = set_jpl_file("bad\0name.eph").expect_err("NUL file name");
    assert_eq!(nul.kind(), ErrorKind::InvalidInput);
    let long = set_jpl_file(&"a".repeat(256)).expect_err("overlong file name");
    assert_eq!(long.kind(), ErrorKind::InvalidInput);
    let nan = set_tid_acc(f64::NAN).expect_err("NaN tidal acceleration");
    assert_eq!(nan.kind(), ErrorKind::InvalidInput);
    let inf = set_tid_acc(f64::INFINITY).expect_err("infinite tidal acceleration");
    assert_eq!(inf.kind(), ErrorKind::InvalidInput);
    let dt = set_delta_t_userdef(Some(f64::NAN)).expect_err("NaN Delta T");
    assert_eq!(dt.kind(), ErrorKind::InvalidInput);
}

/// Sequenced stateful checks: JPL selection, tidal acceleration and the
/// user Delta-T override. Order matters because native state is
/// process-global; everything is restored before the test ends.
#[test]
fn stateful_centers_and_models() {
    // File-based checks use an explicit external data directory (else the
    // checkout data). Without data only section 1 is skipped; the
    // fallback/tidal/Delta-T steps below still run.
    let data_dir = test_data::ephemeris_dir();

    // 1. Planet-centric provenance with data files: the returned flags must
    // name the file source, with structural output to match.
    if let Some(ref data_dir) = data_dir {
        set_ephe_path(Some(data_dir)).expect("point at shipped data");
        let moon = calc_pctr(J2000_ET, MOON, MARS, FLG_SWIEPH | FLG_SPEED).expect("pctr via files");
        assert!(
            moon.returned_flags & FLG_SWIEPH != 0,
            "expected file-based source flag, got {:#X} ({})",
            moon.returned_flags,
            moon.diagnostic
        );
        assert!((0.0..360.0).contains(&moon.longitude()));
        assert!(
            moon.distance() > 0.1,
            "mars-moon distance, got {}",
            moon.distance()
        );
        assert!(moon.values.iter().all(|v| v.is_finite()));
        // File-based barycentric positions are supported natively.
        let bary = calc(J2000_ET, SUN, FLG_SWIEPH | FLG_SPEED | FLG_BARYCTR).expect("barycentric");
        assert!(bary.values.iter().all(|v| v.is_finite()));
    } else {
        eprintln!(
            "SKIP stateful_centers_and_models file-based section: no ephemeris data found; set \
             SWISSEPH_EPHE_DIR to a directory containing the .se1 files"
        );
    }

    // 2. JPL request with no JPL file selected: the engine must fall back
    // visibly , never mislabeled as JPL precision.
    let empty = std::env::temp_dir().join("swisseph-bindings-empty-ephe3");
    std::fs::create_dir_all(&empty).expect("empty ephe dir");
    set_ephe_path(Some(empty.to_str().expect("utf8 temp path"))).expect("empty data path");
    set_jpl_file("").expect("no JPL file selected");
    let fallback = calc(J2000_ET, SUN, FLG_JPLEPH | FLG_SPEED).expect("jpl fallback succeeds");
    assert!(
        fallback.returned_flags & FLG_JPLEPH == 0,
        "fallback must not claim JPL precision, got {:#X}",
        fallback.returned_flags
    );
    assert!(
        fallback.returned_flags & FLG_MOSEPH != 0,
        "fallback must be reported as analytical, got {:#X}",
        fallback.returned_flags
    );
    if let Some(ref data_dir) = data_dir {
        set_ephe_path(Some(data_dir)).expect("restore shipped data");
    }

    // 3. Tidal acceleration round trip: explicit value reads back exactly;
    // the automatic sentinel restores the default.
    set_tid_acc(SE_TIDAL_DE421).expect("set de421");
    assert_eq!(get_tid_acc().expect("read de421"), SE_TIDAL_DE421);
    // Historical Delta T (1800) follows the setting; modern J2000 Delta T
    // is pinned by observations and must not move with it.
    let jd_1800 = julday(1800, 1, 1, 12.0, Calendar::Gregorian);
    let dt_421 = deltat(jd_1800).expect("deltat 1800 de421");
    set_tid_acc(SE_TIDAL_DE431).expect("set de431");
    let dt_431 = deltat(jd_1800).expect("deltat 1800 de431");
    assert_ne!(
        dt_421, dt_431,
        "historical Delta T must follow tidal acceleration"
    );
    let modern_421 = {
        set_tid_acc(SE_TIDAL_DE421).expect("set de421 again");
        deltat(J2000_ET).expect("deltat j2000 de421")
    };
    let modern_431 = {
        set_tid_acc(SE_TIDAL_DE431).expect("set de431 again");
        deltat(J2000_ET).expect("deltat j2000 de431")
    };
    assert!(
        (modern_421 - modern_431).abs() < 1e-9,
        "modern Delta T must not move: {modern_421} vs {modern_431}"
    );
    set_tid_acc(SE_TIDAL_AUTOMATIC).expect("restore automatic");
    assert_eq!(get_tid_acc().expect("read default"), SE_TIDAL_DEFAULT);

    // 4. User Delta T pins both Delta-T entries exactly; clearing the
    // override restores the computed ~64 s band at J2000.
    set_delta_t_userdef(Some(65.0 / 86400.0)).expect("pin delta-t");
    assert_eq!(deltat(J2000_ET).expect("pinned deltat"), 65.0 / 86400.0);
    assert_eq!(
        deltat_ex(J2000_ET, 0).expect("pinned deltat_ex").value,
        65.0 / 86400.0
    );
    set_delta_t_userdef(None).expect("clear override");
    let restored = deltat(J2000_ET).expect("restored deltat");
    assert!(
        (0.0005..0.002).contains(&restored),
        "restored Delta T near 64 s in days, got {restored}"
    );

    // 5. Restore the default search path for a clean handoff.
    set_ephe_path(None).expect("restore default path");
}
