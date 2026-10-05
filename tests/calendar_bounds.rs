//! Calendar domains follow native integer conversions, not the JD magnitude.
//! Inputs and structural assertions only; no native output vectors stored.

use swisseph_bindings::{
    Calendar, ErrorKind, date_conversion, day_of_week, jdet_to_utc, jdut1_to_utc, julday, revjul,
    set_delta_t_userdef, utc_to_jd,
};

fn invalid(result: Result<impl std::fmt::Debug, swisseph_bindings::Error>) {
    assert_eq!(
        result.expect_err("unrepresentable input").kind(),
        ErrorKind::InvalidInput
    );
}

#[test]
fn inverse_calendar_accepts_representable_years_beyond_i32_days() {
    for cal in [Calendar::Julian, Calendar::Gregorian] {
        for year in [i32::MIN, -6000000, 6000000, i32::MAX] {
            for (month, day) in [(1, 1), (12, 31)] {
                let jd = julday(year, month, day, 12.0, cal);
                let date = revjul(jd, cal).expect("representable calendar year");
                assert_eq!(
                    (date.year, date.month, date.day, date.hour),
                    (year, month, day, 12.0)
                );
            }
        }
        let first = julday(i32::MIN, 1, 1, 0.0, cal);
        let end = julday(i32::MAX, 12, 31, 24.0, cal);
        assert_eq!(revjul(first, cal).expect("first midnight").year, i32::MIN);
        assert_eq!(
            revjul(end.next_down(), cal)
                .expect("last representable instant")
                .year,
            i32::MAX
        );
        invalid(revjul(first.next_down(), cal));
        invalid(revjul(end, cal));
        for jd in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1e100, 1e100] {
            invalid(revjul(jd, cal));
        }
        assert!(date_conversion(6000000, 1, 1, 12.0, cal).is_ok());
        invalid(date_conversion(2000, 1, 1, 1e100, cal));
    }
}

#[test]
fn weekday_checks_the_cast_offset_at_both_edges() {
    let first = f64::from(i32::MIN) + 2433283.5;
    let end = f64::from(i32::MAX) + 2433283.5 + 1.0;
    for jd in [first, end.next_down(), 2451545.0] {
        assert!((0..7).contains(&day_of_week(jd).expect("representable weekday offset")));
    }
    // One adjacent input rounds the intermediate subtraction back onto
    // i32::MIN; the next one is the first unrepresentable native offset.
    assert!(day_of_week(first.next_down()).is_ok());
    invalid(day_of_week(first.next_down().next_down()));
    invalid(day_of_week(end));
    // The old symmetric JD guard admitted this undefined native cast.
    invalid(day_of_week(-2147483647.0));
}

#[test]
fn utc_checks_shifted_years_and_native_leap_table_keys() {
    // This is the sole state-mutating test in this binary. Pin Delta T
    // to isolate integer representability from its astronomical model.
    set_delta_t_userdef(Some(0.0)).expect("pin Delta T");
    for cal in [Calendar::Julian, Calendar::Gregorian] {
        let old = julday(-6000000, 1, 1, 12.0, cal);
        for date in [jdet_to_utc(old, cal), jdut1_to_utc(old, cal)] {
            let date = date.expect("large BCE year still fits the pre-1972 path");
            assert_eq!((date.year, date.month, date.day), (-6000000, 1, 1));
        }
        let future = julday(6000000, 1, 1, 12.0, cal);
        invalid(jdet_to_utc(future, cal));
        invalid(jdut1_to_utc(future, cal));
        invalid(utc_to_jd(6000000, 1, 1, 12, 0, 0.0, cal));
    }
    let last_key = julday(214748, 12, 31, 12.0, Calendar::Gregorian);
    assert!(jdet_to_utc(last_key, Calendar::Gregorian).is_ok());
    assert!(jdut1_to_utc(last_key, Calendar::Gregorian).is_ok());
    assert!(utc_to_jd(214748, 12, 31, 12, 0, 0.0, Calendar::Gregorian).is_ok());
    let overflowing_key = julday(214749, 1, 2, 12.0, Calendar::Gregorian);
    invalid(jdet_to_utc(overflowing_key, Calendar::Gregorian));
    invalid(jdut1_to_utc(overflowing_key, Calendar::Gregorian));
    invalid(utc_to_jd(214749, 1, 1, 12, 0, 0.0, Calendar::Gregorian));
    // TT is outside both calendars, but the pre-1972 branch only
    // decomposes the representable final UT1 value. Compare with the
    // native calendar result in memory rather than storing native fields.
    let j2000 = 2451545.0;
    let baseline = [Calendar::Julian, Calendar::Gregorian]
        .map(|cal| revjul(j2000, cal).expect("native calendar baseline"));
    let shift = -1e12;
    set_delta_t_userdef(Some(shift)).expect("large negative override");
    for (cal, expected) in [Calendar::Julian, Calendar::Gregorian]
        .into_iter()
        .zip(baseline)
    {
        let shifted_tt = j2000 + shift;
        invalid(revjul(shifted_tt, cal));
        let via_ut = jdut1_to_utc(j2000, cal).expect("only final UT1 needs calendar decomposition");
        let via_tt =
            jdet_to_utc(shifted_tt, cal).expect("out-of-calendar TT with representable UT1");
        assert_eq!(via_ut, via_tt);
        assert_eq!(
            (via_ut.year, via_ut.month, via_ut.day),
            (expected.year, expected.month, expected.day)
        );
        assert_eq!(f64::from(via_ut.hour), expected.hour);
        assert_eq!((via_ut.minute, via_ut.second), (0, 0.0));
        // An unrepresentable final UT1 must still fail in this branch.
        invalid(jdet_to_utc(2.0 * shift, cal));
    }
    set_delta_t_userdef(Some(1e100)).expect("finite extreme override");
    invalid(jdet_to_utc(2451545.0, Calendar::Gregorian));
    invalid(jdut1_to_utc(2451545.0, Calendar::Gregorian));
    set_delta_t_userdef(None).expect("restore computed Delta T");
}
