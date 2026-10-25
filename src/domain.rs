//! Arithmetic preconditions for native date-based computations.
//!
//! These checks bound integer conversions and configured time shifts;
//! astronomical results and all source selection remain native.

use crate::{Calendar, Error, FLG_JPLEPH, FLG_MOSEPH, FLG_SWIEPH};

/// Bound observer-dependent light-time dates before native calendar
/// conversions. The conservative ceiling is i32::MAX AU in metres,
/// leaving more than two orders of magnitude between its light-travel
/// time in days and the calendar-year limits. Geographic heights and
/// near-Earth space observers remain far inside this arithmetic ceiling.
pub(crate) fn check_observer_height(function: &str, altitude: f64) -> Result<(), Error> {
    let ceiling = f64::from(i32::MAX) * crate::AUNIT_TO_KM * 1000.0;
    if !altitude.is_finite() || altitude.abs() > ceiling {
        return Err(Error::invalid_input(format!(
            "{function}: observer height exceeds the topocentric arithmetic limit of ±{ceiling} metres"
        )));
    }
    Ok(())
}

/// Require a date whose native file-calendar year is representable.
/// Both calendars are checked because native file selection can use
/// either. This is an integer-representation limit, not a data coverage
/// or astronomical-accuracy claim.
pub(crate) fn check_et(function: &str, jd: f64) -> Result<(), Error> {
    crate::time::check_calendar_jd(function, jd, Calendar::Gregorian)?;
    crate::time::check_calendar_jd(function, jd, Calendar::Julian)
}

/// Check dates plus any active UT sidereal reference used by this request.
/// Caller holds native access across this check and the computation.
pub(crate) fn check_et_locked(function: &str, jd: f64, flags: i32) -> Result<(), Error> {
    check_et(function, jd)?;
    if flags & crate::FLG_SIDEREAL != 0 {
        crate::config::check_sidereal_epoch_locked(function, flags)?;
    }
    Ok(())
}

/// Check a UT date and effective shifts without checking sidereal state.
/// `-1` means inherited tidal selection: do not select alternate sources.
/// Explicit sources also probe Swiss/Moshier fallbacks and restore the
/// requested selection on both success and validation failure.
pub(crate) fn check_ut_date_locked(function: &str, jd: f64, flags: i32) -> Result<(), Error> {
    check_ut_date_with_restore_locked(function, jd, flags, true)
}

fn check_ut_date_with_restore_locked(
    function: &str,
    jd: f64,
    flags: i32,
    restore_on_success: bool,
) -> Result<(), Error> {
    check_et(function, jd)?;
    // SAFETY: finite calendar-bounded date; null diagnostics supported.
    // Caller holds native access. -1 preserves the inherited selection.
    let dt = unsafe { crate::ffi::swe_deltat_ex(jd, flags, std::ptr::null_mut()) };
    check_et(function, jd + dt)?;
    if flags == -1 {
        return Ok(());
    }
    let fallback = (flags & !(FLG_SWIEPH | FLG_JPLEPH | FLG_MOSEPH)) | FLG_MOSEPH;
    let swiss = (flags & !(FLG_SWIEPH | FLG_JPLEPH | FLG_MOSEPH)) | FLG_SWIEPH;
    let mut result = Ok(());
    for source in [swiss, fallback] {
        // Only the first Swiss probe can immediately repeat the exact source
        // and effective date already checked above. No configuration changed
        // between them. Keep Moshier: alternate source selections can change
        // automatic tidal acceleration. The owner restores it below or in
        // the immediately following native calculation prefix.
        if source == swiss && flags == swiss {
            continue;
        }
        // SAFETY: calendar-bounded finite input; null diagnostics are
        // supported. The caller holds native access through computation.
        let dt = unsafe { crate::ffi::swe_deltat_ex(jd, source, std::ptr::null_mut()) };
        if let Err(error) = check_et(function, jd + dt) {
            result = Err(error);
            break;
        }
    }
    if restore_on_success || result.is_err() {
        // SAFETY: same bounded input; restore the requested source after
        // fallback validation, including a rejected shift. Auto/manual mode stays.
        unsafe { crate::ffi::swe_deltat_ex(jd, flags, std::ptr::null_mut()) };
    }
    result
}

/// Validate a UT Julian Day immediately before native `swe_calc_ut`.
///
/// This is not a general preflight: the caller holds native access and its
/// next production native call must be `swe_calc_ut`. On non-sidereal success,
/// that C prefix canonicalizes the flags and overwrites automatic tidal
/// selection before calculating ET. Flag canonicalization does not consume
/// that selection; raw and canonical source flags need not be identical.
/// Errors, sidereal references and inherited `-1` keep the original sequence.
pub(crate) fn check_before_calc_ut_locked(
    function: &str,
    jd: f64,
    flags: i32,
) -> Result<(), Error> {
    if flags != -1 && flags & crate::FLG_SIDEREAL == 0 {
        check_ut_date_with_restore_locked(function, jd, flags, false)
    } else {
        check_ut_locked(function, jd, flags)
    }
}

/// Validate the UT input, shifts and requested sidereal configuration.
/// Caller holds native access throughout the dependent computation.
pub(crate) fn check_ut_locked(function: &str, jd: f64, flags: i32) -> Result<(), Error> {
    check_ut_date_locked(function, jd, flags)?;
    if flags != -1 && flags & crate::FLG_SIDEREAL != 0 {
        crate::config::check_sidereal_epoch_locked(function, flags)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
