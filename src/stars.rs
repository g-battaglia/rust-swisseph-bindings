//! Fixed-star positions and magnitudes.
//!
//! [`fixstar`] and [`fixstar_ut`] compute fixed-star positions with the
//! native engine, [`fixstar2`] and [`fixstar2_ut`] add the extended search
//! semantics, and [`fixstar_mag`] / [`fixstar2_mag`] report visual
//! magnitudes without a position computation. Every position result carries
//! all six native components, the resolved star name written back by the
//! engine, the returned flag set, and the native diagnostic text.

use std::ffi::c_char;

use crate::error::Error;
use crate::ffi::{self, POSITION_LEN, SERR_LEN, STAR_BUF_LEN, read_native_text};
use crate::state::with_native_access;

/// Computed fixed-star position: all six native components plus provenance.
///
/// Component order and units depend on the request flags exactly as in
/// [`crate::calc_ut`]. For the default ecliptic output the layout is
/// longitude, latitude (degrees), distance (AU), then the three matching
/// daily rates; speeds are daily rates only when `FLG_SPEED` (or
/// `FLG_SPEED3`) was requested.
#[derive(Debug, Clone, PartialEq)]
pub struct StarPosition {
    /// The six native output components; see the type documentation.
    pub values: [f64; POSITION_LEN],
    /// Star name resolved by the engine.
    ///
    /// The native call overwrites the input search string with the full
    /// catalog name (`"Name,Nomenclature"`, for example
    /// `"Sirius,alCMa"`); that text is copied into owned Rust storage
    /// before the native lock is released.
    pub resolved_name: String,
    /// Flag set returned by the native call.
    ///
    /// This names the ephemeris source actually used, exactly as in
    /// [`crate::Position::returned_flags`].
    pub returned_flags: i32,
    /// Native diagnostic text (`serr`).
    ///
    /// Empty on a clean computation; carries warnings on success and the
    /// failure reason on error.
    pub diagnostic: String,
}

impl StarPosition {
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

/// Visual magnitude of a fixed star.
///
/// Returned by [`fixstar_mag`] and [`fixstar2_mag`], which perform no
/// position computation: only the catalog magnitude and the resolved name
/// are reported.
#[derive(Debug, Clone, PartialEq)]
pub struct StarMagnitude {
    /// Visual magnitude (for example `-1.46` for Sirius).
    pub magnitude: f64,
    /// Star name resolved by the engine; see [`StarPosition::resolved_name`].
    pub resolved_name: String,
    /// Native diagnostic text (`serr`).
    pub diagnostic: String,
}

/// Check a star query and Julian Day before any native call.
fn check_star_inputs(function: &str, star: &str, jd: f64) -> Result<(), Error> {
    if !jd.is_finite() {
        return Err(Error::invalid_input(format!(
            "{function}: Julian Day must be finite"
        )));
    }
    check_star_name(function, star)
}

/// Check a star query before any native call.
fn check_star_name(function: &str, star: &str) -> Result<(), Error> {
    if star.as_bytes().contains(&0) {
        return Err(Error::invalid_input(format!(
            "{function}: star name must not contain NUL bytes"
        )));
    }
    if star.len() >= STAR_BUF_LEN {
        return Err(Error::invalid_input(format!(
            "{function}: star name must fit the {STAR_BUF_LEN}-byte native buffer"
        )));
    }
    Ok(())
}

/// Copy a star query into the owned native in/out buffer.
///
/// The buffer owns [`STAR_BUF_LEN`] zero-initialized bytes; the query is
/// copied in and the engine overwrites it with the resolved full name, so
/// no caller allocation is ever exposed to native code.
fn prepare_star_buffer(star: &str) -> [c_char; STAR_BUF_LEN] {
    let mut buffer = [0 as c_char; STAR_BUF_LEN];
    for (index, byte) in star.bytes().enumerate() {
        buffer[index] = byte as c_char;
    }
    buffer
}

/// Probe the first catalog entry through the legacy magnitude API, which
/// safely reports missing/empty catalogs. The extended native lookup
/// otherwise performs null-pointer arithmetic on an absent catalog.
/// Caller holds native access; no Rust path mirror or filesystem race is
/// introduced, and native environment/default/legacy-file search remains
/// authoritative.
fn check_extended_catalog_locked(function: &str) -> Result<(), Error> {
    let mut name = prepare_star_buffer("1");
    let mut magnitude = 0.0;
    let mut diagnostic = [0 as c_char; SERR_LEN];
    // SAFETY: the legacy lookup accepts a sequential catalog number and
    // handles absent catalogs; every output buffer is owned, initialized
    // and correctly sized. The caller holds the native lock.
    let ret =
        unsafe { ffi::swe_fixstar_mag(name.as_mut_ptr(), &mut magnitude, diagnostic.as_mut_ptr()) };
    if ret < 0 {
        // SAFETY: initialized diagnostic storage, bounded owned read.
        let text = unsafe { read_native_text(diagnostic.as_ptr(), SERR_LEN) };
        return Err(Error::native(format!(
            "{function}: fixed-star catalog unavailable; configure set_ephe_path or SE_EPHE_PATH: {text}"
        )));
    }
    Ok(())
}

/// Calculate a fixed-star position for an Ephemeris Time Julian Day.
///
/// `star` is the traditional star name (for example `"Sirius"`); `jd_et`
/// is Ephemeris Time in days; `flags` combines the same source, geometry
/// and output bits as [`crate::calc`] (the default `0` requests apparent
/// geocentric ecliptic positions without speeds).
///
/// # Data and configuration dependence
///
/// Star data comes from the fixed-star catalog (`sefstars.txt`) found
/// through the ephemeris search path (see [`crate::set_ephe_path`]); in a
/// checkout the submodule data keeps catalog lookups working, otherwise
/// point the search path at an explicit external data directory.
/// Topocentric output requires [`crate::set_topo`], sidereal output
/// requires [`crate::set_sid_mode`]. Held under the process-wide native
/// lock together with the active configuration.
///
/// # Errors
///
/// Unknown star names and missing catalog data fail natively with the
/// engine diagnostic kept. A non-negative native status — including
/// results carrying a warning — is `Ok`, with details in
/// [`StarPosition::returned_flags`] and [`StarPosition::diagnostic`].
/// NUL-containing or overlong queries and non-finite dates are rejected
/// before any native call.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{fixstar_ut, set_ephe_path};
///
/// // Catalog lookups use the shipped data (doctests run with the package
/// // root as the working directory).
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
/// let sirius = fixstar_ut("Sirius", 2451545.0, 0)?;
/// assert!(sirius.resolved_name.contains("Sirius"));
/// assert!((0.0..360.0).contains(&sirius.longitude()));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn fixstar(star: &str, jd_et: f64, flags: i32) -> Result<StarPosition, Error> {
    check_star_inputs("fixstar", star, jd_et)?;
    with_native_access(|| {
        crate::domain::check_et_locked("fixstar", jd_et, flags)?;
        let mut star_buf = prepare_star_buffer(star);
        let mut xx = [0.0; POSITION_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `star_buf` owns STAR_BUF_LEN writable bytes (the native
        // in/out name contract), `xx` owns POSITION_LEN writable f64
        // slots, `serr` owns SERR_LEN writable bytes; all outlive this
        // call and no other thread can access native state while the lock
        // is held. Results are only exposed after the native status is
        // checked.
        let ret = unsafe {
            ffi::swe_fixstar(
                star_buf.as_mut_ptr(),
                jd_et,
                flags,
                xx.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: both buffers are our own fully initialized storage; the
        // reads are bounded by their capacities. Copied before the lock
        // is released.
        let (resolved_name, diagnostic) = unsafe {
            (
                read_native_text(star_buf.as_ptr(), STAR_BUF_LEN),
                read_native_text(serr.as_ptr(), SERR_LEN),
            )
        };
        if ret < 0 {
            Err(Error::native(describe_star_failure(
                "fixstar",
                star,
                jd_et,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(StarPosition {
                values: xx,
                resolved_name,
                returned_flags: ret,
                diagnostic,
            })
        }
    })
}

/// Calculate a fixed-star position for a Universal Time Julian Day.
///
/// Identical to [`fixstar`] except that `jd_ut` is Universal Time.
/// Argument order, flags, result layout and error behavior match
/// [`fixstar`] exactly.
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn fixstar_ut(star: &str, jd_ut: f64, flags: i32) -> Result<StarPosition, Error> {
    check_star_inputs("fixstar_ut", star, jd_ut)?;
    with_native_access(|| {
        crate::domain::check_ut_locked("fixstar_ut", jd_ut, flags)?;
        let mut star_buf = prepare_star_buffer(star);
        let mut xx = [0.0; POSITION_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`fixstar`].
        let ret = unsafe {
            ffi::swe_fixstar_ut(
                star_buf.as_mut_ptr(),
                jd_ut,
                flags,
                xx.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: same contract as in [`fixstar`].
        let (resolved_name, diagnostic) = unsafe {
            (
                read_native_text(star_buf.as_ptr(), STAR_BUF_LEN),
                read_native_text(serr.as_ptr(), SERR_LEN),
            )
        };
        if ret < 0 {
            Err(Error::native(describe_star_failure(
                "fixstar_ut",
                star,
                jd_ut,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(StarPosition {
                values: xx,
                resolved_name,
                returned_flags: ret,
                diagnostic,
            })
        }
    })
}

/// Calculate a fixed-star position with the extended search semantics.
///
/// Identical to [`fixstar`] except for the query language: besides the
/// traditional name, the engine also accepts a `",nomenclature"` lookup
/// (for example `",alCMa"`), a sequential catalog number (for example
/// `"65"`), or a trailing-`'%'` prefix wildcard (for example
/// `"Siri%"`). The resolved full name (`"Name,Nomenclature"`) is
/// reported in [`StarPosition::resolved_name`], which identifies which
/// catalog entry a wildcard or number matched.
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
///
/// An unavailable or empty fixed-star catalog returns an actionable
/// `Native` error before the extended native lookup. Configure
/// [`crate::set_ephe_path`] or `SE_EPHE_PATH` to a directory containing
/// the catalog. The native search path and query semantics are preserved.
pub fn fixstar2(star: &str, jd_et: f64, flags: i32) -> Result<StarPosition, Error> {
    check_star_inputs("fixstar2", star, jd_et)?;
    with_native_access(|| {
        crate::domain::check_et_locked("fixstar2", jd_et, flags)?;
        check_extended_catalog_locked("fixstar2")?;
        let mut star_buf = prepare_star_buffer(star);
        let mut xx = [0.0; POSITION_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`fixstar`].
        let ret = unsafe {
            ffi::swe_fixstar2(
                star_buf.as_mut_ptr(),
                jd_et,
                flags,
                xx.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: same contract as in [`fixstar`].
        let (resolved_name, diagnostic) = unsafe {
            (
                read_native_text(star_buf.as_ptr(), STAR_BUF_LEN),
                read_native_text(serr.as_ptr(), SERR_LEN),
            )
        };
        if ret < 0 {
            Err(Error::native(describe_star_failure(
                "fixstar2",
                star,
                jd_et,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(StarPosition {
                values: xx,
                resolved_name,
                returned_flags: ret,
                diagnostic,
            })
        }
    })
}

/// Calculate a fixed-star position with the extended search semantics for
/// a Universal Time Julian Day.
///
/// Identical to [`fixstar2`] except that `jd_ut` is Universal Time.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{fixstar2_ut, set_ephe_path};
///
/// // The nomenclature lookup resolves to the same catalog entry as the
/// // traditional name (doctests run with the package root as the
/// // working directory).
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
/// let by_name = fixstar2_ut("Sirius", 2451545.0, 0)?;
/// let by_nomenclature = fixstar2_ut(",alCMa", 2451545.0, 0)?;
/// assert_eq!(by_name.resolved_name, by_nomenclature.resolved_name);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
///
/// An unavailable or empty fixed-star catalog returns an actionable
/// `Native` error before the extended native lookup. Configure
/// [`crate::set_ephe_path`] or `SE_EPHE_PATH` to a directory containing
/// the catalog. The native search path and query semantics are preserved.
pub fn fixstar2_ut(star: &str, jd_ut: f64, flags: i32) -> Result<StarPosition, Error> {
    check_star_inputs("fixstar2_ut", star, jd_ut)?;
    with_native_access(|| {
        crate::domain::check_ut_locked("fixstar2_ut", jd_ut, flags)?;
        check_extended_catalog_locked("fixstar2_ut")?;
        let mut star_buf = prepare_star_buffer(star);
        let mut xx = [0.0; POSITION_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`fixstar`].
        let ret = unsafe {
            ffi::swe_fixstar2_ut(
                star_buf.as_mut_ptr(),
                jd_ut,
                flags,
                xx.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: same contract as in [`fixstar`].
        let (resolved_name, diagnostic) = unsafe {
            (
                read_native_text(star_buf.as_ptr(), STAR_BUF_LEN),
                read_native_text(serr.as_ptr(), SERR_LEN),
            )
        };
        if ret < 0 {
            Err(Error::native(describe_star_failure(
                "fixstar2_ut",
                star,
                jd_ut,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(StarPosition {
                values: xx,
                resolved_name,
                returned_flags: ret,
                diagnostic,
            })
        }
    })
}

/// Report the visual magnitude of a fixed star.
///
/// `star` is the traditional star name (for example `"Sirius"`). No
/// position is computed: only the catalog magnitude and the resolved
/// name are returned. Catalog and path dependence match [`fixstar`];
/// unknown stars and missing catalog data fail natively with the engine
/// diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{fixstar_mag, set_ephe_path};
///
/// // Doctests run with the package root as the working directory.
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
/// let sirius = fixstar_mag("Sirius")?;
/// assert!((sirius.magnitude - (-1.46)).abs() < 0.01);
/// assert!(sirius.resolved_name.contains("Sirius"));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn fixstar_mag(star: &str) -> Result<StarMagnitude, Error> {
    check_star_name("fixstar_mag", star)?;
    with_native_access(|| {
        let mut star_buf = prepare_star_buffer(star);
        let mut mag = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `star_buf` owns STAR_BUF_LEN writable bytes (the native
        // in/out name contract), `mag` addresses one writable slot,
        // `serr` owns SERR_LEN writable bytes; all outlive this call
        // under the native lock. Results are only exposed after the
        // native status is checked.
        let ret =
            unsafe { ffi::swe_fixstar_mag(star_buf.as_mut_ptr(), &mut mag, serr.as_mut_ptr()) };
        // SAFETY: same bounded reads as in [`fixstar`].
        let (resolved_name, diagnostic) = unsafe {
            (
                read_native_text(star_buf.as_ptr(), STAR_BUF_LEN),
                read_native_text(serr.as_ptr(), SERR_LEN),
            )
        };
        if ret < 0 {
            Err(Error::native(if diagnostic.is_empty() {
                format!("fixstar_mag: native computation failed (star={star:?})")
            } else {
                format!("fixstar_mag: {diagnostic} (star={star:?})")
            }))
        } else {
            Ok(StarMagnitude {
                magnitude: mag,
                resolved_name,
                diagnostic,
            })
        }
    })
}

/// Report the visual magnitude of a fixed star with the extended search
/// semantics.
///
/// Identical to [`fixstar_mag`] except that the query accepts the
/// [`fixstar2`] search forms (traditional name, `",nomenclature"`,
/// catalog number, `"prefix%"` wildcard).
///
/// An unavailable or empty fixed-star catalog returns an actionable
/// `Native` error before the extended native lookup. Configure
/// [`crate::set_ephe_path`] or `SE_EPHE_PATH` to a directory containing
/// the catalog. The native search path and query semantics are preserved.
pub fn fixstar2_mag(star: &str) -> Result<StarMagnitude, Error> {
    check_star_name("fixstar2_mag", star)?;
    with_native_access(|| {
        check_extended_catalog_locked("fixstar2_mag")?;
        let mut star_buf = prepare_star_buffer(star);
        let mut mag = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`fixstar_mag`].
        let ret =
            unsafe { ffi::swe_fixstar2_mag(star_buf.as_mut_ptr(), &mut mag, serr.as_mut_ptr()) };
        // SAFETY: same bounded reads as in [`fixstar`].
        let (resolved_name, diagnostic) = unsafe {
            (
                read_native_text(star_buf.as_ptr(), STAR_BUF_LEN),
                read_native_text(serr.as_ptr(), SERR_LEN),
            )
        };
        if ret < 0 {
            Err(Error::native(if diagnostic.is_empty() {
                format!("fixstar2_mag: native computation failed (star={star:?})")
            } else {
                format!("fixstar2_mag: {diagnostic} (star={star:?})")
            }))
        } else {
            Ok(StarMagnitude {
                magnitude: mag,
                resolved_name,
                diagnostic,
            })
        }
    })
}

/// Build the error message for a failed star-position computation, keeping
/// the native diagnostic when the engine provided one.
fn describe_star_failure(
    function: &str,
    star: &str,
    jd: f64,
    flags: i32,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!("{function}: native computation failed (star={star:?}, jd={jd}, flags={flags})")
    } else {
        format!("{function}: {diagnostic} (star={star:?}, jd={jd}, flags={flags})")
    }
}
