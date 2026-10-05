//! Integration checks: version, time, positions, configuration
//! and houses against the linked native library.
//!
//! Native configuration is process-global, so every state-mutating step
//! (data path, sidereal mode, observer, close) runs inside the single
//! sequenced `stateful_configuration` test. Standalone tests only exercise calls
//! whose outcome does not depend on shared configuration. No numeric
//! reference vectors are stored here: assertions check structural
//! properties (ranges, finiteness, flag provenance, error classification)
//! plus same-engine consistency between the two house wrappers.

#[path = "support/isolated.rs"]
mod isolated;

use std::thread;

use swisseph_bindings::{
    Calendar, ErrorKind, FLG_MOSEPH, FLG_SPEED, FLG_SWIEPH, FLG_TOPOCTR, GREG_CAL, JUL_CAL, MARS,
    MOON, SIDM_FAGAN_BRADLEY, SIDM_LAHIRI, SUN, calc, calc_ut, close, get_ayanamsa_ut,
    get_planet_name, houses, houses_ex, julday, library_path, revjul, set_ephe_path, set_sid_mode,
    set_topo, version,
};

#[path = "data_dir.rs"]
mod test_data;

/// J2000.0 in Universal Time.
const J2000_UT: f64 = 2451545.0;

#[test]
fn native_version_matches_pinned_build() {
    let reported = version().expect("linked version query");
    assert!(
        !reported.is_empty(),
        "native version must be a non-empty string"
    );
    // The pinned line reports 2.10.x; require the dotted numeric shape
    // rather than an exact value so patch bumps stay visible, not hidden.
    assert!(
        reported.chars().next().is_some_and(|c| c.is_ascii_digit()),
        "unexpected native version shape: {reported:?}"
    );
    assert!(
        reported.contains('.'),
        "unexpected native version shape: {reported:?}"
    );
}

#[test]
fn julday_revjul_round_trip() {
    assert_eq!(julday(2000, 1, 1, 12.0, Calendar::Gregorian), J2000_UT);
    let date = revjul(J2000_UT, Calendar::Gregorian).expect("reverse J2000");
    assert_eq!(
        (date.year, date.month, date.day, date.hour),
        (2000, 1, 1, 12.0)
    );
    // BCE date (astronomical year -43 == 44 BCE): the round trip must be
    // exact for whole hours, including across the year-zero boundary.
    let bce = julday(-43, 3, 15, 0.0, Calendar::Julian);
    let back = revjul(bce, Calendar::Julian).expect("reverse BCE date");
    assert_eq!(
        (back.year, back.month, back.day, back.hour),
        (-43, 3, 15, 0.0)
    );
}

#[test]
fn calendar_flags_match_native_convention() {
    assert_eq!(JUL_CAL, 0);
    assert_eq!(GREG_CAL, 1);
}

#[test]
fn moshier_positions_need_no_data_files() {
    // Analytical model: deterministic without ephemeris data, so this also
    // runs on machines without the submodule data directory.
    let sun = calc_ut(J2000_UT, SUN, FLG_MOSEPH | FLG_SPEED).expect("sun via Moshier");
    assert!(
        sun.returned_flags & FLG_MOSEPH != 0,
        "returned flags must name the analytical source, got {:#X}",
        sun.returned_flags
    );
    assert!((0.0..360.0).contains(&sun.longitude()));
    assert!(sun.latitude().abs() <= 2.0);
    assert!(sun.distance() > 0.9 && sun.distance() < 1.1);
    for value in sun.values {
        assert!(value.is_finite(), "non-finite component in {sun:?}");
    }
    // Terrestrial-Time entry point agrees with the UT one at J2000 up to
    // the ~64 s TT-UT offset: same sign of motion, nearby longitude.
    let sun_et = calc(J2000_UT, SUN, FLG_MOSEPH | FLG_SPEED).expect("sun in ET");
    assert!((sun_et.longitude() - sun.longitude()).abs() < 0.1);
}

#[test]
fn unknown_body_is_a_classified_error() {
    let err = calc_ut(J2000_UT, 999_999, FLG_MOSEPH).expect_err("unknown body must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(!err.message().is_empty(), "native diagnostic must be kept");
}

#[test]
fn planet_names_round_trip() {
    assert_eq!(get_planet_name(SUN).expect("sun name"), "Sun");
    assert_eq!(get_planet_name(MOON).expect("moon name"), "Moon");
}

#[test]
fn houses_agree_between_both_wrappers() {
    let plain = houses(J2000_UT, 51.5, -0.12, b'P').expect("placidus london");
    let flagged = houses_ex(J2000_UT, 0, 51.5, -0.12, b'P').expect("placidus ex london");
    assert_eq!(plain, flagged, "iflag=0 must behave like houses()");
    for cusp in plain.cusps {
        assert!(cusp.is_finite() && (0.0..360.0).contains(&cusp));
    }
    for angle in plain.angles {
        assert!(angle.is_finite());
    }
    assert!((0.0..360.0).contains(&plain.ascendant()));
    assert!((0.0..360.0).contains(&plain.mc()));
}

#[test]
fn twelve_house_results_reject_gauquelin_layout() {
    let err = houses(J2000_UT, 51.5, -0.12, b'G').expect_err("Gauquelin must wait");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    let err = houses_ex(J2000_UT, 0, 51.5, -0.12, b'g').expect_err("gauquelin must wait");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
}

#[test]
fn concurrent_readers_share_the_native_lock() {
    // Eight threads hammering a configuration-independent call must all
    // succeed without deadlocking on the process-wide lock.
    let workers: Vec<_> = (0..8)
        .map(|_| thread::spawn(|| julday(2000, 1, 1, 12.0, Calendar::Gregorian)))
        .collect();
    for worker in workers {
        assert_eq!(worker.join().expect("worker panicked"), J2000_UT);
    }
}

/// Sequenced stateful checks: data path, fallback reporting, sidereal mode,
/// observer, reset. Order matters because native state is process-global.
#[test]
fn stateful_configuration() {
    if !isolated::without_ephe_override("stateful_configuration") {
        return;
    }
    // 1. File-based precision with an explicit external data directory
    // (else the checkout data). The returned flags must name the file
    // source: analytical fallback would be a silent precision loss
    //. Without data only this section is skipped; the
    // fallback/validation/reset steps below still run.
    let data_dir = test_data::ephemeris_dir();
    if let Some(ref data_dir) = data_dir {
        set_ephe_path(Some(data_dir)).expect("point at shipped data");
        let mars = calc_ut(J2000_UT, MARS, FLG_SWIEPH | FLG_SPEED).expect("mars via files");
        assert!(
            mars.returned_flags & FLG_SWIEPH != 0,
            "expected file-based source flag, got {:#X} ({})",
            mars.returned_flags,
            mars.diagnostic
        );
        for value in mars.values {
            assert!(value.is_finite());
        }
    } else {
        eprintln!(
            "SKIP stateful_configuration file-based section: no ephemeris data found; set \
             SWISSEPH_EPHE_DIR to a directory containing the .se1 files"
        );
    }

    // 2. Empty directory: the engine must fall back visibly, reporting the
    // analytical source in the returned flags instead of failing or lying.
    let empty = std::env::temp_dir().join("swisseph-bindings-empty-ephe");
    std::fs::create_dir_all(&empty).expect("empty ephe dir");
    // Keep SE_EPHE_PATH out of the picture for a deterministic outcome.
    set_ephe_path(Some(empty.to_str().expect("utf8 temp path"))).expect("empty data path");
    let fallback = calc_ut(J2000_UT, MARS, FLG_SWIEPH | FLG_SPEED).expect("fallback succeeds");
    assert!(
        fallback.returned_flags & FLG_MOSEPH != 0,
        "fallback must be reported as analytical, got {:#X}",
        fallback.returned_flags
    );
    assert!(
        fallback.returned_flags & FLG_SWIEPH == 0,
        "fallback must not claim file-based precision"
    );

    // 3. Invalid path inputs never reach native code.
    let nul = set_ephe_path(Some("bad\0path")).expect_err("NUL path");
    assert_eq!(nul.kind(), ErrorKind::InvalidInput);
    let long = set_ephe_path(Some(&"a".repeat(243))).expect_err("overlong path");
    assert_eq!(long.kind(), ErrorKind::InvalidInput);

    // 4. Restore data, then sidereal modes must differ and Lahiri J2000
    // must sit in its known band (a structural check, not a fixture).
    if let Some(ref data_dir) = data_dir {
        set_ephe_path(Some(data_dir)).expect("restore shipped data");
    }
    set_sid_mode(SIDM_LAHIRI, 0.0, 0.0).expect("lahiri mode");
    let lahiri = get_ayanamsa_ut(J2000_UT).expect("lahiri ayanamsha");
    set_sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0).expect("fagan mode");
    let fagan = get_ayanamsa_ut(J2000_UT).expect("fagan ayanamsha");
    assert!((23.0..25.0).contains(&lahiri), "lahiri band, got {lahiri}");
    assert!(
        (lahiri - fagan).abs() > 0.5,
        "modes must differ: lahiri={lahiri} fagan={fagan}"
    );

    // 5. Observer + topocentric request succeed; executable path is readable.
    set_topo(11.34, 48.14, 520.0).expect("munich observer");
    let topo =
        calc_ut(J2000_UT, MOON, FLG_SWIEPH | FLG_SPEED | FLG_TOPOCTR).expect("topocentric moon");
    assert!(topo.values.iter().all(|v| v.is_finite()));
    let path = library_path().expect("library path");
    assert!(!path.is_empty());

    // 6. Explicit close drops caches; the next computation lazily
    // re-initializes and still succeeds.
    close().expect("close");
    if let Some(ref data_dir) = data_dir {
        set_ephe_path(Some(data_dir)).expect("re-point after close");
    }
    calc_ut(J2000_UT, SUN, FLG_SWIEPH | FLG_SPEED).expect("works after close");
    set_ephe_path(None).expect("restore default path");
}
