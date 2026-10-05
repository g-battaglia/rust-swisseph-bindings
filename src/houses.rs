//! House cusps, angles, speeds and related lookups.
//!
//! [`houses`] covers the standard twelve-cusp systems through the native
//! `swe_houses`; [`houses_ex`] adds ephemeris flags for systems that need
//! an internal solar position. Both return the same [`Houses`] layout: the
//! twelve cusps (houses 1-12, no index-0 padding) and the eight principal
//! angles.
//!
//! [`houses_ex2`] adds daily cusp/angle speeds ([`HousesWithSpeeds`]);
//! [`houses_armc`] and [`houses_armc_ex2`] compute from an explicit ARMC
//! instead of a Julian Day; [`houses_gauquelin`] covers the 36-sector
//! layout that does not fit the twelve-cusp shape ([`GauquelinSectors`]).
//! [`house_pos`] locates a point inside the houses (or sectors),
//! [`house_name`] resolves a system letter to its native name, and
//! [`gauquelin_sector`] computes a body's Gauquelin sector in time.

use std::ffi::{c_char, c_int};

use crate::error::Error;
use crate::ffi::{
    self, ASCMC_NATIVE_LEN, CUSPS_GAUQUELIN_LEN, CUSPS_NATIVE_LEN, SERR_LEN, TEXT_BUF_LEN,
    copy_returned_string, read_native_text,
};
use crate::state::with_native_access;

/// House cusps and principal angles for one time and place.
///
/// `cusps[i]` (0-based) is the ecliptic longitude in degrees of house
/// `i + 1`, i.e. the native `cusps[1..=12]` entries without the reserved
/// index-0 slot. `angles` holds the eight native `ascmc[0..8]` values:
///
/// 1. Ascendant,
/// 2. Medium Coeli (MC),
/// 3. ARMC (sidereal time in degrees),
/// 4. Vertex,
/// 5. Equatorial ascendant,
/// 6. Koch co-ascendant,
/// 7. Munkasey co-ascendant,
/// 8. Polar ascendant.
///
/// All values are ecliptic longitudes in degrees, except `angles[2]`
/// (ARMC, equatorial degrees of sidereal time).
#[derive(Debug, Clone, PartialEq)]
pub struct Houses {
    /// Twelve house-cusp longitudes in degrees (houses 1-12).
    pub cusps: [f64; 12],
    /// Eight principal angles; see the type documentation.
    pub angles: [f64; 8],
}

impl Houses {
    /// Ascendant longitude in degrees (`angles[0]`).
    pub fn ascendant(&self) -> f64 {
        self.angles[0]
    }

    /// Medium Coeli longitude in degrees (`angles[1]`).
    pub fn mc(&self) -> f64 {
        self.angles[1]
    }
}

/// House-system identifier for the Gauquelin sector layout.
///
/// The native engine writes 36 sector cusps for this system instead of
/// twelve house cusps, which does not fit [`Houses`]. Both wrappers reject
/// it (case-insensitively); use [`houses_gauquelin`] for its 36-sector layout.
const GAUGUELIN_SYSTEM: u8 = b'G';

/// Compute houses while native access is already held.
///
/// Caller must hold the native lock; inputs must already be validated.
pub(crate) fn houses_locked(
    jd_ut: f64,
    latitude: f64,
    longitude: f64,
    hsys: u8,
) -> Result<Houses, Error> {
    crate::domain::check_ut_locked("houses", jd_ut, -1)?;
    let mut cusps = [0.0; CUSPS_NATIVE_LEN];
    let mut ascmc = [0.0; ASCMC_NATIVE_LEN];
    // SAFETY: `cusps` owns CUSPS_NATIVE_LEN writable slots and `ascmc`
    // owns ASCMC_NATIVE_LEN writable slots; both outlive the call and the
    // caller holds the native lock. Results are only exposed after the
    // OK/ERR status is checked.
    let ret = unsafe {
        ffi::swe_houses(
            jd_ut,
            latitude,
            longitude,
            c_int::from(hsys),
            cusps.as_mut_ptr(),
            ascmc.as_mut_ptr(),
        )
    };
    if ret < 0 {
        return Err(Error::native(format!(
            "houses: native computation failed (jd_ut={jd_ut}, lat={latitude}, lon={longitude}, hsys={})",
            hsys as char,
        )));
    }
    Ok(pack_houses(&cusps, &ascmc))
}

/// Compute houses with flags while native access is already held.
///
/// Caller must hold the native lock; inputs must already be validated.
pub(crate) fn houses_ex_locked(
    jd_ut: f64,
    flags: i32,
    latitude: f64,
    longitude: f64,
    hsys: u8,
) -> Result<Houses, Error> {
    crate::domain::check_ut_locked("houses_ex", jd_ut, -1)?;
    if flags & crate::FLG_SIDEREAL != 0 {
        crate::config::check_sidereal_epoch_locked("houses_ex: reference epoch", -1)?;
    }
    let mut cusps = [0.0; CUSPS_NATIVE_LEN];
    let mut ascmc = [0.0; ASCMC_NATIVE_LEN];
    // SAFETY: same contract as in [`houses_locked`].
    let ret = unsafe {
        ffi::swe_houses_ex(
            jd_ut,
            flags,
            latitude,
            longitude,
            c_int::from(hsys),
            cusps.as_mut_ptr(),
            ascmc.as_mut_ptr(),
        )
    };
    if ret < 0 {
        return Err(Error::native(format!(
            "houses_ex: native computation failed (jd_ut={jd_ut}, flags={flags}, lat={latitude}, lon={longitude}, hsys={})",
            hsys as char,
        )));
    }
    Ok(pack_houses(&cusps, &ascmc))
}

/// Check the requested house system before any native call.
pub(crate) fn check_system(hsys: u8) -> Result<(), Error> {
    if hsys == GAUGUELIN_SYSTEM || hsys == b'g' {
        return Err(Error::invalid_input(
            "houses: the Gauquelin ('G') 36-sector layout does not fit the twelve-cusp result; use houses_gauquelin for this layout",
        ));
    }
    if !hsys.is_ascii_alphabetic() {
        return Err(Error::invalid_input(format!(
            "houses: house system must be an ASCII letter, got {hsys:#04X}"
        )));
    }
    Ok(())
}

/// Check house time/observer inputs before any native call.
pub(crate) fn check_houses_inputs(
    function: &str,
    jd_ut: f64,
    latitude: f64,
    longitude: f64,
) -> Result<(), Error> {
    if !jd_ut.is_finite() || !latitude.is_finite() || !longitude.is_finite() {
        return Err(Error::invalid_input(format!(
            "{function}: Julian Day, latitude and longitude must be finite"
        )));
    }
    Ok(())
}

/// Calculate house cusps and angles.
///
/// `jd_ut` is Universal Time; `latitude`/`longitude` are geographic
/// degrees (latitude positive north, longitude positive east); `hsys` is
/// the house-system letter as a byte (for example `b'P'` for Placidus,
/// `b'K'` for Koch, `b'E'` for Equal).
///
/// # Polar behavior
///
/// Near the poles some systems have no defined cusps; the native engine
/// reports `ERR` and the wrapper returns [`Error`] — there is no silent
/// fallback to another system inside the bindings.
///
/// # Errors
///
/// Non-finite time/observer inputs, non-letter system codes and the Gauquelin sector layout are rejected
/// before any native call (the 36-sector layout does not fit [`Houses`]);
/// native `ERR` results become [`Error`] since `swe_houses` provides no
/// diagnostic buffer.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::houses;
///
/// // London, Placidus, J2000.0.
/// let london = houses(2451545.0, 51.5, -0.12, b'P')?;
/// assert!((0.0..360.0).contains(&london.ascendant()));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T under the computation lock while
/// preserving inherited tidal selection. Sidereal requests also validate the
/// active UT-based user reference epoch; violations return `InvalidInput`.
pub fn houses(jd_ut: f64, latitude: f64, longitude: f64, hsys: u8) -> Result<Houses, Error> {
    check_system(hsys)?;
    check_houses_inputs("houses", jd_ut, latitude, longitude)?;
    with_native_access(|| houses_locked(jd_ut, latitude, longitude, hsys))
}

/// Calculate house cusps and angles with ephemeris flags.
///
/// Identical to [`houses`] except that `flags` (ephemeris source and
/// calculation bits) is forwarded to the native `swe_houses_ex` for house
/// systems that compute an internal solar position. `0` behaves like
/// [`houses`].
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T under the computation lock while
/// preserving inherited tidal selection. Sidereal requests also validate the
/// active UT-based user reference epoch; violations return `InvalidInput`.
pub fn houses_ex(
    jd_ut: f64,
    flags: i32,
    latitude: f64,
    longitude: f64,
    hsys: u8,
) -> Result<Houses, Error> {
    check_system(hsys)?;
    check_houses_inputs("houses_ex", jd_ut, latitude, longitude)?;
    with_native_access(|| houses_ex_locked(jd_ut, flags, latitude, longitude, hsys))
}

/// Map the native `cusps[13]` / `ascmc[10]` buffers onto [`Houses`]:
/// cusps 1-12 become `cusps[0..12]` and the first eight angles are kept.
fn pack_houses(cusps: &[f64; CUSPS_NATIVE_LEN], ascmc: &[f64; ASCMC_NATIVE_LEN]) -> Houses {
    let mut packed_cusps = [0.0; 12];
    packed_cusps.copy_from_slice(&cusps[1..13]);
    let mut packed_angles = [0.0; 8];
    packed_angles.copy_from_slice(&ascmc[0..8]);
    Houses {
        cusps: packed_cusps,
        angles: packed_angles,
    }
}

/// House cusps, angles and their daily speeds for one time and place.
///
/// The position fields match [`Houses`]; the speed fields are the native
/// `cusp_speed[1..=12]` / `ascmc_speed[0..8]` daily rates in degrees per
/// day. Speeds are always computed by the engine for this call shape —
/// they do not depend on `FLG_SPEED`.
#[derive(Debug, Clone, PartialEq)]
pub struct HousesWithSpeeds {
    /// Twelve house-cusp longitudes in degrees (houses 1-12).
    pub cusps: [f64; 12],
    /// Eight principal angles; see [`Houses`] for the layout.
    pub angles: [f64; 8],
    /// Daily speeds of the twelve cusps in degrees per day.
    pub cusp_speeds: [f64; 12],
    /// Daily speeds of the eight angles in degrees per day.
    pub angle_speeds: [f64; 8],
}

impl HousesWithSpeeds {
    /// Ascendant longitude in degrees (`angles[0]`).
    pub fn ascendant(&self) -> f64 {
        self.angles[0]
    }

    /// Medium Coeli longitude in degrees (`angles[1]`).
    pub fn mc(&self) -> f64 {
        self.angles[1]
    }
}

/// Gauquelin sectors with angles and daily speeds.
///
/// The native 36-sector layout (`'G'`) writes 36 sector cusps instead of
/// twelve house cusps, so it needs its own shape: `sectors[i]` (0-based)
/// is the ecliptic longitude in degrees of sector `i + 1` (native
/// `cusp[1..=36]` without the reserved index-0 slot), with matching
/// daily speeds. `angles`/`angle_speeds` are the same eight principal
/// angles and rates as in [`HousesWithSpeeds`].
#[derive(Debug, Clone, PartialEq)]
pub struct GauquelinSectors {
    /// Thirty-six sector-cusp longitudes in degrees (sectors 1-36).
    pub sectors: [f64; 36],
    /// Eight principal angles; see [`Houses`] for the layout.
    pub angles: [f64; 8],
    /// Daily speeds of the thirty-six sectors in degrees per day.
    pub sector_speeds: [f64; 36],
    /// Daily speeds of the eight angles in degrees per day.
    pub angle_speeds: [f64; 8],
}

/// Position of one point inside the houses (or Gauquelin sectors).
///
/// `value` is the fractional house number: the integer part is the
/// 1-based house (1-12, plus the fraction inside it), or the 1-based
/// sector (1-36) for the Gauquelin system. Exactly `0.0` is the native
/// Koch circumpolar failure sentinel, never a valid house: inspect
/// [`HousePosition::diagnostic`] alongside it.
#[derive(Debug, Clone, PartialEq)]
pub struct HousePosition {
    /// Fractional house (1-12) or sector (1-36) number; `0.0` is the
    /// native Koch circumpolar failure sentinel.
    pub value: f64,
    /// Native diagnostic text (`serr`).
    ///
    /// Empty on a clean computation; carries notes such as the Ludwig
    /// circumpolar procedure, the simplified-algorithm fallback for
    /// systems without a dedicated branch, or the Koch failure reason.
    /// It is informational and preserved verbatim.
    pub diagnostic: String,
}

/// Gauquelin sector position of one body at one time.
///
/// `value` is the fractional sector number (1-36 plus the fraction
/// inside the sector); sectors run clockwise from the Ascendant (1 =
/// rising, 10 = upper culmination, 19 = setting, 28 = lower
/// culmination).
#[derive(Debug, Clone, PartialEq)]
pub struct GauquelinSector {
    /// Fractional Gauquelin sector number in [1, 37).
    pub value: f64,
    /// Native diagnostic text (`serr`); empty on a clean computation.
    pub diagnostic: String,
}

/// Calculate house cusps, angles and their daily speeds.
///
/// Identical to [`houses_ex`] except that the native `swe_houses_ex2`
/// additionally reports cusp and angle speeds, returned in
/// [`HousesWithSpeeds`]. Speeds are always computed for this call shape;
/// no speed flag is required.
///
/// # Errors
///
/// The Gauquelin sector layout (`'G'`/`'g'`) is rejected before any
/// native call — its 36 sectors do not fit [`HousesWithSpeeds`]; use
/// [`houses_gauquelin`]. Non-letter system codes are rejected likewise.
/// Native `ERR` results become [`Error`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::houses_ex2;
///
/// // London, Placidus, J2000.0.
/// let london = houses_ex2(2451545.0, 0, 51.5, -0.12, b'P')?;
/// assert!((0.0..360.0).contains(&london.ascendant()));
/// assert!(london.cusp_speeds.iter().all(|s| s.is_finite()));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T under the computation lock while
/// preserving inherited tidal selection. Sidereal requests also validate the
/// active UT-based user reference epoch; violations return `InvalidInput`.
pub fn houses_ex2(
    jd_ut: f64,
    flags: i32,
    latitude: f64,
    longitude: f64,
    hsys: u8,
) -> Result<HousesWithSpeeds, Error> {
    check_system(hsys)?;
    check_houses_inputs("houses_ex2", jd_ut, latitude, longitude)?;
    with_native_access(|| {
        crate::domain::check_ut_locked("houses_ex2", jd_ut, -1)?;
        if flags & crate::FLG_SIDEREAL != 0 {
            crate::config::check_sidereal_epoch_locked("houses_ex2: reference epoch", -1)?;
        }
        let mut cusps = [0.0; CUSPS_NATIVE_LEN];
        let mut ascmc = [0.0; ASCMC_NATIVE_LEN];
        let mut cusp_speed = [0.0; CUSPS_NATIVE_LEN];
        let mut ascmc_speed = [0.0; ASCMC_NATIVE_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: each buffer owns exactly its native width (13/10/13/10
        // doubles, 256 diagnostic bytes), all outlive the call, and the
        // lock excludes every other native access. Results are only
        // exposed after the OK/ERR status is checked.
        let ret = unsafe {
            ffi::swe_houses_ex2(
                jd_ut,
                flags,
                latitude,
                longitude,
                c_int::from(hsys),
                cusps.as_mut_ptr(),
                ascmc.as_mut_ptr(),
                cusp_speed.as_mut_ptr(),
                ascmc_speed.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        if ret < 0 {
            // SAFETY: `serr` is our own fully initialized buffer; the read
            // is bounded by its capacity.
            let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
            return Err(Error::native(format!(
                "houses_ex2: native computation failed (jd_ut={jd_ut}, flags={flags}, hsys={}): {diagnostic}",
                hsys as char,
            )));
        }
        Ok(pack_houses_with_speeds(
            &cusps,
            &ascmc,
            &cusp_speed,
            &ascmc_speed,
        ))
    })
}

/// Calculate the Gauquelin 36-sector layout with angles and speeds.
///
/// This is the sector counterpart of [`houses_ex2`]: it calls the same
/// native entry point with the `'G'` system and returns the 36 sector
/// cusps (native `cusp[1..=36]`) in [`GauquelinSectors`]. Twelve-cusp
/// systems must use [`houses_ex2`] instead; there is no silent
/// truncation of sectors to houses.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::houses_gauquelin;
///
/// // Paris, J2000.0: 36 finite sector cusps.
/// let paris = houses_gauquelin(2451545.0, 0, 48.85, 2.35)?;
/// assert_eq!(paris.sectors.len(), 36);
/// assert!(paris.sectors.iter().all(|s| (0.0..360.0).contains(s)));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T under the computation lock while
/// preserving inherited tidal selection. Sidereal requests also validate the
/// active UT-based user reference epoch; violations return `InvalidInput`.
pub fn houses_gauquelin(
    jd_ut: f64,
    flags: i32,
    latitude: f64,
    longitude: f64,
) -> Result<GauquelinSectors, Error> {
    check_houses_inputs("houses_gauquelin", jd_ut, latitude, longitude)?;
    with_native_access(|| {
        crate::domain::check_ut_locked("houses_gauquelin", jd_ut, -1)?;
        if flags & crate::FLG_SIDEREAL != 0 {
            crate::config::check_sidereal_epoch_locked("houses_gauquelin: reference epoch", -1)?;
        }
        let mut cusps = [0.0; CUSPS_GAUQUELIN_LEN];
        let mut ascmc = [0.0; ASCMC_NATIVE_LEN];
        let mut cusp_speed = [0.0; CUSPS_GAUQUELIN_LEN];
        let mut ascmc_speed = [0.0; ASCMC_NATIVE_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: sector buffers own the full 37-slot native width so the
        // engine can never write past them; otherwise the same contract
        // as in [`houses_ex2`].
        let ret = unsafe {
            ffi::swe_houses_ex2(
                jd_ut,
                flags,
                latitude,
                longitude,
                c_int::from(b'G'),
                cusps.as_mut_ptr(),
                ascmc.as_mut_ptr(),
                cusp_speed.as_mut_ptr(),
                ascmc_speed.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        if ret < 0 {
            // SAFETY: same bounded read as in [`houses_ex2`].
            let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
            return Err(Error::native(format!(
                "houses_gauquelin: native computation failed (jd_ut={jd_ut}, flags={flags}): {diagnostic}"
            )));
        }
        let mut sectors = [0.0; 36];
        sectors.copy_from_slice(&cusps[1..37]);
        let mut sector_speeds = [0.0; 36];
        sector_speeds.copy_from_slice(&cusp_speed[1..37]);
        let mut angles = [0.0; 8];
        angles.copy_from_slice(&ascmc[0..8]);
        let mut angle_speeds = [0.0; 8];
        angle_speeds.copy_from_slice(&ascmc_speed[0..8]);
        Ok(GauquelinSectors {
            sectors,
            angles,
            sector_speeds,
            angle_speeds,
        })
    })
}

/// Map twelve-house native buffers (positions plus speeds) onto
/// [`HousesWithSpeeds`].
fn pack_houses_with_speeds(
    cusps: &[f64; CUSPS_NATIVE_LEN],
    ascmc: &[f64; ASCMC_NATIVE_LEN],
    cusp_speed: &[f64; CUSPS_NATIVE_LEN],
    ascmc_speed: &[f64; ASCMC_NATIVE_LEN],
) -> HousesWithSpeeds {
    let mut packed_cusps = [0.0; 12];
    packed_cusps.copy_from_slice(&cusps[1..13]);
    let mut packed_angles = [0.0; 8];
    packed_angles.copy_from_slice(&ascmc[0..8]);
    let mut packed_cusp_speeds = [0.0; 12];
    packed_cusp_speeds.copy_from_slice(&cusp_speed[1..13]);
    let mut packed_angle_speeds = [0.0; 8];
    packed_angle_speeds.copy_from_slice(&ascmc_speed[0..8]);
    HousesWithSpeeds {
        cusps: packed_cusps,
        angles: packed_angles,
        cusp_speeds: packed_cusp_speeds,
        angle_speeds: packed_angle_speeds,
    }
}

/// Check ARMC-variant inputs before any native call.
fn check_armc_inputs(armc: f64, latitude: f64, eps: f64, hsys: u8) -> Result<(), Error> {
    check_system(hsys)?;
    if !armc.is_finite() || !latitude.is_finite() || !eps.is_finite() {
        return Err(Error::invalid_input(
            "houses_armc: armc, latitude and eps must be finite",
        ));
    }
    Ok(())
}

/// Calculate house cusps and angles from an explicit ARMC.
///
/// `armc` is the right ascension of the MC in degrees (normalized
/// natively), `latitude` the geographic latitude in degrees, and `eps`
/// the true obliquity of the ecliptic in degrees. Unlike [`houses`],
/// no Julian Day is involved, so sidereal (`FLG_SIDEREAL`) corrections
/// that need a date cannot apply here.
///
/// The Sunshine system (`'I'`/`'i'`) reads its solar declination from
/// the native `ascmc[9]` slot, which this wrapper leaves at `0.0`; use
/// [`houses_armc_ex2`] to supply the Sun's declination explicitly.
///
/// # Errors
///
/// Same system-code policy as [`houses`]: the Gauquelin layout and
/// non-letter codes are rejected before any native call. Native `ERR`
/// (for example polar failures) becomes [`Error`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::houses_armc;
///
/// // ARMC is echoed back in angles[2], normalized to 0-360.
/// let houses = houses_armc(280.0, 51.5, 23.44, b'P')?;
/// assert!((houses.angles[2] - 280.0).abs() < 1e-9);
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn houses_armc(armc: f64, latitude: f64, eps: f64, hsys: u8) -> Result<Houses, Error> {
    check_armc_inputs(armc, latitude, eps, hsys)?;
    with_native_access(|| {
        let mut cusps = [0.0; CUSPS_NATIVE_LEN];
        let mut ascmc = [0.0; ASCMC_NATIVE_LEN];
        // SAFETY: same buffer contract as in [`houses`]; the zeroed
        // `ascmc[9]` slot is the documented 0° Sunshine declination.
        let ret = unsafe {
            ffi::swe_houses_armc(
                armc,
                latitude,
                eps,
                c_int::from(hsys),
                cusps.as_mut_ptr(),
                ascmc.as_mut_ptr(),
            )
        };
        if ret < 0 {
            return Err(Error::native(format!(
                "houses_armc: native computation failed (armc={armc}, lat={latitude}, eps={eps}, hsys={})",
                hsys as char,
            )));
        }
        Ok(pack_houses(&cusps, &ascmc))
    })
}

/// Calculate house cusps, angles and daily speeds from an explicit ARMC.
///
/// Same computation as [`houses_armc`] through the native
/// `swe_houses_armc_ex2`, plus cusp/angle speeds
/// ([`HousesWithSpeeds`]) and a diagnostic buffer. `sun_declination`
/// (degrees) is forwarded as the solar-declination input the Sunshine
/// system (`'I'`/`'i'`) requires; every other system ignores it, in
/// which case only finiteness is required.
///
/// # Errors
///
/// Same system-code policy as [`houses_armc`]. Native `ERR` results
/// become [`Error`] with the native diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::houses_armc_ex2;
///
/// let houses = houses_armc_ex2(280.0, 51.5, 23.44, b'P', 0.0)?;
/// assert!(houses.cusp_speeds.iter().all(|s| s.is_finite()));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn houses_armc_ex2(
    armc: f64,
    latitude: f64,
    eps: f64,
    hsys: u8,
    sun_declination: f64,
) -> Result<HousesWithSpeeds, Error> {
    check_armc_inputs(armc, latitude, eps, hsys)?;
    if !sun_declination.is_finite() {
        return Err(Error::invalid_input(
            "houses_armc_ex2: sun_declination must be finite",
        ));
    }
    with_native_access(|| {
        let mut cusps = [0.0; CUSPS_NATIVE_LEN];
        let mut ascmc = [0.0; ASCMC_NATIVE_LEN];
        ascmc[9] = sun_declination;
        let mut cusp_speed = [0.0; CUSPS_NATIVE_LEN];
        let mut ascmc_speed = [0.0; ASCMC_NATIVE_LEN];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same buffer contract as in [`houses_ex2`]; writing
        // `ascmc[9]` before the call is the native Sunshine input
        // channel, and the engine rewrites the slot on exit.
        let ret = unsafe {
            ffi::swe_houses_armc_ex2(
                armc,
                latitude,
                eps,
                c_int::from(hsys),
                cusps.as_mut_ptr(),
                ascmc.as_mut_ptr(),
                cusp_speed.as_mut_ptr(),
                ascmc_speed.as_mut_ptr(),
                serr.as_mut_ptr(),
            )
        };
        if ret < 0 {
            // SAFETY: same bounded read as in [`houses_ex2`].
            let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
            return Err(Error::native(format!(
                "houses_armc_ex2: native computation failed (armc={armc}, hsys={}): {diagnostic}",
                hsys as char,
            )));
        }
        Ok(pack_houses_with_speeds(
            &cusps,
            &ascmc,
            &cusp_speed,
            &ascmc_speed,
        ))
    })
}

/// Locate an ecliptic point inside the houses (or Gauquelin sectors).
///
/// `armc` is the right ascension of the MC in degrees, `latitude` the
/// geographic latitude in degrees, and `eps` the true obliquity of the
/// ecliptic in degrees; `hsys` selects the system. `longitude` /
/// `latitude_body` are the point's ecliptic longitude/latitude in
/// degrees (tropical input, even for sidereal work). The result is a
/// fractional house number — integer part 1-12 plus the fraction inside
/// that house — or, for `'G'`/`'g'`, a fractional sector 1-36.
///
/// # System notes
///
/// - The engine folds letter case itself, so `'g'` behaves as `'G'`.
/// - Systems without a dedicated branch use a documented simplified
///   algorithm (reported in the diagnostic).
/// - Sunshine (`'I'`) positions need the Sun's declination, which this
///   call shape cannot receive: the engine reuses the declination saved
///   by an earlier native houses call, or `0.0` when none was saved.
/// - A Koch (`'K'`) circumpolar failure returns exactly `0.0` with the
///   reason in the diagnostic; that sentinel is preserved as data, never
///   raised as a Rust error, because the native call has no error return.
///
/// # Errors
///
/// Non-finite coordinates are rejected before any native call.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::house_pos;
///
/// // The Ascendant sits exactly on the first-house cusp: house 1.
/// let asc = house_pos(280.0, 51.5, 23.44, b'P', 100.0, 0.0)?;
/// assert!((1.0..13.0).contains(&asc.value));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn house_pos(
    armc: f64,
    latitude: f64,
    eps: f64,
    hsys: u8,
    longitude: f64,
    latitude_body: f64,
) -> Result<HousePosition, Error> {
    if !armc.is_finite()
        || !latitude.is_finite()
        || !eps.is_finite()
        || !longitude.is_finite()
        || !latitude_body.is_finite()
    {
        return Err(Error::invalid_input(
            "house_pos: all coordinates must be finite",
        ));
    }
    with_native_access(|| {
        let xpin = [longitude, latitude_body];
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `xpin` owns two readable doubles, `serr` owns SERR_LEN
        // writable bytes; both outlive the call under the native lock.
        // The return value has no error encoding (see the failure
        // sentinel documented on [`HousePosition`]).
        let value = unsafe {
            ffi::swe_house_pos(
                armc,
                latitude,
                eps,
                c_int::from(hsys),
                xpin.as_ptr(),
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by its capacity. Copied before the lock is released.
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        Ok(HousePosition { value, diagnostic })
    })
}

/// Short native name of a house system.
///
/// `hsys` is the house-system letter as a byte (for example `b'P'`
/// yields `"Placidus"` and `b'G'` yields `"Gauquelin sectors"`).
/// Selectors without a dedicated name fall through to `"Placidus"`
/// inside the native table; that mapping is preserved verbatim and
/// documented here rather than second-guessed.
///
/// This is a pure table lookup over native static storage: it takes no
/// lock-tracked state, and a poisoned lock degrades to the same lookup
/// instead of failing.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::house_name;
///
/// assert_eq!(house_name(b'P'), "Placidus");
/// assert_eq!(house_name(b'G'), "Gauquelin sectors");
/// ```
pub fn house_name(hsys: u8) -> String {
    with_native_access(|| {
        // SAFETY: pure static-table lookup, no shared state touched; the
        // result is copied into owned storage before it can be observed.
        Ok(unsafe { copy_returned_string(ffi::swe_house_name(c_int::from(hsys)), b"") })
    })
    .unwrap_or_else(|_| {
        // SAFETY: same pure call; reachable only when the lock is
        // poisoned, in which case no native state is at risk.
        unsafe { copy_returned_string(ffi::swe_house_name(c_int::from(hsys)), b"") }
    })
}

/// Check a Gauquelin-sector request before any native call.
fn check_gauquelin_inputs(
    jd_ut: f64,
    star_name: Option<&str>,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atpress: f64,
    attemp: f64,
) -> Result<(), Error> {
    crate::domain::check_observer_height("gauquelin_sector", altitude)?;
    if !jd_ut.is_finite()
        || !longitude.is_finite()
        || !latitude.is_finite()
        || !altitude.is_finite()
        || !atpress.is_finite()
        || !attemp.is_finite()
    {
        return Err(Error::invalid_input(
            "gauquelin_sector: time, observer and atmosphere inputs must be finite",
        ));
    }
    if let Some(name) = star_name
        && (name.as_bytes().contains(&0) || name.len() >= TEXT_BUF_LEN)
    {
        return Err(Error::invalid_input(
            "gauquelin_sector: star name must not contain NUL and must fit the 256-byte native buffer",
        ));
    }
    Ok(())
}

/// Gauquelin sector position of a body at one time and place.
///
/// `body` is a body number (for example [`crate::MARS`]); pass a star
/// via `star_name` (`Some` selects the fixed-star path, `None` the body
/// path — an empty name behaves as `None` natively). `flags` combines
/// the ephemeris source bits; `method` selects the computation: 0 with
/// latitude, 1 without latitude, 2/3 from rise/set of the disc center
/// (3 with refraction), 4/5 from rise/set of the disc edge (5 with
/// refraction). `longitude`/`latitude`/`altitude` locate the observer
/// (east-positive degrees, degrees north, meters); `atpress` (mbar) and
/// `attemp` (°C) feed the rise/set methods, where `0.0` selects the
/// native defaults.
///
/// Methods 2-5 search rise/set events internally and can fail for
/// circumpolar bodies; out-of-range methods fail natively. Both are
/// preserved as [`Error`] with the engine diagnostic.
///
/// # Errors
///
/// Non-finite time/observer/atmosphere inputs and overlong or
/// NUL-containing star names are rejected before any native call.
/// Native `ERR` results become [`Error`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{MARS, gauquelin_sector};
///
/// // Paris, J2000.0, geometric method.
/// let sector = gauquelin_sector(2451545.0, MARS, None, 0, 0, 2.35, 48.85, 0.0, 0.0, 0.0)?;
/// assert!((1.0..37.0).contains(&sector.value));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires finite input and effective dates with
/// calendar years representable as `i32` in both native calendars. UT
/// conversions check configured Delta-T under the computation lock while
/// preserving inherited tidal selection. Sidereal requests also validate the
/// active UT-based user reference epoch; violations return `InvalidInput`.
///
/// Observer height must satisfy the arithmetic ceiling documented by
/// [`crate::set_topo`]; excessive height returns `InvalidInput` before
/// observer configuration or computation. Native geographic limits and
/// data errors are still reported by the engine.
#[allow(clippy::too_many_arguments)]
pub fn gauquelin_sector(
    jd_ut: f64,
    body: i32,
    star_name: Option<&str>,
    flags: i32,
    method: i32,
    longitude: f64,
    latitude: f64,
    altitude: f64,
    atpress: f64,
    attemp: f64,
) -> Result<GauquelinSector, Error> {
    check_gauquelin_inputs(
        jd_ut, star_name, longitude, latitude, altitude, atpress, attemp,
    )?;
    // Caller-owned star buffer: the native call may read the name while
    // resolving a fixed star, and every access stays inside this buffer.
    let mut star_buffer = [0 as c_char; TEXT_BUF_LEN];
    if let Some(name) = star_name {
        for (slot, byte) in star_buffer.iter_mut().zip(name.bytes()) {
            *slot = byte as c_char;
        }
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("gauquelin_sector", jd_ut, flags)?;
        let geopos = [longitude, latitude, altitude];
        let mut sector = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `star_buffer` owns TEXT_BUF_LEN bytes holding a
        // NUL-terminated name (or all zeros for the body path),
        // `geopos` owns three readable doubles, `sector` one writable
        // slot and `serr` SERR_LEN writable bytes; all outlive the call
        // under the native lock. Results are only exposed after the
        // OK/ERR status is checked.
        let ret = unsafe {
            ffi::swe_gauquelin_sector(
                jd_ut,
                body,
                star_buffer.as_mut_ptr(),
                flags,
                method,
                geopos.as_ptr(),
                atpress,
                attemp,
                &mut sector,
                serr.as_mut_ptr(),
            )
        };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by its capacity. Copied before the lock is released.
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 {
            return Err(Error::native(format!(
                "gauquelin_sector: native computation failed (jd_ut={jd_ut}, body={body}, method={method}): {diagnostic}"
            )));
        }
        Ok(GauquelinSector {
            value: sector,
            diagnostic,
        })
    })
}
