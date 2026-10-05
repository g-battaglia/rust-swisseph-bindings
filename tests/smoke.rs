//! data-independent smoke: version, calendar conversion, the
//! analytical model, houses, visible fallback and missing-data behavior.
//!
//! Every test here runs without ephemeris files, so this binary is the
//! smoke run for an extracted minimal package (no bundled data) as well
//! as for a plain checkout. File-based precision is covered by the
//! integration suites with `SWISSEPH_EPHE_DIR` set, not here: no assertion
//! below may require the `FLG_SWIEPH` source bit.

#[path = "support/isolated.rs"]
mod isolated;

use std::sync::{Mutex, MutexGuard};

use swisseph_bindings::{
    Calendar, ErrorKind, FLG_MOSEPH, FLG_SPEED, FLG_SWIEPH, SUN, calc_ut, fixstar_ut, houses,
    houses_ex, julday, revjul, set_ephe_path, version,
};

/// J2000.0 in Universal Time.
const J2000_UT: f64 = 2451545.0;

/// Serializes the state-mutating smoke test. Only one test in this binary
/// mutates native state, but the guard keeps that invariant explicit and
/// runs inside an environment-controlled child process. Always held outside
/// the native lock, so no deadlock is possible.
static SMOKE_SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SMOKE_SERIAL.lock().expect("smoke serial lock")
}

#[test]
fn smoke_native_version_has_dotted_shape() {
    let reported = version().expect("linked version query");
    assert!(!reported.is_empty(), "native version must be non-empty");
    assert!(
        reported.chars().next().is_some_and(|c| c.is_ascii_digit()) && reported.contains('.'),
        "unexpected native version shape: {reported:?}"
    );
}

#[test]
fn smoke_julday_revjul_round_trip() {
    assert_eq!(julday(2000, 1, 1, 12.0, Calendar::Gregorian), J2000_UT);
    let date = revjul(J2000_UT, Calendar::Gregorian).expect("reverse J2000");
    assert_eq!(
        (date.year, date.month, date.day, date.hour),
        (2000, 1, 1, 12.0)
    );
}

#[test]
fn smoke_moshier_sun_needs_no_data_files() {
    let sun = calc_ut(J2000_UT, SUN, FLG_MOSEPH | FLG_SPEED).expect("sun via Moshier");
    assert!(
        sun.returned_flags & FLG_MOSEPH != 0,
        "returned flags must name the analytical source, got {:#X}",
        sun.returned_flags
    );
    assert!((0.0..360.0).contains(&sun.longitude()));
    assert!(sun.values.iter().all(|v| v.is_finite()));
}

#[test]
fn smoke_houses_need_no_data_files() {
    let plain = houses(J2000_UT, 51.5, -0.12, b'P').expect("placidus london");
    let flagged = houses_ex(J2000_UT, 0, 51.5, -0.12, b'P').expect("placidus ex london");
    assert_eq!(plain, flagged, "iflag=0 must behave like houses()");
    assert!((0.0..360.0).contains(&plain.ascendant()));
    assert!((0.0..360.0).contains(&plain.mc()));
}

/// Sequenced data-free slice: visible analytical fallback, missing-catalog
/// behavior and input validation, with the default path restored at the end.
#[test]
fn smoke_fallback_and_missing_data_are_explicit() {
    if !isolated::without_ephe_override("smoke_fallback_and_missing_data_are_explicit") {
        return;
    }
    let _serial = serial();
    // Keep SE_EPHE_PATH out of the picture for a deterministic outcome.

    // 1. An empty data directory: the engine must fall back visibly to
    // the analytical model, never fail or mislabel the source.
    let empty = std::env::temp_dir().join("swisseph-bindings-smoke-empty");
    std::fs::create_dir_all(&empty).expect("empty ephe dir");
    set_ephe_path(Some(empty.to_str().expect("utf8 temp path"))).expect("empty data path");
    let fallback = calc_ut(J2000_UT, SUN, FLG_SWIEPH | FLG_SPEED).expect("fallback succeeds");
    assert!(
        fallback.returned_flags & FLG_MOSEPH != 0,
        "fallback must be reported as analytical, got {:#X}",
        fallback.returned_flags
    );
    assert!(
        fallback.returned_flags & FLG_SWIEPH == 0,
        "fallback must not claim file-based precision"
    );

    // 2. Without catalog data the engine reports the gap instead of
    // inventing a star.
    let err = fixstar_ut("Sirius", J2000_UT, FLG_SWIEPH | FLG_SPEED)
        .expect_err("star lookup without catalog must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
    assert!(!err.message().is_empty(), "native diagnostic must be kept");

    // 3. Invalid path inputs never reach native code, even in the
    // data-free configuration.
    let nul = set_ephe_path(Some("bad\0path")).expect_err("NUL path");
    assert_eq!(nul.kind(), ErrorKind::InvalidInput);

    // 4. Restore the default native search path.
    set_ephe_path(None).expect("restore default path");
}
