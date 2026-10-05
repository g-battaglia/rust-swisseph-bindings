//! regression: safe FFI input domains.
//!
//! One minimal regression per proven undefined-behavior mechanism plus the
//! adjacent finite-domain guards. Invalid inputs are rejected with
//! [`ErrorKind::InvalidInput`] before any native call; invalid setters
//! leave the previously configured native state untouched. No native
//! output vectors are persisted here: assertions check rejection,
//! classification and state preservation only.

use swisseph_bindings::{
    Calendar, ErrorKind, FLG_MOSEPH, FLG_SPEED, FLG_TOPOCTR, MOON, SIDM_FAGAN_BRADLEY, SIDM_USER,
    calc_ut, cs2degstr, cs2lonlatstr, cs2timestr, csroundsec, date_conversion, day_of_week, deltat,
    difcs2n, difcsn, houses, jdet_to_utc, jdut1_to_utc, lat_to_lmt, lmt_to_lat, set_sid_mode,
    set_topo, sidtime, split_deg, time_equ, utc_time_zone, utc_to_jd,
};

fn assert_invalid(result: Result<impl std::fmt::Debug, swisseph_bindings::Error>, context: &str) {
    let err = result.expect_err(context);
    assert_eq!(
        err.kind(),
        ErrorKind::InvalidInput,
        "{context}: wrong error kind: {err:?}"
    );
}

#[test]
fn split_deg_rejects_unrepresentable_magnitudes() {
    // Proven UBSan case: out-of-range float-to-integer conversion.
    assert_invalid(split_deg(1e100, 0), "split_deg(1e100)");
    assert_invalid(split_deg(f64::NAN, 0), "split_deg(NaN)");
    assert_invalid(split_deg(f64::INFINITY, 0), "split_deg(inf)");
    // Ordinary angles still split.
    let plain = split_deg(45.5, 0).expect("plain split");
    assert_eq!(
        (plain.deg, plain.min, plain.sec, plain.sign),
        (45, 30, 0, 1)
    );
}

#[test]
fn integer_subtraction_overflow_is_rejected() {
    // Proven UBSan cases: signed subtraction overflow.
    assert_invalid(difcsn(i32::MAX, i32::MIN), "difcsn(MAX, MIN)");
    assert_invalid(difcs2n(i32::MAX, i32::MIN), "difcs2n(MAX, MIN)");
    assert_invalid(difcsn(i32::MIN, 1), "difcsn(MIN, 1)");
    // Ordinary pairs still resolve.
    assert_eq!(difcsn(720000, 360000).expect("difcsn"), 360000);
    assert_eq!(difcs2n(360000, 720000).expect("difcs2n"), -360000);
}

#[test]
fn rounding_addition_overflow_is_rejected() {
    // Proven UBSan case: signed addition overflow inside rounding.
    assert_invalid(csroundsec(i32::MAX), "csroundsec(MAX)");
    assert_invalid(csroundsec(i32::MIN), "csroundsec(MIN)");
    // Ordinary values still round.
    assert_eq!(csroundsec(150).expect("round"), 200);
    assert_eq!(csroundsec(-150).expect("round"), -100);
}

#[test]
fn format_negation_overflow_is_rejected() {
    // Proven UBSan case: signed negation overflow of i32::MIN.
    assert_invalid(cs2lonlatstr(i32::MIN, b'E', b'W'), "cs2lonlatstr(MIN)");
    assert_invalid(cs2timestr(i32::MIN, b':', false), "cs2timestr(MIN)");
    assert_invalid(cs2degstr(i32::MIN), "cs2degstr(MIN)");
    // Ordinary values still format.
    assert_eq!(cs2lonlatstr(360000, b'N', b'S').expect("lonlat"), "1N00");
}

#[test]
fn calendar_decomposition_rejects_unrepresentable_jd() {
    // Proven UBSan case: float-to-integer conversion on NaN.
    assert_invalid(
        swisseph_bindings::revjul(f64::NAN, Calendar::Gregorian),
        "revjul(NaN)",
    );
    assert_invalid(
        swisseph_bindings::revjul(1e100, Calendar::Gregorian),
        "revjul(1e100)",
    );
    assert_invalid(day_of_week(f64::NAN), "day_of_week(NaN)");
    assert_invalid(
        jdet_to_utc(f64::NAN, Calendar::Gregorian),
        "jdet_to_utc(NaN)",
    );
    assert_invalid(
        jdut1_to_utc(f64::INFINITY, Calendar::Gregorian),
        "jdut1_to_utc(inf)",
    );
    // Ordinary and BCE dates still convert.
    let date = swisseph_bindings::revjul(2451545.0, Calendar::Gregorian).expect("reverse J2000");
    assert_eq!((date.year, date.month, date.day), (2000, 1, 1));
    assert_eq!(day_of_week(2451545.0).expect("weekday"), 5);
}

#[test]
fn time_entry_points_reject_non_finite_fields() {
    assert_invalid(
        utc_to_jd(2000, 1, 1, 12, 0, f64::NAN, Calendar::Gregorian),
        "utc_to_jd NaN second",
    );
    assert_invalid(
        utc_time_zone(2000, 1, 1, 12, 0, f64::NAN, 2.0),
        "utc_time_zone NaN second",
    );
    assert_invalid(
        utc_time_zone(2000, 1, 1, 12, 0, 0.0, f64::INFINITY),
        "utc_time_zone inf tz",
    );
    assert_invalid(
        date_conversion(2000, 1, 1, f64::NAN, Calendar::Gregorian),
        "date_conversion NaN hour",
    );
    assert_invalid(deltat(f64::NAN), "deltat(NaN)");
    assert_invalid(time_equ(f64::NAN), "time_equ(NaN)");
    assert_invalid(lmt_to_lat(f64::NAN, 0.0), "lmt_to_lat(NaN)");
    assert_invalid(lat_to_lmt(2451545.0, f64::NAN), "lat_to_lmt(NaN lon)");
    assert_invalid(sidtime(f64::NAN), "sidtime(NaN)");
    // A valid conversion still succeeds.
    let days = utc_to_jd(2000, 1, 1, 12, 0, 0.0, Calendar::Gregorian).expect("j2000 utc");
    assert!((days.jd_ut - 2451545.0).abs() < 0.001);
}

#[test]
fn utc_seconds_cannot_overflow_native_diagnostics() {
    for cal in [Calendar::Julian, Calendar::Gregorian] {
        for second in [
            f64::MAX,
            -f64::MAX,
            1e100,
            -f64::MIN_POSITIVE,
            61.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            assert_invalid(
                utc_to_jd(2000, 1, 1, 12, 0, second, cal),
                "UTC seconds outside the safe diagnostic domain",
            );
        }
        for second in [-0.0, 0.0, f64::MIN_POSITIVE, 59.999999] {
            let days = utc_to_jd(2000, 1, 1, 12, 0, second, cal)
                .expect("ordinary seconds remain accepted");
            assert!(days.jd_et.is_finite() && days.jd_ut.is_finite());
        }
        // Ordinary native invalid-date/clock diagnostics remain native errors,
        // including the maximum decimal width of each remaining clock field.
        for (month, day, hour, minute) in [
            (2, 30, 12, 0),
            (1, 1, 24, 0),
            (1, 1, 12, 60),
            (1, 1, i32::MIN, i32::MIN),
            (1, 1, i32::MAX, i32::MAX),
        ] {
            let error = utc_to_jd(2000, month, day, hour, minute, 60.5, cal)
                .expect_err("native date/clock validation");
            assert_eq!(error.kind(), ErrorKind::Native);
            assert!(!error.message().is_empty());
        }
    }
    for second in [60.0, 60.5, f64::from_bits(61.0_f64.to_bits() - 1)] {
        for (day, cal) in [(31, Calendar::Gregorian), (18, Calendar::Julian)] {
            utc_to_jd(2016, 12, day, 23, 59, second, cal)
                .expect("valid fractional leap seconds remain accepted");
        }
        let error = utc_to_jd(2016, 6, 15, 23, 59, second, Calendar::Gregorian)
            .expect_err("no leap second on this date");
        assert_eq!(error.kind(), ErrorKind::Native);
    }
}

#[test]
fn position_and_house_calls_reject_non_finite_jd() {
    assert_invalid(calc_ut(f64::NAN, MOON, FLG_MOSEPH), "calc_ut(NaN)");
    assert_invalid(houses(f64::NAN, 51.5, -0.12, b'P'), "houses(NaN jd)");
    assert_invalid(houses(2451545.0, f64::NAN, -0.12, b'P'), "houses(NaN lat)");
    // A valid Moshier calculation still succeeds.
    let moon = calc_ut(2451545.0, MOON, FLG_MOSEPH | FLG_SPEED).expect("moon");
    assert!((0.0..360.0).contains(&moon.longitude()));
}

#[test]
fn invalid_setters_preserve_previous_valid_state() {
    // Install a known-valid observer and take a serial baseline.
    set_topo(12.5, 41.9, 0.0).expect("valid topo");
    let baseline = calc_ut(2451545.0, MOON, FLG_MOSEPH | FLG_TOPOCTR | FLG_SPEED)
        .expect("topocentric baseline");
    // A NaN setter must fail without mutating native state.
    assert_invalid(set_topo(f64::NAN, 41.9, 0.0), "set_topo(NaN)");
    let after = calc_ut(2451545.0, MOON, FLG_MOSEPH | FLG_TOPOCTR | FLG_SPEED)
        .expect("topocentric after rejected setter");
    assert_eq!(
        baseline.values, after.values,
        "rejected set_topo must preserve the previous observer"
    );
    assert!(after.values.iter().all(|v| v.is_finite()));

    // Same discipline for the user sidereal parameters.
    set_sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0).expect("default sidereal");
    assert_invalid(
        set_sid_mode(SIDM_USER, 2451545.0, f64::NAN),
        "set_sid_mode(NaN)",
    );

    // Restore process defaults for tidiness.
    set_topo(0.0, 0.0, 0.0).expect("restore default observer");
    set_sid_mode(SIDM_FAGAN_BRADLEY, 0.0, 0.0).expect("restore default sidereal");
}
