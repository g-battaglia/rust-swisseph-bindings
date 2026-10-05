//! Native configuration: data path, observer, sidereal mode.
//!
//! All settings are process-global inside the native library, so every
//! function here serializes with computation through the single native
//! lock from [`crate::state`]. There is deliberately no Rust handle whose
//! `Drop` would reset shared state: [`close`] is the only reset and it is
//! explicit and process-wide.

use std::ffi::{CString, c_char, c_double};
use std::sync::Mutex;

use crate::error::Error;
use crate::ffi::{self, SERR_LEN, read_native_text};
use crate::state::with_native_access;

/// Native `SE_SIDBIT_USER_UT`, verified in the pinned public header.
const USER_UT_BIT: i32 = 1024;

/// Binding-owned validation metadata for write-only native settings.
/// Access order is native lock, then this mutex. No raw-C caller outside
/// the binding's synchronization contract is supported by this metadata.
#[derive(Default)]
struct ValidationSettings {
    /// A binding sidereal setter has initialized the native defaults.
    sidereal_initialized: bool,
    /// UT reference epoch of the active user sidereal mode, if any.
    sidereal_ut_epoch: Option<f64>,
    /// Native Delta-T override; None represents automatic computation.
    delta_t: Option<f64>,
}

/// Fresh native defaults; explicit close resets both settings (ABI probe).
static VALIDATION_SETTINGS: Mutex<ValidationSettings> = Mutex::new(ValidationSettings {
    sidereal_initialized: false,
    sidereal_ut_epoch: None,
    delta_t: None,
});

/// Initialize native defaults before writes that native first use would
/// otherwise erase. An explicit binding sidereal mode always marks this
/// initialized; before that, reapplying its Fagan default is equivalent.
/// Caller holds native access; never expose configuration metadata as a
/// substitute for native calculation or an unavailable public getter.
fn initialize_sidereal_locked() -> Result<(), Error> {
    let mut settings = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?;
    if !settings.sidereal_initialized {
        // SAFETY: scalar default-mode setter under native access, before
        // any binding sidereal setter. Initializes native defaults without
        // replacing a user-selected mode; raw-C configuration is outside
        // the binding's supported shared-state contract.
        unsafe { ffi::swe_set_sid_mode(crate::SIDM_FAGAN_BRADLEY, 0.0, 0.0) };
        settings.sidereal_initialized = true;
    }
    Ok(())
}

/// Check the effective reference epoch, with native access already held.
pub(crate) fn check_sidereal_epoch_locked(function: &str, flags: i32) -> Result<(), Error> {
    let epoch = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?
        .sidereal_ut_epoch;
    if let Some(epoch) = epoch {
        crate::domain::check_ut_date_locked(function, epoch, flags)?;
    }
    Ok(())
}

/// Restore a temporary builder-only Delta-T change before native access
/// is released, including validation errors and Rust unwinding.
struct RestoreDeltaT(f64);

impl Drop for RestoreDeltaT {
    fn drop(&mut self) {
        // SAFETY: constructed only inside native access; finite override
        // or automatic sentinel. The setter copies a scalar, no pointers.
        unsafe { ffi::swe_set_delta_t_userdef(self.0) };
    }
}

/// Validate a proposed user epoch against an explicit or inherited shift.
/// Builder model/source changes are checked after applying its complete
/// configuration at computation time; known pinned overrides are checked
/// immediately. Caller holds native access; no lasting setter side effect.
pub(crate) fn check_sidereal_request_locked(
    function: &str,
    mode: i32,
    t0: f64,
    delta_t: Option<Option<f64>>,
    model_changes: bool,
) -> Result<(), Error> {
    if mode & 255 != crate::SIDM_USER {
        return Ok(());
    }
    crate::domain::check_et(function, t0)?;
    if mode & USER_UT_BIT == 0 {
        return Ok(());
    }
    if let Some(Some(dt)) = delta_t
        && dt != crate::SE_DELTAT_AUTOMATIC
    {
        return crate::domain::check_et(function, t0 + dt);
    }
    let saved = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?
        .delta_t;
    if delta_t.is_none()
        && let Some(dt) = saved
    {
        return crate::domain::check_et(function, t0 + dt);
    }
    if model_changes {
        return Ok(());
    }
    initialize_sidereal_locked()?;
    if delta_t.is_some() {
        let _restore = RestoreDeltaT(saved.unwrap_or(crate::SE_DELTAT_AUTOMATIC));
        // SAFETY: caller holds native access; clear only for this native
        // Delta-T preflight, then RestoreDeltaT restores the exact override.
        unsafe { ffi::swe_set_delta_t_userdef(crate::SE_DELTAT_AUTOMATIC) };
        crate::domain::check_ut_date_locked(function, t0, -1)
    } else {
        crate::domain::check_ut_date_locked(function, t0, -1)
    }
}

/// Maximum ephemeris-path length accepted by the safe wrapper, in bytes.
///
/// The native setter silently substitutes its compiled default when the
/// given path exceeds `AS_MAXCH - 1 - 13` (256 - 14 = 242) bytes. The
/// wrapper rejects such input with [`Error`] instead of configuring a
/// path the caller did not ask for.
pub const MAX_EPHE_PATH_LEN: usize = 242;

/// Apply an ephemeris path while native access is already held.
///
/// # Safety contract
///
/// The caller must hold the process-wide native lock (see
/// [`crate::state`): this function performs the raw setter without
/// acquiring it. `path` follows the same convention as [`set_ephe_path`]
/// (`None` = null = compiled default).
pub(crate) fn apply_ephe_path_locked(path: Option<&CString>) {
    let pointer = path.map_or(std::ptr::null(), |owned| owned.as_ptr());
    // SAFETY: `pointer` is either null or addresses a live `CString`
    // kept alive by the caller for the whole call. The native setter
    // copies the path into its own storage, so no reference outlives this
    // scope. The caller holds the native lock because the setter also
    // closes cached file data.
    unsafe {
        ffi::swe_set_ephe_path(pointer);
    }
}

/// Apply an observer position while native access is already held.
///
/// Caller must hold the native lock; pure value write serialized with
/// later computations.
pub(crate) fn apply_topo_locked(longitude: f64, latitude: f64, altitude: f64) {
    // SAFETY: pure value call; the caller holds the native lock because
    // this mutates shared configuration read by later computations.
    unsafe {
        ffi::swe_set_topo(
            longitude as c_double,
            latitude as c_double,
            altitude as c_double,
        );
    }
}

/// Apply a sidereal mode while native access is already held.
///
/// Caller must hold the native lock; same reason as [`apply_topo_locked`].
pub(crate) fn apply_sid_mode_locked(mode: i32, t0: f64, ayan_t0: f64) -> Result<(), Error> {
    let mut settings = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?;
    // SAFETY: pure value call; serialized for the same reason as
    // [`apply_topo_locked`].
    unsafe {
        ffi::swe_set_sid_mode(mode, t0 as c_double, ayan_t0 as c_double);
    }
    settings.sidereal_ut_epoch =
        (mode & 255 == crate::SIDM_USER && mode & USER_UT_BIT != 0).then_some(t0);
    settings.sidereal_initialized = true;
    Ok(())
}

/// Apply a JPL file selection while native access is already held.
///
/// Caller must hold the native lock; the setter copies the name and
/// closes cached file data.
pub(crate) fn apply_jpl_file_locked(native_name: &CString) {
    // SAFETY: `native_name` is a live `CString`, hence non-null and
    // NUL-terminated as the native setter requires. The setter copies the
    // name, so no reference outlives this scope.
    unsafe {
        ffi::swe_set_jpl_file(native_name.as_ptr());
    }
}

/// Apply a tidal acceleration while native access is already held.
///
/// Caller must hold the native lock.
pub(crate) fn apply_tid_acc_locked(acc: f64) -> Result<(), Error> {
    initialize_sidereal_locked()?;
    // SAFETY: pure value call; serialized because it mutates shared
    // native configuration read by later Delta-T computations.
    unsafe {
        ffi::swe_set_tid_acc(acc as c_double);
    }
    Ok(())
}

/// Apply a Delta-T override while native access is already held.
///
/// `native_dt` is the raw value passed to the engine
/// (`SE_DELTAT_AUTOMATIC` clears the override). Caller must hold the
/// native lock.
pub(crate) fn apply_delta_t_locked(native_dt: f64) -> Result<(), Error> {
    initialize_sidereal_locked()?;
    let mut settings = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?;
    // SAFETY: pure value call; serialized like [`apply_tid_acc_locked`].
    unsafe {
        ffi::swe_set_delta_t_userdef(native_dt as c_double);
    }
    settings.delta_t = (native_dt != crate::SE_DELTAT_AUTOMATIC).then_some(native_dt);
    Ok(())
}

/// Select the directory searched for ephemeris data files.
///
/// `None` (a null pointer natively) restores the compiled default search
/// path. The native library copies the path, so the Rust string does not
/// need to outlive the call.
///
/// # Environment interaction
///
/// When the `SE_EPHE_PATH` environment variable is set and non-empty, the
/// native library prefers it over the path given here. Tests and services
/// that need deterministic data selection must control that variable.
///
/// # Errors
///
/// Interior NUL bytes and paths longer than [`MAX_EPHE_PATH_LEN`] bytes
/// are rejected before any native call. These checks exist because the
/// native setter would otherwise truncate semantics silently (overlong
/// paths fall back to the default without notice).
///
/// # Examples
///
/// ```
/// // Restore the default search path.
/// swisseph_bindings::set_ephe_path(None)?;
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn set_ephe_path(path: Option<&str>) -> Result<(), Error> {
    let native_path = match path {
        Some(text) => {
            if text.as_bytes().contains(&0) {
                return Err(Error::invalid_input(
                    "set_ephe_path: path contains an interior NUL byte",
                ));
            }
            if text.len() > MAX_EPHE_PATH_LEN {
                return Err(Error::invalid_input(format!(
                    "set_ephe_path: path is {} bytes, longer than the {MAX_EPHE_PATH_LEN}-byte native limit",
                    text.len()
                )));
            }
            Some(CString::new(text).map_err(|_| {
                Error::invalid_input("set_ephe_path: path contains an interior NUL byte")
            })?)
        }
        None => None,
    };
    with_native_access(|| {
        apply_ephe_path_locked(native_path.as_ref());
        Ok(())
    })
}

/// Set the geographic observer position for topocentric computations.
///
/// `longitude` is east-positive degrees, `latitude` is degrees north, and
/// `altitude` is meters above sea level. The setting applies to subsequent
/// [`crate::calc_ut`]/[`crate::calc`] calls with `FLG_TOPOCTR` and to
/// observer-dependent calls in later families.
///
/// The observer-taking searches ([`crate::rise_trans`],
/// [`crate::rise_trans_true_hor`], [`crate::heliacal_ut`],
/// [`crate::heliacal_pheno_ut`], [`crate::vis_limit_mag`],
/// [`crate::sol_eclipse_when_loc`], [`crate::sol_eclipse_how`],
/// [`crate::lun_eclipse_when_loc`], [`crate::lun_eclipse_how`] and
/// [`crate::lun_occult_when_loc`]) install their observer the same way as
/// a side effect (established by probing the pinned build): a topocentric
/// calculation always sees the most recently installed observer, whether
/// it came from here or from such a search.
///
/// # Errors
///
/// Non-finite inputs are rejected with
/// [`ErrorKind::InvalidInput`](crate::ErrorKind) before any native call,
/// leaving the previously configured observer untouched: a NaN observer
/// would otherwise contaminate every later topocentric computation with
/// non-finite components (established by probing). Heights whose magnitude
/// exceeds `i32::MAX * AUNIT_TO_KM * 1000` metres are also `InvalidInput`: this
/// arithmetic ceiling bounds observer-dependent light-time dates before
/// native calendar conversions. Rejection preserves the existing observer.
pub fn set_topo(longitude: f64, latitude: f64, altitude: f64) -> Result<(), Error> {
    if !longitude.is_finite() || !latitude.is_finite() || !altitude.is_finite() {
        return Err(Error::invalid_input(
            "set_topo: longitude, latitude and altitude must be finite",
        ));
    }
    crate::domain::check_observer_height("set_topo", altitude)?;
    with_native_access(|| {
        apply_topo_locked(longitude, latitude, altitude);
        Ok(())
    })
}

/// Select the sidereal (ayanamsha) mode.
///
/// `mode` is one of the `SIDM_*` constants (for example
/// [`crate::SIDM_LAHIRI`]); `t0` and `ayan_t0` parameterize the
/// user-defined mode [`crate::SIDM_USER`] and are otherwise the epoch and
/// offset the native mode definition requires (usually `0.0`). The setting
/// applies to subsequent calculations with `FLG_SIDEREAL` and to
/// [`get_ayanamsa_ut`].
/// `t0` is an ET Julian Day and `ayan_t0` an offset in degrees. Native
/// option bit 1024 (`SE_SIDBIT_USER_UT`) makes a user reference epoch UT;
/// its effective ET then includes the active Delta-T configuration.
///
/// # Errors
///
/// Non-finite `t0`/`ayan_t0` are rejected with
/// [`ErrorKind::InvalidInput`](crate::ErrorKind) before any native call,
/// leaving the previously configured sidereal mode untouched.
/// In `SIDM_USER` (including its option bits), `t0` must also have an
/// `i32`-representable year in both native calendars. A UT reference also
/// requires a representable effective ET, checked under the native lock.
/// Dependent sidereal calls recheck that reference after later configuration
/// changes. Rejection preserves the previous sidereal mode.
pub fn set_sid_mode(mode: i32, t0: f64, ayan_t0: f64) -> Result<(), Error> {
    if !t0.is_finite() || !ayan_t0.is_finite() {
        return Err(Error::invalid_input(
            "set_sid_mode: t0 and ayan_t0 must be finite",
        ));
    }
    if mode & 255 == crate::SIDM_USER {
        crate::domain::check_et("set_sid_mode: user reference epoch", t0)?;
    }
    with_native_access(|| {
        check_sidereal_request_locked("set_sid_mode: user reference epoch", mode, t0, None, false)?;
        apply_sid_mode_locked(mode, t0, ayan_t0)
    })
}

/// Maximum JPL file-name length accepted by the safe wrapper, in bytes.
///
/// The native setter silently truncates names of `AS_MAXCH` (256) bytes or
/// more into its fixed-size storage. The wrapper rejects such input with
/// [`Error`] instead of selecting a file the caller did not ask for.
pub const MAX_JPL_FILE_LEN: usize = 255;

/// Select the JPL file used for `FLG_JPLEPH` calculations.
///
/// `fname` is a file name, optionally containing a directory part (any
/// directory part is also filled into the native search path). The native
/// library copies the name and immediately tries to open the file; when the
/// file cannot be opened, later `FLG_JPLEPH` calculations fall back with a
/// warning carried in their diagnostic text rather than failing here —
/// this function itself never fails natively. Passing an empty string
/// closes any currently open JPL file (the native setter closes cached
/// file data on every call) and leaves no JPL file selected.
///
/// Like [`set_ephe_path`], the lookup also honors the directory selected
/// by [`set_ephe_path`] and the `SE_EPHE_PATH` environment variable, so
/// tests needing deterministic selection must control both.
///
/// # Errors
///
/// Interior NUL bytes and names longer than [`MAX_JPL_FILE_LEN`] bytes are
/// rejected before any native call. Unlike [`set_ephe_path`], there is no
/// `None` reset value: the native setter dereferences its argument
/// unconditionally, so a null pointer is not a valid input.
///
/// # Examples
///
/// ```no_run
/// // Requires a JPL data file visible through the ephemeris path.
/// swisseph_bindings::set_jpl_file("de431.eph")?;
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn set_jpl_file(fname: &str) -> Result<(), Error> {
    if fname.as_bytes().contains(&0) {
        return Err(Error::invalid_input(
            "set_jpl_file: file name contains an interior NUL byte",
        ));
    }
    if fname.len() > MAX_JPL_FILE_LEN {
        return Err(Error::invalid_input(format!(
            "set_jpl_file: file name is {} bytes, longer than the {MAX_JPL_FILE_LEN}-byte native limit",
            fname.len()
        )));
    }
    let native_name = CString::new(fname).map_err(|_| {
        Error::invalid_input("set_jpl_file: file name contains an interior NUL byte")
    })?;
    with_native_access(|| {
        apply_jpl_file_locked(&native_name);
        Ok(())
    })
}

/// Set the tidal acceleration of the Moon used in Delta-T calculations.
///
/// `acc` is arcsec/cy²; [`crate::SE_TIDAL_AUTOMATIC`] restores automatic
/// selection. The setting rescales the Delta T returned by [`crate::deltat`]
/// and [`crate::deltat_ex`] for dates before 1955.0; from 1955.0 onwards
/// Delta T is pinned by modern observations and unaffected. A user Delta-T
/// override from [`set_delta_t_userdef`] takes precedence over this setting.
///
/// In automatic mode the engine tracks the ephemeris in use: opening data
/// files can update the held value to that ephemeris' intrinsic tidal
/// acceleration (visible through [`get_tid_acc`]), so the read-back value
/// after file-based calculations is not necessarily
/// [`crate::SE_TIDAL_DEFAULT`].
///
/// # Errors
///
/// Non-finite values are rejected before any native call; every finite
/// value, including the automatic sentinel, is passed through natively.
pub fn set_tid_acc(acc: f64) -> Result<(), Error> {
    if !acc.is_finite() {
        return Err(Error::invalid_input(
            "set_tid_acc: tidal acceleration must be finite",
        ));
    }
    with_native_access(|| apply_tid_acc_locked(acc))
}

/// Return the tidal acceleration of the Moon in arcsec/cy².
///
/// This is the value currently used by [`crate::deltat`] and
/// [`crate::deltat_ex`]: either set explicitly by [`set_tid_acc`] or, in
/// automatic mode, the native default ([`crate::SE_TIDAL_DEFAULT`]) as
/// possibly updated to the intrinsic value of opened data files (see
/// [`set_tid_acc`]).
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{SE_TIDAL_AUTOMATIC, SE_TIDAL_DE421, get_tid_acc, set_tid_acc};
///
/// set_tid_acc(SE_TIDAL_DE421)?;
/// assert_eq!(get_tid_acc()?, SE_TIDAL_DE421);
/// set_tid_acc(SE_TIDAL_AUTOMATIC)?;
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn get_tid_acc() -> Result<f64, Error> {
    with_native_access(|| {
        // SAFETY: pure value read of shared native configuration; the lock
        // pairs the read with any concurrent configuration write.
        Ok(unsafe { ffi::swe_get_tid_acc() })
    })
}

/// Fix the Delta T (TT minus UT, in days) returned by [`crate::deltat`]
/// and [`crate::deltat_ex`], or resume computed values.
///
/// `Some(dt)` pins Delta T to `dt` days; `None` clears the override by
/// passing the native [`crate::SE_DELTAT_AUTOMATIC`] sentinel. While an
/// override is active it takes precedence over [`set_tid_acc`].
/// There is no native getter for the override state: callers that need to
/// know whether an override is active must track it themselves.
///
/// # Errors
///
/// Non-finite values are rejected before any native call.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{deltat, set_delta_t_userdef};
///
/// set_delta_t_userdef(Some(65.0 / 86400.0))?;
/// assert!((deltat(2451545.0)? - 65.0 / 86400.0).abs() < 1e-12);
/// set_delta_t_userdef(None)?;
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn set_delta_t_userdef(dt: Option<f64>) -> Result<(), Error> {
    let native_dt = match dt {
        Some(value) => {
            if !value.is_finite() {
                return Err(Error::invalid_input(
                    "set_delta_t_userdef: Delta T must be finite",
                ));
            }
            value
        }
        None => crate::SE_DELTAT_AUTOMATIC,
    };
    with_native_access(|| apply_delta_t_locked(native_dt))
}

/// Ayanamsha (tropical-sidereal offset) with provenance.
///
/// Returned by [`get_ayanamsa_ex`] and [`get_ayanamsa_ex_ut`]: `value` is
/// the offset in degrees, `returned_flags` the flag set the native call
/// reports (naming the ephemeris source actually used), and `diagnostic`
/// the native text, which carries a warning when data files are missing
/// for star-based modes.
#[derive(Debug, Clone, PartialEq)]
pub struct Ayanamsha {
    /// Tropical-sidereal offset in degrees.
    pub value: f64,
    /// Flag set returned by the native call.
    pub returned_flags: i32,
    /// Native diagnostic text (`serr`); empty on a clean computation.
    pub diagnostic: String,
}

/// Read the ayanamsha while native access is already held.
///
/// Caller must hold the native lock; pairs the sidereal-mode read with
/// the computation.
pub(crate) fn get_ayanamsa_ut_locked(jd_ut: f64) -> Result<f64, Error> {
    crate::domain::check_ut_locked("get_ayanamsa_ut", jd_ut, -1)?;
    check_sidereal_epoch_locked("get_ayanamsa_ut: reference epoch", -1)?;
    // SAFETY: pure value call reading the active sidereal mode; the
    // caller holds the native lock.
    let value = unsafe { ffi::swe_get_ayanamsa_ut(jd_ut as c_double) };
    checked_ayanamsha("get_ayanamsa_ut", value)
}

/// Scalar native ayanamsha entry points have no status result. Never
/// expose a non-finite result as a successful astronomical value.
fn checked_ayanamsha(function: &str, value: f64) -> Result<f64, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::native(format!(
            "{function}: non-finite native ayanamsha; check sidereal epoch and Delta-T configuration"
        )))
    }
}

/// Return the ayanamsha (tropical-sidereal offset) in degrees.
///
/// The value follows the sidereal mode selected by [`set_sid_mode`]
/// (Fagan/Bradley by default in a fresh process). `jd_ut` is Universal
/// Time.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{SIDM_LAHIRI, get_ayanamsa_ut, set_sid_mode};
///
/// set_sid_mode(SIDM_LAHIRI, 0.0, 0.0)?;
/// let ayan = get_ayanamsa_ut(2451545.0)?;
/// assert!((23.0..25.0).contains(&ayan));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires a finite date with an `i32`-representable
/// year in both native calendars, including the active user reference epoch
/// after any UT-to-ET shift. The inherited tidal selection is preserved.
/// Domain violations return `InvalidInput`; non-finite native results return
/// `Native`.
pub fn get_ayanamsa_ut(jd_ut: f64) -> Result<f64, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(format!(
            "get_ayanamsa_ut: Julian Day must be finite ({jd_ut})"
        )));
    }
    with_native_access(|| get_ayanamsa_ut_locked(jd_ut))
}

/// Return the ayanamsha in degrees for an Ephemeris Time Julian Day.
///
/// Same value as [`get_ayanamsa_ut`] but taking `jd_et`
/// (Ephemeris/Terrestrial Time) directly instead of converting from UT
/// internally. The active mode comes from [`set_sid_mode`].
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{SIDM_LAHIRI, get_ayanamsa, set_sid_mode};
///
/// set_sid_mode(SIDM_LAHIRI, 0.0, 0.0)?;
/// let ayan = get_ayanamsa(2451545.0)?;
/// assert!((23.0..25.0).contains(&ayan));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires a finite date with an `i32`-representable
/// year in both native calendars, including the active user reference epoch
/// after any UT-to-ET shift. The inherited tidal selection is preserved.
/// Domain violations return `InvalidInput`; non-finite native results return
/// `Native`.
pub fn get_ayanamsa(jd_et: f64) -> Result<f64, Error> {
    if !jd_et.is_finite() {
        return Err(Error::invalid_input(format!(
            "get_ayanamsa: Julian Day must be finite ({jd_et})"
        )));
    }
    with_native_access(|| {
        crate::domain::check_et("get_ayanamsa", jd_et)?;
        check_sidereal_epoch_locked("get_ayanamsa: reference epoch", -1)?;
        // SAFETY: pure value call like [`get_ayanamsa_ut`], in the ET
        // scale the native entry point expects.
        let value = unsafe { ffi::swe_get_ayanamsa(jd_et as c_double) };
        checked_ayanamsha("get_ayanamsa", value)
    })
}

/// Return the ayanamsha with an explicit ephemeris flag.
///
/// Same value as [`get_ayanamsa`] through the native
/// `swe_get_ayanamsa_ex`, but `flags` selects the ephemeris source for
/// star-based modes (for example the true-Citra or galactic-center
/// modes, which resolve a fixed star internally) and the returned
/// [`Ayanamsha`] keeps the reported source flags and diagnostics.
///
/// # Errors
///
/// A negative native status (for example an unresolvable star-based
/// mode without data files) becomes [`Error`] with the diagnostic kept.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_SWIEPH, SIDM_LAHIRI, get_ayanamsa_ex, set_sid_mode};
///
/// set_sid_mode(SIDM_LAHIRI, 0.0, 0.0)?;
/// let ayan = get_ayanamsa_ex(2451545.0, FLG_SWIEPH)?;
/// assert!((23.0..25.0).contains(&ayan.value));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires a finite date with an `i32`-representable
/// year in both native calendars, including the active user reference epoch
/// after any UT-to-ET shift. Source-aware UT preflights also check possible
/// fallback shifts under the computation lock. Domain violations return
/// `InvalidInput`; non-finite native results return `Native` with diagnostics.
pub fn get_ayanamsa_ex(jd_et: f64, flags: i32) -> Result<Ayanamsha, Error> {
    if !jd_et.is_finite() {
        return Err(Error::invalid_input(format!(
            "get_ayanamsa_ex: Julian Day must be finite ({jd_et})"
        )));
    }
    with_native_access(|| {
        crate::domain::check_et("get_ayanamsa_ex", jd_et)?;
        check_sidereal_epoch_locked("get_ayanamsa_ex: reference epoch", flags)?;
        let mut daya = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: `daya` owns one writable slot and `serr` owns SERR_LEN
        // writable bytes; both outlive the call under the native lock.
        // The value is only exposed after the status is checked.
        let ret = unsafe { ffi::swe_get_ayanamsa_ex(jd_et, flags, &mut daya, serr.as_mut_ptr()) };
        // SAFETY: `serr` is our own fully initialized buffer; the read is
        // bounded by its capacity. Copied before the lock is released.
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 || !daya.is_finite() {
            return Err(Error::native(format!(
                "get_ayanamsa_ex: native computation failed (jd_et={jd_et}, flags={flags}): {diagnostic}"
            )));
        }
        Ok(Ayanamsha {
            value: daya,
            returned_flags: ret,
            diagnostic,
        })
    })
}

/// Return the ayanamsha with an explicit ephemeris flag for a UT input.
///
/// Same contract as [`get_ayanamsa_ex`] through the native
/// `swe_get_ayanamsa_ex_ut`, converting `jd_ut` (Universal Time) to
/// Ephemeris Time internally.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{FLG_SWIEPH, SIDM_LAHIRI, get_ayanamsa_ex_ut, set_sid_mode};
///
/// set_sid_mode(SIDM_LAHIRI, 0.0, 0.0)?;
/// let ayan = get_ayanamsa_ex_ut(2451545.0, FLG_SWIEPH)?;
/// assert!((23.0..25.0).contains(&ayan.value));
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
///
/// Native date arithmetic requires a finite date with an `i32`-representable
/// year in both native calendars, including the active user reference epoch
/// after any UT-to-ET shift. Source-aware UT preflights also check possible
/// fallback shifts under the computation lock. Domain violations return
/// `InvalidInput`; non-finite native results return `Native` with diagnostics.
pub fn get_ayanamsa_ex_ut(jd_ut: f64, flags: i32) -> Result<Ayanamsha, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(format!(
            "get_ayanamsa_ex_ut: Julian Day must be finite ({jd_ut})"
        )));
    }
    with_native_access(|| {
        crate::domain::check_ut_locked("get_ayanamsa_ex_ut", jd_ut, flags)?;
        check_sidereal_epoch_locked("get_ayanamsa_ex_ut: reference epoch", flags)?;
        let mut daya = 0.0;
        let mut serr = [0 as c_char; SERR_LEN];
        // SAFETY: same buffer contract as in [`get_ayanamsa_ex`].
        let ret =
            unsafe { ffi::swe_get_ayanamsa_ex_ut(jd_ut, flags, &mut daya, serr.as_mut_ptr()) };
        // SAFETY: same bounded read as in [`get_ayanamsa_ex`].
        let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
        if ret < 0 || !daya.is_finite() {
            return Err(Error::native(format!(
                "get_ayanamsa_ex_ut: native computation failed (jd_ut={jd_ut}, flags={flags}): {diagnostic}"
            )));
        }
        Ok(Ayanamsha {
            value: daya,
            returned_flags: ret,
            diagnostic,
        })
    })
}

/// Short native name of a sidereal (ayanamsha) mode.
///
/// `mode` is one of the `SIDM_*` constants (for example
/// [`crate::SIDM_LAHIRI`] yields `"Lahiri"`). Modes without a predefined
/// name fail natively (null return) and become [`Error`].
///
/// # Errors
///
/// Negative modes are rejected before any native call: the native lookup
/// folds its argument modulo 256 and indexes its name table directly, so
/// a negative value would read out of bounds.
///
/// # Examples
///
/// ```
/// use swisseph_bindings::{SIDM_FAGAN_BRADLEY, SIDM_LAHIRI, get_ayanamsa_name};
///
/// assert_eq!(get_ayanamsa_name(SIDM_LAHIRI)?, "Lahiri");
/// assert_eq!(get_ayanamsa_name(SIDM_FAGAN_BRADLEY)?, "Fagan/Bradley");
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn get_ayanamsa_name(mode: i32) -> Result<String, Error> {
    if mode < 0 {
        return Err(Error::invalid_input(format!(
            "get_ayanamsa_name: sidereal mode must be non-negative, got {mode}"
        )));
    }
    with_native_access(|| {
        // SAFETY: `mode` is non-negative, so the native modulo/index
        // sequence stays in bounds; a null return means "no predefined
        // name" and becomes an error. The text aliases static storage
        // and is copied into owned storage before the lock is released.
        let name = unsafe {
            let ptr = ffi::swe_get_ayanamsa_name(mode);
            if ptr.is_null() {
                return Err(Error::native(format!(
                    "get_ayanamsa_name: no predefined name for sidereal mode {mode}"
                )));
            }
            crate::ffi::copy_returned_string(ptr, b"")
        };
        Ok(name)
    })
}

/// Release native caches and close open ephemeris files.
///
/// This is process-wide and explicit: cached data is dropped for every
/// subsequent computation, which then lazily re-initializes on next use.
/// The pinned native close also resets the sidereal mode and Delta-T override;
/// their private validation metadata is cleared under the same native lock.
/// It is the only reset operation; in particular, dropping a Rust value
/// never closes anything, so one consumer cannot invalidate another's
/// data or configuration.
pub fn close() -> Result<(), Error> {
    with_native_access(|| {
        let mut settings = VALIDATION_SETTINGS
            .lock()
            .map_err(|_| Error::lock_poisoned())?;
        // SAFETY: takes no pointers and only touches native global state,
        // which the lock exclusively owns here.
        unsafe {
            ffi::swe_close();
        }
        *settings = ValidationSettings::default();
        Ok(())
    })
}

/// Identity of the ephemeris file backing one native file slot.
///
/// Returned by [`get_current_file_data`]: `path` is the file the engine
/// actually read (for example `"swisseph/ephe/sepl_18.se1"`), `tfstart`
/// and `tfend` bound the covered range in Julian Days, and `denum` is
/// the JPL ephemeris number (for example `441`).
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentFileData {
    /// Path of the file backing the slot, as reported by the engine.
    pub path: String,
    /// First Julian Day covered by the file.
    pub tfstart: f64,
    /// Last Julian Day covered by the file.
    pub tfend: f64,
    /// JPL Development Ephemeris number (for example `441`).
    pub denum: i32,
}

/// Read one native file slot while native access is already held.
///
/// Caller must hold the native lock. A null return means "no file for
/// this slot" and yields `None`.
pub(crate) fn get_current_file_data_locked(ifno: i32) -> Option<CurrentFileData> {
    let mut tfstart: c_double = 0.0;
    let mut tfend: c_double = 0.0;
    let mut denum: std::ffi::c_int = 0;
    // SAFETY: each out variable owns exactly one writable slot of
    // the declared type and outlives this call under the caller's native
    // lock. A null return means "no file for this slot" and leaves
    // the out parameters untouched (established by probing); a
    // non-null return aliases native static storage and is copied
    // into owned storage before the lock is released.
    unsafe {
        let ptr = ffi::swe_get_current_file_data(
            ifno as std::ffi::c_int,
            &mut tfstart,
            &mut tfend,
            &mut denum,
        );
        if ptr.is_null() {
            return None;
        }
        Some(CurrentFileData {
            path: crate::ffi::copy_returned_string(ptr, b""),
            tfstart,
            tfend,
            denum,
        })
    }
}

/// Report which ephemeris file backs one native file slot.
///
/// `ifno` selects the slot: `0` is the planet file, `1` the Moon file;
/// any other integer passes through engine-defined. Returns `Ok(None)`
/// when no file backs the slot — before any file-based computation, or
/// for slots the engine never fills (probed: slots `2`-`4` and
/// out-of-range ids report null with the out parameters untouched) —
/// never an error: absence of a file is data, not a failure.
///
/// The report reflects the files the engine has actually opened, so a
/// file-based computation (for example [`crate::calc_ut`] with
/// [`crate::FLG_SWIEPH`]) must run first; the query itself holds the
/// native lock with its read.
///
/// # Examples
///
/// ```
/// // The query always succeeds; whether a file is loaded depends on
/// // the process-global data configuration and computation history.
/// let _report = swisseph_bindings::get_current_file_data(0)?;
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn get_current_file_data(ifno: i32) -> Result<Option<CurrentFileData>, Error> {
    with_native_access(|| Ok(get_current_file_data_locked(ifno)))
}
