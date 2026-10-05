//! Observer events and physical quantities.
//!
//! [`nod_aps`] and [`nod_aps_ut`] compute the ascending/descending nodes
//! and the perihelion/aphelion (or focal point) of a body with the native
//! engine. [`rise_trans`] and [`rise_trans_true_hor`] search the next
//! rising, setting or meridian transit after a Universal Time instant for
//! a body or a fixed star seen from an explicit observer position.
//!
//! [`azalt`] and [`azalt_rev`] convert between celestial and horizontal
//! (azimuth/altitude) coordinates; [`refrac`] and [`refrac_extended`]
//! convert between true and apparent altitude, with [`set_lapse_rate`]
//! overriding the process-global lapse rate used by elevation-dependent
//! native steps. [`pheno`] and [`pheno_ut`] report phase, elongation,
//! apparent size and magnitude; [`get_orbital_elements`] reports the
//! osculating Keplerian elements and [`orbit_max_min_true_distance`] the
//! extreme and current geocentric distances.
//!
//! Every call holds the process-wide native lock across its complete
//! native sequence, so any thread may call any function without external
//! synchronization. Native diagnostics are preserved; a native failure,
//! the absence of an event (circumpolar rise/set) and valid data are
//! distinct states.

use std::ffi::c_char;

use crate::error::Error;
use crate::ffi::{
    self, CELESTIAL_LEN, HORIZONTAL_LEN, ORBEL_LEN, PHENO_LEN, POSITION_LEN,
    REFRACTION_DETAILS_LEN, SERR_LEN, TEXT_BUF_LEN, read_native_text,
};
use crate::state::with_native_access;

/// Nodes and apsides of one body: the points where its orbital plane
/// crosses the ecliptic plus its closest/farthest points from the Sun.
///
/// Each vector holds six native components in the same layout as
/// [`crate::Position`]: ecliptic longitude/latitude in degrees, distance
/// in AU, then the three matching daily rates when speeds were requested
/// (`FLG_SPEED`, otherwise the rate slots are zero). With
/// [`crate::NODBIT_FOPOINT`] the second focal point of the orbital
/// ellipse is returned in `aphelion` instead of the aphelion.
#[derive(Debug, Clone, PartialEq)]
pub struct NodesApsides {
    /// Ascending-node vector (six components; see the type documentation).
    pub ascending: [f64; POSITION_LEN],
    /// Descending-node vector (six components).
    pub descending: [f64; POSITION_LEN],
    /// Perihelion vector (six components).
    pub perihelion: [f64; POSITION_LEN],
    /// Aphelion vector — or the focal point with `NODBIT_FOPOINT`.
    pub aphelion: [f64; POSITION_LEN],
    /// Native return status echoed as an integer.
    ///
    /// Unlike [`crate::Position::returned_flags`] this carries no
    /// ephemeris-source bits: the engine reports plain `OK` (0) here on
    /// success. It is preserved so callers can distinguish the native
    /// status from the input flags without re-issuing the call.
    pub returned_flags: i32,
    /// Native diagnostic text (`serr`); empty on a clean computation.
    pub diagnostic: String,
}

/// Outcome of a rise/set/transit search.
///
/// A circumpolar body may never rise or set at the observer latitude;
/// the engine reports that absence of an event with a dedicated status
/// that is preserved here instead of being turned into an error or into
/// invented time data.
#[derive(Debug, Clone, PartialEq)]
pub enum RiseTransitOutcome {
    /// The event was found: `time_ut` is its Universal Time Julian Day.
    Event {
        /// Event time as a Julian Day in Universal Time.
        time_ut: f64,
        /// Native diagnostic text (`serr`); usually empty.
        diagnostic: String,
    },
    /// No rise/set event exists (circumpolar object): there is no event
    /// time, only the preserved native diagnostic.
    Circumpolar {
        /// Native diagnostic text (`serr`); usually empty.
        diagnostic: String,
    },
}

/// Calculate nodes and apsides for an Ephemeris Time Julian Day.
///
/// `body` is a body number (for example [`crate::MARS`]); `flags`
/// combines the same source/geometry/output bits as [`crate::calc`]
/// (ephemeris flag, heliocentric, speed, ...). `method` selects the
/// model: [`crate::NODBIT_MEAN`] (mean elements where defined, the
/// default — also used when `method` is 0), [`crate::NODBIT_OSCU`]
/// (osculating elements for every body),
/// [`crate::NODBIT_OSCU_BAR`] (barycentric ellipse beyond Jupiter) and
/// [`crate::NODBIT_FOPOINT`] (focal point in the aphelion slot,
/// combinable with any other bit). Any integer passes through to the
/// engine; there is no Rust-side method validation.
///
/// # Time scale
///
/// `jd_et` is Ephemeris Time. For Universal Time use [`nod_aps_ut`].
///
/// # Configuration dependence
///
/// File-based sources require data files visible through
/// [`crate::set_ephe_path`]; the active configuration is captured under
/// the process-wide native lock together with the computation.
///
/// # Errors
///
/// Non-finite `jd_et` is rejected before any native call. A negative
/// native status (unknown body, missing data) becomes [`Error`] with the
/// diagnostic kept; a non-negative status is `Ok` with details in
/// [`NodesApsides::returned_flags`] and [`NodesApsides::diagnostic`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, FLG_SPEED, MARS, NODBIT_MEAN, nod_aps};
///
/// // The Moshier model needs no data files, so this runs anywhere.
/// let nodes = nod_aps(2451545.0, MARS, FLG_MOSEPH | FLG_SPEED, NODBIT_MEAN)?;
/// assert!((0.0..360.0).contains(&nodes.ascending[0]));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn nod_aps(jd_et: f64, body: i32, flags: i32, method: i32) -> Result<NodesApsides, Error> {
    if !jd_et.is_finite() {
        return Err(Error::invalid_input("nod_aps: Julian Day must be finite"));
    }
    with_native_access(|| {
        crate::domain::check_et_locked("nod_aps", jd_et, flags)?;
        let mut xnasc = [0.0; POSITION_LEN];
        let mut xndsc = [0.0; POSITION_LEN];
        let mut xperi = [0.0; POSITION_LEN];
        let mut xaphe = [0.0; POSITION_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: each vector owns POSITION_LEN writable f64 slots and
        // `serr` owns SERR_LEN writable bytes; all outlive this call and
        // no other thread can access native state while the lock is held.
        // Results are only exposed after the native status is checked.
        let ret = unsafe {
            ffi::swe_nod_aps(
                jd_et,
                body,
                flags,
                method,
                xnasc.as_mut_ptr(),
                xndsc.as_mut_ptr(),
                xperi.as_mut_ptr(),
                xaphe.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_nodes_failure(
                "nod_aps",
                jd_et,
                body,
                flags,
                method,
                &diagnostic,
            )))
        } else {
            Ok(NodesApsides {
                ascending: xnasc,
                descending: xndsc,
                perihelion: xperi,
                aphelion: xaphe,
                returned_flags: ret,
                diagnostic,
            })
        }
    })
}

/// Calculate nodes and apsides for a Universal Time Julian Day.
///
/// Identical to [`nod_aps`] except that `jd_ut` is Universal Time.
/// Argument order, flags, method, result layout and error behavior match
/// [`nod_aps`] exactly.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, FLG_SPEED, MARS, NODBIT_MEAN, nod_aps_ut};
///
/// let nodes = nod_aps_ut(2451545.0, MARS, FLG_MOSEPH | FLG_SPEED, NODBIT_MEAN)?;
/// assert!((0.0..360.0).contains(&nodes.perihelion[0]));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn nod_aps_ut(jd_ut: f64, body: i32, flags: i32, method: i32) -> Result<NodesApsides, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(
            "nod_aps_ut: Julian Day must be finite",
        ));
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("nod_aps_ut", jd_ut, flags)?;
        let mut xnasc = [0.0; POSITION_LEN];
        let mut xndsc = [0.0; POSITION_LEN];
        let mut xperi = [0.0; POSITION_LEN];
        let mut xaphe = [0.0; POSITION_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`nod_aps`].
        let ret = unsafe {
            ffi::swe_nod_aps_ut(
                jd_ut,
                body,
                flags,
                method,
                xnasc.as_mut_ptr(),
                xndsc.as_mut_ptr(),
                xperi.as_mut_ptr(),
                xaphe.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: same contract as in [`nod_aps`].
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_nodes_failure(
                "nod_aps_ut",
                jd_ut,
                body,
                flags,
                method,
                &diagnostic,
            )))
        } else {
            Ok(NodesApsides {
                ascending: xnasc,
                descending: xndsc,
                perihelion: xperi,
                aphelion: xaphe,
                returned_flags: ret,
                diagnostic,
            })
        }
    })
}

/// Build the error message for a failed nodes/apsides computation,
/// keeping the native diagnostic when the engine provided one.
fn describe_nodes_failure(
    function: &str,
    jd: f64,
    body: i32,
    flags: i32,
    method: i32,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!(
            "{function}: native computation failed (jd={jd}, body={body}, flags={flags}, method={method})"
        )
    } else {
        format!("{function}: {diagnostic} (jd={jd}, body={body}, flags={flags}, method={method})")
    }
}

/// Search rise/set/transit while native access is already held.
///
/// Caller must hold the native lock; all inputs must already be
/// validated by [`check_rise_inputs`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn rise_trans_locked(
    jd_ut: f64,
    body: i32,
    star_name: Option<&str>,
    epheflag: i32,
    rsmi: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atpress: f64,
    attemp: f64,
) -> Result<RiseTransitOutcome, Error> {
    crate::domain::check_ut_locked("rise_trans", jd_ut, epheflag)?;
    // Caller-owned star buffer: the native call reads the name while
    // resolving a fixed star, and every access stays inside this buffer.
    // The body path passes null, which the engine treats as "no star".
    let mut star_buffer = [0 as c_char; TEXT_BUF_LEN];
    if let Some(name) = star_name {
        for (slot, byte) in star_buffer.iter_mut().zip(name.bytes()) {
            *slot = byte as c_char;
        }
    }
    let geopos = [longitude, latitude, altitude];
    let mut tret = 0.0;
    let mut serr = [0 as c_char; SERR_LEN];
    // SAFETY: `star_buffer` owns TEXT_BUF_LEN bytes holding a
    // NUL-terminated name (or is unused when null is passed),
    // `geopos` owns three readable doubles, `tret` one writable slot
    // and `serr` SERR_LEN writable bytes; all outlive the call and the
    // caller holds the native lock. Results are only exposed after the
    // native status is classified.
    let ret = unsafe {
        ffi::swe_rise_trans(
            jd_ut,
            body,
            if star_name.is_none() {
                std::ptr::null_mut()
            } else {
                star_buffer.as_mut_ptr()
            },
            epheflag,
            rsmi,
            geopos.as_ptr(),
            atpress,
            attemp,
            &mut tret,
            serr.as_mut_ptr(),
        )
    };
    // SAFETY: `serr` is our own fully initialized buffer; the read is
    // bounded by its capacity. Copied before the lock is released.
    let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
    if ret == 0 {
        Ok(RiseTransitOutcome::Event {
            time_ut: tret,
            diagnostic,
        })
    } else if ret == -2 {
        Ok(RiseTransitOutcome::Circumpolar { diagnostic })
    } else {
        Err(Error::native(describe_rise_failure(
            "rise_trans",
            jd_ut,
            body,
            rsmi,
            &diagnostic,
        )))
    }
}

/// Check a rise/set/transit request before any native call.
#[allow(clippy::too_many_arguments)]
pub(crate) fn check_rise_inputs(
    function: &str,
    jd_ut: f64,
    star_name: Option<&str>,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atpress: f64,
    attemp: f64,
    horhgt: Option<f64>,
) -> Result<(), Error> {
    crate::domain::check_observer_height(function, altitude)?;
    if !jd_ut.is_finite()
        || !longitude.is_finite()
        || !latitude.is_finite()
        || !altitude.is_finite()
        || !atpress.is_finite()
        || !attemp.is_finite()
        || horhgt.is_some_and(|h| !h.is_finite())
    {
        return Err(Error::invalid_input(format!(
            "{function}: time, observer, atmosphere and horizon inputs must be finite"
        )));
    }
    if let Some(name) = star_name
        && (name.as_bytes().contains(&0) || name.len() >= TEXT_BUF_LEN)
    {
        return Err(Error::invalid_input(format!(
            "{function}: star name must not contain NUL and must fit the 256-byte native buffer"
        )));
    }
    Ok(())
}

/// Search the next rising, setting or meridian transit after a
/// Universal Time instant.
///
/// `body` is a body number (for example [`crate::SUN`]); pass a star via
/// `star_name` (`Some` selects the fixed-star path, `None` the body
/// path — an empty name behaves as `None` natively). `epheflag` carries
/// the ephemeris source bits
/// (`FLG_SWIEPH`/`FLG_JPLEPH`/`FLG_MOSEPH`). `rsmi` selects the event
/// and its modifiers: [`crate::CALC_RISE`], [`crate::CALC_SET`],
/// [`crate::CALC_MTRANSIT`], [`crate::CALC_ITRANSIT`], optionally
/// combined with disc/refraction/twilight bits
/// ([`crate::BIT_DISC_CENTER`], [`crate::BIT_NO_REFRACTION`],
/// [`crate::BIT_CIVIL_TWILIGHT`], ...; [`crate::BIT_HINDU_RISING`]
/// selects the Hindu convention). `longitude`/`latitude`/`altitude`
/// locate the observer (east-positive degrees, degrees north, meters);
/// `atpress` (mbar, 0 = estimated from the altitude) and `attemp` (°C)
/// feed refraction.
///
/// Observer side effect: the native search installs this observer as the
/// process-global observer (as if by [`crate::set_topo`]), so later
/// topocentric [`crate::calc_ut`]/[`crate::calc`] calls observe it until
/// [`crate::set_topo`] runs again. Established by probing the pinned
/// build.
///
/// A found event reports its Universal Time Julian Day; a circumpolar
/// rise/set with no event is [`RiseTransitOutcome::Circumpolar`], never
/// an error and never invented time data. Native failures (unknown
/// body/star, missing data) are [`Error`] with the diagnostic kept.
///
/// # Configuration dependence
///
/// File-based sources and the fixed-star path require data/catalog files
/// visible through [`crate::set_ephe_path`]; the active configuration is
/// captured under the process-wide native lock together with the search.
///
/// # Errors
///
/// Non-finite time/observer/atmosphere inputs and overlong or
/// NUL-containing star names are rejected before any native call.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{CALC_RISE, FLG_SWIEPH, SUN, rise_trans};
///
/// // Sunrise over Greenwich on 2000-01-01 (explicit external data, else
/// // the checkout data, keeps this runnable).
/// use swisseph_bindings::set_ephe_path;
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
/// let outcome = rise_trans(2451544.5, SUN, None, FLG_SWIEPH, CALC_RISE, 0.0, 51.5, 0.0, 1013.25, 15.0)?;
/// assert!(matches!(outcome, swisseph_bindings::RiseTransitOutcome::Event {.. }));
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
pub fn rise_trans(
    jd_ut: f64,
    body: i32,
    star_name: Option<&str>,
    epheflag: i32,
    rsmi: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atpress: f64,
    attemp: f64,
) -> Result<RiseTransitOutcome, Error> {
    check_rise_inputs(
        "rise_trans",
        jd_ut,
        star_name,
        longitude,
        latitude,
        altitude,
        atpress,
        attemp,
        None,
    )?;
    with_native_access(|| {
        rise_trans_locked(
            jd_ut, body, star_name, epheflag, rsmi, longitude, latitude, altitude, atpress, attemp,
        )
    })
}

/// Search the next rising, setting or transit for a local horizon of
/// apparent height `horhgt` (degrees) at the rise/set point.
///
/// Identical to [`rise_trans`] except for the extra `horhgt` parameter;
/// result layout and error behavior match [`rise_trans`] exactly,
/// including the process-global observer side effect documented there.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{CALC_SET, FLG_SWIEPH, SUN, rise_trans_true_hor, set_ephe_path};
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
/// let outcome = rise_trans_true_hor(2451544.5, SUN, None, FLG_SWIEPH, CALC_SET, 0.0, 51.5, 0.0, 1013.25, 15.0, 0.0)?;
/// assert!(matches!(outcome, swisseph_bindings::RiseTransitOutcome::Event {.. }));
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
pub fn rise_trans_true_hor(
    jd_ut: f64,
    body: i32,
    star_name: Option<&str>,
    epheflag: i32,
    rsmi: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atpress: f64,
    attemp: f64,
    horhgt: f64,
) -> Result<RiseTransitOutcome, Error> {
    check_rise_inputs(
        "rise_trans_true_hor",
        jd_ut,
        star_name,
        longitude,
        latitude,
        altitude,
        atpress,
        attemp,
        Some(horhgt),
    )?;
    let mut star_buffer = [0 as c_char; TEXT_BUF_LEN];
    if let Some(name) = star_name {
        for (slot, byte) in star_buffer.iter_mut().zip(name.bytes()) {
            *slot = byte as c_char;
        }
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("rise_trans_true_hor", jd_ut, epheflag)?;
        let geopos = [longitude, latitude, altitude];
        let mut tret = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`rise_trans`].
        let ret = unsafe {
            ffi::swe_rise_trans_true_hor(
                jd_ut,
                body,
                if star_name.is_none() {
                    std::ptr::null_mut()
                } else {
                    star_buffer.as_mut_ptr()
                },
                epheflag,
                rsmi,
                geopos.as_ptr(),
                atpress,
                attemp,
                horhgt,
                &mut tret,
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: same contract as in [`rise_trans`].
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret == 0 {
            Ok(RiseTransitOutcome::Event {
                time_ut: tret,
                diagnostic,
            })
        } else if ret == -2 {
            Ok(RiseTransitOutcome::Circumpolar { diagnostic })
        } else {
            Err(Error::native(describe_rise_failure(
                "rise_trans_true_hor",
                jd_ut,
                body,
                rsmi,
                &diagnostic,
            )))
        }
    })
}

/// Build the error message for a failed rise/set/transit search, keeping
/// the native diagnostic when the engine provided one.
fn describe_rise_failure(
    function: &str,
    jd_ut: f64,
    body: i32,
    rsmi: i32,
    diagnostic: &str,
) -> String {
    if diagnostic.is_empty() {
        format!("{function}: native search failed (jd_ut={jd_ut}, body={body}, rsmi={rsmi})")
    } else {
        format!("{function}: {diagnostic} (jd_ut={jd_ut}, body={body}, rsmi={rsmi})")
    }
}

/// Horizontal position of a celestial object as computed by [`azalt`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Horizontal {
    /// Azimuth in degrees, measured from the South westward
    /// (0 = South, 90 = West, 180 = North, 270 = East).
    ///
    /// This is the reference-API convention, not the common
    /// from-North-eastward one; convert with `(azimuth + 180.0) % 360.0`.
    pub azimuth: f64,
    /// True (geometric) altitude above the horizon in degrees, without
    /// refraction.
    pub true_altitude: f64,
    /// Apparent altitude above the horizon in degrees, with the engine's
    /// refraction step applied.
    pub apparent_altitude: f64,
}

/// Celestial position as computed by [`azalt_rev`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Celestial {
    /// Right ascension ([`crate::HOR2EQU`]) or ecliptic longitude
    /// ([`crate::HOR2ECL`]) in degrees.
    pub longitude_or_ra: f64,
    /// Declination ([`crate::HOR2EQU`]) or ecliptic latitude
    /// ([`crate::HOR2ECL`]) in degrees.
    pub latitude_or_dec: f64,
}

/// Finite refraction conversion as computed by [`refrac_extended`].
///
/// A non-finite converted altitude or detail is returned as a native error,
/// never as a partially valid result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Refraction {
    /// Converted altitude in degrees: apparent altitude for
    /// [`crate::TRUE_TO_APP`], true altitude for [`crate::APP_TO_TRUE`].
    pub converted: f64,
    /// True (geometric) altitude in degrees (input or computed).
    pub true_altitude: f64,
    /// Apparent altitude in degrees (input or computed).
    pub apparent_altitude: f64,
    /// Refraction amount in degrees (`apparent − true`).
    pub refraction: f64,
    /// Dip of the horizon in degrees (negative for elevated observers;
    /// zero at sea level).
    pub dip: f64,
}

/// Check horizontal-conversion inputs before any native call.
#[allow(clippy::too_many_arguments)]
fn check_horizontal_inputs(
    function: &str,
    jd_ut: f64,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atpress: f64,
    attemp: f64,
    input_a: f64,
    input_b: f64,
) -> Result<(), Error> {
    crate::domain::check_observer_height(function, altitude)?;
    if !jd_ut.is_finite()
        || !longitude.is_finite()
        || !latitude.is_finite()
        || !altitude.is_finite()
        || !atpress.is_finite()
        || !attemp.is_finite()
        || !input_a.is_finite()
        || !input_b.is_finite()
    {
        return Err(Error::invalid_input(format!(
            "{function}: time, observer, atmosphere and coordinate inputs must be finite"
        )));
    }
    Ok(())
}

/// Convert celestial coordinates to horizontal (azimuth/altitude) for a
/// Universal Time instant and an explicit observer position.
///
/// `calc_flag` selects the input frame: [`crate::ECL2HOR`] (ecliptic
/// longitude/latitude) or [`crate::EQU2HOR`] (right ascension/
/// declination). Only those two values are defined; any other integer
/// passes through to the engine with engine-defined behavior (probing the
/// pinned build shows a nonzero flag taking the equatorial branch — rely
/// on the defined values, not on that observation).
/// `longitude`/`latitude`/`altitude` locate the observer (east-positive
/// degrees, degrees north, meters); `atpress` (mbar) and `attemp` (°C)
/// feed the engine's refraction step. `lon_or_ra`/`lat_or_dec` carry the
/// input pair in degrees; distance plays no role (the engine ignores the
/// third input slot, established by probing, so no distance argument
/// exists here).
///
/// The output azimuth follows the reference-API convention (from the
/// South, westward). The engine's pressure handling inside this call is
/// preserved verbatim: callers needing explicit pressure control over
/// the refraction step should use [`refrac`] or [`refrac_extended`].
///
/// # Configuration dependence
///
/// The conversion reads the native Delta-T state for sidereal time, so it
/// runs under the process-wide native lock together with that read.
///
/// # Errors
///
/// Non-finite time/observer/atmosphere/coordinate inputs are rejected
/// before any native call. The native call itself has no failure mode;
/// `Result` also covers the arithmetic-domain checks described below and
/// lock poisoning.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{EQU2HOR, azalt};
///
/// let h = azalt(2451545.0, EQU2HOR, 12.5, 41.9, 0.0, 1013.25, 15.0, 200.0, 60.0)?;
/// assert!((0.0..360.0).contains(&h.azimuth));
/// assert!(h.apparent_altitude >= h.true_altitude);
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
pub fn azalt(
    jd_ut: f64,
    calc_flag: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atpress: f64,
    attemp: f64,
    lon_or_ra: f64,
    lat_or_dec: f64,
) -> Result<Horizontal, Error> {
    check_horizontal_inputs(
        "azalt", jd_ut, longitude, latitude, altitude, atpress, attemp, lon_or_ra, lat_or_dec,
    )?;
    with_native_access(|| {
        crate::domain::check_ut_locked("azalt", jd_ut, -1)?;
        let geopos = [longitude, latitude, altitude];
        // The third input slot is engine-ignored (see `HORIZONTAL_LEN`);
        // an initialized 0.0 keeps the buffer fully defined regardless.
        let xin = [lon_or_ra, lat_or_dec, 0.0];
        let mut xaz = [0.0; HORIZONTAL_LEN];
        // SAFETY: `geopos`/`xin` own three readable doubles each and
        // `xaz` owns three writable slots; all outlive this call under
        // the native lock. The call has no failure mode.
        unsafe {
            ffi::swe_azalt(
                jd_ut,
                calc_flag,
                geopos.as_ptr(),
                atpress,
                attemp,
                xin.as_ptr(),
                xaz.as_mut_ptr(),
            );
        }
        Ok(Horizontal {
            azimuth: xaz[0],
            true_altitude: xaz[1],
            apparent_altitude: xaz[2],
        })
    })
}

/// Convert horizontal (azimuth/altitude) coordinates back to celestial
/// coordinates for a Universal Time instant and an explicit observer
/// position.
///
/// `calc_flag` selects the output frame: [`crate::HOR2ECL`] (ecliptic
/// longitude/latitude) or [`crate::HOR2EQU`] (right ascension/
/// declination); flag pass-through follows [`azalt`]. The observer
/// arguments match [`azalt`]; `azimuth` uses the same from-South-westward
/// convention as [`Horizontal::azimuth`], and `true_altitude` is the
/// geometric altitude (convert an apparent altitude with [`refrac`] or
/// [`refrac_extended`] first).
///
/// This is the inverse of [`azalt`] up to refraction handling: a round
/// trip through both calls reproduces the input pair.
///
/// # Errors
///
/// Same finite-input, date, observer-height and locking contract as [`azalt`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{EQU2HOR, HOR2EQU, azalt, azalt_rev};
///
/// let h = azalt(2451545.0, EQU2HOR, 12.5, 41.9, 0.0, 0.0, 15.0, 200.0, 60.0)?;
/// let c = azalt_rev(2451545.0, HOR2EQU, 12.5, 41.9, 0.0, h.azimuth, h.true_altitude)?;
/// assert!((c.longitude_or_ra - 200.0).abs() < 1e-6);
/// assert!((c.latitude_or_dec - 60.0).abs() < 1e-6);
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
pub fn azalt_rev(
    jd_ut: f64,
    calc_flag: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    azimuth: f64,
    true_altitude: f64,
) -> Result<Celestial, Error> {
    check_horizontal_inputs(
        "azalt_rev",
        jd_ut,
        longitude,
        latitude,
        altitude,
        0.0,
        0.0,
        azimuth,
        true_altitude,
    )?;
    with_native_access(|| {
        crate::domain::check_ut_locked("azalt_rev", jd_ut, -1)?;
        let geopos = [longitude, latitude, altitude];
        let xin = [azimuth, true_altitude, 0.0];
        let mut xout = [0.0; CELESTIAL_LEN];
        // SAFETY: same contract as in [`azalt`], with a two-slot output
        // buffer the engine fills completely (established by probing).
        unsafe {
            ffi::swe_azalt_rev(
                jd_ut,
                calc_flag,
                geopos.as_ptr(),
                xin.as_ptr(),
                xout.as_mut_ptr(),
            );
        }
        Ok(Celestial {
            longitude_or_ra: xout[0],
            latitude_or_dec: xout[1],
        })
    })
}

/// Convert between true (geometric) and apparent altitude.
///
/// `calc_flag` selects the direction: [`crate::TRUE_TO_APP`] adds
/// refraction, [`crate::APP_TO_TRUE`] removes it. Only those two values
/// are defined; any other integer passes through with engine-defined
/// behavior. Zero pressure disables the correction (the input altitude is
/// returned unchanged); every input must still be finite.
///
/// The two directions are not exact numerical inverses (horizon and
/// zenith clamps are preserved, never silently dropped).
///
/// # Errors
///
/// Non-finite inputs return [`crate::ErrorKind::InvalidInput`] before any
/// native call. Finite inputs are passed through without clamping; if the
/// native result is non-finite, the binding returns [`crate::ErrorKind::Native`]
/// with a binding-generated diagnostic (this native API has no error buffer).
/// Lock poisoning is reported separately.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{TRUE_TO_APP, refrac};
///
/// // Refraction lifts the apparent position at 10° by about 0.09°.
/// let apparent = refrac(10.0, 1013.25, 15.0, TRUE_TO_APP)?;
/// assert!((apparent - 10.0888).abs() < 1e-3);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn refrac(altitude: f64, atpress: f64, attemp: f64, calc_flag: i32) -> Result<f64, Error> {
    if !altitude.is_finite() || !atpress.is_finite() || !attemp.is_finite() {
        return Err(Error::invalid_input(
            "refrac: altitude, pressure and temperature must be finite",
        ));
    }
    with_native_access(|| {
        // SAFETY: pure function of its arguments; no pointers involved.
        // It runs under the lock with the other atmospheric calls so a
        // concurrent `set_lapse_rate` cannot interleave mid-sequence.
        let converted = unsafe { ffi::swe_refrac(altitude, atpress, attemp, calc_flag) };
        if !converted.is_finite() {
            return Err(Error::native(
                "refrac: native calculation produced a non-finite converted altitude",
            ));
        }
        Ok(converted)
    })
}

/// Convert between true and apparent altitude with observer elevation
/// and an explicit lapse rate.
///
/// Arguments match [`refrac`] plus `geoalt` (observer altitude above sea
/// level in meters) and `lapse_rate` (temperature lapse rate dT/dh in
/// K/m; pass `0.0065` for the conventional standard-atmosphere value).
/// The rate is explicit at every call site; the process-global override
/// for other native operations lives in
/// [`set_lapse_rate`].
///
/// Below the dipped horizon the input altitude is returned unchanged with
/// zero refraction; the dip itself is reported when finite. Every returned
/// component must be finite. In particular, the pinned engine can produce
/// a non-finite dip for observers below sea level; that is a native error,
/// not a substituted sea-level calculation.
///
/// # Errors
///
/// Non-finite inputs return [`crate::ErrorKind::InvalidInput`] before any
/// native call. Finite inputs are passed through without clamping; if the
/// native result is non-finite, the binding returns [`crate::ErrorKind::Native`]
/// with a binding-generated diagnostic (this native API has no error buffer).
/// Lock poisoning is reported separately.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{TRUE_TO_APP, refrac_extended};
///
/// // Sea level: no horizon dip; refraction of about 0.087° at 10°.
/// let r = refrac_extended(10.0, 0.0, 1013.25, 15.0, 0.0065, TRUE_TO_APP)?;
/// assert!((r.refraction - 0.0867).abs() < 1e-3);
/// assert_eq!(r.dip, 0.0);
/// // At 1000 m the horizon dips by about 0.88°.
/// let hi = refrac_extended(10.0, 1000.0, 1013.25, 15.0, 0.0065, TRUE_TO_APP)?;
/// assert!(hi.dip < -0.8 && hi.dip > -1.0);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn refrac_extended(
    altitude: f64,
    geoalt: f64,
    atpress: f64,
    attemp: f64,
    lapse_rate: f64,
    calc_flag: i32,
) -> Result<Refraction, Error> {
    if !altitude.is_finite()
        || !geoalt.is_finite()
        || !atpress.is_finite()
        || !attemp.is_finite()
        || !lapse_rate.is_finite()
    {
        return Err(Error::invalid_input(
            "refrac_extended: altitude, observer altitude, pressure, temperature and lapse rate must be finite",
        ));
    }
    with_native_access(|| {
        let mut dret = [0.0; REFRACTION_DETAILS_LEN];
        // SAFETY: `dret` owns four writable slots the engine fills
        // completely (established by probing); it outlives this call
        // under the native lock.
        let converted = unsafe {
            ffi::swe_refrac_extended(
                altitude,
                geoalt,
                atpress,
                attemp,
                lapse_rate,
                calc_flag,
                dret.as_mut_ptr(),
            )
        };
        if !converted.is_finite() || dret.iter().any(|value| !value.is_finite()) {
            return Err(Error::native(
                "refrac_extended: native calculation produced a non-finite converted altitude or detail",
            ));
        }
        Ok(Refraction {
            converted,
            true_altitude: dret[0],
            apparent_altitude: dret[1],
            refraction: dret[2],
            dip: dret[3],
        })
    })
}

/// Apply a lapse rate while native access is already held.
///
/// Caller must hold the native lock.
pub(crate) fn apply_lapse_rate_locked(lapse_rate: f64) {
    // SAFETY: value write to shared native configuration; the caller
    // holds the process-wide lock like every other `set_*` call.
    unsafe {
        ffi::swe_set_lapse_rate(lapse_rate);
    }
}

/// Override the process-global atmospheric lapse rate.
///
/// `lapse_rate` is dT/dh in K/m for the elevation-dependent native steps
/// (for example the horizon dip inside rise/set searches). There is no
/// native getter: the override state is write-only natively, so callers
/// track the active value themselves (pass `0.0065` to restore the
/// conventional standard-atmosphere value).
///
/// # Errors
///
/// Non-finite rates are rejected before any native call.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::set_lapse_rate;
///
/// set_lapse_rate(0.0065)?;
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn set_lapse_rate(lapse_rate: f64) -> Result<(), Error> {
    if !lapse_rate.is_finite() {
        return Err(Error::invalid_input(
            "set_lapse_rate: lapse rate must be finite",
        ));
    }
    with_native_access(|| {
        apply_lapse_rate_locked(lapse_rate);
        Ok(())
    })
}

/// Planetary phenomena: phase, elongation, apparent size and magnitude.
///
/// Component order and units are exactly the native `attr` layout:
///
/// 1. phase angle (Earth-planet-Sun) in degrees,
/// 2. illuminated fraction of the disc (0.0 = new, 1.0 = full),
/// 3. elongation from the Sun in degrees,
/// 4. apparent disc diameter in degrees,
/// 5. apparent visual magnitude,
///
/// with slots 5-19 reserved as 0.0. For the Sun the engine reports phase
/// 0, elongation 0 and an illuminated fraction of 0 (preserved verbatim,
/// not reinterpreted).
#[derive(Debug, Clone, PartialEq)]
pub struct Phenomena {
    /// The twenty native output components; see the type documentation.
    pub values: [f64; PHENO_LEN],
    /// Flag set returned by the native call (source bits, as in
    /// [`crate::Position::returned_flags`]).
    pub returned_flags: i32,
    /// Native diagnostic text (`serr`); empty on a clean computation.
    pub diagnostic: String,
}

impl Phenomena {
    /// Phase angle (Earth-planet-Sun) in degrees (component 0).
    pub fn phase_angle(&self) -> f64 {
        self.values[0]
    }

    /// Illuminated fraction of the disc, 0.0 to 1.0 (component 1).
    pub fn illuminated_fraction(&self) -> f64 {
        self.values[1]
    }

    /// Elongation of the planet from the Sun in degrees (component 2).
    pub fn elongation(&self) -> f64 {
        self.values[2]
    }

    /// Apparent diameter of the disc in degrees (component 3).
    pub fn apparent_diameter(&self) -> f64 {
        self.values[3]
    }

    /// Apparent visual magnitude (component 4).
    pub fn magnitude(&self) -> f64 {
        self.values[4]
    }
}

/// Compute planetary phenomena for an Ephemeris Time Julian Day.
///
/// `body` is a body number (for example [`crate::MARS`]); `flags`
/// combines the same source/geometry bits as [`crate::calc`]. There is
/// no body-or-star union here: fixed-star phenomena are not a native
/// operation.
///
/// # Configuration dependence
///
/// File-based sources require data files visible through
/// [`crate::set_ephe_path`]; the active configuration is captured under
/// the process-wide native lock together with the computation.
///
/// # Errors
///
/// Non-finite `jd_et` is rejected before any native call. A negative
/// native status (unknown body, missing elements) becomes [`Error`] with
/// the diagnostic kept; a non-negative status is `Ok` with details in
/// [`Phenomena::returned_flags`] and [`Phenomena::diagnostic`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, MARS, pheno};
///
/// // The Moshier model needs no data files, so this runs anywhere.
/// let ph = pheno(2451545.0, MARS, FLG_MOSEPH)?;
/// assert!((0.0..1.0).contains(&ph.illuminated_fraction()));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn pheno(jd_et: f64, body: i32, flags: i32) -> Result<Phenomena, Error> {
    if !jd_et.is_finite() {
        return Err(Error::invalid_input("pheno: Julian Day must be finite"));
    }
    with_native_access(|| {
        crate::domain::check_et_locked("pheno", jd_et, flags)?;
        let mut attr = [0.0; PHENO_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `attr` owns PHENO_LEN writable slots and `serr` owns
        // SERR_LEN writable bytes; both outlive this call under the
        // native lock. Results are only exposed after the native status
        // is checked.
        let ret =
            unsafe { ffi::swe_pheno(jd_et, body, flags, attr.as_mut_ptr(), serr.as_mut_ptr()) };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_physical_failure(
                "pheno",
                jd_et,
                body,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(Phenomena {
                values: attr,
                returned_flags: ret,
                diagnostic,
            })
        }
    })
}

/// Compute planetary phenomena for a Universal Time Julian Day.
///
/// Identical to [`pheno`] except that `jd_ut` is Universal Time.
/// Result layout and error behavior match [`pheno`] exactly.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, MARS, pheno_ut};
///
/// let ph = pheno_ut(2451545.0, MARS, FLG_MOSEPH)?;
/// assert!((0.0..180.0).contains(&ph.elongation()));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn pheno_ut(jd_ut: f64, body: i32, flags: i32) -> Result<Phenomena, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input("pheno_ut: Julian Day must be finite"));
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("pheno_ut", jd_ut, flags)?;
        let mut attr = [0.0; PHENO_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same contract as in [`pheno`].
        let ret =
            unsafe { ffi::swe_pheno_ut(jd_ut, body, flags, attr.as_mut_ptr(), serr.as_mut_ptr()) };
        // SAFETY: same contract as in [`pheno`].
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_physical_failure(
                "pheno_ut",
                jd_ut,
                body,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(Phenomena {
                values: attr,
                returned_flags: ret,
                diagnostic,
            })
        }
    })
}

/// Osculating Keplerian elements as computed by
/// [`get_orbital_elements`].
///
/// The engine writes slots 0-16; slots 17-49 are reserved and read as
/// 0.0 because the wrapper zero-fills them. Named slots:
///
/// 0. semi-major axis in AU,
/// 1. eccentricity,
/// 2. inclination in degrees,
/// 3. longitude of the ascending node in degrees,
/// 4. argument of perihelion in degrees,
/// 5. longitude of periapsis in degrees,
/// 6. mean anomaly at epoch in degrees,
/// 7. true anomaly at epoch in degrees,
/// 8. eccentric anomaly at epoch in degrees,
/// 9. mean longitude at epoch in degrees,
/// 10. sidereal orbital period in tropical years,
/// 11. mean daily motion in degrees per day,
/// 12. tropical period in years,
/// 13. synodic period in days (negative for inner planets and the Moon),
/// 14. time of perihelion passage as a Julian Day,
/// 15. perihelion distance in AU,
/// 16. aphelion distance in AU.
#[derive(Debug, Clone, PartialEq)]
pub struct OrbitalElements {
    /// The fifty compatibility-layout components; see the type
    /// documentation.
    pub values: [f64; ORBEL_LEN],
    /// Native diagnostic text (`serr`); empty on a clean computation.
    pub diagnostic: String,
}

impl OrbitalElements {
    /// Semi-major axis in AU (component 0).
    pub fn semi_major_axis_au(&self) -> f64 {
        self.values[0]
    }

    /// Eccentricity: 0 = circle, < 1 = ellipse (component 1).
    pub fn eccentricity(&self) -> f64 {
        self.values[1]
    }

    /// Inclination in degrees, relative to the ecliptic (component 2).
    pub fn inclination_deg(&self) -> f64 {
        self.values[2]
    }

    /// Longitude of the ascending node in degrees (component 3).
    pub fn ascending_node_deg(&self) -> f64 {
        self.values[3]
    }

    /// Argument of perihelion in degrees (component 4).
    pub fn argument_of_perihelion_deg(&self) -> f64 {
        self.values[4]
    }

    /// Longitude of periapsis in degrees (component 5).
    pub fn longitude_of_periapsis_deg(&self) -> f64 {
        self.values[5]
    }

    /// Mean anomaly at epoch in degrees (component 6).
    pub fn mean_anomaly_deg(&self) -> f64 {
        self.values[6]
    }

    /// True anomaly at epoch in degrees (component 7).
    pub fn true_anomaly_deg(&self) -> f64 {
        self.values[7]
    }

    /// Eccentric anomaly at epoch in degrees (component 8).
    pub fn eccentric_anomaly_deg(&self) -> f64 {
        self.values[8]
    }

    /// Mean longitude at epoch in degrees (component 9).
    pub fn mean_longitude_deg(&self) -> f64 {
        self.values[9]
    }

    /// Sidereal orbital period in tropical years (component 10).
    pub fn sidereal_period_years(&self) -> f64 {
        self.values[10]
    }

    /// Mean daily motion in degrees per day (component 11).
    pub fn mean_daily_motion_deg(&self) -> f64 {
        self.values[11]
    }

    /// Tropical period in years (component 12).
    pub fn tropical_period_years(&self) -> f64 {
        self.values[12]
    }

    /// Synodic period in days, negative for inner planets and the Moon
    /// (component 13).
    pub fn synodic_period_days(&self) -> f64 {
        self.values[13]
    }

    /// Time of perihelion passage as a Julian Day (component 14).
    pub fn perihelion_passage_jd(&self) -> f64 {
        self.values[14]
    }

    /// Perihelion distance in AU (component 15).
    pub fn perihelion_distance_au(&self) -> f64 {
        self.values[15]
    }

    /// Aphelion distance in AU (component 16).
    pub fn aphelion_distance_au(&self) -> f64 {
        self.values[16]
    }
}

/// Compute osculating Keplerian elements for an Ephemeris Time Julian
/// Day.
///
/// `body` is a body number (for example [`crate::MARS`]); `flags`
/// combines the same source/geometry bits as [`crate::calc`]. The
/// elements describe the ellipse the body would follow if all
/// perturbations ceased at that instant. There is no Universal Time
/// variant natively — convert the epoch with the time functions first
/// (an explicit capability gap, not a silent omission).
///
/// With `FLG_BARYCTR` the fit of trans-Jovian bodies is re-referenced to
/// the solar-system barycentre; with `FLG_ORBEL_AA` (which shares its bit
/// with `FLG_TOPOCTR`, meaningless here) the Astronomical Almanac central
/// mass is used. Both behaviors are engine-defined and preserved
/// verbatim.
///
/// # Configuration dependence
///
/// Same contract as [`pheno`]: data files via [`crate::set_ephe_path`],
/// configuration captured under the process-wide native lock.
///
/// # Errors
///
/// Non-finite `jd_et` is rejected before any native call. Bodies without
/// an element set (the Sun, lunar nodes/apsides, out-of-range numbers)
/// fail natively with the diagnostic kept (for example `"object 0 not
/// valid"`).
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, MARS, get_orbital_elements};
///
/// // The Moshier model needs no data files, so this runs anywhere.
/// let el = get_orbital_elements(2451545.0, MARS, FLG_MOSEPH)?;
/// assert!((el.semi_major_axis_au() - 1.524).abs() < 0.01);
/// assert!((el.eccentricity() - 0.093).abs() < 0.01);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn get_orbital_elements(jd_et: f64, body: i32, flags: i32) -> Result<OrbitalElements, Error> {
    if !jd_et.is_finite() {
        return Err(Error::invalid_input(
            "get_orbital_elements: Julian Day must be finite",
        ));
    }
    with_native_access(|| {
        crate::domain::check_et_locked("get_orbital_elements", jd_et, flags)?;
        let mut dret = [0.0; ORBEL_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `dret` owns ORBEL_LEN writable slots (more than the 17
        // the engine writes, established by probing) and `serr` owns
        // SERR_LEN writable bytes; both outlive this call under the
        // native lock. Results are only exposed after the native status
        // is checked.
        let ret = unsafe {
            ffi::swe_get_orbital_elements(jd_et, body, flags, dret.as_mut_ptr(), serr.as_mut_ptr())
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_physical_failure(
                "get_orbital_elements",
                jd_et,
                body,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(OrbitalElements {
                values: dret,
                diagnostic,
            })
        }
    })
}

/// Extreme and current geocentric distances as computed by
/// [`orbit_max_min_true_distance`].
///
/// The field order is maximum, minimum, current: it mirrors the native
/// out-pointer order `(max, min, true)` rather than an ascending sort.
#[derive(Debug, Clone, PartialEq)]
pub struct DistanceExtremes {
    /// Maximum true geocentric distance in AU over the synodic cycle.
    pub max_distance: f64,
    /// Minimum true geocentric distance in AU over the synodic cycle.
    pub min_distance: f64,
    /// Current true geocentric distance in AU at the requested epoch.
    pub true_distance: f64,
    /// Native diagnostic text (`serr`); empty on a clean computation.
    pub diagnostic: String,
}

/// Compute the maximum, minimum and current true geocentric distances
/// for an Ephemeris Time Julian Day.
///
/// For outer planets the minimum occurs around opposition and the
/// maximum around conjunction; for inner planets the minimum occurs near
/// inferior conjunction and the maximum near superior conjunction. `body`
/// and `flags` follow [`get_orbital_elements`].
///
/// # Configuration dependence
///
/// Same contract as [`pheno`].
///
/// # Errors
///
/// Non-finite `jd_et` is rejected before any native call; native failures
/// (bodies without an element set) become [`Error`] with the diagnostic
/// kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_MOSEPH, MARS, orbit_max_min_true_distance};
///
/// // The Moshier model needs no data files, so this runs anywhere.
/// let d = orbit_max_min_true_distance(2451545.0, MARS, FLG_MOSEPH)?;
/// assert!(d.min_distance < d.true_distance && d.true_distance < d.max_distance);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T and possible source-fallback
/// shifts under the computation lock; violations return `InvalidInput`.
/// Sidereal requests also validate the active UT-based user reference epoch
/// described by [`crate::set_sid_mode`].
pub fn orbit_max_min_true_distance(
    jd_et: f64,
    body: i32,
    flags: i32,
) -> Result<DistanceExtremes, Error> {
    if !jd_et.is_finite() {
        return Err(Error::invalid_input(
            "orbit_max_min_true_distance: Julian Day must be finite",
        ));
    }
    with_native_access(|| {
        crate::domain::check_et_locked("orbit_max_min_true_distance", jd_et, flags)?;
        let mut dmax = 0.0;
        let mut dmin = 0.0;
        let mut dtrue = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: each distance pointer addresses exactly one owned,
        // initialized local and `serr` owns SERR_LEN writable bytes; all
        // outlive this call under the native lock. Results are only
        // exposed after the native status is checked.
        let ret = unsafe {
            ffi::swe_orbit_max_min_true_distance(
                jd_et,
                body,
                flags,
                &mut dmax,
                &mut dmin,
                &mut dtrue,
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by SERR_LEN (see `read_native_text`).
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            Err(Error::native(describe_physical_failure(
                "orbit_max_min_true_distance",
                jd_et,
                body,
                flags,
                &diagnostic,
            )))
        } else {
            Ok(DistanceExtremes {
                max_distance: dmax,
                min_distance: dmin,
                true_distance: dtrue,
                diagnostic,
            })
        }
    })
}

/// Build the error message for a failed phenomena/orbital computation,
/// keeping the native diagnostic when the engine provided one.
fn describe_physical_failure(
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
