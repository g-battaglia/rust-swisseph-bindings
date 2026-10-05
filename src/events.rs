//! Longitude crossings, eclipse searches and occultations
//! and heliacal visibility events.
//!
//! [`solcross`] and [`solcross_ut`] find the next crossing of the Sun over
//! an ecliptic longitude, [`mooncross`] and [`mooncross_ut`] the same for
//! the Moon, [`mooncross_node`] and [`mooncross_node_ut`] the next passage
//! of the Moon over its own orbital node, and [`helio_cross`] /
//! [`helio_cross_ut`] the next (or previous) crossing of a planet over a
//! heliocentric longitude.
//!
//! [`sol_eclipse_when_glob`] and [`lun_eclipse_when`] search the next (or
//! previous) global solar/lunar eclipse; [`sol_eclipse_where`],
//! [`sol_eclipse_how`] and [`lun_eclipse_how`] report the geographic and
//! photometric circumstances of an eclipse at a given instant.
//!
//! [`sol_eclipse_when_loc`] and [`lun_eclipse_when_loc`] search the next
//! (or previous) solar/lunar eclipse visible from one observer;
//! [`lun_occult_when_glob`], [`lun_occult_when_loc`] and
//! [`lun_occult_where`] are the lunar-occultation counterparts (the global
//! occultation search also covers solar eclipses, less efficiently than
//! the dedicated search).
//!
//! [`heliacal_ut`] searches the next heliacal rising/setting (first/last
//! visibility) of a planet or fixed star for one observer;
//! [`heliacal_pheno_ut`] reports the visibility circumstances at an
//! instant and [`vis_limit_mag`] the visual limiting magnitude there.
//!
//! Every call holds the process-wide native lock across its complete
//! native sequence, so any thread may call any function without external
//! synchronization. Native diagnostics are preserved; a native failure
//! and valid data are distinct states.

use std::ffi::c_char;

use crate::ECL_ONE_TRY;
use crate::error::Error;
use crate::ffi::{
    self, ECLIPSE_ATTR_LEN, ECLIPSE_GEOPOS_LEN, ECLIPSE_TRET_LEN, HELIACAL_ATM_LEN,
    HELIACAL_DRET_LEN, HELIACAL_GEO_LEN, HELIACAL_OBS_LEN, HELIACAL_PHENO_LEN,
    LOCAL_SOLAR_TRET_LEN, SERR_LEN, TEXT_BUF_LEN, VISLIM_DRET_LEN, read_native_text,
};
use crate::state::with_native_access;

/// Next crossing of a body over an ecliptic longitude.
#[derive(Debug, Clone, PartialEq)]
pub struct LongitudeCrossing {
    /// Julian Day of the crossing. The scale follows the function that
    /// produced it: Ephemeris Time for [`solcross`], [`mooncross`],
    /// [`mooncross_node`] and [`helio_cross`], Universal Time for the
    /// `_ut` variants.
    pub time: f64,
    /// Native diagnostic text (`serr`); usually empty.
    pub diagnostic: String,
}

/// Next passage of the Moon over its own orbital node.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeCrossing {
    /// Julian Day of the crossing (Ephemeris Time for
    /// [`mooncross_node`], Universal Time for [`mooncross_node_ut`]).
    pub time: f64,
    /// Ecliptic longitude in degrees at the crossing.
    pub longitude: f64,
    /// Ecliptic latitude in degrees at the crossing (zero up to the
    /// native search precision).
    pub latitude: f64,
    /// Native diagnostic text (`serr`); usually empty.
    pub diagnostic: String,
}

/// Check crossing inputs before any native call.
fn check_crossing_inputs(function: &str, target: f64, jd: f64) -> Result<(), Error> {
    if !target.is_finite() || !jd.is_finite() {
        return Err(Error::invalid_input(format!(
            "{function}: target longitude and Julian Day must be finite"
        )));
    }
    if jd - 1.0 >= jd {
        return Err(Error::invalid_input(format!(
            "{function}: native error sentinel jd - 1 must be distinguishable from the epoch"
        )));
    }
    Ok(())
}

/// Classify the `double`-returning crossing result: the engine reports an
/// error by returning a Julian Day below the search epoch (`jd - 1`).
fn classify_crossing(
    function: &str,
    jd: f64,
    target: f64,
    flags: i32,
    ret: f64,
    diagnostic: String,
) -> Result<LongitudeCrossing, Error> {
    if !ret.is_finite() || ret < jd {
        Err(Error::native(describe_crossing_failure(
            function,
            jd,
            target,
            flags,
            &diagnostic,
        )))
    } else {
        Ok(LongitudeCrossing {
            time: ret,
            diagnostic,
        })
    }
}

/// Build the error message for a failed crossing search, keeping the
/// native diagnostic when the engine provided one.
fn describe_crossing_failure(
    function: &str,
    jd: f64,
    target: f64,
    flags: i32,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!("{function}: native search failed (jd={jd}, target={target}, flags={flags})")
    } else {
        format!("{function}: {diagnostic} (jd={jd}, target={target}, flags={flags})")
    }
}

/// The heliocentric Sun has no changing longitude/speed. The pinned
/// solar crossing solver does not terminate for this configuration.
fn check_solar_crossing_flags(function: &str, flags: i32) -> Result<(), Error> {
    if flags & crate::FLG_HELCTR != 0 {
        return Err(Error::invalid_input(format!(
            "{function}: heliocentric Sun crossings are undefined (FLG_HELCTR)"
        )));
    }
    Ok(())
}

/// Find the next crossing of the Sun over an ecliptic longitude.
///
/// `x2cross` is the target longitude in degrees (any finite value; the
/// engine normalizes it) and `jd_et` the Ephemeris Time search epoch.
/// `flags` carries the ephemeris source bits
/// (`FLG_SWIEPH`/`FLG_JPLEPH`/`FLG_MOSEPH`).
///
/// The native search always runs forward: the result is strictly after
/// `jd_et`. There is no native backward search; pass an earlier epoch
/// to search for an earlier crossing.
///
/// # Configuration dependence
///
/// File-based sources require data files visible through
/// [`crate::set_ephe_path`]; the active configuration is captured under
/// the process-wide native lock together with the search.
///
/// # Errors
///
/// Non-finite `x2cross`/`jd_et` are rejected before any native call. A
/// native failure (failed position computation) becomes [`Error`] with
/// the diagnostic kept.
/// `FLG_HELCTR` returns `InvalidInput` before acquiring native access:
/// the heliocentric Sun has no changing longitude or speed and the
/// pinned native solar-crossing search cannot terminate for this choice.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, solcross};
///
/// // Next 0° Aries ingress of the Moshier Sun after J2000.
/// let cross = solcross(0.0, 2451545.0, FLG_MOSEPH)?;
/// assert!(cross.time > 2451545.0 && cross.time < 2451545.0 + 400.0);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn solcross(x2cross: f64, jd_et: f64, flags: i32) -> Result<LongitudeCrossing, Error> {
    check_solar_crossing_flags("solcross", flags)?;
    check_crossing_inputs("solcross", x2cross, jd_et)?;
    with_native_access(|| {
        crate::domain::check_et_locked("solcross", jd_et, flags)?;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `serr` owns SERR_LEN writable bytes and outlives this
        // call under the native lock. The result is only exposed after
        // the below-epoch error sentinel is checked.
        let ret = unsafe { ffi::swe_solcross(x2cross, jd_et, flags, serr.as_mut_ptr()) };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        classify_crossing("solcross", jd_et, x2cross, flags, ret, diagnostic)
    })
}

/// Find the next crossing of the Sun over an ecliptic longitude for a
/// Universal Time epoch.
///
/// Identical to [`solcross`] except that `jd_ut` is Universal Time and
/// the result is Universal Time. `FLG_HELCTR` is likewise rejected with
/// `InvalidInput` before acquiring native access.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, solcross_ut};
///
/// let cross = solcross_ut(0.0, 2451545.0, FLG_MOSEPH)?;
/// assert!(cross.time > 2451545.0 && cross.time < 2451545.0 + 400.0);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn solcross_ut(x2cross: f64, jd_ut: f64, flags: i32) -> Result<LongitudeCrossing, Error> {
    check_solar_crossing_flags("solcross_ut", flags)?;
    check_crossing_inputs("solcross_ut", x2cross, jd_ut)?;
    with_native_access(|| {
        crate::domain::check_ut_locked("solcross_ut", jd_ut, flags)?;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`solcross`].
        let ret = unsafe { ffi::swe_solcross_ut(x2cross, jd_ut, flags, serr.as_mut_ptr()) };
        // SAFETY: same contract as in [`solcross`].
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        classify_crossing("solcross_ut", jd_ut, x2cross, flags, ret, diagnostic)
    })
}

/// Find the next crossing of the Moon over an ecliptic longitude.
///
/// Identical to [`solcross`] except that the Moon is searched: the result
/// follows within one sidereal month (~27.3 days).
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, mooncross};
///
/// let cross = mooncross(0.0, 2451545.0, FLG_MOSEPH)?;
/// assert!(cross.time > 2451545.0 && cross.time < 2451545.0 + 30.0);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn mooncross(x2cross: f64, jd_et: f64, flags: i32) -> Result<LongitudeCrossing, Error> {
    check_crossing_inputs("mooncross", x2cross, jd_et)?;
    with_native_access(|| {
        crate::domain::check_et_locked("mooncross", jd_et, flags)?;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`solcross`].
        let ret = unsafe { ffi::swe_mooncross(x2cross, jd_et, flags, serr.as_mut_ptr()) };
        // SAFETY: same contract as in [`solcross`].
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        classify_crossing("mooncross", jd_et, x2cross, flags, ret, diagnostic)
    })
}

/// Find the next crossing of the Moon over an ecliptic longitude for a
/// Universal Time epoch.
///
/// Identical to [`mooncross`] except that `jd_ut` is Universal Time and
/// the result is Universal Time.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, mooncross_ut};
///
/// let cross = mooncross_ut(0.0, 2451545.0, FLG_MOSEPH)?;
/// assert!(cross.time > 2451545.0 && cross.time < 2451545.0 + 30.0);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn mooncross_ut(x2cross: f64, jd_ut: f64, flags: i32) -> Result<LongitudeCrossing, Error> {
    check_crossing_inputs("mooncross_ut", x2cross, jd_ut)?;
    with_native_access(|| {
        crate::domain::check_ut_locked("mooncross_ut", jd_ut, flags)?;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`solcross`].
        let ret = unsafe { ffi::swe_mooncross_ut(x2cross, jd_ut, flags, serr.as_mut_ptr()) };
        // SAFETY: same contract as in [`solcross`].
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        classify_crossing("mooncross_ut", jd_ut, x2cross, flags, ret, diagnostic)
    })
}

/// Find the next passage of the Moon over its own orbital node.
///
/// The Moon crosses a node when its ecliptic latitude is zero, either
/// ascending (south to north) or descending — whichever comes first after
/// `jd_et` (Ephemeris Time). `flags` carries the ephemeris source bits.
/// The search always runs forward (same explicit gap as [`solcross`]).
///
/// # Configuration dependence
///
/// Same contract as [`solcross`].
///
/// # Errors
///
/// Non-finite `jd_et` is rejected before any native call. A native
/// failure becomes [`Error`] with the diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, mooncross_node};
///
/// let cross = mooncross_node(2451545.0, FLG_MOSEPH)?;
/// assert!(cross.time > 2451545.0 && cross.time < 2451545.0 + 15.0);
/// assert!(cross.latitude.abs() < 1e-3);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn mooncross_node(jd_et: f64, flags: i32) -> Result<NodeCrossing, Error> {
    if !jd_et.is_finite() {
        return Err(Error::invalid_input(
            "mooncross_node: Julian Day must be finite",
        ));
    }
    with_native_access(|| {
        crate::domain::check_et_locked("mooncross_node", jd_et, flags)?;
        let mut xlon = 0.0;
        let mut xlat = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: each of `xlon`/`xlat` addresses exactly one owned,
        // initialized local and `serr` owns SERR_LEN writable bytes; all
        // outlive this call under the native lock. Results are only
        // exposed after the below-epoch error sentinel is checked.
        let ret = unsafe {
            ffi::swe_mooncross_node(jd_et, flags, &mut xlon, &mut xlat, serr.as_mut_ptr())
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if !ret.is_finite() || ret < jd_et {
            Err(Error::native(describe_node_failure(
                "mooncross_node",
                jd_et,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(NodeCrossing {
                time: ret,
                longitude: xlon,
                latitude: xlat,
                diagnostic,
            })
        }
    })
}

/// Find the next passage of the Moon over its own orbital node for a
/// Universal Time epoch.
///
/// Identical to [`mooncross_node`] except that `jd_ut` is Universal Time
/// and the result is Universal Time.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, mooncross_node_ut};
///
/// let cross = mooncross_node_ut(2451545.0, FLG_MOSEPH)?;
/// assert!(cross.time > 2451545.0 && cross.time < 2451545.0 + 15.0);
/// assert!(cross.latitude.abs() < 1e-3);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn mooncross_node_ut(jd_ut: f64, flags: i32) -> Result<NodeCrossing, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(
            "mooncross_node_ut: Julian Day must be finite",
        ));
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("mooncross_node_ut", jd_ut, flags)?;
        let mut xlon = 0.0;
        let mut xlat = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`mooncross_node`].
        let ret = unsafe {
            ffi::swe_mooncross_node_ut(jd_ut, flags, &mut xlon, &mut xlat, serr.as_mut_ptr())
        };
        // SAFETY: same contract as in [`mooncross_node`].
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if !ret.is_finite() || ret < jd_ut {
            Err(Error::native(describe_node_failure(
                "mooncross_node_ut",
                jd_ut,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(NodeCrossing {
                time: ret,
                longitude: xlon,
                latitude: xlat,
                diagnostic,
            })
        }
    })
}

/// Build the error message for a failed node search, keeping the native
/// diagnostic when the engine provided one.
fn describe_node_failure(function: &str, jd: f64, flags: i32, diagnostic: &str) -> String {
    if diagnostic.is_empty() {
        format!("{function}: native search failed (jd={jd}, flags={flags})")
    } else {
        format!("{function}: {diagnostic} (jd={jd}, flags={flags})")
    }
}

/// Find the next (or previous) crossing of a planet over a heliocentric
/// ecliptic longitude.
///
/// `body` is a body number (for example [`crate::MARS`]); `x2cross` is
/// the target heliocentric longitude in degrees (any finite value) and
/// `jd_et` the Ephemeris Time search epoch. `flags` carries the
/// ephemeris source bits. `dir` selects the search direction natively:
/// `dir >= 0` searches forward (result after `jd_et`), `dir < 0`
/// searches backward (result before `jd_et`).
///
/// The Sun, the Moon, the nodes/apsides and out-of-range numbers have no
/// heliocentric longitude and are rejected natively with a diagnostic
/// (for example `"swe_helio_cross: not possible for object 0 = Sun"`).
///
/// # Configuration dependence
///
/// Same contract as [`solcross`].
///
/// # Errors
///
/// Non-finite `x2cross`/`jd_et` are rejected before any native call. A
/// negative native status becomes [`Error`] with the diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, MARS, helio_cross};
///
/// let cross = helio_cross(MARS, 0.0, 2451545.0, FLG_MOSEPH, 1)?;
/// assert!(cross.time > 2451545.0);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn helio_cross(
    body: i32,
    x2cross: f64,
    jd_et: f64,
    flags: i32,
    dir: i32,
) -> Result<LongitudeCrossing, Error> {
    check_crossing_inputs("helio_cross", x2cross, jd_et)?;
    with_native_access(|| {
        crate::domain::check_et_locked("helio_cross", jd_et, flags)?;
        let mut jd_cross = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `jd_cross` addresses exactly one owned, initialized
        // local and `serr` owns SERR_LEN writable bytes; both outlive
        // this call under the native lock. The result is only exposed
        // after the native status is checked.
        let ret = unsafe {
            ffi::swe_helio_cross(
                body,
                x2cross,
                jd_et,
                flags,
                dir,
                &mut jd_cross,
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_helio_failure(
                "helio_cross",
                body,
                jd_et,
                x2cross,
                flags,
                dir,
                &diagnostic,
            )))
        } else {
            Ok(LongitudeCrossing {
                time: jd_cross,
                diagnostic,
            })
        }
    })
}

/// Find the next (or previous) crossing of a planet over a heliocentric
/// ecliptic longitude for a Universal Time epoch.
///
/// Identical to [`helio_cross`] except that `jd_ut` is Universal Time and
/// the result is Universal Time.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, MARS, helio_cross_ut};
///
/// let cross = helio_cross_ut(MARS, 0.0, 2451545.0, FLG_MOSEPH, 1)?;
/// assert!(cross.time > 2451545.0);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn helio_cross_ut(
    body: i32,
    x2cross: f64,
    jd_ut: f64,
    flags: i32,
    dir: i32,
) -> Result<LongitudeCrossing, Error> {
    check_crossing_inputs("helio_cross_ut", x2cross, jd_ut)?;
    with_native_access(|| {
        crate::domain::check_ut_locked("helio_cross_ut", jd_ut, flags)?;
        let mut jd_cross = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`helio_cross`].
        let ret = unsafe {
            ffi::swe_helio_cross_ut(
                body,
                x2cross,
                jd_ut,
                flags,
                dir,
                &mut jd_cross,
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: same contract as in [`helio_cross`].
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_helio_failure(
                "helio_cross_ut",
                body,
                jd_ut,
                x2cross,
                flags,
                dir,
                &diagnostic,
            )))
        } else {
            Ok(LongitudeCrossing {
                time: jd_cross,
                diagnostic,
            })
        }
    })
}

/// Build the error message for a failed heliocentric search, keeping the
/// native diagnostic when the engine provided one.
#[allow(clippy::too_many_arguments)]
fn describe_helio_failure(
    function: &str,
    body: i32,
    jd: f64,
    target: f64,
    flags: i32,
    dir: i32,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!(
            "{function}: native search failed (body={body}, jd={jd}, target={target}, flags={flags}, dir={dir})"
        )
    } else {
        format!(
            "{function}: {diagnostic} (body={body}, jd={jd}, target={target}, flags={flags}, dir={dir})"
        )
    }
}

/// Next global solar eclipse: type and phase times in Universal Time.
///
/// The `times` layout is the native `tret` order:
///
/// 0. time of maximum eclipse,
/// 1. time when the eclipse is central at local apparent noon
///    (0.0 when there is none),
/// 2. eclipse begins (penumbra first touches the Earth anywhere),
/// 3. eclipse ends (penumbra last leaves the Earth anywhere),
/// 4. totality/annularity begins (umbra/antumbra first touches),
/// 5. totality/annularity ends,
/// 6. central line begins,
/// 7. central line ends,
/// 8. hybrid transition to total (reserved, 0.0 — not implemented),
/// 9. hybrid transition back to annular (reserved, 0.0 — not implemented).
///
/// Entries that do not apply to the found type read 0.0; they are absent
/// data, never errors.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalSolarEclipse {
    /// Found eclipse-type bitmask (`ECL_TOTAL`, `ECL_PARTIAL`, ...,
    /// combined with `ECL_CENTRAL`/`ECL_NONCENTRAL`).
    pub eclipse_type: i32,
    /// The ten native phase times; see the type documentation.
    pub times: [f64; ECLIPSE_TRET_LEN],
    /// Native diagnostic text (`serr`); usually empty.
    pub diagnostic: String,
}

impl GlobalSolarEclipse {
    /// Time of maximum eclipse in Universal Time (component 0).
    pub fn maximum(&self) -> f64 {
        self.times[0]
    }

    /// Eclipse begins: penumbra first touches the Earth (component 2,
    /// 0.0 when unresolved).
    pub fn begin(&self) -> f64 {
        self.times[2]
    }

    /// Eclipse ends: penumbra last leaves the Earth (component 3, 0.0
    /// when unresolved).
    pub fn end(&self) -> f64 {
        self.times[3]
    }

    /// Totality/annularity begins anywhere on Earth (component 4, 0.0
    /// when there is none).
    pub fn totality_begin(&self) -> f64 {
        self.times[4]
    }

    /// Totality/annularity ends anywhere on Earth (component 5, 0.0 when
    /// there is none).
    pub fn totality_end(&self) -> f64 {
        self.times[5]
    }

    /// Central line begins (component 6, 0.0 when there is none).
    pub fn centerline_begin(&self) -> f64 {
        self.times[6]
    }

    /// Central line ends (component 7, 0.0 when there is none).
    pub fn centerline_end(&self) -> f64 {
        self.times[7]
    }
}

/// Next global lunar eclipse: type and phase times in Universal Time.
///
/// The `times` layout is the native `tret` order:
///
/// 0. time of maximum eclipse,
/// 1. reserved (0.0),
/// 2. partial eclipse begins (Moon enters the umbra),
/// 3. partial eclipse ends (Moon leaves the umbra),
/// 4. total eclipse begins (0.0 when not total),
/// 5. total eclipse ends (0.0 when not total),
/// 6. penumbral eclipse begins,
/// 7. penumbral eclipse ends,
/// 8. reserved (0.0),
/// 9. reserved (0.0).
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalLunarEclipse {
    /// Found eclipse-type bitmask (`ECL_TOTAL`, `ECL_PARTIAL`,
    /// `ECL_PENUMBRAL`).
    pub eclipse_type: i32,
    /// The ten native phase times; see the type documentation.
    pub times: [f64; ECLIPSE_TRET_LEN],
    /// Native diagnostic text (`serr`); usually empty.
    pub diagnostic: String,
}

impl GlobalLunarEclipse {
    /// Time of maximum eclipse in Universal Time (component 0).
    pub fn maximum(&self) -> f64 {
        self.times[0]
    }

    /// Partial eclipse begins: umbra entry (component 2).
    pub fn partial_begin(&self) -> f64 {
        self.times[2]
    }

    /// Partial eclipse ends: umbra exit (component 3).
    pub fn partial_end(&self) -> f64 {
        self.times[3]
    }

    /// Total eclipse begins (component 4, 0.0 when not total).
    pub fn total_begin(&self) -> f64 {
        self.times[4]
    }

    /// Total eclipse ends (component 5, 0.0 when not total).
    pub fn total_end(&self) -> f64 {
        self.times[5]
    }

    /// Penumbral eclipse begins (component 6).
    pub fn penumbral_begin(&self) -> f64 {
        self.times[6]
    }

    /// Penumbral eclipse ends (component 7).
    pub fn penumbral_end(&self) -> f64 {
        self.times[7]
    }
}

/// Check eclipse-search inputs before any native call.
fn check_eclipse_search_inputs(function: &str, jd_start: f64) -> Result<(), Error> {
    // Whole-day bounds rounded inward from the pinned native signed
    // lunation-counter casts, established by instrumented public-ABI
    // probes. These preserve the full supported ephemeris coverage.
    if !jd_start.is_finite() || !(-63_412_861_279.0..=63_417_764_339.0).contains(&jd_start) {
        return Err(Error::invalid_input(format!(
            "{function}: search epoch exceeds the native i32 lunation-counter domain"
        )));
    }
    Ok(())
}

/// Find the next (or previous) global solar eclipse.
///
/// `jd_start` is the Universal Time search epoch. `flags` carries the
/// ephemeris source bits. `eclipse_type` filters the wanted types
/// (`ECL_TOTAL`, `ECL_ANNULAR`, `ECL_PARTIAL`, `ECL_ANNULAR_TOTAL`,
/// combined with `ECL_CENTRAL`/`ECL_NONCENTRAL`; 0 accepts any type).
/// `backward` selects the search direction (`false` = forward).
///
/// Impossible combinations (for example central + partial) fail natively
/// with a diagnostic instead of searching.
///
/// # Configuration dependence
///
/// Same contract as [`solcross`].
///
/// # Errors
///
/// Non-finite `jd_start` is rejected before any native call. A negative
/// native status becomes [`Error`] with the diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{ECL_TOTAL, FLG_MOSEPH, sol_eclipse_when_glob};
///
/// // Next total solar eclipse after 2024-01-01 (Moshier needs no data).
/// let ecl = sol_eclipse_when_glob(2460310.5, FLG_MOSEPH, ECL_TOTAL, false)?;
/// assert!(ecl.maximum() > 2460310.5);
/// assert!(ecl.begin() <= ecl.maximum() && ecl.maximum() <= ecl.end());
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
/// Eclipse search epochs additionally lie within JD
/// `-63412861279.0..=63417764339.0`, the inward-rounded bounds of the
/// pinned native integer lunation counter. These are arithmetic limits,
/// independent of actual ephemeris file coverage.
pub fn sol_eclipse_when_glob(
    jd_start: f64,
    flags: i32,
    eclipse_type: i32,
    backward: bool,
) -> Result<GlobalSolarEclipse, Error> {
    check_eclipse_search_inputs("sol_eclipse_when_glob", jd_start)?;
    with_native_access(|| {
        crate::domain::check_ut_locked("sol_eclipse_when_glob", jd_start, flags)?;
        let mut tret = [0.0; ECLIPSE_TRET_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `tret` owns ECLIPSE_TRET_LEN writable slots and `serr`
        // owns SERR_LEN writable bytes; both outlive this call under the
        // native lock. Results are only exposed after the native status
        // is checked.
        let ret = unsafe {
            ffi::swe_sol_eclipse_when_glob(
                jd_start,
                flags,
                eclipse_type,
                tret.as_mut_ptr(),
                i32::from(backward),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_eclipse_search_failure(
                "sol_eclipse_when_glob",
                jd_start,
                flags,
                eclipse_type,
                backward,
                &diagnostic,
            )))
        } else {
            Ok(GlobalSolarEclipse {
                eclipse_type: ret,
                times: tret,
                diagnostic,
            })
        }
    })
}

/// Find the next (or previous) global lunar eclipse.
///
/// `jd_start` is the Universal Time search epoch. `flags` carries the
/// ephemeris source bits. `eclipse_type` filters the wanted types
/// (`ECL_TOTAL`, `ECL_PARTIAL`, `ECL_PENUMBRAL`; 0 accepts any type —
/// central/noncentral bits are meaningless here and annular-only
/// requests fail natively). `backward` selects the search direction.
///
/// # Configuration dependence
///
/// Same contract as [`solcross`].
///
/// # Errors
///
/// Same contract as [`sol_eclipse_when_glob`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{ECL_TOTAL, FLG_MOSEPH, lun_eclipse_when};
///
/// // Next total lunar eclipse after 2024-01-01 (Moshier needs no data).
/// let ecl = lun_eclipse_when(2460310.5, FLG_MOSEPH, ECL_TOTAL, false)?;
/// assert!(ecl.maximum() > 2460310.5);
/// assert!(ecl.partial_begin() <= ecl.maximum() && ecl.maximum() <= ecl.partial_end());
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
/// Eclipse search epochs additionally lie within JD
/// `-63412861279.0..=63417764339.0`, the inward-rounded bounds of the
/// pinned native integer lunation counter. These are arithmetic limits,
/// independent of actual ephemeris file coverage.
pub fn lun_eclipse_when(
    jd_start: f64,
    flags: i32,
    eclipse_type: i32,
    backward: bool,
) -> Result<GlobalLunarEclipse, Error> {
    check_eclipse_search_inputs("lun_eclipse_when", jd_start)?;
    with_native_access(|| {
        crate::domain::check_ut_locked("lun_eclipse_when", jd_start, flags)?;
        let mut tret = [0.0; ECLIPSE_TRET_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`sol_eclipse_when_glob`].
        let ret = unsafe {
            ffi::swe_lun_eclipse_when(
                jd_start,
                flags,
                eclipse_type,
                tret.as_mut_ptr(),
                i32::from(backward),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: same contract as in [`sol_eclipse_when_glob`].
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_eclipse_search_failure(
                "lun_eclipse_when",
                jd_start,
                flags,
                eclipse_type,
                backward,
                &diagnostic,
            )))
        } else {
            Ok(GlobalLunarEclipse {
                eclipse_type: ret,
                times: tret,
                diagnostic,
            })
        }
    })
}

/// Build the error message for a failed eclipse search, keeping the
/// native diagnostic when the engine provided one.
fn describe_eclipse_search_failure(
    function: &str,
    jd_start: f64,
    flags: i32,
    eclipse_type: i32,
    backward: bool,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!(
            "{function}: native search failed (jd_start={jd_start}, flags={flags}, eclipse_type={eclipse_type}, backward={backward})"
        )
    } else {
        format!(
            "{function}: {diagnostic} (jd_start={jd_start}, flags={flags}, eclipse_type={eclipse_type}, backward={backward})"
        )
    }
}

/// Geographic circumstances of the solar eclipse in progress at an
/// instant, as computed by [`sol_eclipse_where`].
#[derive(Debug, Clone, PartialEq)]
pub struct SolarEclipseGeometry {
    /// Eastern longitude in degrees of the greatest eclipse.
    pub longitude: f64,
    /// Northern latitude in degrees of the greatest eclipse.
    pub latitude: f64,
    /// Eclipse-type bitmask (`ECL_TOTAL`, `ECL_PARTIAL`, ..., 0 when no
    /// eclipse is in progress at this instant — kept as data, not an
    /// error).
    pub eclipse_type: i32,
    /// The twenty native circumstance components; see
    /// [`SolarEclipseCircumstances`] for the slot order shared with
    /// [`sol_eclipse_how`].
    pub attributes: [f64; ECLIPSE_ATTR_LEN],
    /// Native diagnostic text (`serr`); carries e.g. `"no solar eclipse
    /// at tjd = ..."` when `eclipse_type` is 0.
    pub diagnostic: String,
}

/// Local circumstances of a solar eclipse at an instant and observer, as
/// computed by [`sol_eclipse_how`] (and shared with
/// [`sol_eclipse_where`]).
///
/// Component order and units are exactly the native `attr` layout:
///
/// 0. fraction of the solar diameter covered by the Moon (magnitude),
/// 1. ratio of the lunar diameter to the solar one,
/// 2. fraction of the solar disc covered by the Moon (obscuration),
/// 3. diameter of the core shadow in kilometers,
/// 4. azimuth of the Sun in degrees,
/// 5. true altitude of the Sun above the horizon in degrees,
/// 6. apparent altitude of the Sun above the horizon in degrees,
/// 7. angular distance of the Moon from the Sun in degrees,
/// 8. magnitude acc. to NASA (`= attr[0]` for partial, `= attr[1]` for
///    annular/total eclipses),
/// 9. saros series number,
/// 10. saros series member number,
///
/// with slots 11-19 reserved as 0.0. When no eclipse is in progress the
/// engine zeroes the magnitude slots and reports type 0 (preserved
/// verbatim, not reinterpreted).
#[derive(Debug, Clone, PartialEq)]
pub struct SolarEclipseCircumstances {
    /// Eclipse phase bitmask at this place and instant (`ECL_TOTAL`,
    /// `ECL_PARTIAL`, ..., 0 when no eclipse is in progress — kept as
    /// data, not an error).
    pub eclipse_type: i32,
    /// The twenty native circumstance components; see the type
    /// documentation.
    pub attributes: [f64; ECLIPSE_ATTR_LEN],
    /// Native diagnostic text (`serr`); empty on a clean computation.
    pub diagnostic: String,
}

impl SolarEclipseCircumstances {
    /// Fraction of the solar diameter covered (magnitude, component 0).
    pub fn magnitude(&self) -> f64 {
        self.attributes[0]
    }

    /// Ratio of the lunar diameter to the solar one (component 1).
    pub fn diameter_ratio(&self) -> f64 {
        self.attributes[1]
    }

    /// Fraction of the solar disc covered (obscuration, component 2).
    pub fn obscuration(&self) -> f64 {
        self.attributes[2]
    }

    /// Diameter of the core shadow in kilometers (component 3).
    pub fn core_shadow_km(&self) -> f64 {
        self.attributes[3]
    }

    /// Azimuth of the Sun in degrees (component 4).
    pub fn azimuth(&self) -> f64 {
        self.attributes[4]
    }

    /// True altitude of the Sun in degrees (component 5).
    pub fn true_altitude(&self) -> f64 {
        self.attributes[5]
    }

    /// Apparent altitude of the Sun in degrees (component 6).
    pub fn apparent_altitude(&self) -> f64 {
        self.attributes[6]
    }

    /// Angular distance of the Moon from the Sun in degrees
    /// (component 7).
    pub fn separation(&self) -> f64 {
        self.attributes[7]
    }

    /// Saros series number (component 9).
    pub fn saros_series(&self) -> f64 {
        self.attributes[9]
    }

    /// Saros series member number (component 10).
    pub fn saros_member(&self) -> f64 {
        self.attributes[10]
    }
}

/// Circumstances of a lunar eclipse at an instant, as computed by
/// [`lun_eclipse_how`].
///
/// Component order and units are exactly the native `attr` layout:
///
/// 0. umbral magnitude at the instant,
/// 1. penumbral magnitude at the instant,
/// 2. unused (0.0),
/// 3. unused (0.0),
/// 4. azimuth of the Moon in degrees (0.0 for the geocentric call),
/// 5. true altitude of the Moon above the horizon in degrees (0.0 for
///    the geocentric call),
/// 6. apparent altitude of the Moon above the horizon in degrees (0.0
///    for the geocentric call),
/// 7. distance of the Moon from opposition in degrees,
/// 8. umbral magnitude (`= attr[0]`),
/// 9. saros series number,
/// 10. saros series member number,
///
/// with slots 11-19 reserved as 0.0. When no eclipse is in progress the
/// engine reports type 0 (preserved verbatim, not reinterpreted).
#[derive(Debug, Clone, PartialEq)]
pub struct LunarEclipseCircumstances {
    /// Eclipse-type bitmask (`ECL_TOTAL`, `ECL_PARTIAL`,
    /// `ECL_PENUMBRAL`, 0 when no (visible) eclipse is in progress —
    /// kept as data, not an error).
    pub eclipse_type: i32,
    /// The twenty native circumstance components; see the type
    /// documentation.
    pub attributes: [f64; ECLIPSE_ATTR_LEN],
    /// Native diagnostic text (`serr`); empty on a clean computation.
    pub diagnostic: String,
}

impl LunarEclipseCircumstances {
    /// Umbral magnitude at the instant (component 0).
    pub fn umbral_magnitude(&self) -> f64 {
        self.attributes[0]
    }

    /// Penumbral magnitude at the instant (component 1).
    pub fn penumbral_magnitude(&self) -> f64 {
        self.attributes[1]
    }

    /// Azimuth of the Moon in degrees (component 4; 0.0 geocentric).
    pub fn azimuth(&self) -> f64 {
        self.attributes[4]
    }

    /// True altitude of the Moon in degrees (component 5; 0.0
    /// geocentric).
    pub fn true_altitude(&self) -> f64 {
        self.attributes[5]
    }

    /// Apparent altitude of the Moon in degrees (component 6; 0.0
    /// geocentric).
    pub fn apparent_altitude(&self) -> f64 {
        self.attributes[6]
    }

    /// Distance of the Moon from opposition in degrees (component 7).
    pub fn opposition_distance(&self) -> f64 {
        self.attributes[7]
    }

    /// Saros series number (component 9).
    pub fn saros_series(&self) -> f64 {
        self.attributes[9]
    }

    /// Saros series member number (component 10).
    pub fn saros_member(&self) -> f64 {
        self.attributes[10]
    }
}

/// Report the geographic circumstances of the solar eclipse in progress
/// at an instant.
///
/// `jd_ut` is Universal Time; `flags` carries the ephemeris source bits.
/// The result holds the longitude/latitude of the greatest eclipse plus
/// the same twenty circumstance components as [`sol_eclipse_how`].
///
/// A return type of 0 means no solar eclipse is in progress at `jd_ut`:
/// that is a dedicated `Ok` state with the native diagnostic kept (for
/// example `"no solar eclipse at tjd = ..."`), never an error and never
/// invented geometry.
///
/// # Configuration dependence
///
/// Same contract as [`solcross`].
///
/// # Errors
///
/// Non-finite `jd_ut` is rejected before any native call. A negative
/// native status (failed position computation) becomes [`Error`] with
/// the diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, sol_eclipse_where};
///
/// // Greatest eclipse of the 2024-04-08 total solar eclipse.
/// let geo = sol_eclipse_where(2460409.26, FLG_MOSEPH)?;
/// assert!(geo.eclipse_type != 0);
/// assert!((geo.longitude + 104.0).abs() < 5.0);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn sol_eclipse_where(jd_ut: f64, flags: i32) -> Result<SolarEclipseGeometry, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(
            "sol_eclipse_where: Julian Day must be finite",
        ));
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("sol_eclipse_where", jd_ut, flags)?;
        let mut geopos = [0.0; ECLIPSE_GEOPOS_LEN];
        let mut attr = [0.0; ECLIPSE_ATTR_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `geopos` owns ECLIPSE_GEOPOS_LEN writable slots and
        // `attr` owns ECLIPSE_ATTR_LEN writable slots (more than the
        // engine fills: slots 0-10, established by probing); `serr` owns
        // SERR_LEN writable bytes. All outlive this call under the native
        // lock. Results are only exposed after the native status is
        // classified.
        let ret = unsafe {
            ffi::swe_sol_eclipse_where(
                jd_ut,
                flags,
                geopos.as_mut_ptr(),
                attr.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_eclipse_moment_failure(
                "sol_eclipse_where",
                jd_ut,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(SolarEclipseGeometry {
                longitude: geopos[0],
                latitude: geopos[1],
                eclipse_type: ret,
                attributes: attr,
                diagnostic,
            })
        }
    })
}

/// Report the local circumstances of the solar eclipse at an instant and
/// observer position.
///
/// `jd_ut` is Universal Time; `flags` carries the ephemeris source bits.
/// `longitude`/`latitude`/`altitude` locate the observer (east-positive
/// degrees, degrees north, meters; altitudes outside −500…25000 m fail
/// natively with a diagnostic).
///
/// Observer side effect: although no search runs, the native call still
/// installs this observer as the process-global observer (as if by
/// [`crate::set_topo`]), so later topocentric [`crate::calc_ut`]/
/// [`crate::calc`] calls observe it until [`crate::set_topo`] runs
/// again. Established by probing the pinned build.
///
/// A return type of 0 means no eclipse is in progress at this place and
/// instant: a dedicated `Ok` state with zeroed magnitude slots, never an
/// error.
///
/// # Configuration dependence
///
/// Same contract as [`solcross`].
///
/// # Errors
///
/// Non-finite time/observer inputs are rejected before any native call. A
/// negative native status becomes [`Error`] with the diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, sol_eclipse_how};
///
/// // Totality near the greatest eclipse (25.3°N, 104.1°W) at maximum.
/// let loc = sol_eclipse_how(2460409.26, FLG_MOSEPH, -104.1, 25.3, 0.0)?;
/// assert!(loc.eclipse_type != 0);
/// assert!(loc.magnitude() > 0.9);
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
/// Observer height must satisfy the arithmetic ceiling documented by
/// [`crate::set_topo`]; excessive height returns `InvalidInput` before
/// observer configuration or computation. Native geographic limits and
/// data errors are still reported by the engine.
#[allow(clippy::too_many_arguments)]
pub fn sol_eclipse_how(
    jd_ut: f64,
    flags: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
) -> Result<SolarEclipseCircumstances, Error> {
    crate::domain::check_observer_height("sol_eclipse_how", altitude)?;
    if !jd_ut.is_finite()
        || !longitude.is_finite()
        || !latitude.is_finite()
        || !altitude.is_finite()
    {
        return Err(Error::invalid_input(
            "sol_eclipse_how: time and observer inputs must be finite",
        ));
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("sol_eclipse_how", jd_ut, flags)?;
        let geopos = [longitude, latitude, altitude];
        let mut attr = [0.0; ECLIPSE_ATTR_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `geopos` owns three readable doubles, `attr` owns
        // ECLIPSE_ATTR_LEN writable slots and `serr` owns SERR_LEN
        // writable bytes; all outlive this call under the native lock.
        // Results are only exposed after the native status is classified.
        let ret = unsafe {
            ffi::swe_sol_eclipse_how(
                jd_ut,
                flags,
                geopos.as_ptr(),
                attr.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_eclipse_moment_failure(
                "sol_eclipse_how",
                jd_ut,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(SolarEclipseCircumstances {
                eclipse_type: ret,
                attributes: attr,
                diagnostic,
            })
        }
    })
}

/// Report the circumstances of the lunar eclipse at an instant.
///
/// `jd_ut` is Universal Time; `flags` carries the ephemeris source bits.
/// `observer` is either `None` (geocentric magnitudes; azimuth/altitude
/// stay 0.0) or `Some((longitude, latitude, altitude))` with the same
/// observer convention and altitude domain as [`sol_eclipse_how`].
///
/// Observer side effect: although no search runs, the native call still
/// installs a `Some` observer as the process-global observer (as if by
/// [`crate::set_topo`]), so later topocentric [`crate::calc_ut`]/
/// [`crate::calc`] calls observe it until [`crate::set_topo`] runs
/// again. Established by probing the pinned build.
///
/// A return type of 0 means no (from the observer: visible) eclipse is
/// in progress: a dedicated `Ok` state, never an error.
///
/// # Configuration dependence
///
/// Same contract as [`solcross`].
///
/// # Errors
///
/// Non-finite time/observer inputs are rejected before any native call. A
/// negative native status becomes [`Error`] with the diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, lun_eclipse_how};
///
/// // Near the maximum of the 2024-09-18 partial lunar eclipse.
/// let lun = lun_eclipse_how(2460571.61, FLG_MOSEPH, None)?;
/// assert!(lun.eclipse_type != 0);
/// assert!(lun.umbral_magnitude() > 0.0);
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
/// Observer height must satisfy the arithmetic ceiling documented by
/// [`crate::set_topo`]; excessive height returns `InvalidInput` before
/// observer configuration or computation. Native geographic limits and
/// data errors are still reported by the engine.
pub fn lun_eclipse_how(
    jd_ut: f64,
    flags: i32,
    observer: Option<(f64, f64, f64)>,
) -> Result<LunarEclipseCircumstances, Error> {
    if let Some((_, _, altitude)) = observer {
        crate::domain::check_observer_height("lun_eclipse_how", altitude)?;
    }
    if !jd_ut.is_finite()
        || observer
            .is_some_and(|(lon, lat, alt)| !lon.is_finite() || !lat.is_finite() || !alt.is_finite())
    {
        return Err(Error::invalid_input(
            "lun_eclipse_how: time and observer inputs must be finite",
        ));
    }
    // Caller-owned observer buffer: the native call only reads it, and
    // every access stays inside this buffer. `None` passes null, which
    // the engine treats as a geocentric request.
    let geopos = observer.map(|(lon, lat, alt)| [lon, lat, alt]);
    with_native_access(|| {
        crate::domain::check_ut_locked("lun_eclipse_how", jd_ut, flags)?;
        let mut attr = [0.0; ECLIPSE_ATTR_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `geopos` (when present) owns three readable doubles,
        // `attr` owns ECLIPSE_ATTR_LEN writable slots and `serr` owns
        // SERR_LEN writable bytes; all outlive this call under the native
        // lock. Results are only exposed after the native status is
        // classified.
        let ret = unsafe {
            ffi::swe_lun_eclipse_how(
                jd_ut,
                flags,
                geopos.as_ref().map_or(std::ptr::null(), |g| g.as_ptr()),
                attr.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_eclipse_moment_failure(
                "lun_eclipse_how",
                jd_ut,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(LunarEclipseCircumstances {
                eclipse_type: ret,
                attributes: attr,
                diagnostic,
            })
        }
    })
}

/// Build the error message for a failed eclipse-moment call, keeping the
/// native diagnostic when the engine provided one.
fn describe_eclipse_moment_failure(
    function: &str,
    jd_ut: f64,
    flags: i32,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!("{function}: native computation failed (jd_ut={jd_ut}, flags={flags})")
    } else {
        format!("{function}: {diagnostic} (jd_ut={jd_ut}, flags={flags})")
    }
}

/// Next solar eclipse visible from one observer: type, phase times and
/// circumstances.
///
/// The `times` layout is the native `tret` order for the local search:
///
/// 0. time of maximum eclipse (local),
/// 1. time of first contact (partial begins),
/// 2. time of second contact (totality/annularity begins, 0.0 when the
///    eclipse stays partial),
/// 3. time of third contact (totality/annularity ends, 0.0 when partial),
/// 4. time of fourth contact (partial ends),
/// 5. time of sunrise between first and fourth contact (0.0 when the
///    whole eclipse happens while the Sun is up),
/// 6. time of sunset between first and fourth contact (0.0 likewise).
///
/// Entries that do not apply read 0.0; they are absent data, never
/// errors.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalSolarEclipse {
    /// Local eclipse-type bitmask (`ECL_TOTAL`, `ECL_ANNULAR`,
    /// `ECL_PARTIAL`, combined with `ECL_VISIBLE` and the
    /// `ECL_1ST_VISIBLE`/`ECL_2ND_VISIBLE`/`ECL_3RD_VISIBLE`/
    /// `ECL_4TH_VISIBLE`/`ECL_MAX_VISIBLE` contact bits).
    pub eclipse_type: i32,
    /// The seven native phase times; see the type documentation.
    pub times: [f64; LOCAL_SOLAR_TRET_LEN],
    /// The twenty native circumstance components at maximum, with the
    /// [`SolarEclipseCircumstances`] slot order (slots 0-10 meaningful,
    /// 11-19 reserved 0.0).
    pub attributes: [f64; ECLIPSE_ATTR_LEN],
    /// Native diagnostic text (`serr`); usually empty.
    pub diagnostic: String,
}

impl LocalSolarEclipse {
    /// Time of maximum eclipse in Universal Time (component 0).
    pub fn maximum(&self) -> f64 {
        self.times[0]
    }

    /// First contact: partial eclipse begins (component 1).
    pub fn first_contact(&self) -> f64 {
        self.times[1]
    }

    /// Second contact: totality/annularity begins (component 2, 0.0 when
    /// the eclipse stays partial).
    pub fn second_contact(&self) -> f64 {
        self.times[2]
    }

    /// Third contact: totality/annularity ends (component 3, 0.0 when
    /// partial).
    pub fn third_contact(&self) -> f64 {
        self.times[3]
    }

    /// Fourth contact: partial eclipse ends (component 4).
    pub fn fourth_contact(&self) -> f64 {
        self.times[4]
    }

    /// Sunrise between first and fourth contact (component 5, 0.0 when
    /// the eclipse does not span sunrise).
    pub fn sunrise(&self) -> f64 {
        self.times[5]
    }

    /// Sunset between first and fourth contact (component 6, 0.0 when
    /// the eclipse does not span sunset).
    pub fn sunset(&self) -> f64 {
        self.times[6]
    }

    /// Fraction of the solar diameter covered (magnitude, attribute 0).
    pub fn magnitude(&self) -> f64 {
        self.attributes[0]
    }

    /// Ratio of the lunar diameter to the solar one (attribute 1).
    pub fn diameter_ratio(&self) -> f64 {
        self.attributes[1]
    }

    /// Fraction of the solar disc covered (obscuration, attribute 2).
    pub fn obscuration(&self) -> f64 {
        self.attributes[2]
    }

    /// Diameter of the core shadow in kilometers (attribute 3).
    pub fn core_shadow_km(&self) -> f64 {
        self.attributes[3]
    }

    /// Azimuth of the Sun in degrees (attribute 4).
    pub fn azimuth(&self) -> f64 {
        self.attributes[4]
    }

    /// True altitude of the Sun in degrees (attribute 5).
    pub fn true_altitude(&self) -> f64 {
        self.attributes[5]
    }

    /// Apparent altitude of the Sun in degrees (attribute 6).
    pub fn apparent_altitude(&self) -> f64 {
        self.attributes[6]
    }

    /// Angular distance of the Moon from the Sun in degrees
    /// (attribute 7).
    pub fn separation(&self) -> f64 {
        self.attributes[7]
    }

    /// Saros series number (attribute 9).
    pub fn saros_series(&self) -> f64 {
        self.attributes[9]
    }

    /// Saros series member number (attribute 10).
    pub fn saros_member(&self) -> f64 {
        self.attributes[10]
    }
}

/// Next lunar eclipse visible from one observer: type, phase times and
/// circumstances.
///
/// The `times` layout is the native `tret` order for the local search:
///
/// 0. time of maximum eclipse (possibly shifted to moonrise/moonset when
///    the maximum itself is below the horizon),
/// 1. reserved (0.0),
/// 2. time of partial phase begin,
/// 3. time of partial phase end,
/// 4. time of totality begin (0.0 when not total),
/// 5. time of totality end (0.0 when not total),
/// 6. time of penumbral phase begin,
/// 7. time of penumbral phase end,
/// 8. time of moonrise, when it occurs during the eclipse (0.0
///    otherwise),
/// 9. time of moonset, when it occurs during the eclipse (0.0
///    otherwise).
///
/// Entries that do not apply read 0.0; they are absent data, never
/// errors.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalLunarEclipse {
    /// Local eclipse-type bitmask (`ECL_TOTAL`, `ECL_PARTIAL`,
    /// `ECL_PENUMBRAL`, combined with `ECL_VISIBLE` and the
    /// `ECL_MAX_VISIBLE`/`ECL_PARTBEG_VISIBLE`/`ECL_PARTEND_VISIBLE`/
    /// `ECL_TOTBEG_VISIBLE`/`ECL_TOTEND_VISIBLE`/`ECL_PENUMBBEG_VISIBLE`/
    /// `ECL_PENUMBEND_VISIBLE` phase-visibility bits).
    pub eclipse_type: i32,
    /// The ten native phase times; see the type documentation.
    pub times: [f64; ECLIPSE_TRET_LEN],
    /// The twenty native circumstance components at maximum, with the
    /// [`LunarEclipseCircumstances`] slot order (slots 0-10 meaningful,
    /// 11-19 reserved 0.0).
    pub attributes: [f64; ECLIPSE_ATTR_LEN],
    /// Native diagnostic text (`serr`); usually empty.
    pub diagnostic: String,
}

impl LocalLunarEclipse {
    /// Time of maximum eclipse in Universal Time (component 0).
    pub fn maximum(&self) -> f64 {
        self.times[0]
    }

    /// Partial phase begins: umbra entry (component 2).
    pub fn partial_begin(&self) -> f64 {
        self.times[2]
    }

    /// Partial phase ends: umbra exit (component 3).
    pub fn partial_end(&self) -> f64 {
        self.times[3]
    }

    /// Totality begins (component 4, 0.0 when not total).
    pub fn total_begin(&self) -> f64 {
        self.times[4]
    }

    /// Totality ends (component 5, 0.0 when not total).
    pub fn total_end(&self) -> f64 {
        self.times[5]
    }

    /// Penumbral phase begins (component 6).
    pub fn penumbral_begin(&self) -> f64 {
        self.times[6]
    }

    /// Penumbral phase ends (component 7).
    pub fn penumbral_end(&self) -> f64 {
        self.times[7]
    }

    /// Moonrise during the eclipse (component 8, 0.0 otherwise).
    pub fn moonrise(&self) -> f64 {
        self.times[8]
    }

    /// Moonset during the eclipse (component 9, 0.0 otherwise).
    pub fn moonset(&self) -> f64 {
        self.times[9]
    }

    /// Umbral magnitude at maximum (attribute 0).
    pub fn umbral_magnitude(&self) -> f64 {
        self.attributes[0]
    }

    /// Penumbral magnitude at maximum (attribute 1).
    pub fn penumbral_magnitude(&self) -> f64 {
        self.attributes[1]
    }

    /// Azimuth of the Moon in degrees (attribute 4).
    pub fn azimuth(&self) -> f64 {
        self.attributes[4]
    }

    /// True altitude of the Moon in degrees (attribute 5).
    pub fn true_altitude(&self) -> f64 {
        self.attributes[5]
    }

    /// Apparent altitude of the Moon in degrees (attribute 6).
    pub fn apparent_altitude(&self) -> f64 {
        self.attributes[6]
    }

    /// Distance of the Moon from opposition in degrees (attribute 7).
    pub fn opposition_distance(&self) -> f64 {
        self.attributes[7]
    }

    /// Saros series number (attribute 9).
    pub fn saros_series(&self) -> f64 {
        self.attributes[9]
    }

    /// Saros series member number (attribute 10).
    pub fn saros_member(&self) -> f64 {
        self.attributes[10]
    }
}

/// Check local-search inputs before any native call.
fn check_local_search_inputs(
    function: &str,
    jd_start: f64,
    longitude: f64,
    latitude: f64,
    altitude: f64,
) -> Result<(), Error> {
    crate::domain::check_observer_height(function, altitude)?;
    check_eclipse_search_inputs(function, jd_start)?;
    if !jd_start.is_finite()
        || !longitude.is_finite()
        || !latitude.is_finite()
        || !altitude.is_finite()
    {
        return Err(Error::invalid_input(format!(
            "{function}: search epoch and observer inputs must be finite"
        )));
    }
    Ok(())
}

/// Check a body-or-star selection before any native call: `None` selects
/// the body path, `Some` the fixed-star path (an empty name behaves as
/// `None` natively).
fn check_occult_star_input(function: &str, star_name: Option<&str>) -> Result<(), Error> {
    if let Some(name) = star_name
        && (name.as_bytes().contains(&0) || name.len() >= TEXT_BUF_LEN)
    {
        return Err(Error::invalid_input(format!(
            "{function}: star name must not contain NUL and must fit the 256-byte native buffer"
        )));
    }
    Ok(())
}

/// Reject the Moon on the occultation body path before any native call.
///
/// A lunar occultation of the Moon by itself has no defined native
/// verdict: the search compares the Moon against itself and never
/// terminates (established by probing the pinned build with a watchdog:
/// no return within 25 s, while every other body answers in
/// milliseconds). The rejection applies only
/// when the body path is effective (`None` or an empty star name); with
/// a real star name the engine searches the star and the body number
/// only feeds the type filter.
fn check_occult_body_input(
    function: &str,
    body: i32,
    star_name: Option<&str>,
) -> Result<(), Error> {
    if body == crate::MOON && star_name.is_none_or(|name| name.is_empty()) {
        return Err(Error::invalid_input(format!(
            "{function}: the Moon has no lunar occultation of its own (rejected before the native search, which does not terminate)"
        )));
    }
    Ok(())
}

/// Build the error message for a failed local search, keeping the native
/// diagnostic when the engine provided one.
#[allow(clippy::too_many_arguments)]
fn describe_local_search_failure(
    function: &str,
    jd_start: f64,
    flags: i32,
    backward: bool,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!(
            "{function}: native search failed (jd_start={jd_start}, flags={flags}, backward={backward})"
        )
    } else {
        format!(
            "{function}: {diagnostic} (jd_start={jd_start}, flags={flags}, backward={backward})"
        )
    }
}

/// Find the next (or previous) solar eclipse visible from a geographic
/// location.
///
/// `jd_start` is the Universal Time search epoch. `flags` carries the
/// ephemeris source bits. `longitude`/`latitude`/`altitude` locate the
/// observer (east-positive degrees, degrees north, meters; altitudes
/// outside −500…25000 m fail natively with a diagnostic).
/// `backward` selects the search direction (`false` = forward).
///
/// Observer side effect: the native search installs this observer as the
/// process-global observer (as if by [`crate::set_topo`]), so later
/// topocentric [`crate::calc_ut`]/[`crate::calc`] calls observe it until
/// [`crate::set_topo`] runs again. Established by probing the pinned
/// build.
///
/// The search advances lunation by lunation until a visible eclipse is
/// found, so the call can take a while far from eclipse seasons; there
/// is no "absent event" return, only success or a native failure.
///
/// The native `SE_ECL_ONE_TRY` single-conjunction mode is decoded only
/// by the lunar occultation searches (see [`OccultSearchOptions`]); this
/// eclipse search always runs until an event is found.
///
/// # Configuration dependence
///
/// Same contract as [`solcross`].
///
/// # Errors
///
/// Non-finite search/observer inputs are rejected before any native
/// call. A negative native status becomes [`Error`] with the diagnostic
/// kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_SWIEPH, set_ephe_path, sol_eclipse_when_loc};
///
/// // Next solar eclipse visible from Dallas after 2024-01-01 (explicit
/// // external data, else the checkout data, keeps this runnable).
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
/// let ecl = sol_eclipse_when_loc(2460310.5, FLG_SWIEPH, -96.8, 32.8, 0.0, false)?;
/// assert!(ecl.maximum() > 2460310.5);
/// assert!(ecl.first_contact() <= ecl.maximum() && ecl.maximum() <= ecl.fourth_contact());
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
/// Eclipse search epochs additionally lie within JD
/// `-63412861279.0..=63417764339.0`, the inward-rounded bounds of the
/// pinned native integer lunation counter. These are arithmetic limits,
/// independent of actual ephemeris file coverage.
///
/// Observer height must satisfy the arithmetic ceiling documented by
/// [`crate::set_topo`]; excessive height returns `InvalidInput` before
/// observer configuration or computation. Native geographic limits and
/// data errors are still reported by the engine.
pub fn sol_eclipse_when_loc(
    jd_start: f64,
    flags: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    backward: bool,
) -> Result<LocalSolarEclipse, Error> {
    check_local_search_inputs(
        "sol_eclipse_when_loc",
        jd_start,
        longitude,
        latitude,
        altitude,
    )?;
    with_native_access(|| {
        crate::domain::check_ut_locked("sol_eclipse_when_loc", jd_start, flags)?;
        let geopos = [longitude, latitude, altitude];
        // The engine writes exactly seven slots but leaves inapplicable
        // contacts (and sunrise/sunset) untouched, so the buffer starts
        // zeroed: 0.0 means "absent", never uninitialized memory.
        let mut tret = [0.0; LOCAL_SOLAR_TRET_LEN];
        let mut attr = [0.0; ECLIPSE_ATTR_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `geopos` owns three readable doubles, `tret` owns
        // LOCAL_SOLAR_TRET_LEN writable slots, `attr` owns
        // ECLIPSE_ATTR_LEN writable slots and `serr` owns SERR_LEN
        // writable bytes; all outlive this call under the native lock.
        // Results are only exposed after the native status is checked.
        let ret = unsafe {
            ffi::swe_sol_eclipse_when_loc(
                jd_start,
                flags,
                geopos.as_ptr(),
                tret.as_mut_ptr(),
                attr.as_mut_ptr(),
                i32::from(backward),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_local_search_failure(
                "sol_eclipse_when_loc",
                jd_start,
                flags,
                backward,
                &diagnostic,
            )))
        } else {
            Ok(LocalSolarEclipse {
                eclipse_type: ret,
                times: tret,
                attributes: attr,
                diagnostic,
            })
        }
    })
}

/// Find the next (or previous) lunar eclipse visible from a geographic
/// location.
///
/// `jd_start` is the Universal Time search epoch. `flags` carries the
/// ephemeris source bits. `longitude`/`latitude`/`altitude` locate the
/// observer (same convention and altitude domain as
/// [`sol_eclipse_when_loc`]; unlike [`lun_eclipse_how`] the observer is
/// required because the search also runs moonrise/moonset transits on
/// it). `backward` selects the search direction.
///
/// Observer side effect: the native search installs this observer as the
/// process-global observer (as if by [`crate::set_topo`]), so later
/// topocentric [`crate::calc_ut`]/[`crate::calc`] calls observe it until
/// [`crate::set_topo`] runs again. Established by probing the pinned
/// build.
///
/// The search advances eclipse by eclipse until a visible one is found;
/// there is no "absent event" return, only success or a native failure.
/// The native `SE_ECL_ONE_TRY` single-conjunction mode is decoded only
/// by the lunar occultation searches (see [`OccultSearchOptions`]); this
/// wrapper passes only the direction bit.
///
/// # Configuration dependence
///
/// Same contract as [`solcross`].
///
/// # Errors
///
/// Same contract as [`sol_eclipse_when_loc`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_SWIEPH, lun_eclipse_when_loc, set_ephe_path};
///
/// // Next lunar eclipse visible from London after 2024-04-15 (the
/// // 2024-09-18 partial eclipse).
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
/// let ecl = lun_eclipse_when_loc(2460415.5, FLG_SWIEPH, 0.0, 51.5, 0.0, false)?;
/// assert!(ecl.maximum() > 2460415.5);
/// assert!(ecl.partial_begin() <= ecl.maximum() && ecl.maximum() <= ecl.partial_end());
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
/// Eclipse search epochs additionally lie within JD
/// `-63412861279.0..=63417764339.0`, the inward-rounded bounds of the
/// pinned native integer lunation counter. These are arithmetic limits,
/// independent of actual ephemeris file coverage.
///
/// Observer height must satisfy the arithmetic ceiling documented by
/// [`crate::set_topo`]; excessive height returns `InvalidInput` before
/// observer configuration or computation. Native geographic limits and
/// data errors are still reported by the engine.
pub fn lun_eclipse_when_loc(
    jd_start: f64,
    flags: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    backward: bool,
) -> Result<LocalLunarEclipse, Error> {
    check_local_search_inputs(
        "lun_eclipse_when_loc",
        jd_start,
        longitude,
        latitude,
        altitude,
    )?;
    with_native_access(|| {
        crate::domain::check_ut_locked("lun_eclipse_when_loc", jd_start, flags)?;
        let geopos = [longitude, latitude, altitude];
        let mut tret = [0.0; ECLIPSE_TRET_LEN];
        let mut attr = [0.0; ECLIPSE_ATTR_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`sol_eclipse_when_loc`].
        let ret = unsafe {
            ffi::swe_lun_eclipse_when_loc(
                jd_start,
                flags,
                geopos.as_ptr(),
                tret.as_mut_ptr(),
                attr.as_mut_ptr(),
                i32::from(backward),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: same contract as in [`sol_eclipse_when_loc`].
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_local_search_failure(
                "lun_eclipse_when_loc",
                jd_start,
                flags,
                backward,
                &diagnostic,
            )))
        } else {
            Ok(LocalLunarEclipse {
                eclipse_type: ret,
                times: tret,
                attributes: attr,
                diagnostic,
            })
        }
    })
}

/// Next lunar occultation visible somewhere on Earth: type and phase
/// times in Universal Time.
///
/// The `times` layout is the native `tret` order shared with
/// [`GlobalSolarEclipse`]: maximum, local-apparent-noon peak, begin,
/// end, totality begin/end, center-line begin/end, two reserved hybrid
/// slots (0.0). Entries that do not apply to the found type read 0.0;
/// they are absent data, never errors.
///
/// This search also finds solar eclipses (the Moon occulting the Sun),
/// but less efficiently than [`sol_eclipse_when_glob`]; prefer the
/// dedicated search for the Sun.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalOccultation {
    /// Found occultation-type bitmask (`ECL_TOTAL`, `ECL_PARTIAL`,
    /// combined with `ECL_CENTRAL`/`ECL_NONCENTRAL`; the annular bits
    /// apply to solar targets only).
    pub occult_type: i32,
    /// The ten native phase times; see the type documentation.
    pub times: [f64; ECLIPSE_TRET_LEN],
    /// Native diagnostic text (`serr`); usually empty.
    pub diagnostic: String,
}

impl GlobalOccultation {
    /// Time of maximum occultation in Universal Time (component 0).
    pub fn maximum(&self) -> f64 {
        self.times[0]
    }

    /// Occultation begins anywhere on Earth (component 2, 0.0 when
    /// unresolved).
    pub fn begin(&self) -> f64 {
        self.times[2]
    }

    /// Occultation ends anywhere on Earth (component 3, 0.0 when
    /// unresolved).
    pub fn end(&self) -> f64 {
        self.times[3]
    }

    /// Totality begins anywhere on Earth (component 4, 0.0 when there
    /// is none).
    pub fn totality_begin(&self) -> f64 {
        self.times[4]
    }

    /// Totality ends anywhere on Earth (component 5, 0.0 when there is
    /// none).
    pub fn totality_end(&self) -> f64 {
        self.times[5]
    }

    /// Center line begins (component 6, 0.0 when there is none).
    pub fn centerline_begin(&self) -> f64 {
        self.times[6]
    }

    /// Center line ends (component 7, 0.0 when there is none).
    pub fn centerline_end(&self) -> f64 {
        self.times[7]
    }
}

/// Next lunar occultation visible from one observer: type, phase times
/// and circumstances.
///
/// The `times` layout is the native `tret` order for the local search:
///
/// 0. time of maximum occultation (local),
/// 1. time of first contact (partial begins),
/// 2. time of second contact (totality begins, 0.0 when the event
///    stays partial; for point-source stars contacts 1 and 4 coincide
///    with contacts 2 and 3),
/// 3. time of third contact (totality ends, 0.0 when partial),
/// 4. time of fourth contact (partial ends),
/// 5. time when the occulted body rises at the observer location
///    (0.0 when it does not rise during the event),
/// 6. time when the occulted body sets at the observer location
///    (0.0 likewise),
/// 7. reserved (0.0),
/// 8. reserved (0.0),
/// 9. reserved (0.0 — the engine zeroes ten slots on entry but uses
///    only the first seven).
///
/// Entries that do not apply read 0.0; they are absent data, never
/// errors.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalOccultation {
    /// Local occultation-type bitmask (`ECL_TOTAL`, `ECL_PARTIAL`,
    /// combined with `ECL_VISIBLE`, the `ECL_1ST_VISIBLE`/
    /// `ECL_2ND_VISIBLE`/`ECL_3RD_VISIBLE`/`ECL_4TH_VISIBLE`/
    /// `ECL_MAX_VISIBLE` contact bits and the
    /// `ECL_OCC_BEG_DAYLIGHT`/`ECL_OCC_END_DAYLIGHT` daylight bits,
    /// which report that the begin/end happens while the Sun is up).
    pub occult_type: i32,
    /// The ten native phase times; see the type documentation.
    pub times: [f64; ECLIPSE_TRET_LEN],
    /// The twenty native circumstance components at maximum:
    ///
    /// 0. fraction of the occulted diameter covered by the Moon
    ///    (magnitude),
    /// 1. ratio of the lunar diameter to the occulted diameter,
    /// 2. fraction of the occulted disc covered by the Moon
    ///    (obscuration),
    /// 3. diameter of the core shadow in kilometers,
    /// 4. azimuth of the occulted body in degrees,
    /// 5. true altitude of the occulted body above the horizon in
    ///    degrees,
    /// 6. apparent altitude of the occulted body above the horizon in
    ///    degrees,
    /// 7. angular distance of the Moon from the occulted body in
    ///    degrees,
    ///
    /// with slots 8-10 (the solar NASA-magnitude/saros fields) reading
    /// 0.0 for non-solar targets and slots 11-19 reserved as 0.0.
    pub attributes: [f64; ECLIPSE_ATTR_LEN],
    /// Native diagnostic text (`serr`); usually empty.
    pub diagnostic: String,
}

impl LocalOccultation {
    /// Time of maximum occultation in Universal Time (component 0).
    pub fn maximum(&self) -> f64 {
        self.times[0]
    }

    /// First contact: partial phase begins (component 1).
    pub fn first_contact(&self) -> f64 {
        self.times[1]
    }

    /// Second contact: totality begins (component 2, 0.0 when partial;
    /// equal to the first contact for point-source stars).
    pub fn second_contact(&self) -> f64 {
        self.times[2]
    }

    /// Third contact: totality ends (component 3, 0.0 when partial;
    /// equal to the fourth contact for point-source stars).
    pub fn third_contact(&self) -> f64 {
        self.times[3]
    }

    /// Fourth contact: partial phase ends (component 4).
    pub fn fourth_contact(&self) -> f64 {
        self.times[4]
    }

    /// Rise of the occulted body at the observer location during the
    /// event (component 5, 0.0 otherwise).
    pub fn body_rise(&self) -> f64 {
        self.times[5]
    }

    /// Set of the occulted body at the observer location during the
    /// event (component 6, 0.0 otherwise).
    pub fn body_set(&self) -> f64 {
        self.times[6]
    }

    /// Fraction of the occulted diameter covered (magnitude, attribute
    /// 0).
    pub fn magnitude(&self) -> f64 {
        self.attributes[0]
    }

    /// Ratio of the lunar diameter to the occulted diameter (attribute
    /// 1).
    pub fn diameter_ratio(&self) -> f64 {
        self.attributes[1]
    }

    /// Fraction of the occulted disc covered (obscuration, attribute 2).
    pub fn obscuration(&self) -> f64 {
        self.attributes[2]
    }

    /// Diameter of the core shadow in kilometers (attribute 3).
    pub fn core_shadow_km(&self) -> f64 {
        self.attributes[3]
    }

    /// Azimuth of the occulted body in degrees (attribute 4).
    pub fn azimuth(&self) -> f64 {
        self.attributes[4]
    }

    /// True altitude of the occulted body in degrees (attribute 5).
    pub fn true_altitude(&self) -> f64 {
        self.attributes[5]
    }

    /// Apparent altitude of the occulted body in degrees (attribute 6).
    pub fn apparent_altitude(&self) -> f64 {
        self.attributes[6]
    }

    /// Angular distance of the Moon from the occulted body in degrees
    /// (attribute 7).
    pub fn separation(&self) -> f64 {
        self.attributes[7]
    }
}

/// Geographic circumstances of the lunar occultation in progress at an
/// instant, as computed by [`lun_occult_where`].
#[derive(Debug, Clone, PartialEq)]
pub struct OccultationGeometry {
    /// Eastern longitude in degrees of the greatest occultation.
    pub longitude: f64,
    /// Northern latitude in degrees of the greatest occultation.
    pub latitude: f64,
    /// Occultation-type bitmask (`ECL_TOTAL`, `ECL_PARTIAL`, ...,
    /// combined with `ECL_CENTRAL`/`ECL_NONCENTRAL`; 0 when no
    /// occultation is in progress at this instant — kept as data, not
    /// an error).
    pub occult_type: i32,
    /// The twenty native circumstance components, with the
    /// [`LocalOccultation`] slot order (slots 0-7 meaningful, 8-10 read
    /// 0.0 for non-solar targets, 11-19 reserved 0.0).
    pub attributes: [f64; ECLIPSE_ATTR_LEN],
    /// Native diagnostic text (`serr`); carries the quiet-instant note
    /// when `occult_type` is 0.
    pub diagnostic: String,
}

/// Search controls for the lunar occultation searches
/// ([`lun_occult_when_glob_with_options`] and
/// [`lun_occult_when_loc_with_options`]).
///
/// The native `backward` integer carries two independent controls, which
/// this struct exposes as separate booleans instead of one overloaded
/// integer:
///
/// - `backward`: search direction (`false` = forward from the epoch,
///   `true` = backward). This is native bit 0.
/// - `one_try`: examine only the conjunction of the Moon with the body
///   nearest the epoch (native `SE_ECL_ONE_TRY`, value
///   [`ECL_ONE_TRY`]). When set and that single conjunction has no
///   (visible) occultation, the search reports a miss instead of
///   advancing lunation by lunation.
///
/// Only the two lunar occultation searches decode the one-try bit:
/// no other `backward`-taking native function in the pinned build reads
/// it (established by inventorying every `SE_ECL_ONE_TRY` use in the
/// pinned `swecl.c`), so the mode is occultation-only natively rather
/// than a gap in the remaining searches.
///
/// A miss is an [`Ok`] result with `occult_type == 0` (the same no-event
/// shape as [`lun_occult_where`] at a quiet instant), with `times[0]`
/// holding the examined conjunction epoch — a date suitable for the next
/// try — and the native diagnostic kept. It is distinct from a native
/// failure ([`Error`]) and from a valid event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OccultSearchOptions {
    /// Search direction: `false` = forward, `true` = backward.
    pub backward: bool,
    /// Examine only the conjunction nearest the epoch.
    pub one_try: bool,
}

impl OccultSearchOptions {
    /// Forward search until an event is found (the direction-only
    /// convenience behavior).
    pub fn forward() -> Self {
        Self {
            backward: false,
            one_try: false,
        }
    }

    /// Backward search until an event is found.
    pub fn backward() -> Self {
        Self {
            backward: true,
            one_try: false,
        }
    }
}

/// Encode search options as the native `backward` integer: bit 0 carries
/// the direction, the `SE_ECL_ONE_TRY` bit the single-conjunction mode.
///
/// SAFETY-adjacent contract note (not `unsafe` code): both pinned
/// occultation functions mask the direction with `backward &= 1` after
/// extracting the one-try bit, so exactly these two bits are meaningful
/// and no other bit is ever set here.
fn encode_occult_backward(search: OccultSearchOptions) -> i32 {
    i32::from(search.backward) | if search.one_try { ECL_ONE_TRY } else { 0 }
}

/// Find the next (or previous) lunar occultation visible somewhere on
/// Earth.
///
/// `jd_start` is the Universal Time search epoch. `body` selects the
/// occulted planet (for example [`crate::VENUS`]); pass a star via
/// `star_name` (`Some` selects the fixed-star path, `None` the body
/// path — an empty name behaves as `None` natively). Negative body
/// numbers fold to the Sun natively; `SE_AST_OFFSET + 134340` folds to
/// Pluto. `flags` carries the ephemeris source bits. `eclipse_type`
/// filters the wanted types (`ECL_TOTAL`, `ECL_PARTIAL`,
/// `ECL_CENTRAL`/`ECL_NONCENTRAL`; 0 accepts any type — central +
/// partial fails natively, and annular-only requests fail natively for
/// non-solar targets). `backward` selects the search direction.
///
/// Fixed stars with |ecliptic latitude| > 7° can never be occulted and
/// fail natively with a diagnostic. For bodies with ecliptic latitudes
/// above ~5° the search may run long before it reaches an event. To
/// examine only the conjunction nearest the epoch — with a miss reported
/// as `Ok` with `occult_type == 0` instead of advancing further — use
/// [`lun_occult_when_glob_with_options`] with
/// [`OccultSearchOptions`]`.one_try`.
///
/// # Configuration dependence
///
/// File-based sources and the fixed-star path require data/catalog
/// files visible through [`crate::set_ephe_path`]; the active
/// configuration is captured under the process-wide native lock
/// together with the search.
///
/// # Errors
///
/// Non-finite `jd_start` and overlong or NUL-containing star names are
/// rejected before any native call, as is the Moon on the body path
/// (its self-occultation search does not terminate natively). A
/// negative native status becomes [`Error`] with the diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, VENUS, lun_occult_when_glob};
///
/// // Next lunar occultation of Venus after J2000 (Moshier needs no
/// // data).
/// let occ = lun_occult_when_glob(2451545.0, VENUS, None, FLG_MOSEPH, 0, false)?;
/// assert!(occ.maximum() > 2451545.0);
/// assert!(occ.begin() <= occ.maximum() && occ.maximum() <= occ.end());
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
#[allow(clippy::too_many_arguments)]
pub fn lun_occult_when_glob(
    jd_start: f64,
    body: i32,
    star_name: Option<&str>,
    flags: i32,
    eclipse_type: i32,
    backward: bool,
) -> Result<GlobalOccultation, Error> {
    lun_occult_when_glob_impl(
        jd_start,
        body,
        star_name,
        flags,
        eclipse_type,
        OccultSearchOptions {
            backward,
            one_try: false,
        },
    )
}

/// Find the next (or previous) lunar occultation visible somewhere on
/// Earth, with full native search controls.
///
/// This is the options entry point behind [`lun_occult_when_glob`]:
/// argument selection, units, result layout and errors match that
/// function, except `backward`/`one_try` arrive as one
/// [`OccultSearchOptions`] value encoded onto the native `backward`
/// integer (bit 0 = direction, [`ECL_ONE_TRY`] = single-conjunction
/// mode). With `one_try: false` the result is identical to the
/// direction-only call; with `one_try: true` only the conjunction
/// nearest `jd_start` is examined and a miss comes back as `Ok` with
/// `occult_type == 0` and `times[0]` holding the examined conjunction
/// epoch (a date suitable for the next try), with the native diagnostic
/// kept.
///
/// # Configuration dependence
///
/// Same contract as [`lun_occult_when_glob`].
///
/// # Errors
///
/// Same contract as [`lun_occult_when_glob`], including the pre-call
/// rejection of the Moon on the body path (its self-occultation search
/// does not terminate natively even for a single try).
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{
///     FLG_MOSEPH, OccultSearchOptions, VENUS, lun_occult_when_glob,
///     lun_occult_when_glob_with_options,
/// };
///
/// // Next lunar occultation of Venus after J2000 (Moshier needs no
/// // data): the extended call without `one_try` agrees with the
/// // direction-only convenience call.
/// let classic = lun_occult_when_glob(2451545.0, VENUS, None, FLG_MOSEPH, 0, false)?;
/// let extended = lun_occult_when_glob_with_options(
///     2451545.0,
///     VENUS,
///     None,
///     FLG_MOSEPH,
///     0,
///     OccultSearchOptions::forward(),
/// )?;
/// assert_eq!(extended.occult_type, classic.occult_type);
/// assert_eq!(extended.times, classic.times);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
#[allow(clippy::too_many_arguments)]
pub fn lun_occult_when_glob_with_options(
    jd_start: f64,
    body: i32,
    star_name: Option<&str>,
    flags: i32,
    eclipse_type: i32,
    search: OccultSearchOptions,
) -> Result<GlobalOccultation, Error> {
    lun_occult_when_glob_impl(jd_start, body, star_name, flags, eclipse_type, search)
}

/// Shared implementation behind [`lun_occult_when_glob`] and
/// [`lun_occult_when_glob_with_options`]; `search` is encoded once via
/// [`encode_occult_backward`] so both spellings reach the native call
/// with identical integers for identical options.
#[allow(clippy::too_many_arguments)]
fn lun_occult_when_glob_impl(
    jd_start: f64,
    body: i32,
    star_name: Option<&str>,
    flags: i32,
    eclipse_type: i32,
    search: OccultSearchOptions,
) -> Result<GlobalOccultation, Error> {
    if !jd_start.is_finite() {
        return Err(Error::invalid_input(
            "lun_occult_when_glob: search epoch must be finite",
        ));
    }
    check_occult_star_input("lun_occult_when_glob", star_name)?;
    check_occult_body_input("lun_occult_when_glob", body, star_name)?;
    // Caller-owned star buffer: the native call reads the name while
    // resolving a fixed star (and `swe_fixstar` writes the resolved name
    // back), so every access must stay inside this buffer. The body path
    // passes null, which the engine treats as "no star".
    let mut star_buffer = [0 as c_char; TEXT_BUF_LEN];
    if let Some(name) = star_name {
        for (slot, byte) in star_buffer.iter_mut().zip(name.bytes()) {
            *slot = byte as c_char;
        }
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("lun_occult_when_glob_impl", jd_start, flags)?;
        let mut tret = [0.0; ECLIPSE_TRET_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `star_buffer` owns TEXT_BUF_LEN bytes holding a
        // NUL-terminated name (or is unused when null is passed),
        // `tret` owns ECLIPSE_TRET_LEN writable slots and `serr` owns
        // SERR_LEN writable bytes; all outlive the call under the native
        // lock. Results are only exposed after the native status is
        // checked.
        let ret = unsafe {
            ffi::swe_lun_occult_when_glob(
                jd_start,
                body,
                if star_name.is_none() {
                    std::ptr::null_mut()
                } else {
                    star_buffer.as_mut_ptr()
                },
                flags,
                eclipse_type,
                tret.as_mut_ptr(),
                encode_occult_backward(search),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_occult_search_failure(
                "lun_occult_when_glob",
                jd_start,
                body,
                flags,
                eclipse_type,
                search,
                &diagnostic,
            )))
        } else {
            Ok(GlobalOccultation {
                occult_type: ret,
                times: tret,
                diagnostic,
            })
        }
    })
}

/// Find the next (or previous) lunar occultation visible from a
/// geographic location.
///
/// `jd_start` is the Universal Time search epoch; body/star selection
/// matches [`lun_occult_when_glob`]. `longitude`/`latitude`/`altitude`
/// locate the observer (same convention and altitude domain as
/// [`sol_eclipse_when_loc`]). `flags` carries the ephemeris source
/// bits. `backward` selects the search direction.
///
/// Observer side effect: the native search installs this observer as the
/// process-global observer (as if by [`crate::set_topo`]), so later
/// topocentric [`crate::calc_ut`]/[`crate::calc`] calls observe it until
/// [`crate::set_topo`] runs again. Established by probing the pinned
/// build.
///
/// For point-source fixed stars the first/fourth contacts coincide with
/// the second/third. To examine only the conjunction nearest the epoch —
/// with a miss reported as `Ok` with `occult_type == 0` instead of
/// searching on — use [`lun_occult_when_loc_with_options`] with
/// [`OccultSearchOptions`]`.one_try`.
///
/// # Configuration dependence
///
/// Same contract as [`lun_occult_when_glob`].
///
/// # Errors
///
/// Non-finite search/observer inputs and overlong or NUL-containing
/// star names are rejected before any native call, as is the Moon on
/// the body path (its self-occultation search does not terminate
/// natively). A negative native status becomes [`Error`] with the
/// diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, VENUS, lun_occult_when_loc};
///
/// // Next lunar occultation of Venus visible from Greenwich after
/// // J2000 (Moshier needs no data files, so no data path is set here).
/// let occ = lun_occult_when_loc(2451545.0, VENUS, None, FLG_MOSEPH, 0.0, 51.5, 0.0, false)?;
/// assert!(occ.maximum() > 2451545.0);
/// assert!(occ.first_contact() <= occ.maximum() && occ.maximum() <= occ.fourth_contact());
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
/// Observer height must satisfy the arithmetic ceiling documented by
/// [`crate::set_topo`]; excessive height returns `InvalidInput` before
/// observer configuration or computation. Native geographic limits and
/// data errors are still reported by the engine.
#[allow(clippy::too_many_arguments)]
pub fn lun_occult_when_loc(
    jd_start: f64,
    body: i32,
    star_name: Option<&str>,
    flags: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    backward: bool,
) -> Result<LocalOccultation, Error> {
    lun_occult_when_loc_impl(
        jd_start,
        body,
        star_name,
        flags,
        longitude,
        latitude,
        altitude,
        OccultSearchOptions {
            backward,
            one_try: false,
        },
    )
}

/// Find the next (or previous) lunar occultation visible from a
/// geographic location, with full native search controls.
///
/// This is the options entry point behind [`lun_occult_when_loc`]:
/// argument selection, units, result layout, observer side effect and
/// errors match that function, except `backward`/`one_try` arrive as one
/// [`OccultSearchOptions`] value encoded onto the native `backward`
/// integer (bit 0 = direction, [`ECL_ONE_TRY`] = single-conjunction
/// mode). With `one_try: false` the result is identical to the
/// direction-only call; with `one_try: true` only the conjunction
/// nearest `jd_start` is examined and a miss comes back as `Ok` with
/// `occult_type == 0` and `times[0]` holding a date suitable for the
/// next try, with the native diagnostic kept.
///
/// # Configuration dependence
///
/// Same contract as [`lun_occult_when_loc`].
///
/// # Errors
///
/// Same contract as [`lun_occult_when_loc`], including the pre-call
/// rejection of the Moon on the body path.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{
///     FLG_MOSEPH, OccultSearchOptions, VENUS, lun_occult_when_loc,
///     lun_occult_when_loc_with_options,
/// };
///
/// // Next lunar occultation of Venus visible from Greenwich after
/// // J2000 (Moshier needs no data files, so no data path is set here):
/// // the extended call without `one_try` agrees with the
/// // direction-only convenience call.
/// let classic = lun_occult_when_loc(2451545.0, VENUS, None, FLG_MOSEPH, 0.0, 51.5, 0.0, false)?;
/// let extended = lun_occult_when_loc_with_options(
///     2451545.0,
///     VENUS,
///     None,
///     FLG_MOSEPH,
///     0.0,
///     51.5,
///     0.0,
///     OccultSearchOptions::forward(),
/// )?;
/// assert_eq!(extended.occult_type, classic.occult_type);
/// assert_eq!(extended.times, classic.times);
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
/// Observer height must satisfy the arithmetic ceiling documented by
/// [`crate::set_topo`]; excessive height returns `InvalidInput` before
/// observer configuration or computation. Native geographic limits and
/// data errors are still reported by the engine.
#[allow(clippy::too_many_arguments)]
pub fn lun_occult_when_loc_with_options(
    jd_start: f64,
    body: i32,
    star_name: Option<&str>,
    flags: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    search: OccultSearchOptions,
) -> Result<LocalOccultation, Error> {
    lun_occult_when_loc_impl(
        jd_start, body, star_name, flags, longitude, latitude, altitude, search,
    )
}

/// Shared implementation behind [`lun_occult_when_loc`] and
/// [`lun_occult_when_loc_with_options`]; `search` is encoded once via
/// [`encode_occult_backward`] so both spellings reach the native call
/// with identical integers for identical options.
#[allow(clippy::too_many_arguments)]
fn lun_occult_when_loc_impl(
    jd_start: f64,
    body: i32,
    star_name: Option<&str>,
    flags: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    search: OccultSearchOptions,
) -> Result<LocalOccultation, Error> {
    check_local_search_inputs(
        "lun_occult_when_loc",
        jd_start,
        longitude,
        latitude,
        altitude,
    )?;
    check_occult_star_input("lun_occult_when_loc", star_name)?;
    check_occult_body_input("lun_occult_when_loc", body, star_name)?;
    // Same caller-owned star buffer contract as in
    // [`lun_occult_when_glob`].
    let mut star_buffer = [0 as c_char; TEXT_BUF_LEN];
    if let Some(name) = star_name {
        for (slot, byte) in star_buffer.iter_mut().zip(name.bytes()) {
            *slot = byte as c_char;
        }
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("lun_occult_when_loc_impl", jd_start, flags)?;
        let geopos = [longitude, latitude, altitude];
        // The engine zeroes ten slots on entry and fills the first
        // seven, so all ten are passed and exposed; slots 7-9 stay 0.0
        // by construction (reserved, never uninitialized memory).
        let mut tret = [0.0; ECLIPSE_TRET_LEN];
        let mut attr = [0.0; ECLIPSE_ATTR_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `star_buffer` owns TEXT_BUF_LEN bytes (or is unused
        // when null is passed), `geopos` owns three readable doubles,
        // `tret` owns ECLIPSE_TRET_LEN writable slots, `attr` owns
        // ECLIPSE_ATTR_LEN writable slots and `serr` owns SERR_LEN
        // writable bytes; all outlive the call under the native lock.
        // Results are only exposed after the native status is checked.
        let ret = unsafe {
            ffi::swe_lun_occult_when_loc(
                jd_start,
                body,
                if star_name.is_none() {
                    std::ptr::null_mut()
                } else {
                    star_buffer.as_mut_ptr()
                },
                flags,
                geopos.as_ptr(),
                tret.as_mut_ptr(),
                attr.as_mut_ptr(),
                encode_occult_backward(search),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_occult_search_failure(
                "lun_occult_when_loc",
                jd_start,
                body,
                flags,
                0,
                search,
                &diagnostic,
            )))
        } else {
            Ok(LocalOccultation {
                occult_type: ret,
                times: tret,
                attributes: attr,
                diagnostic,
            })
        }
    })
}

/// Report the geographic circumstances of the lunar occultation in
/// progress at an instant.
///
/// `jd_ut` is Universal Time; body/star selection matches
/// [`lun_occult_when_glob`] and `flags` carries the ephemeris source
/// bits. The result holds the longitude/latitude of the greatest
/// occultation plus the same twenty circumstance components as
/// [`lun_occult_when_loc`].
///
/// A return type of 0 means no occultation is in progress at `jd_ut`:
/// that is a dedicated `Ok` state with the native diagnostic kept,
/// never an error and never invented geometry.
///
/// # Configuration dependence
///
/// Same contract as [`lun_occult_when_glob`].
///
/// # Errors
///
/// Non-finite `jd_ut` and overlong or NUL-containing star names are
/// rejected before any native call, as is the Moon on the body path
/// (meaningless even as a single computation). A negative native status
/// becomes [`Error`] with the diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, VENUS, lun_occult_when_glob, lun_occult_where};
///
/// // Circumstances of the next Venus occultation after J2000 at its
/// // own maximum.
/// let occ = lun_occult_when_glob(2451545.0, VENUS, None, FLG_MOSEPH, 0, false)?;
/// let geo = lun_occult_where(occ.maximum(), VENUS, None, FLG_MOSEPH)?;
/// assert!(geo.occult_type != 0);
/// assert!(geo.magnitude() > 0.0);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn lun_occult_where(
    jd_ut: f64,
    body: i32,
    star_name: Option<&str>,
    flags: i32,
) -> Result<OccultationGeometry, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(
            "lun_occult_where: Julian Day must be finite",
        ));
    }
    check_occult_star_input("lun_occult_where", star_name)?;
    check_occult_body_input("lun_occult_where", body, star_name)?;
    // Same caller-owned star buffer contract as in
    // [`lun_occult_when_glob`].
    let mut star_buffer = [0 as c_char; TEXT_BUF_LEN];
    if let Some(name) = star_name {
        for (slot, byte) in star_buffer.iter_mut().zip(name.bytes()) {
            *slot = byte as c_char;
        }
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("lun_occult_where", jd_ut, flags)?;
        let mut geopos = [0.0; ECLIPSE_GEOPOS_LEN];
        let mut attr = [0.0; ECLIPSE_ATTR_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `star_buffer` owns TEXT_BUF_LEN bytes (or is unused
        // when null is passed), `geopos` owns ECLIPSE_GEOPOS_LEN writable
        // slots, `attr` owns ECLIPSE_ATTR_LEN writable slots and `serr`
        // owns SERR_LEN writable bytes; all outlive the call under the
        // native lock. Results are only exposed after the native status
        // is classified.
        let ret = unsafe {
            ffi::swe_lun_occult_where(
                jd_ut,
                body,
                if star_name.is_none() {
                    std::ptr::null_mut()
                } else {
                    star_buffer.as_mut_ptr()
                },
                flags,
                geopos.as_mut_ptr(),
                attr.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_occult_moment_failure(
                "lun_occult_where",
                jd_ut,
                body,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(OccultationGeometry {
                longitude: geopos[0],
                latitude: geopos[1],
                occult_type: ret,
                attributes: attr,
                diagnostic,
            })
        }
    })
}

impl OccultationGeometry {
    /// Fraction of the occulted diameter covered (magnitude, component
    /// 0).
    pub fn magnitude(&self) -> f64 {
        self.attributes[0]
    }

    /// Ratio of the lunar diameter to the occulted diameter (component
    /// 1).
    pub fn diameter_ratio(&self) -> f64 {
        self.attributes[1]
    }

    /// Fraction of the occulted disc covered (obscuration, component 2).
    pub fn obscuration(&self) -> f64 {
        self.attributes[2]
    }

    /// Diameter of the core shadow in kilometers (component 3).
    pub fn core_shadow_km(&self) -> f64 {
        self.attributes[3]
    }

    /// Azimuth of the occulted body in degrees (component 4).
    pub fn azimuth(&self) -> f64 {
        self.attributes[4]
    }

    /// True altitude of the occulted body in degrees (component 5).
    pub fn true_altitude(&self) -> f64 {
        self.attributes[5]
    }

    /// Apparent altitude of the occulted body in degrees (component 6).
    pub fn apparent_altitude(&self) -> f64 {
        self.attributes[6]
    }

    /// Angular distance of the Moon from the occulted body in degrees
    /// (component 7).
    pub fn separation(&self) -> f64 {
        self.attributes[7]
    }
}

/// Build the error message for a failed occultation search, keeping the
/// native diagnostic when the engine provided one.
#[allow(clippy::too_many_arguments)]
fn describe_occult_search_failure(
    function: &str,
    jd_start: f64,
    body: i32,
    flags: i32,
    eclipse_type: i32,
    search: OccultSearchOptions,
    diagnostic: &str,
) -> String {
    let backward = search.backward;
    let one_try = search.one_try;
    if diagnostic.is_empty() {
        format!(
            "{function}: native search failed (jd_start={jd_start}, body={body}, flags={flags}, eclipse_type={eclipse_type}, backward={backward}, one_try={one_try})"
        )
    } else {
        format!(
            "{function}: {diagnostic} (jd_start={jd_start}, body={body}, flags={flags}, eclipse_type={eclipse_type}, backward={backward}, one_try={one_try})"
        )
    }
}

/// Build the error message for a failed occultation-moment call, keeping
/// the native diagnostic when the engine provided one.
fn describe_occult_moment_failure(
    function: &str,
    jd_ut: f64,
    body: i32,
    flags: i32,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!("{function}: native computation failed (jd_ut={jd_ut}, body={body}, flags={flags})")
    } else {
        format!("{function}: {diagnostic} (jd_ut={jd_ut}, body={body}, flags={flags})")
    }
}

// ---------------------------------------------------------------------------
// Heliacal visibility events
// ---------------------------------------------------------------------------

/// Next heliacal visibility event of a planet or fixed star.
///
/// The ten native `dret` slots hold the beginning (0), optimum (1) and end
/// (2) of visibility as Julian Days in Universal Time; slots 3-9 read 0.0
/// (reserved). Arcus-visionis search kinds are rejected by [`heliacal_ut`]
/// because of an internal buffer defect in the pinned native build.
#[derive(Debug, Clone, PartialEq)]
pub struct HeliacalEvent {
    /// The ten native event-time slots; see the type documentation.
    pub times: [f64; HELIACAL_DRET_LEN],
    /// Native diagnostic text (`serr`); usually empty.
    pub diagnostic: String,
}

impl HeliacalEvent {
    /// Beginning of visibility as a Julian Day in Universal Time (slot 0).
    pub fn begin(&self) -> f64 {
        self.times[0]
    }

    /// Optimum visibility as a Julian Day in Universal Time (slot 1).
    pub fn optimum(&self) -> f64 {
        self.times[1]
    }

    /// End of visibility as a Julian Day in Universal Time (slot 2).
    pub fn end(&self) -> f64 {
        self.times[2]
    }
}

/// Heliacal visibility circumstances of a planet or fixed star at an
/// instant.
///
/// Component layout (degrees, Julian Days and plain numbers as noted;
/// slots 28-29 are engine-reserved and read 0.0):
///
/// - 0 `AltO`: topocentric altitude of the object, unrefracted (deg)
/// - 1 `AppAltO`: apparent (refracted) altitude of the object (deg)
/// - 2 `GeoAltO`: geocentric altitude of the object (deg)
/// - 3 `AziO`: azimuth of the object (deg)
/// - 4 `AltS`: topocentric altitude of the Sun (deg)
/// - 5 `AziS`: azimuth of the Sun (deg)
/// - 6 `TAVact`: actual topocentric arcus visionis (deg)
/// - 7 `ARCVact`: actual geocentric arcus visionis (deg)
/// - 8 `DAZact`: azimuth difference between object and Sun (deg)
/// - 9 `ARCLact`: longitude difference between object and Sun (deg)
/// - 10 `kact`: extinction coefficient (-)
/// - 11 `minTAV`: smallest topocentric arcus visionis (deg)
/// - 12 `TfistVR`: first time the object is visible, VR criterion (JD UT)
/// - 13 `TbVR`: optimum time the object is visible, VR criterion (JD UT)
/// - 14 `TlastVR`: last time the object is visible, VR criterion (JD UT)
/// - 15 `TbYallop`: best time the object is visible, Yallop criterion
///   (JD UT; [`crate::TJD_INVALID`] for non-lunar objects)
/// - 16 `WMoon`: crescent width of the Moon (deg)
/// - 17 `qYal`: Yallop q-test value (-)
/// - 18 `qCrit`: Yallop q-test criterion (-)
/// - 19 `ParO`: parallax of the object (deg)
/// - 20 `Magn`: magnitude of the object (-)
/// - 21 `RiseO`: rise/set time of the object (JD UT)
/// - 22 `RiseS`: rise/set time of the Sun (JD UT)
/// - 23 `Lag`: rise/set time of the object minus rise/set time of the
///   Sun (days)
/// - 24 `TvisVR`: visibility duration (days)
/// - 25 `LMoon`: crescent length of the Moon (deg)
/// - 26 `CVAact` (deg)
/// - 27 `Illum`: lunar illumination (%)
#[derive(Debug, Clone, PartialEq)]
pub struct HeliacalPhenomena {
    /// The thirty native circumstance slots; see the type documentation.
    pub values: [f64; HELIACAL_PHENO_LEN],
    /// Native diagnostic text (`serr`); usually empty.
    pub diagnostic: String,
}

impl HeliacalPhenomena {
    /// Topocentric altitude of the object, unrefracted, in degrees
    /// (component 0). Negative values (object below the horizon) are data,
    /// never an error.
    pub fn object_altitude_deg(&self) -> f64 {
        self.values[0]
    }

    /// Apparent (refracted) altitude of the object in degrees
    /// (component 1).
    pub fn apparent_altitude_deg(&self) -> f64 {
        self.values[1]
    }

    /// Geocentric altitude of the object in degrees (component 2).
    pub fn geocentric_altitude_deg(&self) -> f64 {
        self.values[2]
    }

    /// Azimuth of the object in degrees (component 3).
    pub fn object_azimuth_deg(&self) -> f64 {
        self.values[3]
    }

    /// Topocentric altitude of the Sun in degrees (component 4).
    pub fn sun_altitude_deg(&self) -> f64 {
        self.values[4]
    }

    /// Azimuth of the Sun in degrees (component 5).
    pub fn sun_azimuth_deg(&self) -> f64 {
        self.values[5]
    }

    /// Actual topocentric arcus visionis in degrees (component 6).
    pub fn topocentric_arcus_visionis_deg(&self) -> f64 {
        self.values[6]
    }

    /// Actual geocentric arcus visionis in degrees (component 7).
    pub fn arcus_visionis_deg(&self) -> f64 {
        self.values[7]
    }

    /// Azimuth difference between object and Sun in degrees
    /// (component 8).
    pub fn azimuth_difference_deg(&self) -> f64 {
        self.values[8]
    }

    /// Longitude difference between object and Sun in degrees
    /// (component 9).
    pub fn longitude_difference_deg(&self) -> f64 {
        self.values[9]
    }

    /// Extinction coefficient (component 10).
    pub fn extinction_coefficient(&self) -> f64 {
        self.values[10]
    }

    /// Smallest topocentric arcus visionis in degrees (component 11).
    pub fn min_topocentric_arcus_visionis_deg(&self) -> f64 {
        self.values[11]
    }

    /// First time the object is visible, VR criterion, as a Julian Day in
    /// Universal Time (component 12).
    pub fn first_visible_jd(&self) -> f64 {
        self.values[12]
    }

    /// Optimum time the object is visible, VR criterion, as a Julian Day
    /// in Universal Time (component 13).
    pub fn best_visible_jd(&self) -> f64 {
        self.values[13]
    }

    /// Last time the object is visible, VR criterion, as a Julian Day in
    /// Universal Time (component 14).
    pub fn last_visible_jd(&self) -> f64 {
        self.values[14]
    }

    /// Best time the object is visible, Yallop criterion, as a Julian Day
    /// in Universal Time (component 15; [`crate::TJD_INVALID`] for
    /// non-lunar objects, which have no Yallop timing).
    pub fn yallop_best_jd(&self) -> f64 {
        self.values[15]
    }

    /// Crescent width of the Moon in degrees (component 16).
    pub fn moon_crescent_width_deg(&self) -> f64 {
        self.values[16]
    }

    /// Yallop q-test value (component 17).
    pub fn yallop_q(&self) -> f64 {
        self.values[17]
    }

    /// Yallop q-test criterion (component 18).
    pub fn yallop_criterion(&self) -> f64 {
        self.values[18]
    }

    /// Parallax of the object in degrees (component 19).
    pub fn parallax_deg(&self) -> f64 {
        self.values[19]
    }

    /// Magnitude of the object (component 20).
    pub fn object_magnitude(&self) -> f64 {
        self.values[20]
    }

    /// Rise/set time of the object as a Julian Day in Universal Time
    /// (component 21).
    pub fn object_rise_set_jd(&self) -> f64 {
        self.values[21]
    }

    /// Rise/set time of the Sun as a Julian Day in Universal Time
    /// (component 22).
    pub fn sun_rise_set_jd(&self) -> f64 {
        self.values[22]
    }

    /// Rise/set time of the object minus rise/set time of the Sun, in
    /// days (component 23).
    pub fn lag_days(&self) -> f64 {
        self.values[23]
    }

    /// Visibility duration in days (component 24).
    pub fn visibility_duration_days(&self) -> f64 {
        self.values[24]
    }

    /// Crescent length of the Moon in degrees (component 25).
    pub fn moon_crescent_length_deg(&self) -> f64 {
        self.values[25]
    }

    /// Actual crescent-visibility arc in degrees (component 26).
    pub fn crescent_visibility_arc_deg(&self) -> f64 {
        self.values[26]
    }

    /// Lunar illumination in percent (component 27).
    pub fn illumination_percent(&self) -> f64 {
        self.values[27]
    }
}

/// Visual limiting magnitude and visibility geometry at an instant.
///
/// Component layout: 0 is the visual limiting magnitude (the object is
/// visible when it is brighter — numerically smaller — than this value);
/// 1-4 are the topocentric object altitude, object azimuth, Sun altitude
/// and Sun azimuth in degrees (each cross-verified against the matching
/// [`HeliacalPhenomena`] component for the same instant); 5-6 are further
/// engine details preserved verbatim; 7 is the magnitude of the object
/// (cross-verified against phenomenon component 20).
#[derive(Debug, Clone, PartialEq)]
pub struct VisibilityLimit {
    /// Native vision-mode status echoed as an integer: 0 photopic, 1
    /// scotopic, plus bit 1 (value 2) near the photopic/scotopic limit.
    /// It is preserved so callers can distinguish the native status from
    /// the input flags without re-issuing the call.
    pub status: i32,
    /// The eight native detail slots; see the type documentation.
    pub values: [f64; VISLIM_DRET_LEN],
    /// Native diagnostic text (`serr`); usually empty.
    pub diagnostic: String,
}

impl VisibilityLimit {
    /// Visual limiting magnitude at the instant (component 0).
    pub fn limiting_magnitude(&self) -> f64 {
        self.values[0]
    }

    /// Topocentric altitude of the object in degrees (component 1).
    pub fn object_altitude_deg(&self) -> f64 {
        self.values[1]
    }

    /// Azimuth of the object in degrees (component 2).
    pub fn object_azimuth_deg(&self) -> f64 {
        self.values[2]
    }

    /// Topocentric altitude of the Sun in degrees (component 3).
    pub fn sun_altitude_deg(&self) -> f64 {
        self.values[3]
    }

    /// Azimuth of the Sun in degrees (component 4).
    pub fn sun_azimuth_deg(&self) -> f64 {
        self.values[4]
    }

    /// Magnitude of the object (component 7).
    pub fn object_magnitude(&self) -> f64 {
        self.values[7]
    }
}

/// The object is below the local horizon: the below-horizon outcome of
/// [`vis_limit_mag`].
///
/// There is no limiting magnitude for an object that cannot be seen; the
/// engine reports this absence with a dedicated status that is preserved
/// here instead of being turned into an error or into invented magnitude
/// data.
#[derive(Debug, Clone, PartialEq)]
pub struct BelowHorizon {
    /// The eight native detail slots as left by the engine: component 0
    /// is the −100 marker, components 1-6 read 0.0, and component 7 is
    /// not written on this path (0.0 from the zero-initialized buffer).
    pub values: [f64; VISLIM_DRET_LEN],
    /// Native diagnostic text (`serr`); the engine reports
    /// `"object is below local horizon"`.
    pub diagnostic: String,
}

/// Outcome of a visual-limiting-magnitude computation.
///
/// An object below the local horizon has no limiting magnitude; the
/// engine reports that absence with a dedicated status that is preserved
/// here instead of being turned into an error or into invented data.
#[derive(Debug, Clone, PartialEq)]
pub enum VisibilityOutcome {
    /// The object is above the horizon: limiting magnitude and geometry.
    Visible(VisibilityLimit),
    /// The object is below the local horizon: marker values only.
    BelowHorizon(BelowHorizon),
}

/// Check heliacal epoch/observer/atmosphere/object inputs before any
/// native call.
#[allow(clippy::too_many_arguments)]
fn check_heliacal_inputs(
    function: &str,
    jd_ut: f64,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atmosphere: &[f64; HELIACAL_ATM_LEN],
    observer: &[f64; HELIACAL_OBS_LEN],
    object: &str,
) -> Result<(), Error> {
    crate::domain::check_observer_height(function, altitude)?;
    if !jd_ut.is_finite()
        || !longitude.is_finite()
        || !latitude.is_finite()
        || !altitude.is_finite()
        || !atmosphere.iter().all(|value| value.is_finite())
        || !observer.iter().all(|value| value.is_finite())
    {
        return Err(Error::invalid_input(format!(
            "{function}: epoch, observer and atmosphere inputs must be finite"
        )));
    }
    if object.as_bytes().contains(&0) || object.len() >= TEXT_BUF_LEN {
        return Err(Error::invalid_input(format!(
            "{function}: object name must not contain NUL and must fit the 256-byte native buffer"
        )));
    }
    Ok(())
}

/// Copy a heliacal object name into a caller-owned native buffer.
///
/// The native calls take a mutable `char *` name; the safe wrappers always
/// pass this owned [`TEXT_BUF_LEN`]-byte buffer, so any native read stays
/// in bounds. Callers must run [`check_heliacal_inputs`] first, which
/// guarantees the copy below fits and is NUL-terminated.
fn heliacal_name_buffer(object: &str) -> [c_char; TEXT_BUF_LEN] {
    let mut buffer = [0 as c_char; TEXT_BUF_LEN];
    for (slot, byte) in buffer.iter_mut().zip(object.bytes()) {
        *slot = byte as c_char;
    }
    buffer
}

/// Find the next heliacal visibility event of a planet or fixed star.
///
/// `jd_start_ut` is the Universal Time search epoch.
/// `longitude`/`latitude`/`altitude` locate the observer (east-positive
/// degrees, degrees north, eye height in meters; heights outside
/// −500…25000 m fail natively with a diagnostic, and latitudes beyond
/// ±60° leave the documented reliable range of the engine).
///
/// Observer side effect: the native search installs this observer as the
/// process-global observer (as if by [`crate::set_topo`]), so later
/// topocentric [`crate::calc_ut`]/[`crate::calc`] calls observe it until
/// [`crate::set_topo`] runs again. Established by probing the pinned
/// build.
/// `atmosphere` carries pressure (mbar), temperature (°C), relative
/// humidity (%) and the meteorological range or extinction coefficient;
/// `observer` carries age (years), Snellen acuity, the binocular flag and
/// — with [`crate::HELFLAG_OPTICAL_PARAMS`] — magnification, aperture
/// (mm) and transmission. Zero entries in either block select the native
/// defaults. `object` names the planet (`"Venus"`, `"Mars"`, `"Moon"`,
///...) or fixed star (`"Sirius"`, ...). `event_type` selects
/// [`crate::HELIACAL_RISING`] (morning first), [`crate::HELIACAL_SETTING`]
/// (evening last), [`crate::EVENING_FIRST`] or [`crate::MORNING_LAST`]
/// (inner planets and the Moon only); any other integer passes through
/// and fails natively (acronychal types 5-6 are declared but "still not
/// implemented" in the pinned header). `helflag` combines the ephemeris
/// source bits (`FLG_SWIEPH`/`FLG_JPLEPH`/`FLG_MOSEPH`) with the
/// [`crate::HELFLAG_LONG_SEARCH`] … [`crate::HELFLAG_AVKIND`] search bits.
///
/// The search always runs forward from the epoch; there is no native
/// backward search. The Sun has no heliacal event and fails natively.
/// Arcus-visionis search kinds (`HELFLAG_AVKIND` bits) are unavailable in
/// this pinned build: its internal search buffer is too small for a native
/// horizontal-coordinate write. The safe wrapper rejects these flags
/// instead of entering that path; it does not substitute another model.
///
/// # Configuration dependence
///
/// File-based sources require data files visible through
/// [`crate::set_ephe_path`]; fixed-star objects additionally need the
/// `sefstars.txt` catalog on that path. The active configuration is
/// captured under the process-wide native lock together with the search.
///
/// # Errors
///
/// Non-finite epoch/observer/atmosphere inputs and overlong or
/// NUL-containing object names are rejected before any native call. A
/// negative native status (unknown object, Sun, impossible type/object
/// combination, out-of-range height) becomes [`Error`] with the
/// diagnostic kept.
/// Any `HELFLAG_AVKIND` bit returns `InvalidInput` before native access or
/// observer configuration, because that search path has a confirmed native
/// stack-buffer overflow. The native dependency remains unchanged.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, HELIACAL_RISING, heliacal_ut};
///
/// // Next heliacal rising of Venus after J2000 (Moshier needs no data).
/// let event = heliacal_ut(
///     2451545.0, 0.0, 51.5, 0.0, [0.0; 4], [0.0; 6],
///     "Venus", HELIACAL_RISING, FLG_MOSEPH,
/// )?;
/// assert!(event.begin() > 2451545.0);
/// assert!(event.begin() <= event.optimum() && event.optimum() <= event.end());
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
///
/// Observer height must satisfy the arithmetic ceiling documented by
/// [`crate::set_topo`]; excessive height returns `InvalidInput` before
/// observer configuration or computation. Native geographic limits and
/// data errors are still reported by the engine.
#[allow(clippy::too_many_arguments)]
pub fn heliacal_ut(
    jd_start_ut: f64,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atmosphere: [f64; HELIACAL_ATM_LEN],
    observer: [f64; HELIACAL_OBS_LEN],
    object: &str,
    event_type: i32,
    helflag: i32,
) -> Result<HeliacalEvent, Error> {
    check_heliacal_inputs(
        "heliacal_ut",
        jd_start_ut,
        longitude,
        latitude,
        altitude,
        &atmosphere,
        &observer,
        object,
    )?;
    if helflag & crate::HELFLAG_AVKIND != 0 {
        return Err(Error::invalid_input(
            "heliacal_ut: arcus-visionis search flags are unavailable in the pinned native library (internal buffer capacity defect)",
        ));
    }
    let mut name_buffer = heliacal_name_buffer(object);
    with_native_access(|| {
        crate::domain::check_ut_locked("heliacal_ut", jd_start_ut, -1)?;
        let mut geopos = [0.0; HELIACAL_GEO_LEN];
        geopos[0] = longitude;
        geopos[1] = latitude;
        geopos[2] = altitude;
        // The engine writes its atmosphere/observer defaults into zero
        // entries, so owned copies are passed and never aliased elsewhere.
        let mut datm = atmosphere;
        let mut dobs = observer;
        // Zero-initialized: the Moon path writes only slots 0-2,
        // so the remaining slots read as
        // 0.0 by construction (never uninitialized memory).
        let mut dret = [0.0; HELIACAL_DRET_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `geopos`/`datm`/`dobs` own HELIACAL_GEO_LEN /
        // HELIACAL_ATM_LEN / HELIACAL_OBS_LEN writable doubles,
        // `name_buffer` owns TEXT_BUF_LEN bytes holding a NUL-terminated
        // name, `dret` owns HELIACAL_DRET_LEN writable slots and `serr`
        // owns SERR_LEN writable bytes; all outlive the call under the
        // native lock. Results are only exposed after the native status
        // is checked.
        let ret = unsafe {
            ffi::swe_heliacal_ut(
                jd_start_ut,
                geopos.as_mut_ptr(),
                datm.as_mut_ptr(),
                dobs.as_mut_ptr(),
                name_buffer.as_mut_ptr(),
                event_type,
                helflag,
                dret.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_heliacal_search_failure(
                "heliacal_ut",
                jd_start_ut,
                object,
                event_type,
                helflag,
                &diagnostic,
            )))
        } else {
            Ok(HeliacalEvent {
                times: dret,
                diagnostic,
            })
        }
    })
}

/// Report the heliacal visibility circumstances of a planet or fixed star
/// at an instant.
///
/// `jd_ut` is Universal Time; observer, atmosphere, object and flag
/// selection match [`heliacal_ut`], except that `helflag` carries the
/// display bits rather than search bits (the instant is given, so no
/// search runs). Unlike [`vis_limit_mag`] this call always computes: an
/// object below the horizon yields negative altitudes as data, never a
/// dedicated status.
///
/// Observer side effect: although no search runs, the native call still
/// installs this observer as the process-global observer (as if by
/// [`crate::set_topo`]), so later topocentric [`crate::calc_ut`] and
/// [`crate::calc`] calls observe it until [`crate::set_topo`] runs
/// again. Established by probing the pinned build.
///
/// # Configuration dependence
///
/// Same contract as [`heliacal_ut`].
///
/// # Errors
///
/// Non-finite instant/observer/atmosphere inputs and overlong or
/// NUL-containing object names are rejected before any native call. A
/// negative native status becomes [`Error`] with the diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, HELIACAL_RISING, heliacal_pheno_ut, heliacal_ut};
///
/// // Circumstances of the next Venus heliacal rising at its optimum
/// // (Moshier needs no data files).
/// let event = heliacal_ut(
///     2451545.0, 0.0, 51.5, 0.0, [0.0; 4], [0.0; 6],
///     "Venus", HELIACAL_RISING, FLG_MOSEPH,
/// )?;
/// let pheno = heliacal_pheno_ut(
///     event.optimum(), 0.0, 51.5, 0.0, [0.0; 4], [0.0; 6],
///     "Venus", HELIACAL_RISING, FLG_MOSEPH,
/// )?;
/// assert!((-5.0..-3.0).contains(&pheno.object_magnitude()));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
///
/// Observer height must satisfy the arithmetic ceiling documented by
/// [`crate::set_topo`]; excessive height returns `InvalidInput` before
/// observer configuration or computation. Native geographic limits and
/// data errors are still reported by the engine.
#[allow(clippy::too_many_arguments)]
pub fn heliacal_pheno_ut(
    jd_ut: f64,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atmosphere: [f64; HELIACAL_ATM_LEN],
    observer: [f64; HELIACAL_OBS_LEN],
    object: &str,
    event_type: i32,
    helflag: i32,
) -> Result<HeliacalPhenomena, Error> {
    check_heliacal_inputs(
        "heliacal_pheno_ut",
        jd_ut,
        longitude,
        latitude,
        altitude,
        &atmosphere,
        &observer,
        object,
    )?;
    let mut name_buffer = heliacal_name_buffer(object);
    with_native_access(|| {
        crate::domain::check_ut_locked("heliacal_pheno_ut", jd_ut, -1)?;
        let mut geopos = [0.0; HELIACAL_GEO_LEN];
        geopos[0] = longitude;
        geopos[1] = latitude;
        geopos[2] = altitude;
        // Same owned-copy contract for the defaulted blocks as in
        // [`heliacal_ut`].
        let mut datm = atmosphere;
        let mut dobs = observer;
        // Zero-initialized: the engine writes slots 0-27 and leaves the
        // reserved slots 28-29 untouched, so they read as 0.0 by
        // construction (never uninitialized memory).
        let mut darr = [0.0; HELIACAL_PHENO_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`heliacal_ut`]: every buffer is
        // owned, correctly sized and held under the native lock, and the
        // results are only exposed after the native status is checked.
        let ret = unsafe {
            ffi::swe_heliacal_pheno_ut(
                jd_ut,
                geopos.as_mut_ptr(),
                datm.as_mut_ptr(),
                dobs.as_mut_ptr(),
                name_buffer.as_mut_ptr(),
                event_type,
                helflag,
                darr.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_heliacal_moment_failure(
                "heliacal_pheno_ut",
                jd_ut,
                object,
                event_type,
                helflag,
                &diagnostic,
            )))
        } else {
            Ok(HeliacalPhenomena {
                values: darr,
                diagnostic,
            })
        }
    })
}

/// Compute the visual limiting magnitude for a planet or fixed star at an
/// instant.
///
/// `jd_ut` is Universal Time; observer, atmosphere, object and flag
/// selection match [`heliacal_ut`], except that no event type applies
/// (the instant is given, so no search runs).
///
/// Observer side effect: although no search runs, the native call still
/// installs this observer as the process-global observer (as if by
/// [`crate::set_topo`]), so later topocentric [`crate::calc_ut`] and
/// [`crate::calc`] calls observe it until [`crate::set_topo`] runs
/// again. Established by probing the pinned build.
///
/// The object is visible when
/// it is brighter (numerically smaller) than the returned limiting
/// magnitude.
///
/// An object below the local horizon has no limiting magnitude: the
/// engine reports that absence with status −2, preserved here as
/// [`VisibilityOutcome::BelowHorizon`] with the diagnostic kept — never
/// an error and never invented magnitude data.
///
/// # Configuration dependence
///
/// Same contract as [`heliacal_ut`].
///
/// # Errors
///
/// Non-finite instant/observer/atmosphere inputs and overlong or
/// NUL-containing object names are rejected before any native call. A
/// native failure (the Sun, which has no limiting magnitude, or an
/// unknown object) becomes [`Error`] with the diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{
///     FLG_MOSEPH, HELIACAL_RISING, VisibilityOutcome, heliacal_ut, vis_limit_mag,
/// };
///
/// // At its heliacal-rising optimum Venus is above the horizon and has
/// // a limiting magnitude (Moshier needs no data files).
/// let event = heliacal_ut(
///     2451545.0, 0.0, 51.5, 0.0, [0.0; 4], [0.0; 6],
///     "Venus", HELIACAL_RISING, FLG_MOSEPH,
/// )?;
/// let outcome = vis_limit_mag(
///     event.optimum(), 0.0, 51.5, 0.0, [0.0; 4], [0.0; 6],
///     "Venus", FLG_MOSEPH,
/// )?;
/// assert!(matches!(outcome, VisibilityOutcome::Visible(_)));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
///
/// Observer height must satisfy the arithmetic ceiling documented by
/// [`crate::set_topo`]; excessive height returns `InvalidInput` before
/// observer configuration or computation. Native geographic limits and
/// data errors are still reported by the engine.
#[allow(clippy::too_many_arguments)]
pub fn vis_limit_mag(
    jd_ut: f64,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atmosphere: [f64; HELIACAL_ATM_LEN],
    observer: [f64; HELIACAL_OBS_LEN],
    object: &str,
    helflag: i32,
) -> Result<VisibilityOutcome, Error> {
    check_heliacal_inputs(
        "vis_limit_mag",
        jd_ut,
        longitude,
        latitude,
        altitude,
        &atmosphere,
        &observer,
        object,
    )?;
    let mut name_buffer = heliacal_name_buffer(object);
    with_native_access(|| {
        crate::domain::check_ut_locked("vis_limit_mag", jd_ut, -1)?;
        let mut geopos = [0.0; HELIACAL_GEO_LEN];
        geopos[0] = longitude;
        geopos[1] = latitude;
        geopos[2] = altitude;
        // Same owned-copy contract for the defaulted blocks as in
        // [`heliacal_ut`].
        let mut datm = atmosphere;
        let mut dobs = observer;
        // Zero-initialized: the below-horizon and error paths write only
        // slots 0-6, so slot 7 reads as 0.0 by construction (never
        // uninitialized memory).
        let mut dret = [0.0; VISLIM_DRET_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`heliacal_ut`]: every buffer is
        // owned, correctly sized and held under the native lock, and the
        // results are only exposed after the native status is classified.
        let ret = unsafe {
            ffi::swe_vis_limit_mag(
                jd_ut,
                geopos.as_mut_ptr(),
                datm.as_mut_ptr(),
                dobs.as_mut_ptr(),
                name_buffer.as_mut_ptr(),
                helflag,
                dret.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret == crate::HELFLAG_BELOW_HORIZON {
            Ok(VisibilityOutcome::BelowHorizon(BelowHorizon {
                values: dret,
                diagnostic,
            }))
        } else if ret < 0 {
            Err(Error::native(describe_vislim_failure(
                "vis_limit_mag",
                jd_ut,
                object,
                helflag,
                &diagnostic,
            )))
        } else {
            Ok(VisibilityOutcome::Visible(VisibilityLimit {
                status: ret,
                values: dret,
                diagnostic,
            }))
        }
    })
}

/// Build the error message for a failed heliacal search, keeping the
/// native diagnostic when the engine provided one.
fn describe_heliacal_search_failure(
    function: &str,
    jd_start_ut: f64,
    object: &str,
    event_type: i32,
    helflag: i32,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!(
            "{function}: native search failed (jd_start_ut={jd_start_ut}, object={object}, event_type={event_type}, helflag={helflag})"
        )
    } else {
        format!(
            "{function}: {diagnostic} (jd_start_ut={jd_start_ut}, object={object}, event_type={event_type}, helflag={helflag})"
        )
    }
}

/// Build the error message for a failed heliacal-moment call, keeping the
/// native diagnostic when the engine provided one.
fn describe_heliacal_moment_failure(
    function: &str,
    jd_ut: f64,
    object: &str,
    event_type: i32,
    helflag: i32,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!(
            "{function}: native computation failed (jd_ut={jd_ut}, object={object}, event_type={event_type}, helflag={helflag})"
        )
    } else {
        format!(
            "{function}: {diagnostic} (jd_ut={jd_ut}, object={object}, event_type={event_type}, helflag={helflag})"
        )
    }
}

/// Build the error message for a failed limiting-magnitude call, keeping
/// the native diagnostic when the engine provided one.
fn describe_vislim_failure(
    function: &str,
    jd_ut: f64,
    object: &str,
    helflag: i32,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!(
            "{function}: native computation failed (jd_ut={jd_ut}, object={object}, helflag={helflag})"
        )
    } else {
        format!("{function}: {diagnostic} (jd_ut={jd_ut}, object={object}, helflag={helflag})")
    }
}
