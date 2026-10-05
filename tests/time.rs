//! Second-slice time checks: UTC/UT/TT conversions, Delta T, equation of
//! time, LMT/LAT and sidereal time against the linked native library.
//!
//! Assertions are structural (ranges, round trips, error classification),
//! never stored reference vectors; see `tests/core.rs` for the policy.

#[path = "support/isolated.rs"]
mod isolated;

use swisseph_bindings::{
    Calendar, ErrorKind, FLG_MOSEPH, FLG_SWIEPH, date_conversion, day_of_week, deltat, deltat_ex,
    jdet_to_utc, jdut1_to_utc, lat_to_lmt, lmt_to_lat, set_ephe_path, sidtime, sidtime0, time_equ,
    utc_time_zone, utc_to_jd,
};

/// J2000.0 in Universal Time.
const J2000_UT: f64 = 2451545.0;

#[test]
fn utc_to_jd_j2000_splits_scales() {
    let days = utc_to_jd(2000, 1, 1, 12, 0, 0.0, Calendar::Gregorian).expect("j2000 utc");
    // UT1 noon is within leap-second distance of the civil J2000 label.
    assert!(
        (days.jd_ut - J2000_UT).abs() < 0.001,
        "jd_ut near J2000, got {}",
        days.jd_ut
    );
    // TT runs about 64 s ahead of UT1 at J2000.
    let diff = days.jd_et - days.jd_ut;
    assert!(
        (0.0005..0.002).contains(&diff),
        "TT-UT near 64 s in days, got {diff}"
    );
}

#[test]
fn utc_round_trips_through_both_branches() {
    let days = utc_to_jd(2000, 1, 1, 12, 0, 0.0, Calendar::Gregorian).expect("utc j2000");
    let via_ut1 = jdut1_to_utc(days.jd_ut, Calendar::Gregorian).expect("back via ut1");
    assert_eq!(
        (
            via_ut1.year,
            via_ut1.month,
            via_ut1.day,
            via_ut1.hour,
            via_ut1.minute
        ),
        (2000, 1, 1, 12, 0)
    );
    assert!(via_ut1.second.abs() < 1.0, "second, got {}", via_ut1.second);
    let via_et = jdet_to_utc(days.jd_et, Calendar::Gregorian).expect("back via et");
    assert_eq!(
        (
            via_et.year,
            via_et.month,
            via_et.day,
            via_et.hour,
            via_et.minute
        ),
        (2000, 1, 1, 12, 0)
    );
    assert!(via_et.second.abs() < 1.0, "second, got {}", via_et.second);
}

#[test]
fn utc_leap_second_accepted_only_where_inserted() {
    // Real leap second: 31 December 2016, 23:59:60 UTC.
    let leap = utc_to_jd(2016, 12, 31, 23, 59, 60.5, Calendar::Gregorian).expect("leap second");
    let back = jdut1_to_utc(leap.jd_ut, Calendar::Gregorian).expect("leap round trip");
    assert_eq!((back.hour, back.minute), (23, 59));
    assert!(
        (60.0..61.0).contains(&back.second),
        "leap second preserved, got {}",
        back.second
    );
    // Same clock label on a date without an insertion must fail.
    let err = utc_to_jd(2016, 6, 15, 23, 59, 60.0, Calendar::Gregorian)
        .expect_err("no leap second on this date");
    assert_eq!(err.kind(), ErrorKind::Native);
}

#[test]
fn utc_rejects_impossible_dates_and_times() {
    for (y, m, d, h, mi, s) in [
        (2000, 13, 1, 12, 0, 0.0),
        (2000, 2, 30, 12, 0, 0.0),
        (2000, 1, 1, 24, 0, 0.0),
        (2000, 1, 1, 12, 60, 0.0),
    ] {
        let err = utc_to_jd(y, m, d, h, mi, s, Calendar::Gregorian)
            .expect_err("invalid utc input must fail");
        assert_eq!(
            err.kind(),
            ErrorKind::Native,
            "for {y}-{m}-{d} {h}:{mi}:{s}"
        );
        assert!(!err.message().is_empty());
    }
}

#[test]
fn utc_before_1972_treated_as_ut1() {
    // Pre-UTC era: the engine treats the input as UT1 and still reports
    // both scales with TT ahead.
    let days = utc_to_jd(1960, 6, 15, 12, 0, 0.0, Calendar::Gregorian).expect("pre-1972 utc");
    assert!(days.jd_et > days.jd_ut);
    assert!((days.jd_et - days.jd_ut) < 0.002);
    for v in [days.jd_et, days.jd_ut] {
        assert!(v.is_finite());
    }
}

#[test]
fn date_conversion_validates_while_julday_flows_through() {
    assert_eq!(
        date_conversion(2000, 1, 1, 12.0, Calendar::Gregorian).expect("valid date"),
        J2000_UT
    );
    let err = date_conversion(2000, 2, 30, 12.0, Calendar::Gregorian)
        .expect_err("impossible date must fail");
    assert_eq!(err.kind(), ErrorKind::Native);
}

#[test]
fn weekday_and_zone_helpers() {
    // 1 January 2000 was a Saturday: Monday = 0.
    assert_eq!(day_of_week(J2000_UT).expect("weekday"), 5);
    // East-positive offset: 12:00 at +2 h is 10:00 UTC.
    let shifted = utc_time_zone(2000, 1, 1, 12, 0, 0.0, 2.0).expect("zone shift");
    assert_eq!(
        (
            shifted.year,
            shifted.month,
            shifted.day,
            shifted.hour,
            shifted.minute
        ),
        (2000, 1, 1, 10, 0)
    );
    assert!(shifted.second.abs() < 1e-9);
}

#[test]
fn delta_t_band_and_flag_selection() {
    let dt = deltat(J2000_UT).expect("delta-t j2000");
    assert!(
        (0.0005..0.001).contains(&dt),
        "about 64 s in days, got {dt}"
    );
    let detailed = deltat_ex(J2000_UT, FLG_SWIEPH).expect("delta-t-ex swieph");
    assert!(
        (detailed.value - dt).abs() < 5e-6,
        "explicit swieph flag near default, got {} vs {dt}",
        detailed.value
    );
    let moshier = deltat_ex(J2000_UT, FLG_MOSEPH).expect("delta-t-ex moseph");
    assert!(moshier.value.is_finite());
    assert!(
        (moshier.value - dt).abs() < 5e-5,
        "moseph tidal model stays close, got {} vs {dt}",
        moshier.value
    );
}

#[test]
fn equation_and_lmt_lat_round_trip() {
    let eq = time_equ(J2000_UT).expect("equation of time");
    assert!(
        eq.value.is_finite() && eq.value.abs() < 0.012,
        "within 17 min, got {}",
        eq.value
    );
    let lat = lmt_to_lat(J2000_UT, 11.0).expect("lmt to lat");
    let back = lat_to_lmt(lat.jd, 11.0).expect("lat to lmt");
    assert!(
        (back.jd - J2000_UT).abs() < 1e-9,
        "lmt/lat round trip, got {}",
        back.jd
    );
}

#[test]
fn time_equ_reports_analytical_fallback() {
    if !isolated::without_ephe_override("time_equ_reports_analytical_fallback") {
        return;
    }
    // Without data files the solar position falls back to the analytical
    // model: the value stays usable but the warning must be preserved,
    // never silently dropped (same provenance rule as ).
    set_ephe_path(None).expect("default data path");
    let eq = time_equ(J2000_UT).expect("equation with fallback");
    assert!(eq.value.is_finite());
    assert!(
        !eq.diagnostic.is_empty(),
        "analytical fallback warning must be kept"
    );
    set_ephe_path(None).expect("leave default data path");
}

#[test]
fn sidereal_time_ranges() {
    let st = sidtime(J2000_UT).expect("greenwich sidereal time");
    assert!((0.0..24.0).contains(&st), "hours in day, got {st}");
    // Explicit-frame variant with J2000 mean obliquity, no nutation:
    // same night sky, within arcminute-scale agreement of the default.
    let st0 = sidtime0(J2000_UT, 23.439_291_1, 0.0).expect("explicit-frame sidereal");
    assert!((0.0..24.0).contains(&st0), "hours in day, got {st0}");
    assert!(
        (st - st0).abs() < 0.01,
        "explicit frame near default, {st} vs {st0}"
    );
}
