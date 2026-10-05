//! Integration checks: heliacal visibility events.
//!
//! Native configuration is process-global, so every state-dependent step
//! runs inside the sequenced `stateful_*` tests. Standalone tests use
//! pure-Rust input validation only, which reaches no native code.
//! Assertions are structural (ranges, ordering, error classification,
//! same-engine consistency), never stored reference vectors.

use swisseph_bindings::{
    EVENING_FIRST, EVENING_LAST, ErrorKind, FLG_MOSEPH, FLG_SWIEPH, HELFLAG_AVKIND_VR,
    HELFLAG_BELOW_HORIZON, HELFLAG_HIGH_PRECISION, HELFLAG_LONG_SEARCH, HELFLAG_MIXED,
    HELFLAG_NO_DETAILS, HELFLAG_OPTICAL_PARAMS, HELFLAG_PHOTOPIC, HELFLAG_SCOTOPIC,
    HELFLAG_SEARCH_1_PERIOD, HELFLAG_VISLIM_DARK, HELFLAG_VISLIM_NOMOON, HELFLAG_VISLIM_PHOTOPIC,
    HELFLAG_VISLIM_SCOTOPIC, HELIACAL_RISING, HELIACAL_SETTING, MIXEDOPIC_FLAG, MORNING_FIRST,
    MORNING_LAST, PHOTOPIC_FLAG, SCOTOPIC_FLAG, TJD_INVALID, VisibilityOutcome, heliacal_pheno_ut,
    heliacal_ut, set_ephe_path, vis_limit_mag,
};

#[path = "data_dir.rs"]
mod test_data;

/// J2000.0 as a Julian Day label.
const J2000: f64 = 2451545.0;
/// Greenwich observer used throughout.
const LON: f64 = 0.0;
const LAT: f64 = 51.5;
/// Native atmosphere/observer defaults (zero entries select them).
const ATM: [f64; 4] = [0.0; 4];
const OBS: [f64; 6] = [0.0; 6];

#[test]
fn heliacal_constants_match_native_values() {
    assert_eq!(HELIACAL_RISING, 1);
    assert_eq!(HELIACAL_SETTING, 2);
    assert_eq!(MORNING_FIRST, HELIACAL_RISING);
    assert_eq!(EVENING_LAST, HELIACAL_SETTING);
    assert_eq!(EVENING_FIRST, 3);
    assert_eq!(MORNING_LAST, 4);
    assert_eq!(HELFLAG_LONG_SEARCH, 128);
    assert_eq!(HELFLAG_HIGH_PRECISION, 256);
    assert_eq!(HELFLAG_OPTICAL_PARAMS, 512);
    assert_eq!(HELFLAG_NO_DETAILS, 1024);
    assert_eq!(HELFLAG_SEARCH_1_PERIOD, 2048);
    assert_eq!(HELFLAG_VISLIM_DARK, 4096);
    assert_eq!(HELFLAG_VISLIM_NOMOON, 8192);
    assert_eq!(HELFLAG_VISLIM_PHOTOPIC, 16384);
    assert_eq!(HELFLAG_VISLIM_SCOTOPIC, 32768);
    assert_eq!(HELFLAG_AVKIND_VR, 65536);
    assert_eq!(HELFLAG_BELOW_HORIZON, -2);
    assert_eq!(HELFLAG_PHOTOPIC, 0);
    assert_eq!(HELFLAG_SCOTOPIC, 1);
    assert_eq!(HELFLAG_MIXED, 2);
    assert_eq!(PHOTOPIC_FLAG, 0);
    assert_eq!(SCOTOPIC_FLAG, 1);
    assert_eq!(MIXEDOPIC_FLAG, 2);
    assert_eq!(TJD_INVALID, 99999999.0);
}

#[test]
fn heliacal_calls_reject_non_finite_input() {
    let bad_atm = [f64::NAN, 0.0, 0.0, 0.0];
    let bad_obs = [0.0, 0.0, f64::INFINITY, 0.0, 0.0, 0.0];
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for result in [
            heliacal_ut(bad, LON, LAT, 0.0, ATM, OBS, "Venus", 1, FLG_MOSEPH).map(|_| ()),
            heliacal_ut(J2000, bad, LAT, 0.0, ATM, OBS, "Venus", 1, FLG_MOSEPH).map(|_| ()),
            heliacal_ut(J2000, LON, LAT, bad, ATM, OBS, "Venus", 1, FLG_MOSEPH).map(|_| ()),
            heliacal_ut(J2000, LON, LAT, 0.0, bad_atm, OBS, "Venus", 1, FLG_MOSEPH).map(|_| ()),
            heliacal_ut(J2000, LON, LAT, 0.0, ATM, bad_obs, "Venus", 1, FLG_MOSEPH).map(|_| ()),
            heliacal_pheno_ut(bad, LON, LAT, 0.0, ATM, OBS, "Venus", 1, FLG_MOSEPH).map(|_| ()),
            heliacal_pheno_ut(J2000, LON, bad, 0.0, ATM, OBS, "Venus", 1, FLG_MOSEPH).map(|_| ()),
            vis_limit_mag(bad, LON, LAT, 0.0, ATM, OBS, "Venus", FLG_MOSEPH).map(|_| ()),
            vis_limit_mag(J2000, LON, LAT, 0.0, bad_atm, OBS, "Venus", FLG_MOSEPH).map(|_| ()),
            vis_limit_mag(J2000, LON, LAT, 0.0, ATM, bad_obs, "Venus", FLG_MOSEPH).map(|_| ()),
        ] {
            let err = result.expect_err("non-finite input must be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidInput);
        }
    }
}

#[test]
fn heliacal_calls_reject_bad_object_names() {
    let overlong = "x".repeat(512);
    for result in [
        heliacal_ut(J2000, LON, LAT, 0.0, ATM, OBS, "Sir\0ius", 1, FLG_MOSEPH).map(|_| ()),
        heliacal_ut(J2000, LON, LAT, 0.0, ATM, OBS, &overlong, 1, FLG_MOSEPH).map(|_| ()),
        heliacal_pheno_ut(J2000, LON, LAT, 0.0, ATM, OBS, "Ve\0nus", 1, FLG_MOSEPH).map(|_| ()),
        heliacal_pheno_ut(J2000, LON, LAT, 0.0, ATM, OBS, &overlong, 1, FLG_MOSEPH).map(|_| ()),
        vis_limit_mag(J2000, LON, LAT, 0.0, ATM, OBS, "Ve\0nus", FLG_MOSEPH).map(|_| ()),
        vis_limit_mag(J2000, LON, LAT, 0.0, ATM, OBS, &overlong, FLG_MOSEPH).map(|_| ()),
    ] {
        let err = result.expect_err("bad object name must be rejected");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}

#[test]
fn stateful_venus_morning_first_moshier() {
    let event = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect("Venus morning-first");
    assert!(event.begin() > J2000);
    assert!(event.begin() <= event.optimum() && event.optimum() <= event.end());
    assert!(event.diagnostic.is_empty());
    // Reserved slots read 0.0 on the visibility-limit planet path.
    assert!(event.times[3..].iter().all(|slot| *slot == 0.0));
    // Repeat calls are bitwise reproducible under the shared lock.
    let repeat = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect("repeat Venus morning-first");
    assert_eq!(event, repeat);
}

#[test]
fn stateful_venus_evening_first_moshier() {
    // Inner planets support evening-first/morning-last around superior
    // conjunction.
    let event = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Venus",
        EVENING_FIRST,
        FLG_MOSEPH,
    )
    .expect("Venus evening-first");
    assert!(event.begin() > J2000);
    assert!(event.begin() <= event.optimum() && event.optimum() <= event.end());
}

#[test]
fn stateful_moon_evening_first_with_data() {
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP file-based assertions: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("data path");
    let event = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Moon",
        EVENING_FIRST,
        FLG_SWIEPH,
    )
    .expect("Moon evening-first");
    assert!(event.begin() > J2000);
    assert!(event.begin() <= event.optimum() && event.optimum() <= event.end());
    // The Moon path writes only the three event slots; the tail still
    // reads 0.0 from the zero-initialized buffer.
    assert!(event.times[3..].iter().all(|slot| *slot == 0.0));
    let repeat = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Moon",
        EVENING_FIRST,
        FLG_SWIEPH,
    )
    .expect("repeat Moon evening-first");
    assert_eq!(event, repeat);
}

#[test]
fn stateful_heliacal_search_failures_are_native() {
    // The Sun has no heliacal event.
    let err = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Sun",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect_err("Sun must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("no heliacal"),
        "got: {}",
        err.message()
    );
    // Acronychal types are declared but not implemented natively.
    let err = heliacal_ut(J2000, LON, LAT, 0.0, ATM, OBS, "Venus", 5, FLG_MOSEPH)
        .expect_err("type 5 must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("not provided"),
        "got: {}",
        err.message()
    );
    // Evening-first exists only for inner planets and the Moon.
    let err = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Mars",
        EVENING_FIRST,
        FLG_MOSEPH,
    )
    .expect_err("Mars evening-first must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("does not exist"),
        "got: {}",
        err.message()
    );
    // Observer heights outside −500…25000 m fail natively.
    let err = heliacal_ut(
        J2000,
        LON,
        LAT,
        30000.0,
        ATM,
        OBS,
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect_err("extreme altitude must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("between -500 and 25000"),
        "got: {}",
        err.message()
    );
    // Unknown objects fail with the engine verdict kept verbatim.
    let err = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "NoSuchBodyXYZ",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect_err("unknown object must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("not found"),
        "got: {}",
        err.message()
    );
    // An empty name reaches the native lookup and fails there (never a
    // Rust-side invention).
    let err = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect_err("empty name must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(err.message().contains("empty"), "got: {}", err.message());
}

#[test]
fn stateful_venus_pheno_at_optimum() {
    let event = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect("Venus morning-first");
    let pheno = heliacal_pheno_ut(
        event.optimum(),
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect("Venus pheno at optimum");
    // Venus at dawn: a few degrees up, Sun a few degrees down, Venus
    // magnitude near −4.
    assert!((0.0..10.0).contains(&pheno.object_altitude_deg()));
    assert!((-10.0..0.0).contains(&pheno.sun_altitude_deg()));
    assert!((-5.0..-3.0).contains(&pheno.object_magnitude()));
    // Non-lunar objects have no Yallop timing: the invalid-time sentinel.
    assert_eq!(pheno.yallop_best_jd(), TJD_INVALID);
    // Reserved slots read 0.0.
    assert_eq!(pheno.values[28], 0.0);
    assert_eq!(pheno.values[29], 0.0);
    let repeat = heliacal_pheno_ut(
        event.optimum(),
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect("repeat Venus pheno");
    assert_eq!(pheno, repeat);
}

#[test]
fn stateful_moon_pheno_has_yallop_timing() {
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP file-based assertions: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("data path");
    let event = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Moon",
        EVENING_FIRST,
        FLG_SWIEPH,
    )
    .expect("Moon evening-first");
    let pheno = heliacal_pheno_ut(
        event.optimum(),
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Moon",
        EVENING_FIRST,
        FLG_SWIEPH,
    )
    .expect("Moon pheno at optimum");
    // The Moon is the Yallop-criterion body: real Yallop timing and
    // crescent details, unlike the Venus sentinel row.
    assert_ne!(pheno.yallop_best_jd(), TJD_INVALID);
    assert!(pheno.yallop_q().is_finite());
    assert!(pheno.yallop_criterion().is_finite());
    assert!(pheno.moon_crescent_width_deg() >= 0.0);
}

#[test]
fn stateful_pheno_below_horizon_is_data() {
    let event = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect("Venus morning-first");
    // Six hours before the optimum (night) Venus is below the horizon;
    // the phenomenon call still computes and reports negative altitudes.
    let pheno = heliacal_pheno_ut(
        event.optimum() - 0.25,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect("Venus pheno below horizon");
    assert!(pheno.object_altitude_deg() < 0.0);
}

#[test]
fn stateful_vislim_visible_and_below_horizon() {
    let event = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Venus",
        HELIACAL_RISING,
        FLG_MOSEPH,
    )
    .expect("Venus morning-first");
    // At the optimum Venus is above the horizon and brighter than the
    // limiting magnitude.
    match vis_limit_mag(
        event.optimum(),
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Venus",
        FLG_MOSEPH,
    )
    .expect("Venus vislim at optimum")
    {
        VisibilityOutcome::Visible(visible) => {
            assert!((0..=3).contains(&visible.status));
            assert!(visible.object_magnitude() < visible.limiting_magnitude());
            assert!(visible.object_altitude_deg() > 0.0);
            // Geometry cross-agrees with the phenomenon call.
            let pheno = heliacal_pheno_ut(
                event.optimum(),
                LON,
                LAT,
                0.0,
                ATM,
                OBS,
                "Venus",
                HELIACAL_RISING,
                FLG_MOSEPH,
            )
            .expect("Venus pheno");
            assert_eq!(visible.object_altitude_deg(), pheno.object_altitude_deg());
            assert_eq!(visible.object_azimuth_deg(), pheno.object_azimuth_deg());
            assert_eq!(visible.sun_altitude_deg(), pheno.sun_altitude_deg());
            assert_eq!(visible.sun_azimuth_deg(), pheno.sun_azimuth_deg());
            assert_eq!(visible.object_magnitude(), pheno.object_magnitude());
        }
        VisibilityOutcome::BelowHorizon(_) => panic!("Venus must be visible at its optimum"),
    }
    // Six hours earlier (night) Venus is below the horizon: a dedicated
    // Ok state with the −100 marker, never an error.
    match vis_limit_mag(
        event.optimum() - 0.25,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Venus",
        FLG_MOSEPH,
    )
    .expect("Venus vislim below horizon")
    {
        VisibilityOutcome::BelowHorizon(below) => {
            assert_eq!(below.values[0], -100.0);
            assert!(below.diagnostic.contains("below local horizon"));
        }
        VisibilityOutcome::Visible(_) => panic!("Venus must be below the horizon at night"),
    }
}

#[test]
fn stateful_vislim_failures_are_native() {
    // The Sun has no limiting magnitude.
    let err = vis_limit_mag(J2000, LON, LAT, 0.0, ATM, OBS, "Sun", FLG_MOSEPH)
        .expect_err("Sun must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(err.message().contains("no sense"), "got: {}", err.message());
    // Unknown objects fail with the engine verdict kept verbatim.
    let err = vis_limit_mag(J2000, LON, LAT, 0.0, ATM, OBS, "NoSuchBodyXYZ", FLG_MOSEPH)
        .expect_err("unknown object must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(
        err.message().contains("not found"),
        "got: {}",
        err.message()
    );
}

#[test]
fn stateful_sirius_star_path_with_catalog() {
    let Some(data_dir) = test_data::ephemeris_dir() else {
        eprintln!(
            "SKIP file-based assertions: no ephemeris data found; set SWISSEPH_EPHE_DIR \
             to a directory containing the .se1 files"
        );
        return;
    };
    set_ephe_path(Some(&data_dir)).expect("data path");
    // Fixed-star search through the shipped catalog.
    let event = heliacal_ut(
        J2000,
        LON,
        LAT,
        0.0,
        ATM,
        OBS,
        "Sirius",
        HELIACAL_RISING,
        FLG_SWIEPH,
    )
    .expect("Sirius morning-first");
    assert!(event.begin() > J2000);
    assert!(event.begin() <= event.optimum() && event.optimum() <= event.end());
    // At Greenwich midnight Sirius is high in a dark sky: scotopic
    // vision with a ~6 mag limit and the catalog −1.46 magnitude.
    match vis_limit_mag(J2000 - 0.5, LON, LAT, 0.0, ATM, OBS, "Sirius", FLG_SWIEPH)
        .expect("Sirius vislim at midnight")
    {
        VisibilityOutcome::Visible(visible) => {
            assert_eq!(visible.status, HELFLAG_SCOTOPIC);
            assert!((6.0..6.4).contains(&visible.limiting_magnitude()));
            assert_eq!(visible.object_magnitude(), -1.46);
        }
        VisibilityOutcome::BelowHorizon(_) => panic!("Sirius must be visible at midnight"),
    }
}

#[test]
fn avkind_search_rejects_native_internal_buffer_defect() {
    // ASan identifies an undersized internal coordinate buffer in this
    // native search path. Every bit selecting the path is rejected; the
    // binding must not clear flags or silently choose a different model.
    for flags in [
        HELFLAG_AVKIND_VR,
        swisseph_bindings::HELFLAG_AVKIND_PTO,
        swisseph_bindings::HELFLAG_AVKIND_MIN7,
        swisseph_bindings::HELFLAG_AVKIND_MIN9,
        swisseph_bindings::HELFLAG_AVKIND,
    ] {
        let error = heliacal_ut(
            J2000,
            LON,
            LAT,
            0.0,
            ATM,
            OBS,
            "Venus",
            HELIACAL_RISING,
            FLG_MOSEPH | flags,
        )
        .expect_err("unsafe native arcus-visionis path must be rejected");
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        assert!(error.message().contains("internal buffer capacity defect"));
    }
}
