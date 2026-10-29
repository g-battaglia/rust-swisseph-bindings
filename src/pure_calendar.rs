//! Typed, configuration-independent calendar results for the primary API.
//! Legacy scalar signatures remain in `native`; no calendar formula is copied.

use crate::{Calendar, CalendarDate, Error};

/// Julian Day from pure calendar arithmetic, without an assigned time scale.
///
/// This producer has no status, diagnostic or source channel. Converting a
/// calendar label does not establish whether it denotes UTC, UT1 or TT.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JulianDay {
    /// Julian Day in days, including the fractional day; never rounded here.
    pub value: f64,
}

/// Validation and normalized calendar fields for one civil date.
///
/// Invalid dates are outcomes, not calculation errors. The normalized date is
/// obtained from the backend's inverse calendar conversion of `julian_day`.
#[derive(Debug, Clone, PartialEq)]
pub struct DateConversion {
    /// Whether the backend accepts the original year/month/day label.
    pub valid: bool,
    /// Actual normalized Julian Day in days, without an assigned time scale.
    pub julian_day: f64,
    /// Inverse calendar conversion, including the fractional hour.
    pub date: CalendarDate,
    /// Actual producer status, or `None` when no integer channel exists.
    pub status: Option<i32>,
    /// Original producer diagnostic, or `None` when no text channel exists.
    pub diagnostic: Option<String>,
}

/// Weekday result from pure calendar arithmetic, with no producer metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weekday {
    /// Backend weekday index: Monday is zero and Sunday is six.
    pub index: i32,
}

/// Convert a calendar label to a typed Julian Day, without time-scale changes.
///
/// `hour` is decimal hours; years use astronomical numbering and the calendar
/// is proleptic. Month/day overflow follows the backend recurrence, not civil
/// date validation. No configuration or ephemeris data is consumed.
///
/// # Errors
/// Rejects a non-finite hour. The legacy numerical producer is otherwise called
/// unchanged; use [`date_conversion`] to obtain civil-date validity as well.
pub fn julday(
    year: i32,
    month: i32,
    day: i32,
    hour: f64,
    calendar: Calendar,
) -> Result<JulianDay, Error> {
    if !hour.is_finite() {
        return Err(Error::invalid_input(format!(
            "julday: hour must be finite ({hour})"
        )));
    }
    Ok(JulianDay {
        value: crate::time::julday(year, month, day, hour, calendar),
    })
}

/// Validate a calendar label while retaining its normalized numerical result.
///
/// Decimal hours and astronomical years use the explicit proleptic calendar.
/// No reform switch or time-scale conversion is applied. An impossible civil
/// date returns `valid=false` with the actual C status and normalized fields;
/// no diagnostic is invented for this text-free native entry.
///
/// # Errors
/// Rejects non-finite hours and normalized dates outside the backend's safe
/// inverse-calendar domain before invoking native date validation. Native
/// locking failures propagate. No ephemeris data or session is required.
pub fn date_conversion(
    year: i32,
    month: i32,
    day: i32,
    hour: f64,
    calendar: Calendar,
) -> Result<DateConversion, Error> {
    let input = julday(year, month, day, hour, calendar)?;
    crate::time::check_calendar_jd("date_conversion", input.value, calendar)?;
    let letter = match calendar {
        Calendar::Julian => b'j',
        Calendar::Gregorian => b'g',
    };
    let (status, julian_day) = crate::state::with_native_access(|| {
        let mut value = 0.0;
        // SAFETY: the checked domain bounds native calendar integer casts.
        // `value` is one initialized, writable double that outlives the call.
        let status = unsafe {
            crate::ffi::swe_date_conversion(
                year,
                month,
                day,
                hour,
                letter as std::ffi::c_char,
                &mut value,
            )
        };
        Ok((status, value))
    })?;
    Ok(DateConversion {
        valid: status >= 0,
        julian_day,
        date: crate::time::revjul(julian_day, calendar)?,
        status: Some(status),
        diagnostic: None,
    })
}

/// Return the weekday of a Julian Day, without assigning a time scale.
///
/// Monday is zero through Sunday six. No mutable configuration is consulted.
///
/// # Errors
/// Rejects non-finite input and dates whose native weekday offset cannot fit
/// `i32`; this is the original native admission domain, not a calendar-year
/// limit. The actual weekday calculation is unchanged.
pub fn day_of_week(julian_day: f64) -> Result<Weekday, Error> {
    crate::time::day_of_week(julian_day).map(|index| Weekday { index })
}
