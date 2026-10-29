//! Pure typed projections of native utilities; no numerical formula is copied.
use super::*;

fn native_integer(operation: &str, value: i64) -> Result<i32, Error> {
    i32::try_from(value).map_err(|_| {
        Error::invalid(
            operation,
            "centiseconds must fit the native signed 32-bit transport",
        )
    })
}
fn native_character(operation: &str, text: &str) -> Result<u8, Error> {
    let bytes = text.as_bytes();
    if bytes.len() == 1 && bytes[0].is_ascii() && bytes[0] != 0 {
        Ok(bytes[0])
    } else {
        Err(Error::invalid(
            operation,
            "separator must be exactly one non-NUL ASCII byte",
        ))
    }
}

/// Return the typed result of native `julday`.
///
/// Calendar years are astronomical and proleptic; Julian days and clock units are preserved.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `year` - Astronomical calendar year; zero denotes 1 BCE.
/// * `month` - Civil month label; native validation/normalization is retained.
/// * `day` - Civil day label; native validation/normalization is retained.
/// * `hour` - Decimal civil hours.
/// * `calendar` - Explicit proleptic Julian or Gregorian calendar.
///
/// # Returns
///
/// Complete JulianDay transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn julday(
    year: i32,
    month: i32,
    day: i32,
    hour: f64,
    calendar: Calendar,
) -> Result<JulianDay, Error> {
    let result = crate::pure_calendar::julday(year, month, day, hour, calendar.native())
        .map_err(Error::from_native)?;
    Ok(JulianDay {
        value: result.value,
    })
}

/// Return the typed result of native `revjul`.
///
/// Calendar years are astronomical and proleptic; Julian days and clock units are preserved.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `jd` - Julian Day in days; pure calendar arithmetic assigns no time scale.
/// * `calendar` - Explicit proleptic Julian or Gregorian calendar.
///
/// # Returns
///
/// Complete CalendarDate transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn revjul(jd: f64, calendar: Calendar) -> Result<CalendarDate, Error> {
    Ok(calendar_date(
        crate::time::revjul(jd, calendar.native()).map_err(Error::from_native)?,
    ))
}

/// Return the typed result of native `date_conversion`.
///
/// Calendar years are astronomical and proleptic; Julian days and clock units are preserved.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `year` - Astronomical calendar year; zero denotes 1 BCE.
/// * `month` - Civil month label; native validation/normalization is retained.
/// * `day` - Civil day label; native validation/normalization is retained.
/// * `hour` - Decimal civil hours.
/// * `calendar` - Explicit proleptic Julian or Gregorian calendar.
///
/// # Returns
///
/// Complete DateConversion transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn date_conversion(
    year: i32,
    month: i32,
    day: i32,
    hour: f64,
    calendar: Calendar,
) -> Result<DateConversion, Error> {
    let result = crate::pure_calendar::date_conversion(year, month, day, hour, calendar.native())
        .map_err(Error::from_native)?;
    Ok(DateConversion {
        valid: result.valid,
        julian_day: result.julian_day,
        date: calendar_date(result.date),
        status: result.status,
        diagnostic: result.diagnostic,
    })
}

/// Return the typed result of native `day_of_week`.
///
/// Calendar years are astronomical and proleptic; Julian days and clock units are preserved.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `jd` - Julian Day in days; pure calendar arithmetic assigns no time scale.
///
/// # Returns
///
/// Complete Weekday transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn day_of_week(jd: f64) -> Result<Weekday, Error> {
    let result = crate::pure_calendar::day_of_week(jd).map_err(Error::from_native)?;
    Ok(Weekday {
        index: result.index,
    })
}

/// Return the typed result of native `utc_time_zone`.
///
/// Calendar years are astronomical and proleptic; Julian days and clock units are preserved.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `year` - Astronomical calendar year; zero denotes 1 BCE.
/// * `month` - Civil month label; native validation/normalization is retained.
/// * `day` - Civil day label; native validation/normalization is retained.
/// * `hour` - Integer clock hour in the supplied UTC/local date.
/// * `minute` - Clock minutes in the supplied local/UTC date.
/// * `second` - Clock seconds, including a fractional part and admitted leap seconds.
/// * `offset` - Local-minus-UTC timezone offset in hours, positive east.
///
/// # Returns
///
/// Complete UtcDateTime transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn utc_time_zone(
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: f64,
    offset: f64,
) -> Result<UtcDateTime, Error> {
    Ok(utc_date(
        crate::time::utc_time_zone(year, month, day, hour, minute, second, offset)
            .map_err(Error::from_native)?,
    ))
}

/// Return the typed result of native `degnorm`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `value` - Angle in degrees.
///
/// # Returns
///
/// Complete Angle transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn degnorm(value: f64) -> Result<Angle, Error> {
    Ok(Angle {
        value: crate::angles::degnorm(value),
    })
}

/// Return the typed result of native `radnorm`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `value` - Angle in radians.
///
/// # Returns
///
/// Complete Angle transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn radnorm(value: f64) -> Result<Angle, Error> {
    Ok(Angle {
        value: crate::angles::radnorm(value),
    })
}

/// Return the typed result of native `difdegn`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `first` - Angular operand in degrees.
/// * `second` - Angular operand in degrees.
///
/// # Returns
///
/// Complete Angle transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn difdegn(first: f64, second: f64) -> Result<Angle, Error> {
    Ok(Angle {
        value: crate::angles::difdegn(first, second),
    })
}

/// Return the typed result of native `difdeg2n`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `first` - Angular operand in degrees.
/// * `second` - Angular operand in degrees.
///
/// # Returns
///
/// Complete Angle transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn difdeg2n(first: f64, second: f64) -> Result<Angle, Error> {
    Ok(Angle {
        value: crate::angles::difdeg2n(first, second),
    })
}

/// Return the typed result of native `difrad2n`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `first` - Angular operand in radians.
/// * `second` - Angular operand in radians.
///
/// # Returns
///
/// Complete Angle transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn difrad2n(first: f64, second: f64) -> Result<Angle, Error> {
    Ok(Angle {
        value: crate::angles::difrad2n(first, second),
    })
}

/// Return the typed result of native `deg_midp`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `first` - Angular operand in degrees.
/// * `second` - Angular operand in degrees.
///
/// # Returns
///
/// Complete Angle transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn deg_midp(first: f64, second: f64) -> Result<Angle, Error> {
    Ok(Angle {
        value: crate::angles::deg_midp(first, second),
    })
}

/// Return the typed result of native `rad_midp`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `first` - Angular operand in radians.
/// * `second` - Angular operand in radians.
///
/// # Returns
///
/// Complete Angle transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn rad_midp(first: f64, second: f64) -> Result<Angle, Error> {
    Ok(Angle {
        value: crate::angles::rad_midp(first, second),
    })
}

/// Return the typed result of native `d2l`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `value` - Scalar to round to a native integer; no physical unit is assigned.
///
/// # Returns
///
/// Complete Integer transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn d2l(value: f64) -> Result<Integer, Error> {
    Ok(Integer {
        value: i64::from(crate::angles::d2l(value).map_err(Error::from_native)?),
    })
}

/// Return the typed result of native `csnorm`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `value` - Centiseconds in a signed 64-bit transport; must fit native i32.
///
/// # Returns
///
/// Complete Integer transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn csnorm(value: i64) -> Result<Integer, Error> {
    Ok(Integer {
        value: i64::from(crate::angles::csnorm(native_integer("csnorm", value)?)),
    })
}

/// Return the typed result of native `csroundsec`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `value` - Centiseconds in a signed 64-bit transport; must fit native i32.
///
/// # Returns
///
/// Complete Integer transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn csroundsec(value: i64) -> Result<Integer, Error> {
    Ok(Integer {
        value: i64::from(
            crate::angles::csroundsec(native_integer("csroundsec", value)?)
                .map_err(Error::from_native)?,
        ),
    })
}

/// Return the typed result of native `difcsn`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `first` - Angular operand in integer centiseconds.
/// * `second` - Angular operand in integer centiseconds.
///
/// # Returns
///
/// Complete Integer transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn difcsn(first: i64, second: i64) -> Result<Integer, Error> {
    Ok(Integer {
        value: i64::from(
            crate::angles::difcsn(
                native_integer("difcsn", first)?,
                native_integer("difcsn", second)?,
            )
            .map_err(Error::from_native)?,
        ),
    })
}

/// Return the typed result of native `difcs2n`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `first` - Angular operand in integer centiseconds.
/// * `second` - Angular operand in integer centiseconds.
///
/// # Returns
///
/// Complete Integer transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn difcs2n(first: i64, second: i64) -> Result<Integer, Error> {
    Ok(Integer {
        value: i64::from(
            crate::angles::difcs2n(
                native_integer("difcs2n", first)?,
                native_integer("difcs2n", second)?,
            )
            .map_err(Error::from_native)?,
        ),
    })
}

/// Return the typed result of native `cs2degstr`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `value` - Centiseconds in a signed 64-bit transport; must fit native i32.
///
/// # Returns
///
/// Complete Text transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn cs2degstr(value: i64) -> Result<Text, Error> {
    Ok(Text {
        value: crate::angles::cs2degstr(native_integer("cs2degstr", value)?)
            .map_err(Error::from_native)?,
    })
}

/// Return the typed result of native `cs2lonlatstr`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `value` - Centiseconds in a signed 64-bit transport; must fit native i32.
/// * `plus` - Exactly one non-NUL ASCII byte for positive coordinates.
/// * `minus` - Exactly one non-NUL ASCII byte for negative coordinates.
///
/// # Returns
///
/// Complete Text transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn cs2lonlatstr(value: i64, plus: &str, minus: &str) -> Result<Text, Error> {
    Ok(Text {
        value: crate::angles::cs2lonlatstr(
            native_integer("cs2lonlatstr", value)?,
            native_character("cs2lonlatstr", plus)?,
            native_character("cs2lonlatstr", minus)?,
        )
        .map_err(Error::from_native)?,
    })
}

/// Return the typed result of native `cs2timestr`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `value` - Centiseconds in a signed 64-bit transport; must fit native i32.
/// * `separator` - Exactly one non-NUL ASCII separator byte, supplied as a string.
/// * `suppress_zero` - Whether native formatting suppresses leading zeroes.
///
/// # Returns
///
/// Complete Text transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn cs2timestr(value: i64, separator: &str, suppress_zero: bool) -> Result<Text, Error> {
    Ok(Text {
        value: crate::angles::cs2timestr(
            native_integer("cs2timestr", value)?,
            native_character("cs2timestr", separator)?,
            suppress_zero,
        )
        .map_err(Error::from_native)?,
    })
}

/// Return the typed result of native `split_deg`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `value` - Angle in degrees.
/// * `flags` - Native source/frame/unit/rate bitmask, forwarded unchanged.
///
/// # Returns
///
/// Complete SplitDeg transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn split_deg(value: f64, flags: i32) -> Result<SplitDeg, Error> {
    let v = crate::angles::split_deg(value, flags).map_err(Error::from_native)?;
    Ok(SplitDeg {
        degrees: i64::from(v.deg),
        minutes: i64::from(v.min),
        seconds: i64::from(v.sec),
        fraction: v.secfr,
        sign: i64::from(v.sign),
    })
}

/// Return the typed result of native `cotrans`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `longitude` - Point longitude/right ascension in degrees.
/// * `latitude` - Point latitude/declination in degrees.
/// * `distance` - Radius in the caller's length unit; rotations do not rescale it.
/// * `obliquity` - Ecliptic obliquity in degrees.
///
/// # Returns
///
/// Complete Spherical transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn cotrans(
    longitude: f64,
    latitude: f64,
    distance: f64,
    obliquity: f64,
) -> Result<Spherical, Error> {
    Ok(Spherical {
        values: crate::angles::cotrans(longitude, latitude, distance, obliquity)
            .map_err(Error::from_native)?,
    })
}

/// Return the typed result of native `cotrans_sp`.
///
/// Angles use degrees (radians for rad*); centisecond transports are checked before C narrowing.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `longitude` - Point longitude/right ascension in degrees.
/// * `latitude` - Point latitude/declination in degrees.
/// * `distance` - Radius in the caller's length unit; rotations do not rescale it.
/// * `longitude_speed` - Longitudinal angular rate in degrees per day.
/// * `latitude_speed` - Latitudinal angular rate in degrees per day.
/// * `distance_speed` - Radial rate in the input length unit per day.
/// * `obliquity` - Ecliptic obliquity in degrees.
///
/// # Returns
///
/// Complete SphericalState transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn cotrans_sp(
    longitude: f64,
    latitude: f64,
    distance: f64,
    longitude_speed: f64,
    latitude_speed: f64,
    distance_speed: f64,
    obliquity: f64,
) -> Result<SphericalState, Error> {
    let (position, velocity) = crate::angles::cotrans_sp(
        longitude,
        latitude,
        distance,
        longitude_speed,
        latitude_speed,
        distance_speed,
        obliquity,
    )
    .map_err(Error::from_native)?;
    Ok(SphericalState { position, velocity })
}

/// Return the typed result of native `house_name`.
///
/// Calendar years are astronomical and proleptic; Julian days and clock units are preserved.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `system` - ASCII house-system letter; fixed twelve-cusp operations reject Gauquelin.
///
/// # Returns
///
/// Complete Text transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn house_name(system: u8) -> Result<Text, Error> {
    Ok(Text {
        value: crate::houses::house_name(system),
    })
}

/// Return the typed result of native `get_ayanamsa_name`.
///
/// Calendar years are astronomical and proleptic; Julian days and clock units are preserved.
/// No process-global configuration is installed by this utility.
///
/// # Arguments
///
/// * `mode` - Explicit native mode input.
///
/// # Returns
///
/// Complete Text transport from the original producer.
///
/// # Errors
///
/// Original native domain/lock failures propagate; integer/character narrowing is checked.
#[allow(clippy::too_many_arguments)]
pub fn get_ayanamsa_name(mode: i32) -> Result<Text, Error> {
    Ok(Text {
        value: crate::config::get_ayanamsa_name(mode).map_err(Error::from_native)?,
    })
}

pub(super) fn calendar_date(v: crate::time::CalendarDate) -> CalendarDate {
    CalendarDate {
        year: v.year,
        month: v.month,
        day: v.day,
        hour: v.hour,
    }
}
pub(super) fn utc_date(v: crate::time::UtcDateTime) -> UtcDateTime {
    UtcDateTime {
        year: v.year,
        month: v.month,
        day: v.day,
        hour: v.hour,
        minute: v.minute,
        second: v.second,
    }
}
