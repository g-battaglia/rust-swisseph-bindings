//! one-try occultation controls: the extended entry points
//! agree with the direction-only calls without `one_try`, hits find the
//! nearest-conjunction event, and misses come back as the typed no-event
//! state (`occult_type == 0` with a continuation epoch) rather than a
//! failure.
//!
//! All searches use the analytical model, so these tests need no
//! ephemeris files. The location search installs its observer as the
//! process-global observer; the default observer is restored at the end
//! of the location test.

use swisseph_bindings::{
    ErrorKind, FLG_MOSEPH, MERCURY, MOON, OccultSearchOptions, VENUS, lun_occult_when_glob,
    lun_occult_when_glob_with_options, lun_occult_when_loc, lun_occult_when_loc_with_options,
    set_topo,
};

/// J2000.0 in Universal Time.
const J2000_UT: f64 = 2451545.0;
/// Search epoch shortly before the known Venus occultation maximum.
const BEFORE_VENUS_EVENT: f64 = 2451600.0;
/// Search epoch shortly after the known Venus occultation maximum.
const AFTER_VENUS_EVENT: f64 = 2451610.0;

/// `one_try: false` must reach the native call with the same integer as
/// the direction-only spelling: every field agrees exactly.
#[test]
fn options_without_one_try_match_direction_calls() {
    for backward in [false, true] {
        let search = OccultSearchOptions {
            backward,
            one_try: false,
        };
        let classic = lun_occult_when_glob(J2000_UT, VENUS, None, FLG_MOSEPH, 0, backward)
            .expect("classic glob");
        let extended =
            lun_occult_when_glob_with_options(J2000_UT, VENUS, None, FLG_MOSEPH, 0, search)
                .expect("extended glob");
        assert_eq!(extended.occult_type, classic.occult_type);
        assert_eq!(extended.times, classic.times);
        assert_eq!(extended.diagnostic, classic.diagnostic);

        let classic =
            lun_occult_when_loc(J2000_UT, VENUS, None, FLG_MOSEPH, 0.0, 51.5, 0.0, backward)
                .expect("classic loc");
        let extended = lun_occult_when_loc_with_options(
            J2000_UT, VENUS, None, FLG_MOSEPH, 0.0, 51.5, 0.0, search,
        )
        .expect("extended loc");
        assert_eq!(extended.occult_type, classic.occult_type);
        assert_eq!(extended.times, classic.times);
        assert_eq!(extended.attributes, classic.attributes);
        assert_eq!(extended.diagnostic, classic.diagnostic);
    }
    set_topo(0.0, 0.0, 0.0).expect("restore default observer");
}

/// A one-try search whose nearest conjunction occults finds the same
/// event as the unbounded search, forward and backward.
#[test]
fn one_try_hit_finds_nearest_conjunction_event() {
    let full = lun_occult_when_glob(J2000_UT, VENUS, None, FLG_MOSEPH, 0, false)
        .expect("full venus search");
    assert!(full.maximum() > BEFORE_VENUS_EVENT);

    let forward = lun_occult_when_glob_with_options(
        BEFORE_VENUS_EVENT,
        VENUS,
        None,
        FLG_MOSEPH,
        0,
        OccultSearchOptions {
            backward: false,
            one_try: true,
        },
    )
    .expect("one-try forward hit");
    assert_ne!(forward.occult_type, 0);
    assert_eq!(forward.maximum(), full.maximum());
    assert_eq!(forward.times, full.times);

    let backward = lun_occult_when_glob_with_options(
        AFTER_VENUS_EVENT,
        VENUS,
        None,
        FLG_MOSEPH,
        0,
        OccultSearchOptions {
            backward: true,
            one_try: true,
        },
    )
    .expect("one-try backward hit");
    assert_ne!(backward.occult_type, 0);
    assert_eq!(backward.maximum(), full.maximum());
}

/// A one-try search whose nearest conjunction does not occult reports
/// the typed no-event state: `Ok` with `occult_type == 0` and a finite
/// continuation epoch in `times[0]`, not a failure.
#[test]
fn one_try_miss_is_typed_no_event() {
    let miss = lun_occult_when_glob_with_options(
        J2000_UT,
        VENUS,
        None,
        FLG_MOSEPH,
        0,
        OccultSearchOptions {
            backward: false,
            one_try: true,
        },
    )
    .expect("one-try forward miss is not an error");
    assert_eq!(miss.occult_type, 0);
    assert!(
        miss.times[0].is_finite() && miss.times[0] > J2000_UT,
        "miss must carry a finite forward continuation epoch, got {}",
        miss.times[0]
    );

    let miss = lun_occult_when_loc_with_options(
        J2000_UT,
        MERCURY,
        None,
        FLG_MOSEPH,
        0.0,
        51.5,
        0.0,
        OccultSearchOptions {
            backward: false,
            one_try: true,
        },
    )
    .expect("one-try location miss is not an error");
    assert_eq!(miss.occult_type, 0);
    assert!(
        miss.times[0].is_finite() && miss.times[0] > J2000_UT,
        "miss must carry a finite forward continuation epoch, got {}",
        miss.times[0]
    );
    set_topo(0.0, 0.0, 0.0).expect("restore default observer");
}

/// Input validation applies on the options path exactly as on the
/// direction-only path: invalid inputs never reach native code.
#[test]
fn options_path_rejects_invalid_inputs() {
    let forward_try = OccultSearchOptions {
        backward: false,
        one_try: true,
    };
    for result in [
        lun_occult_when_glob_with_options(f64::NAN, VENUS, None, FLG_MOSEPH, 0, forward_try)
            .map(|_| ()),
        lun_occult_when_glob_with_options(J2000_UT, MOON, None, FLG_MOSEPH, 0, forward_try)
            .map(|_| ()),
        lun_occult_when_loc_with_options(
            J2000_UT,
            VENUS,
            Some("Sirius\0injected"),
            FLG_MOSEPH,
            0.0,
            51.5,
            0.0,
            forward_try,
        )
        .map(|_| ()),
    ] {
        let err = result.expect_err("invalid options input must fail");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}
