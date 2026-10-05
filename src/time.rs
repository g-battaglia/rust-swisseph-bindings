//! Calendar and Julian Day conversions (TIME-01) plus UTC/UT/TT,
//! Delta T, equation of time and sidereal time (TIME-02/03).
//!
//! [`julday`] converts a civil calendar date to a Julian Day number and
//! [`revjul`] converts back. [`utc_to_jd`], [`jdet_to_utc`] and
//! [`jdut1_to_utc`] handle the UTC leap-second scale, [`deltat`] and
//! [`deltat_ex`] expose the TT-minus-UT offset, [`time_equ`] the equation
//! of time, and [`sidtime`]/[`sidtime0`] Greenwich sidereal time. All call
//! the pinned native implementation, so boundary behavior is the native
//! behavior, not a Rust reimplementation.

use std::ffi::{c_char, c_int};

use crate::error::Error;
use crate::ffi::{self, SERR_LEN, read_native_text};
use crate::state::with_native_access;

/// Calendar system for [`julday`] and [`revjul`].
///
/// Rust has no keyword defaults, so the calendar is always explicit; there
/// is no hidden assumption about Gregorian versus Julian reckoning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Calendar {
    /// Julian calendar (`0` on the native side).
    Julian,
    /// Proleptic Gregorian calendar (`1` on the native side).
    ///
    /// Proleptic means the Gregorian rules apply to every date, including
    /// dates before the 1582 reform: there is no automatic Julian/Gregorian
    /// switch driven by the Julian Day value.
    Gregorian,
}

impl Calendar {
    /// Native flag value for this calendar system.
    fn flag(self) -> c_int {
        match self {
            Calendar::Julian => 0,
            Calendar::Gregorian => 1,
        }
    }

    /// Native `swe_date_conversion` calendar letter.
    fn letter(self) -> c_char {
        match self {
            Calendar::Julian => b'j' as c_char,
            Calendar::Gregorian => b'g' as c_char,
        }
    }
}

/// Civil calendar date with a fractional-hour component, as returned by
/// [`revjul`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalendarDate {
    /// Astronomical year numbering: year 0 is 1 BCE, negative years are BCE.
    pub year: i32,
    /// Month of year, 1-12 for well-formed inputs.
    pub month: i32,
    /// Integer day of month for well-formed inputs.
    pub day: i32,
    /// Decimal hour of day, 0.0 (inclusive) to 24.0 (exclusive).
    pub hour: f64,
}

/// Convert a calendar date to a Julian Day number.
///
/// `hour` is decimal time in hours (12.0 is noon). The conversion is pure
/// calendar arithmetic performed by the native library; out-of-range
/// month/day values flow through the native recurrence exactly as passed,
/// so date validation belongs to the caller.
///
/// # Units
///
/// The result is a Julian Day number in days; JD 0.0 is noon on 1 January
/// 4713 BCE (Julian proleptic). JD 2451545.0 is 1 January 2000, 12:00.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{Calendar, julday};
///
/// // J2000.0: 1 January 2000, 12:00.
/// assert_eq!(julday(2000, 1, 1, 12.0, Calendar::Gregorian), 2451545.0);
/// ```
pub fn julday(year: i32, month: i32, day: i32, hour: f64, cal: Calendar) -> f64 {
    // `swe_julday` is a pure function of its arguments: it reads no native
    // configuration, so no lock scoping beyond the call itself is needed.
    // A non-finite `hour` propagates to a non-finite result, mirroring the
    // native arithmetic; it is not silently clamped.
    // SAFETY: pure FFI call, no pointers involved.
    unsafe { ffi::swe_julday(year, month, day, hour, cal.flag()) }
}

/// Check the inverse calendar domain using the native forward conversion.
/// Only the decomposed year is a large integer conversion in `swe_revjul`;
/// days are doubles. The upper bound is the midnight after `i32::MAX`'s
/// last day, expressed without overflowing a Rust or C year argument.
pub(crate) fn check_calendar_jd(operation: &str, jd: f64, cal: Calendar) -> Result<(), Error> {
    let first = julday(i32::MIN, 1, 1, 0.0, cal);
    let end = julday(i32::MAX, 12, 31, 24.0, cal);
    if !jd.is_finite() || jd < first || jd >= end {
        return Err(Error::invalid_input(format!(
            "{operation}: Julian Day must be finite with a calendar year representable as i32 ({jd})"
        )));
    }
    Ok(())
}

/// Decompose a Julian Day while native access is held. The checked domain
/// also bounds the native month/day conversions and fractional hour.
fn revjul_locked(jd: f64, cal: Calendar) -> Result<CalendarDate, Error> {
    check_calendar_jd("revjul", jd, cal)?;
    let mut year: c_int = 0;
    let mut month: c_int = 0;
    let mut day: c_int = 0;
    let mut hour = 0.0;
    // SAFETY: the calendar-year bounds above keep native integer casts
    // representable. Each pointer addresses one owned, initialized local
    // that outlives the call; the caller holds native access.
    unsafe {
        ffi::swe_revjul(jd, cal.flag(), &mut year, &mut month, &mut day, &mut hour);
    }
    Ok(CalendarDate {
        year,
        month,
        day,
        hour,
    })
}

/// Validate the pinned UTC routines' signed `year * 10000 + month * 100
/// + day` operations. This is a native leap-table lookup key, independent
/// of whether the output calendar year itself fits `i32`.
fn check_leap_date(operation: &str, date: CalendarDate) -> Result<(), Error> {
    if date
        .year
        .checked_mul(10000)
        .and_then(|year| {
            date.month
                .checked_mul(100)
                .and_then(|month| year.checked_add(month))
        })
        .and_then(|value| value.checked_add(date.day))
        .is_none()
    {
        return Err(Error::invalid_input(format!(
            "{operation}: native Gregorian YYYYMMDD leap-table key must fit i32"
        )));
    }
    Ok(())
}

/// Native Delta T used by the UTC conversions, with their exact source
/// selector and no diagnostic buffer. Caller holds native access.
fn utc_delta_t_locked(jd: f64) -> Result<f64, Error> {
    if !jd.is_finite() {
        return Err(Error::invalid_input(
            "UTC conversion: non-finite intermediate Julian Day",
        ));
    }
    // SAFETY: finite scalar input, null diagnostic is supported by this
    // ABI, and the caller holds the native state lock.
    let dt = unsafe { ffi::swe_deltat_ex(jd, -1, std::ptr::null_mut()) };
    if !dt.is_finite() {
        return Err(Error::invalid_input("UTC conversion: non-finite Delta T"));
    }
    Ok(dt)
}

/// Check the actual calendar casts and integer leap-table key reached by
/// `swe_jdet_to_utc`. Only scalar time-scale offsets are evaluated here;
/// native Swiss still performs every conversion and leap-second lookup.
/// All checks and the eventual computation must share native access so a
/// concurrent Delta-T setter cannot invalidate this preflight.
fn check_utc_et_locked(operation: &str, jd_et: f64, cal: Calendar) -> Result<(), Error> {
    let d0 = utc_delta_t_locked(jd_et)?;
    let ut_first = jd_et - utc_delta_t_locked(jd_et - d0)?;
    let ut_final = jd_et - utc_delta_t_locked(ut_first)?;
    // Native pre-1972 branch only decomposes final UT1. TT and the
    // earlier UT estimate need to be finite, but are never decomposed
    // here and therefore need no calendar-year bounds in this branch.
    let utc_start = julday(1972, 1, 1, 0.0, Calendar::Gregorian) + 42.184 / 86400.0;
    if jd_et < utc_start {
        return check_calendar_jd(operation, ut_final, cal);
    }
    // After 1972 the native code first encodes the Gregorian day before
    // the final UT estimate, even if it later falls back to UT1.
    check_leap_date(
        operation,
        revjul_locked(ut_final - 1.0, Calendar::Gregorian)?,
    )?;
    // UTC clock decomposition lies between TT and TT minus one day;
    // the table's bounded leap count is much less than 86400 seconds.
    check_calendar_jd(operation, jd_et, Calendar::Gregorian)?;
    check_calendar_jd(operation, jd_et - 1.0, Calendar::Gregorian)?;
    check_calendar_jd(operation, ut_first, Calendar::Gregorian)?;
    // The native Julian-output branch converts the resulting Gregorian
    // civil midnight, so these same dates must fit the selected calendar.
    check_calendar_jd(operation, jd_et - 1.0, cal)?;
    check_calendar_jd(operation, ut_first - 1.0, cal)
}

/// Convert a Julian Day number back to a civil calendar date.
///
/// Returns [`CalendarDate`] with the astronomical year numbering used by
/// [`julday`] (year 0 is 1 BCE). The Gregorian calendar is proleptic: no
/// automatic reform switchover is applied.
///
/// # Errors
///
/// Non-finite inputs and magnitudes beyond the `i32`-representable
/// calendar range are rejected with
/// [`ErrorKind::InvalidInput`](crate::ErrorKind) before any native call:
/// the native decomposition performs floating-to-integer conversions with
/// no range check. The admissible domain runs from midnight on January 1
/// of year `i32::MIN` through year `i32::MAX`, ending at the following
/// midnight (exclusive), in the selected calendar. Bounds are obtained
/// with [`julday`]; the Julian Day itself does not need to fit `i32`.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{Calendar, revjul};
///
/// let date = revjul(2451545.0, Calendar::Gregorian)?;
/// assert_eq!((date.year, date.month, date.day, date.hour), (2000, 1, 1, 12.0));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn revjul(jd: f64, cal: Calendar) -> Result<CalendarDate, crate::Error> {
    check_calendar_jd("revjul", jd, cal)?;
    with_native_access(|| revjul_locked(jd, cal))
}

/// Julian Day pair produced by [`utc_to_jd`]: Terrestrial Time and
/// Universal Time for the same UTC calendar instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JulDays {
    /// Julian Day in Terrestrial Time (Ephemeris Time of the data).
    pub jd_et: f64,
    /// Julian Day in Universal Time (UT1).
    pub jd_ut: f64,
}

/// UTC calendar instant with split clock fields, as used by [`utc_to_jd`]
/// and returned by [`jdet_to_utc`], [`jdut1_to_utc`] and [`utc_time_zone`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UtcDateTime {
    /// Astronomical year numbering: year 0 is 1 BCE.
    pub year: i32,
    /// Month of year, 1-12 for well-formed inputs.
    pub month: i32,
    /// Day of month for well-formed inputs.
    pub day: i32,
    /// Hour of day, 0-23 for well-formed inputs.
    pub hour: i32,
    /// Minute of hour, 0-59 for well-formed inputs.
    pub minute: i32,
    /// Seconds with fraction; 60.0-60.999... only on a leap-second UTC.
    pub second: f64,
}

/// Delta T value with its native diagnostic text.
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaT {
    /// TT minus UT in days.
    pub value: f64,
    /// Native diagnostic text; empty on a clean computation, informational
    /// otherwise (for example a missing-data notice). It never turns a
    /// returned value into a failure: the value is always usable.
    pub diagnostic: String,
}

/// Convert a UTC calendar date to Julian Days.
///
/// Unlike [`julday`] (which takes a UT1 decimal hour), this function takes
/// UTC clock fields and correctly accounts for leap seconds: it returns
/// both the Terrestrial Time (`jd_et`) and the Universal Time (`jd_ut`)
/// Julian Day. Before 1972 — when UTC with leap seconds did not exist —
/// the native engine treats the input as UT1 directly.
///
/// Seconds must be finite and in `0.0..61.0`; otherwise the binding returns
/// `InvalidInput` before native access. This bounds the pinned engine's
/// invalid-time diagnostic, which formats the seconds into a fixed buffer.
/// Native validation still rejects impossible dates (month 13, February 30)
/// and invalid hours/minutes with `Native`. A `60.x` second is accepted only
/// at 23:59 on a UTC date that actually carries a leap second.
/// Inputs whose normalized calendar year or native Gregorian `YYYYMMDD`
/// leap-table key cannot fit `i32` are rejected with `InvalidInput`.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{Calendar, utc_to_jd};
///
/// // J2000.0 midday UTC: TT runs about a minute ahead of UT1.
/// let days = utc_to_jd(2000, 1, 1, 12, 0, 0.0, Calendar::Gregorian)?;
/// assert!((days.jd_ut - 2451545.0).abs() < 0.001);
/// assert!(days.jd_et > days.jd_ut);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn utc_to_jd(
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: f64,
    cal: Calendar,
) -> Result<JulDays, Error> {
    if !second.is_finite() || !(0.0..61.0).contains(&second) {
        return Err(Error::invalid_input(format!(
            "utc_to_jd: second must be finite and in 0.0..61.0 ({second})"
        )));
    }
    with_native_access(|| {
        let date_jd = julday(year, month, day, 0.0, cal);
        check_calendar_jd("utc_to_jd", date_jd, cal)?;
        if date_jd >= julday(1972, 1, 1, 0.0, Calendar::Gregorian) {
            check_leap_date("utc_to_jd", revjul_locked(date_jd, Calendar::Gregorian)?)?;
        }
        let mut dret = [0.0; 2];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `dret` owns 2 writable slots and `serr` owns SERR_LEN
        // writable bytes; both outlive the call and the lock excludes
        // every other native access. Results are only exposed after the
        // OK/ERR status is checked. Calendar casts and the leap-table
        // integer key were checked above under the same lock. Seconds are
        // bounded to 0..61 before access: their fixed-point diagnostic cannot
        // expand to hundreds of digits; the remaining clock/date fields are i32.
        let ret = unsafe {
            ffi::swe_utc_to_jd(
                year,
                month,
                day,
                hour,
                minute,
                second,
                cal.flag(),
                dret.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(if diagnostic.is_empty() {
                format!(
                    "utc_to_jd: native conversion failed ({year}-{month:02}-{day:02} \
                     {hour:02}:{minute:02}:{second})"
                )
            } else {
                format!("utc_to_jd: {diagnostic}")
            }))
        } else {
            Ok(JulDays {
                jd_et: dret[0],
                jd_ut: dret[1],
            })
        }
    })
}

/// Split an Ephemeris/Terrestrial Time Julian Day into UTC calendar fields.
///
/// This is the inverse of [`utc_to_jd`] on the TT branch, including the
/// leap-second shift: a `jd_et` that falls inside a leap second reports a
/// `60.x` second. Non-finite inputs or intermediates, unrepresentable
/// calendar years actually decomposed by the native branch, and
/// overflowing Gregorian `YYYYMMDD` leap-table keys are rejected with
/// `InvalidInput`. Before the native 1972 TT cutoff, only the final UT1
/// value after configured Delta-T shifts needs a representable calendar
/// year; TT itself may lie outside [`revjul`]'s calendar domain. The
/// checks run under the native lock with the computation; lock poisoning
/// also returns an error.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{Calendar, jdet_to_utc, utc_to_jd};
///
/// let days = utc_to_jd(2000, 1, 1, 12, 0, 0.0, Calendar::Gregorian)?;
/// let back = jdet_to_utc(days.jd_et, Calendar::Gregorian)?;
/// assert_eq!((back.year, back.month, back.day, back.hour, back.minute), (2000, 1, 1, 12, 0));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn jdet_to_utc(jd_et: f64, cal: Calendar) -> Result<UtcDateTime, Error> {
    with_native_access(|| {
        check_utc_et_locked("jdet_to_utc", jd_et, cal)?;
        let mut fields = [0i32; 5];
        let mut second = 0.0;
        // SAFETY: each pointer addresses exactly one owned local; the
        // native function writes each one exactly once and never fails.
        // The preflight above bounds its calendar casts and integer
        // leap-table key with the same Delta-T configuration.
        unsafe {
            ffi::swe_jdet_to_utc(
                jd_et,
                cal.flag(),
                &mut fields[0],
                &mut fields[1],
                &mut fields[2],
                &mut fields[3],
                &mut fields[4],
                &mut second,
            );
        }
        Ok(UtcDateTime {
            year: fields[0],
            month: fields[1],
            day: fields[2],
            hour: fields[3],
            minute: fields[4],
            second,
        })
    })
}

/// Split a Universal Time Julian Day into UTC calendar fields.
///
/// Identical to [`jdet_to_utc`] except that the input is UT1 (`jd_ut`).
/// Round-tripping [`utc_to_jd`] through this function recovers the input
/// clock fields exactly, including a `60.x` leap second.
pub fn jdut1_to_utc(jd_ut: f64, cal: Calendar) -> Result<UtcDateTime, Error> {
    with_native_access(|| {
        let jd_et = jd_ut + utc_delta_t_locked(jd_ut)?;
        check_utc_et_locked("jdut1_to_utc", jd_et, cal)?;
        let mut fields = [0i32; 5];
        let mut second = 0.0;
        // SAFETY: same contract as in [`jdet_to_utc`].
        unsafe {
            ffi::swe_jdut1_to_utc(
                jd_ut,
                cal.flag(),
                &mut fields[0],
                &mut fields[1],
                &mut fields[2],
                &mut fields[3],
                &mut fields[4],
                &mut second,
            );
        }
        Ok(UtcDateTime {
            year: fields[0],
            month: fields[1],
            day: fields[2],
            hour: fields[3],
            minute: fields[4],
            second,
        })
    })
}

/// Shift a UTC calendar instant by a time-zone offset.
///
/// `timezone_hours` is fractional hours east of Greenwich (negative west
/// of Greenwich); the output is the corresponding UTC instant in the same
/// split-field layout. Calendar arithmetic remains native. Non-finite
/// clock values, clock intermediates outside `i32`, or a shifted Gregorian
/// calendar year outside `i32` return `InvalidInput` before FFI.
pub fn utc_time_zone(
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: f64,
    timezone_hours: f64,
) -> Result<UtcDateTime, Error> {
    if !second.is_finite() || !timezone_hours.is_finite() {
        return Err(Error::invalid_input(
            "utc_time_zone: second and timezone offset must be finite",
        ));
    }
    // The native clock splits convert hours to int, then split the
    // fractional remainder into minutes/seconds. Leap seconds subtract
    // one second before this split and restore it afterwards.
    let clock_second = if second >= 60.0 { second - 1.0 } else { second };
    let local_hour = f64::from(hour) + f64::from(minute) / 60.0 + clock_second / 3600.0;
    let shifted_hour = local_hour - timezone_hours;
    let day_shift = if shifted_hour < 0.0 {
        -1.0
    } else if shifted_hour >= 24.0 {
        1.0
    } else {
        0.0
    };
    let split_hour = shifted_hour - day_shift * 24.0;
    for value in [local_hour, shifted_hour, split_hour] {
        if !value.is_finite()
            || value.trunc() < f64::from(i32::MIN)
            || value.trunc() > f64::from(i32::MAX)
        {
            return Err(Error::invalid_input(
                "utc_time_zone: clock intermediate must fit i32",
            ));
        }
    }
    let midnight = julday(year, month, day, 0.0, Calendar::Gregorian);
    check_calendar_jd("utc_time_zone", midnight + day_shift, Calendar::Gregorian)?;
    with_native_access(|| {
        let mut out = [0i32; 5];
        let mut second_out = 0.0;
        // SAFETY: each out pointer addresses exactly one owned local; the
        // native function writes each one exactly once. The preflight bounds
        // its clock casts and shifted Gregorian calendar year.
        unsafe {
            ffi::swe_utc_time_zone(
                year,
                month,
                day,
                hour,
                minute,
                second,
                timezone_hours,
                &mut out[0],
                &mut out[1],
                &mut out[2],
                &mut out[3],
                &mut out[4],
                &mut second_out,
            );
        }
        Ok(UtcDateTime {
            year: out[0],
            month: out[1],
            day: out[2],
            hour: out[3],
            minute: out[4],
            second: second_out,
        })
    })
}

/// Convert a calendar date to a Julian Day number with native validation.
///
/// Unlike [`julday`] (which normalizes out-of-range month/day values
/// through the native recurrence), this function checks the date: an
/// impossible calendar date fails with [`Error`] instead of rolling over.
/// The calendar is explicit; there is no automatic Julian/Gregorian
/// switch.
/// Non-finite hours and normalized Julian Days whose calendar year cannot
/// fit `i32` are rejected with `InvalidInput` before native validation.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{Calendar, date_conversion};
///
/// assert_eq!(date_conversion(2000, 1, 1, 12.0, Calendar::Gregorian)?, 2451545.0);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn date_conversion(
    year: i32,
    month: i32,
    day: i32,
    hour: f64,
    cal: Calendar,
) -> Result<f64, Error> {
    if !hour.is_finite() {
        return Err(Error::invalid_input(format!(
            "date_conversion: hour must be finite ({hour})"
        )));
    }
    check_calendar_jd("date_conversion", julday(year, month, day, hour, cal), cal)?;
    with_native_access(|| {
        let mut tjd = 0.0;
        // SAFETY: `tjd` is one owned slot written exactly once; the call
        // takes no other pointers. The result is exposed only after the
        // OK/ERR status is checked.
        let ret =
            unsafe { ffi::swe_date_conversion(year, month, day, hour, cal.letter(), &mut tjd) };
        if ret < 0 {
            Err(Error::native(format!(
                "date_conversion: invalid date {year}-{month:02}-{day:02} ({cal:?} calendar)"
            )))
        } else {
            Ok(tjd)
        }
    })
}

/// Weekday of a Julian Day number: Monday is 0 through Sunday 6.
///
/// Pure calendar arithmetic over the admissible Julian Day domain.
///
/// # Errors
///
/// Non-finite inputs and days whose native offset cannot fit `i32`
/// are rejected with
/// [`ErrorKind::InvalidInput`](crate::ErrorKind) before any native call:
/// the native weekday step performs floating-to-integer conversions with
/// no range check. Specifically, `floor(jd - 2433282.0 - 1.5)` must fit
/// `i32`, with the native subtraction order preserved.
/// This is a different domain from [`revjul`]'s calendar-year limits.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::day_of_week;
///
/// assert_eq!(day_of_week(2451545.0)?, 5);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn day_of_week(jd: f64) -> Result<i32, Error> {
    // The pinned native ABI utility casts this offset before taking % 7.
    let offset = (jd - 2433282.0 - 1.5).floor();
    if !offset.is_finite() || offset < f64::from(i32::MIN) || offset > f64::from(i32::MAX) {
        return Err(Error::invalid_input(format!(
            "day_of_week: floor(Julian Day - 2433282.0 - 1.5) must fit i32 ({jd})"
        )));
    }
    with_native_access(|| {
        // SAFETY: pure value call, no pointers involved. The domain check
        // above keeps the native floating-to-integer conversion in-range.
        Ok(unsafe { ffi::swe_day_of_week(jd) })
    })
}

/// Read Delta T while native access is already held.
///
/// Caller must hold the native lock; `jd_ut` must already be validated
/// as finite.
pub(crate) fn deltat_locked(jd_ut: f64) -> f64 {
    // SAFETY: pure value call that reads shared Delta T state; the caller
    // holds the native lock.
    unsafe { ffi::swe_deltat(jd_ut) }
}

/// Delta T (Terrestrial Time minus Universal Time) in days.
///
/// Uses the natively configured tidal acceleration and ephemeris guess,
/// including a user-defined override when one is set. Reads shared native
/// state, so the read is serialized. Non-finite dates return `InvalidInput`.
///
/// At J2000 the value is about 64 seconds (roughly 0.00074 days).
///
/// # Examples
///
/// ```
/// use swisseph_bindings::deltat;
///
/// let dt = deltat(2451545.0)?;
/// assert!(dt > 0.0005 && dt < 0.001);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn deltat(jd_ut: f64) -> Result<f64, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(format!(
            "deltat: Julian Day must be finite ({jd_ut})"
        )));
    }
    with_native_access(|| Ok(deltat_locked(jd_ut)))
}

/// Delta T in days with an explicit ephemeris-selection flag.
///
/// `flags` carries the source bits (`FLG_SWIEPH`, `FLG_JPLEPH`,
/// `FLG_MOSEPH`); the flag selects the tidal acceleration for this
/// computation only and never changes the stored configuration. The
/// returned [`DeltaT`] keeps the native diagnostic alongside the always
/// usable value; a non-empty diagnostic is a warning, not a failure.
pub fn deltat_ex(jd_ut: f64, flags: i32) -> Result<DeltaT, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(format!(
            "deltat_ex: Julian Day must be finite ({jd_ut})"
        )));
    }
    with_native_access(|| {
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `serr` owns SERR_LEN writable bytes and outlives the
        // call; the lock excludes every other native access.
        let value = unsafe { ffi::swe_deltat_ex(jd_ut, flags, serr.as_mut_ptr()) };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        Ok(DeltaT { value, diagnostic })
    })
}

/// Equation of time with its native diagnostic text.
#[derive(Debug, Clone, PartialEq)]
pub struct EquationOfTime {
    /// Apparent solar time minus mean solar time, in days.
    pub value: f64,
    /// Native diagnostic text; carries the data-source warning when the
    /// solar position fell back to the analytical model. A non-empty
    /// diagnostic with `Ok` is a warning, never a failure.
    pub diagnostic: String,
}

/// Converted local-time Julian Day with its native diagnostic text.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalTime {
    /// Converted Julian Day (apparent time for [`lmt_to_lat`], mean time
    /// for [`lat_to_lmt`]).
    pub jd: f64,
    /// Native diagnostic text; see [`EquationOfTime::diagnostic`].
    pub diagnostic: String,
}

/// Equation of time in days.
///
/// The value is apparent solar time minus mean solar time for the given
/// Julian Day; it oscillates within roughly ±17 minutes (±0.012 days)
/// over the year. The underlying solar position needs ephemeris data, so
/// the returned [`EquationOfTime`] keeps the native diagnostic: without
/// data files the engine falls back to the analytical model and says so.
///
/// # Errors
///
/// Returns [`Error`] when the native call fails; the diagnostic is kept
/// in the message.
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
pub fn time_equ(jd_ut: f64) -> Result<EquationOfTime, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(format!(
            "time_equ: Julian Day must be finite ({jd_ut})"
        )));
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("time_equ", jd_ut, -1)?;
        let mut te = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `te` is one owned slot and `serr` owns SERR_LEN bytes;
        // both outlive the call. The result is exposed only after the
        // OK/ERR status is checked.
        let ret = unsafe { ffi::swe_time_equ(jd_ut, &mut te, serr.as_mut_ptr()) };
        // SAFETY: bounded read of our own buffer (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(if diagnostic.is_empty() {
                format!("time_equ: native computation failed (jd={jd_ut})")
            } else {
                format!("time_equ: {diagnostic} (jd={jd_ut})")
            }))
        } else {
            Ok(EquationOfTime {
                value: te,
                diagnostic,
            })
        }
    })
}

/// Convert between local mean time and local apparent time.
///
/// `longitude` is east-positive degrees. [`lmt_to_lat`] maps a Local Mean
/// Time Julian Day to Local Apparent Time; [`lat_to_lmt`] is the converse.
/// Both apply the equation of time at the given instant (hence the same
/// data dependence and diagnostic reporting as [`time_equ`]); failures
/// keep the native diagnostic.
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
pub fn lmt_to_lat(jd_lmt: f64, longitude: f64) -> Result<LocalTime, Error> {
    if !jd_lmt.is_finite() || !longitude.is_finite() {
        return Err(Error::invalid_input(
            "lmt_to_lat: Julian Day and longitude must be finite",
        ));
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("lmt_to_lat", jd_lmt - longitude / 360.0, -1)?;
        let mut out = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `out` is one owned slot and `serr` owns SERR_LEN bytes;
        // both outlive the call. The result is exposed only after the
        // OK/ERR status is checked.
        let ret = unsafe { ffi::swe_lmt_to_lat(jd_lmt, longitude, &mut out, serr.as_mut_ptr()) };
        // SAFETY: bounded read of our own buffer (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(if diagnostic.is_empty() {
                format!("lmt_to_lat: native conversion failed (jd={jd_lmt}, lon={longitude})")
            } else {
                format!("lmt_to_lat: {diagnostic} (jd={jd_lmt}, lon={longitude})")
            }))
        } else {
            Ok(LocalTime {
                jd: out,
                diagnostic,
            })
        }
    })
}

/// Converse of [`lmt_to_lat`]: local apparent time to local mean time.
///
/// See [`lmt_to_lat`] for units, data dependence and error behavior.
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
pub fn lat_to_lmt(jd_lat: f64, longitude: f64) -> Result<LocalTime, Error> {
    if !jd_lat.is_finite() || !longitude.is_finite() {
        return Err(Error::invalid_input(
            "lat_to_lmt: Julian Day and longitude must be finite",
        ));
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("lat_to_lmt", jd_lat - longitude / 360.0, -1)?;
        let mut out = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`lmt_to_lat`].
        let ret = unsafe { ffi::swe_lat_to_lmt(jd_lat, longitude, &mut out, serr.as_mut_ptr()) };
        // SAFETY: bounded read of our own buffer (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(if diagnostic.is_empty() {
                format!("lat_to_lmt: native conversion failed (jd={jd_lat}, lon={longitude})")
            } else {
                format!("lat_to_lmt: {diagnostic} (jd={jd_lat}, lon={longitude})")
            }))
        } else {
            Ok(LocalTime {
                jd: out,
                diagnostic,
            })
        }
    })
}

/// Mean sidereal time at Greenwich in hours (0-24) for a UT Julian Day.
///
/// Reads the native Delta T state, so the read is serialized.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::sidtime;
///
/// let st = sidtime(2451545.0)?;
/// assert!((0.0..24.0).contains(&st));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
pub fn sidtime(jd_ut: f64) -> Result<f64, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(format!(
            "sidtime: Julian Day must be finite ({jd_ut})"
        )));
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("sidtime", jd_ut, -1)?;
        // SAFETY: pure value call reading shared Delta T state; the lock
        // pairs the state read with the computation.
        Ok(unsafe { ffi::swe_sidtime(jd_ut) })
    })
}

/// Sidereal time in hours for explicit obliquity and nutation.
///
/// `obliquity_eps` is the true obliquity in degrees and `nutation_lon`
/// the nutation in longitude in degrees; both select the reference frame
/// of the result instead of the natively computed one used by [`sidtime`].
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
pub fn sidtime0(jd_ut: f64, obliquity_eps: f64, nutation_lon: f64) -> Result<f64, Error> {
    if !jd_ut.is_finite() || !obliquity_eps.is_finite() || !nutation_lon.is_finite() {
        return Err(Error::invalid_input(
            "sidtime0: Julian Day, obliquity and nutation must be finite",
        ));
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("sidtime0", jd_ut, -1)?;
        // SAFETY: pure value call; serialized for uniformity with
        // [`sidtime`], which reads shared Delta T state.
        Ok(unsafe { ffi::swe_sidtime0(jd_ut, obliquity_eps, nutation_lon) })
    })
}
