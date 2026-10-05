//! Body positions and body names.
//!
//! [`calc_ut`] and [`calc`] compute apparent body positions with the native
//! engine; [`get_planet_name`] resolves a body number to its native name.
//! Every position result carries all six native components, the returned
//! flag set (which names the ephemeris source actually used), and the
//! native diagnostic text.

use std::ffi::c_char;

use crate::error::Error;
use crate::ffi::{
    self, POSITION_LEN, SERR_LEN, TEXT_BUF_LEN, copy_returned_string, read_native_text,
};
use crate::state::with_native_access;
use crate::{FLG_JPLEPH, FLG_MOSEPH, FLG_SWIEPH};

/// Computed body position: all six native components plus provenance.
///
/// Component order and units depend on the request flags exactly as in the
/// native API. For the default ecliptic output the layout is:
///
/// 1. longitude in degrees (0-360),
/// 2. latitude in degrees,
/// 3. distance in AU,
/// 4. longitude speed in degrees per day,
/// 5. latitude speed in degrees per day,
/// 6. distance speed in AU per day.
///
/// `FLG_XYZ` switches to Cartesian components, `FLG_EQUATORIAL` to right
/// ascension/declination, and `FLG_RADIANS` to radians; speeds are daily
/// rates only when `FLG_SPEED` (or `FLG_SPEED3`) was requested.
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    /// The six native output components; see the type documentation.
    pub values: [f64; POSITION_LEN],
    /// Flag set returned by the native call.
    ///
    /// This is the source of truth for which ephemeris actually served the
    /// request: when data files are missing the engine falls back to the
    /// analytical Moshier model and reports `FLG_MOSEPH` here instead of
    /// the requested `FLG_SWIEPH`. Never assume the requested source was
    /// used — inspect these bits, or enforce them with
    /// [`require_source_flags`].
    pub returned_flags: i32,
    /// Native diagnostic text (`serr`).
    ///
    /// Empty on a clean computation; carries warnings (for example the
    /// fallback notice) on success and the failure reason on error. It is
    /// informational: a non-empty diagnostic with a non-negative return
    /// value is a warning when all position components are finite. A
    /// non-finite native position is classified as `ErrorKind::Native`.
    pub diagnostic: String,
}

impl Position {
    /// Ecliptic longitude in degrees (component 0 of default output).
    pub fn longitude(&self) -> f64 {
        self.values[0]
    }

    /// Ecliptic latitude in degrees (component 1 of default output).
    pub fn latitude(&self) -> f64 {
        self.values[1]
    }

    /// Distance in AU (component 2 of default output).
    pub fn distance(&self) -> f64 {
        self.values[2]
    }
}

/// Compute a UT position while native access is already held.
///
/// Caller must hold the native lock. `jd_ut` must already be validated
/// as finite.
pub(crate) fn calc_ut_locked(jd_ut: f64, body: i32, flags: i32) -> Result<Position, Error> {
    crate::domain::check_ut_locked("calc_ut", jd_ut, flags)?;
    let mut xx = [0.0; POSITION_LEN];
    let mut serr = [0 as c_char; SERR_LEN];
    // SAFETY: `xx` owns POSITION_LEN writable f64 slots and `serr`
    // owns SERR_LEN writable bytes; both outlive this call and the caller
    // holds the native lock. Results are only exposed after the native
    // status and output finiteness are checked. The input/effective dates
    // were calendar-bounded under this same lock.
    let ret = unsafe { ffi::swe_calc_ut(jd_ut, body, flags, xx.as_mut_ptr(), serr.as_mut_ptr()) };
    // SAFETY: `serr` is our own fully initialized buffer; the read is
    // bounded by SERR_LEN (see `read_native_text`).
    let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
    if ret < 0 {
        Err(Error::native(describe_calc_failure(
            "calc_ut",
            jd_ut,
            body,
            flags,
            &diagnostic,
        )))
    } else {
        if !xx.iter().all(|value| value.is_finite()) {
            return Err(Error::native(format!(
                "calc_ut: non-finite native position output; check observer/sidereal configuration: {diagnostic}"
            )));
        }
        Ok(Position {
            values: xx,
            returned_flags: ret,
            diagnostic,
        })
    }
}

/// Calculate a body position for a Universal Time Julian Day.
///
/// `body` is a body number (for example [`crate::MOON`]); `flags`
/// combines source bits (`FLG_SWIEPH`, `FLG_JPLEPH`, `FLG_MOSEPH`),
/// geometry bits (`FLG_HELCTR`, `FLG_TRUEPOS`, `FLG_TOPOCTR`, ...) and
/// output bits (`FLG_SPEED`, `FLG_EQUATORIAL`, `FLG_XYZ`, ...). The
/// default `0` requests apparent geocentric ecliptic positions without
/// speeds.
///
/// # Time scale
///
/// `jd_ut` is Universal Time. For Ephemeris/Terrestrial Time use [`calc`].
///
/// # Configuration dependence
///
/// Topocentric output requires [`crate::set_topo`], sidereal output
/// requires [`crate::set_sid_mode`], and file-based sources require data
/// files visible through [`crate::set_ephe_path`]. Each free call holds
/// the process-wide native lock across its own native sequence; a
/// `set_*` plus this computation remain two acquisitions. Callers needing
/// atomic config-plus-computation must use [`crate::Session`], which holds
/// the same lock across the whole apply-then-compute sequence.
///
/// # Errors
///
/// Non-finite `jd_ut`/`jd_et` is rejected with
/// [`ErrorKind::InvalidInput`](crate::ErrorKind) before any native call.
/// Returns [`Error`] when the native call fails (unknown body, missing or
/// out-of-range data). A non-negative native status — including results
/// computed with a fallback source or carrying a warning — is `Ok` when
/// all components are finite, with
/// details in [`Position::returned_flags`] and [`Position::diagnostic`].
/// Non-finite native components return `ErrorKind::Native` with context
/// and the original diagnostic. A valid zero position is data, never an error.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, FLG_SPEED, MOON, calc_ut};
///
/// // The Moshier model needs no data files, so this runs anywhere.
/// let moon = calc_ut(2451545.0, MOON, FLG_MOSEPH | FLG_SPEED)?;
/// assert!((0.0..360.0).contains(&moon.longitude()));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn calc_ut(jd_ut: f64, body: i32, flags: i32) -> Result<Position, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(format!(
            "calc_ut: Julian Day must be finite ({jd_ut})"
        )));
    }
    with_native_access(|| calc_ut_locked(jd_ut, body, flags))
}

/// Compute an ET position while native access is already held.
///
/// Caller must hold the native lock. `jd_et` must already be validated
/// as finite.
pub(crate) fn calc_locked(jd_et: f64, body: i32, flags: i32) -> Result<Position, Error> {
    crate::domain::check_et_locked("calc", jd_et, flags)?;
    let mut xx = [0.0; POSITION_LEN];
    let mut serr = [0 as c_char; SERR_LEN];
    // SAFETY: same contract as in [`calc_ut_locked`].
    let ret = unsafe { ffi::swe_calc(jd_et, body, flags, xx.as_mut_ptr(), serr.as_mut_ptr()) };
    // SAFETY: same contract as in [`calc_ut_locked`].
    let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
    if ret < 0 {
        Err(Error::native(describe_calc_failure(
            "calc",
            jd_et,
            body,
            flags,
            &diagnostic,
        )))
    } else {
        if !xx.iter().all(|value| value.is_finite()) {
            return Err(Error::native(format!(
                "calc: non-finite native position output; check observer/sidereal configuration: {diagnostic}"
            )));
        }
        Ok(Position {
            values: xx,
            returned_flags: ret,
            diagnostic,
        })
    }
}

/// Calculate a body position for an Ephemeris Time Julian Day.
///
/// Identical to [`calc_ut`] except that `jd_et` is Ephemeris
/// Time (Terrestrial Time plus the small historical offset the native
/// engine applies), i.e. the time scale of the ephemeris data itself.
/// Argument order, flags, result layout and error behavior match
/// [`calc_ut`] exactly.
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn calc(jd_et: f64, body: i32, flags: i32) -> Result<Position, Error> {
    if !jd_et.is_finite() {
        return Err(Error::invalid_input(format!(
            "calc: Julian Day must be finite ({jd_et})"
        )));
    }
    with_native_access(|| calc_locked(jd_et, body, flags))
}

/// Calculate a body position as seen from another body (planet-centric).
///
/// `body` is the target body number, `center` the observing body number
/// (for example [`crate::MARS`]); `flags` combines the same source,
/// geometry and output bits as [`calc_ut`]. The distance component is the
/// distance from `center` to `body` in AU.
///
/// # Time scale
///
/// `jd_et` is Ephemeris Time. Unlike [`calc_ut`]/[`calc`] there is no UT
/// variant natively: the single native entry point takes ET.
///
/// # Configuration dependence
///
/// Same data-path requirements as [`calc_ut`]; `FLG_TOPOCTR` has no
/// topocentric correction on other bodies natively. Held under the
/// process-wide native lock together with the active configuration.
///
/// # Errors
///
/// Unknown body or center numbers and missing data fail natively with the
/// diagnostic kept, exactly like [`calc_ut`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_SPEED, FLG_SWIEPH, MARS, MOON, calc_pctr, set_ephe_path};
///
/// // Planet-centric positions need data files: the Moshier model is not
/// // supported natively for this entry point (an explicit external data
/// // directory, else the checkout data, keeps this example runnable).
/// // Ephemeris data comes from an explicit external directory when set,
/// // otherwise from the checkout data (doctests run with the package root
/// // as the working directory); without either, there is nothing
/// // file-based to demonstrate.
/// let data_dir =
///     std::env::var("SWISSEPH_EPHE_DIR").unwrap_or_else(|_| "swisseph/ephe".to_string());
/// if !std::path::Path::new(&data_dir).is_dir() {
///     return Ok(());
/// }
/// set_ephe_path(Some(&data_dir))?;
/// let moon = calc_pctr(2451545.0, MOON, MARS, FLG_SWIEPH | FLG_SPEED)?;
/// assert!((0.0..360.0).contains(&moon.longitude()));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn calc_pctr(jd_et: f64, body: i32, center: i32, flags: i32) -> Result<Position, Error> {
    if !jd_et.is_finite() {
        return Err(Error::invalid_input(format!(
            "calc_pctr: Julian Day must be finite ({jd_et})"
        )));
    }
    with_native_access(|| {
        crate::domain::check_et_locked("calc_pctr", jd_et, flags)?;
        let mut xx = [0.0; POSITION_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `xx` owns POSITION_LEN writable f64 slots and `serr`
        // owns SERR_LEN writable bytes; both outlive this call and no
        // other thread can access native state while the lock is held.
        // Effective dates are calendar-bounded above, and results are exposed
        // only after native status and output finiteness are checked.
        let ret = unsafe {
            ffi::swe_calc_pctr(
                jd_et,
                body,
                center,
                flags,
                xx.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(format!(
                "calc_pctr: {} (jd_et={jd_et}, body={body}, center={center}, flags={flags})",
                if diagnostic.is_empty() {
                    "native computation failed".to_string()
                } else {
                    diagnostic
                },
            )))
        } else {
            if !xx.iter().all(|value| value.is_finite()) {
                return Err(Error::native(format!(
                    "calc_pctr: non-finite native position output; check observer/sidereal configuration: {diagnostic}"
                )));
            }
            Ok(Position {
                values: xx,
                returned_flags: ret,
                diagnostic,
            })
        }
    })
}

/// Resolve a body number to its native name.
///
/// Returns names such as `"Sun"` or `"Moon"` for the classical bodies and
/// catalog names for asteroids. Unknown numbers retain the native
/// descriptive text as an owned string. An empty name is a native error.
///
/// # Body-number domain
///
/// The full native domain passes through unchanged: classical bodies
/// (`SUN`..`INTP_PERG`, plus the `ECL_NUT` pseudo-body), fictitious bodies
/// (`CUPIDO`..`WALDEMATH`), numbered asteroids (`AST_OFFSET + number`,
/// which alias the dedicated ids where both exist — `AST_OFFSET + 1` is
/// `"Ceres"`, like [`crate::CERES`]) and planetary-moon offsets
/// (`PLMOON_OFFSET +...`). Out-of-range numbers are **not** rejected on
/// the Rust side: the native library answers them with descriptive text
/// that is preserved verbatim (for example `"name not found"`,
/// `"989999: not found (asteroid)"` or `"10000: not found (planetary
/// moon)"`). That text is returned as `Ok` data, exactly as produced —
/// callers that need an error for unknown names must match on the text
/// themselves. Only a truly empty native result becomes a [`Error`]
/// (which no pinned-build input is known to produce; the branch is a
/// safety net, never invented data).
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{
///     AST_OFFSET, CERES, CUPIDO, SUN, get_planet_name, set_ephe_path,
/// };
///
/// // Ephemeris data comes from an explicit external directory when set,
/// // otherwise from the checkout data (doctests run with the package root
/// // as the working directory); without either, there is nothing
/// // file-based to demonstrate.
/// let data_dir =
///     std::env::var("SWISSEPH_EPHE_DIR").unwrap_or_else(|_| "swisseph/ephe".to_string());
/// if !std::path::Path::new(&data_dir).is_dir() {
///     return Ok(());
/// }
/// set_ephe_path(Some(&data_dir))?;
/// assert_eq!(get_planet_name(SUN)?, "Sun");
/// assert_eq!(get_planet_name(CUPIDO)?, "Cupido");
/// assert_eq!(get_planet_name(AST_OFFSET + 1)?, get_planet_name(CERES)?);
/// // Unknown numbers keep the native verdict verbatim instead of failing:
/// assert!(get_planet_name(999)?.contains("not found"));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn get_planet_name(body: i32) -> Result<String, Error> {
    with_native_access(|| {
        let mut buffer = [0 as c_char; TEXT_BUF_LEN];
        // SAFETY: `buffer` owns TEXT_BUF_LEN writable bytes; the returned
        // pointer aliases it (or is null). The name is copied into an
        // owned `String` before the lock is released.
        let (returned, snapshot) = unsafe {
            let returned = ffi::swe_get_planet_name(body, buffer.as_mut_ptr());
            (returned, buffer.map(|b| b as u8))
        };
        let name =
            // SAFETY: `returned` aliases `buffer` (or is null) from the call
            // above; `snapshot` preserves the bytes for the null fallback.
            // The string is copied into owned storage here.
            unsafe { copy_returned_string(returned.cast_const(), &snapshot) };
        if name.is_empty() {
            Err(Error::native(format!(
                "get_planet_name: native library returned no name for body {body}"
            )))
        } else {
            Ok(name)
        }
    })
}

/// Build the error message for a failed position computation, keeping the
/// native diagnostic when the engine provided one.
fn describe_calc_failure(
    function: &str,
    jd: f64,
    body: i32,
    flags: i32,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!("{function}: native computation failed (jd={jd}, body={body}, flags={flags})")
    } else {
        format!("{function}: {diagnostic} (jd={jd}, body={body}, flags={flags})")
    }
}

/// Require that a computed position was served by the requested ephemeris
/// source, rejecting a silent fallback.
///
/// The native engine visibly reports a fallback: when the requested data
/// files are missing it computes with the analytical Moshier model and
/// says so in [`Position::returned_flags`] (the source bits name the
/// model actually used) and [`Position::diagnostic`]. This pure helper
/// turns that visible fallback into a hard error for callers that need
/// file-based precision: it succeeds only when at least one of the source
/// bits requested in `requested_flags` (`FLG_SWIEPH`, `FLG_JPLEPH`,
/// `FLG_MOSEPH`) is also set in `position.returned_flags` and every source
/// reported by a native `using... eph` component-fallback diagnostic is
/// allowed by those requested bits. A Swiss planet result can retain
/// `FLG_SWIEPH` while reporting an analytical Moon contribution; that
/// contribution fails a strict Swiss-only request. Multiple requested
/// source bits allow any of those sources. Unrelated warnings stay data.
///
/// When `requested_flags` carries no source bit (the default `0`, i.e. the
/// native default chain), there is nothing strict to enforce and the
/// helper succeeds unconditionally. Pass the same `flags` value that was
/// given to [`calc_ut`]/[`calc`]/[`calc_pctr`].
///
/// This performs no native call and takes no lock; it only inspects the
/// already-owned [`Position`]. A mismatch is an [`Error`] of kind
/// [`crate::ErrorKind::Native`] carrying both flag sets and the preserved
/// native diagnostic, never invented position data.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{
///     FLG_MOSEPH, FLG_SPEED, FLG_SWIEPH, Position, require_source_flags,
/// };
///
/// // A Moshier result satisfies a Moshier request.
/// let moshier = Position {
///     values: [0.0; 6],
///     returned_flags: FLG_MOSEPH | FLG_SPEED,
///     diagnostic: String::new(),
/// };
/// require_source_flags(&moshier, FLG_MOSEPH | FLG_SPEED)?;
/// //... but not a Swiss-data request: the fallback is rejected.
/// assert!(require_source_flags(&moshier, FLG_SWIEPH | FLG_SPEED).is_err());
/// // No source bit requested means the default chain: nothing to enforce.
/// assert!(require_source_flags(&moshier, FLG_SPEED).is_ok());
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn require_source_flags(position: &Position, requested_flags: i32) -> Result<(), Error> {
    const SOURCE_MASK: i32 = FLG_SWIEPH | FLG_JPLEPH | FLG_MOSEPH;
    let wanted = requested_flags & SOURCE_MASK;
    if wanted == 0 {
        return Ok(());
    }
    let used = position.returned_flags & SOURCE_MASK;
    let mut components = 0;
    for (phrase, source) in [
        (b"using Moshier eph".as_slice(), FLG_MOSEPH),
        (b"using Swiss eph".as_slice(), FLG_SWIEPH),
        (b"using JPL eph".as_slice(), FLG_JPLEPH),
    ] {
        if position
            .diagnostic
            .as_bytes()
            .windows(phrase.len())
            .any(|part| part.eq_ignore_ascii_case(phrase))
        {
            components |= source;
        }
    }
    if used & wanted != 0 && components & !wanted == 0 {
        return Ok(());
    }
    let diagnostic = if position.diagnostic.is_empty() {
        "no native diagnostic".to_string()
    } else {
        position.diagnostic.clone()
    };
    Err(Error::native(format!(
        "require_source_flags: requested source bits {wanted}, returned flags {}, reported component source bits {components}; unexpected primary/component fallback ({diagnostic})",
        position.returned_flags,
    )))
}
