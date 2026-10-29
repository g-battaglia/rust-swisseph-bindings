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
pub(crate) const USER_UT_BIT: i32 = 1024;

thread_local! {
    /// Immutable identity of this thread, independent of native settings.
    static CONFIGURATION_THREAD: std::thread::ThreadId = std::thread::current().id();
}

/// Read the thread identity used to track native path installation.
/// Thread IDs remain fixed throughout their thread's lifetime and are never
/// reused; copying the ID avoids acquiring an owning thread handle per read.
/// This retains no native configuration, computation or result.
fn configuration_thread_id() -> std::thread::ThreadId {
    CONFIGURATION_THREAD.with(|identity| *identity)
}

/// Working-directory identity needed only by a relative native search entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum EphemerisDirectory {
    /// Every effective search entry is absolute; cwd cannot alter its lookup.
    Independent,
    /// A relative entry was installed from this directory.
    Current(std::path::PathBuf),
    /// A required cwd lookup failed; this state is never eligible for reuse.
    #[default]
    Unavailable,
}

/// Binding-owned validation metadata for write-only native settings.
/// Access order is native lock, then this mutex. No raw-C caller outside
/// the binding's synchronization contract is supported by this metadata.
#[derive(Default)]
struct ValidationSettings {
    /// A binding sidereal setter has initialized the native defaults.
    sidereal_initialized: bool,
    /// Complete owned sidereal/model inputs last installed under native access.
    owned_model: Option<(i32, u64, u64)>,
    /// UT reference epoch of the active user sidereal mode, if any.
    sidereal_ut_epoch: Option<f64>,
    /// Native Delta-T override; None represents automatic computation.
    delta_t: Option<f64>,
    /// Requested path plus native environment selection of the last path write.
    ephe_path: Option<(Option<CString>, Option<std::ffi::OsString>)>,
    /// Working-directory identity of the last effective path installation.
    ephe_path_directory: EphemerisDirectory,
    /// Native builds with thread-local storage need an installation per thread.
    ephe_path_thread: Option<std::thread::ThreadId>,
    /// Requested JPL name installed on this native thread, not file metadata.
    jpl_file: Option<(CString, std::thread::ThreadId)>,
    /// Path installation is still eligible for automatic tidal selection.
    /// Manual tidal writes clear this even when the numerical value is equal.
    ephe_path_automatic_tidal: bool,
    /// Last binding-owned observer and full sidereal setting, for cache reset.
    topo: Option<(f64, f64, f64)>,
    sidereal_mode: Option<(i32, f64, f64)>,
    /// Thread whose native sidereal setting and precession model are current.
    /// Path/JPL setters can reset the model without changing the stored mode.
    sidereal_thread: Option<std::thread::ThreadId>,
}

/// Invalidate numerical caches only when another owner/model has intervened.
/// Full session configuration is applied afterward. Repeated owned calls keep
/// files/caches open; foreign process-global sequences cannot supply our models.
pub(crate) fn prepare_owned_model_locked(mode: i32, t0: f64, ayan: f64) -> Result<(), Error> {
    let key = (mode, t0.to_bits(), ayan.to_bits());
    let mut settings = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?;
    if settings.owned_model != Some(key) {
        // SAFETY: caller holds native access; close owns no Rust buffers and
        // releases only the native cache. All owned settings are restored before
        // calculation; previously captured Rust snapshots remain independently owned.
        unsafe { ffi::swe_close() };
        *settings = ValidationSettings::default();
    }
    Ok(())
}

/// Install native defaults then the requested mode after any path/JPL resets.
pub(crate) fn apply_owned_model_locked(mode: i32, t0: f64, ayan: f64) -> Result<(), Error> {
    let key = (mode, t0.to_bits(), ayan.to_bits());
    let current = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?
        .owned_model;
    if current != Some(key) {
        apply_default_models_locked();
        apply_sid_mode_locked(mode, t0, ayan)?;
        VALIDATION_SETTINGS
            .lock()
            .map_err(|_| Error::lock_poisoned())?
            .owned_model = Some(key);
    }
    Ok(())
}

/// Fresh native defaults; explicit close resets both settings (ABI probe).
static VALIDATION_SETTINGS: Mutex<ValidationSettings> = Mutex::new(ValidationSettings {
    sidereal_initialized: false,
    owned_model: None,
    sidereal_ut_epoch: None,
    delta_t: None,
    ephe_path: None,
    ephe_path_directory: EphemerisDirectory::Unavailable,
    ephe_path_thread: None,
    jpl_file: None,
    ephe_path_automatic_tidal: false,
    topo: None,
    sidereal_mode: None,
    sidereal_thread: None,
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
        unsafe { ffi::swe_set_sid_mode(crate::constants::SIDM_FAGAN_BRADLEY, 0.0, 0.0) };
        settings.sidereal_initialized = true;
        settings.sidereal_mode = Some((crate::constants::SIDM_FAGAN_BRADLEY, 0.0, 0.0));
        settings.sidereal_thread = Some(configuration_thread_id());
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
    if mode & 255 != crate::constants::SIDM_USER {
        return Ok(());
    }
    crate::domain::check_et(function, t0)?;
    if mode & USER_UT_BIT == 0 {
        return Ok(());
    }
    if let Some(Some(dt)) = delta_t
        && dt != crate::constants::SE_DELTAT_AUTOMATIC
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
        let _restore = RestoreDeltaT(saved.unwrap_or(crate::constants::SE_DELTAT_AUTOMATIC));
        // SAFETY: caller holds native access; clear only for this native
        // Delta-T preflight, then RestoreDeltaT restores the exact override.
        unsafe { ffi::swe_set_delta_t_userdef(crate::constants::SE_DELTAT_AUTOMATIC) };
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

/// Whether the effective native search path contains a cwd-dependent entry.
///
/// The pinned C setter accepts a nonempty `SE_EPHE_PATH` of at most 242
/// bytes before the requested path, and otherwise uses its compiled default
/// for a missing, empty or oversized request. Unix lookup accepts both colon
/// and semicolon separators; all entries must start with `/` to skip cwd.
/// The compiled default includes `.`, and other platforms remain conservative.
fn ephe_path_depends_on_directory(
    path: Option<&CString>,
    environment: Option<&std::ffi::OsStr>,
) -> bool {
    #[cfg(unix)]
    {
        let admitted = |bytes: &&[u8]| !bytes.is_empty() && bytes.len() <= MAX_EPHE_PATH_LEN;
        let effective = environment
            .map(std::ffi::OsStr::as_encoded_bytes)
            .filter(admitted)
            .or_else(|| path.map(|path| path.as_bytes()).filter(admitted));
        effective.is_none_or(|bytes| {
            bytes
                .split(|byte| *byte == b':' || *byte == b';')
                .any(|entry| !entry.starts_with(b"/"))
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (path, environment);
        true
    }
}

/// Capture cwd only when a native path entry can resolve against it.
/// Failed lookups disable reuse; absolute-only paths need no filesystem call.
fn ephe_path_directory(
    path: Option<&CString>,
    environment: Option<&std::ffi::OsStr>,
) -> EphemerisDirectory {
    if !ephe_path_depends_on_directory(path, environment) {
        return EphemerisDirectory::Independent;
    }
    match std::env::current_dir() {
        Ok(directory) => EphemerisDirectory::Current(directory),
        Err(_) => EphemerisDirectory::Unavailable,
    }
}

/// Immutable external lookup inputs captured when an owned session is built.
/// Native setters consult the environment and relative paths; reject observed
/// changes rather than silently selecting different data for the same session.
#[derive(Debug, Clone)]
pub(crate) struct SessionSearchContext {
    environment: Option<std::ffi::OsString>,
    directory: EphemerisDirectory,
}

impl SessionSearchContext {
    /// Capture only lookup inputs, never inferred data identities or C outputs.
    pub(crate) fn capture(path: Option<&CString>) -> Result<Self, Error> {
        let environment = std::env::var_os("SE_EPHE_PATH");
        let directory = ephe_path_directory(path, environment.as_deref());
        if directory == EphemerisDirectory::Unavailable {
            return Err(Error::invalid_input(
                "SessionBuilder: cannot capture the working directory for a relative native search path",
            ));
        }
        Ok(Self {
            environment,
            directory,
        })
    }

    /// Caller holds native access before any setter or dependent computation.
    /// Concurrent external environment/cwd mutation is outside the binding's
    /// synchronization contract; no process environment mutation is performed.
    pub(crate) fn check(
        &self,
        path: Option<&CString>,
        check_environment: bool,
        check_directory: bool,
    ) -> Result<(), Error> {
        let environment = std::env::var_os("SE_EPHE_PATH");
        let directory_environment = if check_environment {
            &environment
        } else {
            &self.environment
        };
        let directory = ephe_path_directory(path, directory_environment.as_deref());
        if (check_environment && environment != self.environment)
            || (check_directory && directory != self.directory)
        {
            return Err(Error::invalid_input(
                "Session: native search environment or relative working directory changed since build; rebuild the session explicitly",
            ));
        }
        Ok(())
    }
}

/// Apply an ephemeris path while native access is already held.
///
/// # Safety contract
///
/// The caller must hold the process-wide native lock (see
/// [`crate::state`): this function performs the raw setter without
/// acquiring it. `path` follows the same convention as [`set_ephe_path`]
/// (`None` = null = compiled default).
pub(crate) fn apply_ephe_path_locked(path: Option<&CString>) -> Result<(), Error> {
    let key = (path.cloned(), std::env::var_os("SE_EPHE_PATH"));
    let directory = ephe_path_directory(path, key.1.as_deref());
    let thread = configuration_thread_id();
    let mut settings = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?;
    let changed = settings.ephe_path.as_ref() != Some(&key)
        || directory == EphemerisDirectory::Unavailable
        || settings.ephe_path_directory != directory
        || settings.ephe_path_thread != Some(thread);
    let pointer = path.map_or(std::ptr::null(), |owned| owned.as_ptr());
    // Even an equal forced path write clears native astronomical models.
    // A changed path replays the remembered mode below; an unchanged write
    // leaves it for the next configured Session to install explicitly.
    settings.sidereal_thread = None;
    settings.owned_model = None;
    // SAFETY: native access is held; strings remain alive and are copied by C.
    // C's path setter retains the extended catalog. A real path change therefore
    // requires public close before path installation. Replay binding-owned
    // sidereal/topo writes that close resets but the path setter normally keeps.
    unsafe {
        if changed {
            ffi::swe_close();
        }
        ffi::swe_set_ephe_path(pointer);
        if changed {
            if let Some((mode, t0, ayan)) = settings.sidereal_mode {
                ffi::swe_set_sid_mode(mode, t0, ayan);
                settings.sidereal_thread = Some(thread);
                if let Some(dt) = settings.delta_t {
                    ffi::swe_set_delta_t_userdef(dt);
                }
            }
            if let Some((lon, lat, alt)) = settings.topo {
                ffi::swe_set_topo(lon, lat, alt);
            }
        }
    }
    // Every forced path installation may close JPL resources. Invalidate only
    // the setter-input cache; no file identity or successful open is inferred.
    settings.jpl_file = None;
    settings.ephe_path = Some(key);
    settings.ephe_path_directory = directory;
    settings.ephe_path_thread = Some(thread);
    settings.ephe_path_automatic_tidal = true;
    Ok(())
}

/// Install a session's path only when its native configuration has changed.
///
/// Caller holds native access across this helper and the dependent operation.
/// Every binding path write updates the same metadata, and explicit close
/// clears it. Public setters always use `apply_ephe_path_locked`, retaining
/// their forced-reload behavior. Manual tidal writes disable reuse because
/// path installation restores automatic selection, even for an equal value.
pub(crate) fn ensure_ephe_path_locked(path: Option<&CString>) -> Result<(), Error> {
    ensure_ephe_path_with_environment_locked(path, true, true)
}

/// Reuse installed path metadata under native access. When refresh is disabled,
/// only an actual installation samples the environment, as the C setter does.
/// Directory and thread checks use the last globally installed selection.
pub(crate) fn ensure_ephe_path_with_environment_locked(
    path: Option<&CString>,
    refresh_environment: bool,
    reinstall_on_directory_change: bool,
) -> Result<(), Error> {
    ensure_ephe_path_policy_locked(
        path,
        refresh_environment,
        reinstall_on_directory_change,
        true,
    )
}

fn ensure_ephe_path_policy_locked(
    path: Option<&CString>,
    refresh_environment: bool,
    reinstall_on_directory_change: bool,
    restore_automatic_tidal: bool,
) -> Result<(), Error> {
    let environment = refresh_environment.then(|| std::env::var_os("SE_EPHE_PATH"));
    let reusable = {
        let settings = VALIDATION_SETTINGS
            .lock()
            .map_err(|_| Error::lock_poisoned())?;
        settings
            .ephe_path
            .as_ref()
            .is_some_and(|(saved, saved_environment)| {
                if saved.as_ref() != path
                    || environment
                        .as_ref()
                        .is_some_and(|current| current != saved_environment)
                {
                    return false;
                }
                // The saved environment describes what C actually installed, even
                // when a different session last wrote it. The default policy
                // observes cwd changes; native-relative lifecycle keeps C's
                // current path and open files without another setter.
                let directory_reusable = !reinstall_on_directory_change
                    || settings.ephe_path_directory == EphemerisDirectory::Independent
                    || {
                        let directory = ephe_path_directory(path, saved_environment.as_deref());
                        directory != EphemerisDirectory::Unavailable
                            && directory == settings.ephe_path_directory
                    };
                directory_reusable
                && settings.ephe_path_thread == Some(configuration_thread_id())
                // Explicit manual tidal writes require the setter's automatic
                // selection effect, even if the numeric value was unchanged.
                && (!restore_automatic_tidal || settings.ephe_path_automatic_tidal)
            })
    };
    if reusable {
        Ok(())
    } else {
        apply_ephe_path_locked(path)
    }
}

/// Owned requests reapply their own tide; manual tide alone does not reload files.
pub(crate) fn ensure_owned_ephe_path_locked(
    path: Option<&CString>,
    refresh_environment: bool,
    reinstall_on_directory_change: bool,
) -> Result<(), Error> {
    ensure_ephe_path_policy_locked(
        path,
        refresh_environment,
        reinstall_on_directory_change,
        false,
    )
}

/// Apply an observer position while native access is already held.
///
/// Caller must hold the native lock; pure value write serialized with
/// later computations.
pub(crate) fn apply_topo_locked(longitude: f64, latitude: f64, altitude: f64) -> Result<(), Error> {
    let mut settings = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?;
    // SAFETY: pure value call; the caller holds the native lock because
    // this mutates shared configuration read by later computations.
    unsafe {
        ffi::swe_set_topo(
            longitude as c_double,
            latitude as c_double,
            altitude as c_double,
        );
    }
    settings.topo = Some((longitude, latitude, altitude));
    Ok(())
}

/// Install the native current-version model defaults for an owned request.
/// Caller holds native access. Sidereal options can alter precession/nutation
/// models even after a later ordinary sidereal setter, so these write-only
/// defaults must precede the requested sidereal mode. No files are closed.
pub(crate) fn apply_default_models_locked() {
    let mut selector = [0 as c_char; 32];
    // SAFETY: initialized mutable NUL-terminated storage outlives the call.
    // The public ABI's empty selector chooses this engine version's defaults;
    // flags zero requests no historical-version/provider-dependent override.
    unsafe { ffi::swe_set_astro_models(selector.as_mut_ptr(), 0) };
}

/// Apply a sidereal mode while native access is already held.
///
/// Caller must hold the native lock; same reason as [`apply_topo_locked`].
pub(crate) fn apply_sid_mode_locked(mode: i32, t0: f64, ayan_t0: f64) -> Result<(), Error> {
    let mut settings = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?;
    settings.owned_model = None;
    // SAFETY: pure value call; serialized for the same reason as
    // [`apply_topo_locked`].
    unsafe {
        ffi::swe_set_sid_mode(mode, t0 as c_double, ayan_t0 as c_double);
    }
    settings.sidereal_ut_epoch =
        (mode & 255 == crate::constants::SIDM_USER && mode & USER_UT_BIT != 0).then_some(t0);
    settings.sidereal_initialized = true;
    settings.sidereal_mode = Some((mode, t0, ayan_t0));
    settings.sidereal_thread = Some(configuration_thread_id());
    Ok(())
}

/// Apply a JPL file selection while native access is already held.
///
/// Caller must hold the native lock; the setter copies the name and
/// closes cached file data.
pub(crate) fn apply_jpl_file_locked(native_name: &CString) -> Result<(), Error> {
    let mut settings = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?;
    // SAFETY: `native_name` is a live `CString`, hence non-null and
    // NUL-terminated as the native setter requires. The setter copies the
    // name, so no reference outlives this scope.
    unsafe {
        ffi::swe_set_jpl_file(native_name.as_ptr());
    }
    settings.sidereal_thread = None;
    settings.owned_model = None;
    settings.jpl_file = Some((native_name.clone(), std::thread::current().id()));
    Ok(())
}

/// Reuse an identical requested JPL name on this thread without closing files.
/// Path writes and close invalidate this input-only cache. It never claims that
/// the requested file exists or that a calculation was served by JPL.
pub(crate) fn ensure_jpl_file_locked(native_name: &CString) -> Result<(), Error> {
    let reusable = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?
        .jpl_file
        .as_ref()
        .is_some_and(|(saved, thread)| {
            saved == native_name && *thread == std::thread::current().id()
        });
    if reusable {
        Ok(())
    } else {
        apply_jpl_file_locked(native_name)
    }
}

/// Apply a tidal acceleration while native access is already held.
///
/// Caller must hold the native lock.
pub(crate) fn apply_tid_acc_locked(acc: f64) -> Result<(), Error> {
    initialize_sidereal_locked()?;
    let mut settings = VALIDATION_SETTINGS
        .lock()
        .map_err(|_| Error::lock_poisoned())?;
    // SAFETY: pure value call; serialized because it mutates shared
    // native configuration read by later Delta-T computations.
    unsafe {
        ffi::swe_set_tid_acc(acc as c_double);
    }
    settings.ephe_path_automatic_tidal = false;
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
    settings.delta_t = (native_dt != crate::constants::SE_DELTAT_AUTOMATIC).then_some(native_dt);
    Ok(())
}

/// Select the directory searched for ephemeris data files.
///
/// `None` (a null pointer natively) restores the compiled default search
/// path. The native library copies the path, so the Rust string does not
/// need to outlive the call.
/// Every call reloads native files and caches, even for an unchanged path.
/// [`ensure_ephe_path`] and configured [`crate::native::Session`] computations may
/// instead reuse an unchanged installation under the process-wide lock.
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
/// swisseph_bindings::global::set_ephe_path(None)?;
/// # Ok::<(), swisseph_bindings::native::Error>(())
/// ```
pub fn set_ephe_path(path: Option<&str>) -> Result<(), Error> {
    let native_path = ephe_path_argument(path)?;
    with_native_access(|| {
        apply_ephe_path_locked(native_path.as_ref())?;
        Ok(())
    })
}

/// Install an ephemeris search path only when its effective configuration changes.
///
/// `path` uses the same UTF-8 byte limits and `None`/empty/default conventions as
/// [`set_ephe_path`], including native `SE_EPHE_PATH` precedence. An unchanged
/// request retains open files and calculation caches. A changed request,
/// environment, relevant working directory, native thread, explicit [`crate::global::close`]
/// or binding-owned manual tidal setting requires path installation again.
///
/// Use [`set_ephe_path`] to force a reload even when the spelling is unchanged,
/// for example after replacing a data file. Like that setter, this function
/// releases native access before returning: a later free computation needs
/// caller serialization. Prefer [`crate::native::Session`] to configure and compute in
/// one native acquisition. Raw C changes outside these bindings are not tracked.
///
/// # Errors
///
/// Path admission and diagnostics are identical to [`set_ephe_path`]: interior
/// NUL bytes and paths longer than [`MAX_EPHE_PATH_LEN`] bytes are rejected before
/// changing native state. Native lock failures propagate without retry.
pub fn ensure_ephe_path(path: Option<&str>) -> Result<(), Error> {
    let native_path = ephe_path_argument(path)?;
    with_native_access(|| ensure_ephe_path_locked(native_path.as_ref()))
}

/// Admit the exact path bytes shared by explicit and guarded installations.
/// No native state is touched before length/NUL validation succeeds.
fn ephe_path_argument(path: Option<&str>) -> Result<Option<CString>, Error> {
    Ok(match path {
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
    })
}

/// Set the geographic observer position for topocentric computations.
///
/// `longitude` is east-positive degrees, `latitude` is degrees north, and
/// `altitude` is meters above sea level. The setting applies to subsequent
/// [`crate::global::calc_ut`]/[`crate::global::calc`] calls with `FLG_TOPOCTR` and to
/// observer-dependent calls in later families.
///
/// The observer-taking searches ([`crate::global::rise_trans`],
/// [`crate::global::rise_trans_true_hor`], [`crate::global::heliacal_ut`],
/// [`crate::global::heliacal_pheno_ut`], [`crate::global::vis_limit_mag`],
/// [`crate::global::sol_eclipse_when_loc`], [`crate::global::sol_eclipse_how`],
/// [`crate::global::lun_eclipse_when_loc`], [`crate::global::lun_eclipse_how`] and
/// [`crate::global::lun_occult_when_loc`]) install their observer the same way as
/// a side effect (established by probing the pinned build): a topocentric
/// calculation always sees the most recently installed observer, whether
/// it came from here or from such a search.
///
/// # Errors
///
/// Non-finite inputs are rejected with
/// [`ErrorKind::InvalidInput`](crate::native::ErrorKind) before any native call,
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
        apply_topo_locked(longitude, latitude, altitude)?;
        Ok(())
    })
}

/// Select the sidereal (ayanamsha) mode.
///
/// `mode` is one of the `SIDM_*` constants (for example
/// [`crate::constants::SIDM_LAHIRI`]); `t0` and `ayan_t0` parameterize the
/// user-defined mode [`crate::constants::SIDM_USER`] and are otherwise the epoch and
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
/// [`ErrorKind::InvalidInput`](crate::native::ErrorKind) before any native call,
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
    if mode & 255 == crate::constants::SIDM_USER {
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
/// swisseph_bindings::native::set_jpl_file("de431.eph")?;
/// # Ok::<(), swisseph_bindings::native::Error>(())
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
    with_native_access(|| apply_jpl_file_locked(&native_name))
}

/// Set the tidal acceleration of the Moon used in Delta-T calculations.
///
/// `acc` is arcsec/cy²; [`crate::constants::SE_TIDAL_AUTOMATIC`] restores automatic
/// selection. The setting rescales the Delta T returned by [`crate::global::deltat`]
/// and [`crate::global::deltat_ex`] for dates before 1955.0; from 1955.0 onwards
/// Delta T is pinned by modern observations and unaffected. A user Delta-T
/// override from [`set_delta_t_userdef`] takes precedence over this setting.
///
/// In automatic mode the engine tracks the ephemeris in use: opening data
/// files can update the held value to that ephemeris' intrinsic tidal
/// acceleration (visible through [`get_tid_acc`]), so the read-back value
/// after file-based calculations is not necessarily
/// [`crate::constants::SE_TIDAL_DEFAULT`].
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
/// This is the value currently used by [`crate::global::deltat`] and
/// [`crate::global::deltat_ex`]: either set explicitly by [`set_tid_acc`] or, in
/// automatic mode, the native default ([`crate::constants::SE_TIDAL_DEFAULT`]) as
/// possibly updated to the intrinsic value of opened data files (see
/// [`set_tid_acc`]).
///
/// # Examples
///
/// ```
/// use swisseph_bindings::native::{SE_TIDAL_AUTOMATIC, SE_TIDAL_DE421, global::get_tid_acc, global::set_tid_acc};
///
/// set_tid_acc(SE_TIDAL_DE421)?;
/// assert_eq!(get_tid_acc()?, SE_TIDAL_DE421);
/// set_tid_acc(SE_TIDAL_AUTOMATIC)?;
/// # Ok::<(), swisseph_bindings::native::Error>(())
/// ```
pub fn get_tid_acc() -> Result<f64, Error> {
    with_native_access(|| {
        // SAFETY: pure value read of shared native configuration; the lock
        // pairs the read with any concurrent configuration write.
        Ok(unsafe { ffi::swe_get_tid_acc() })
    })
}

/// Fix the Delta T (TT minus UT, in days) returned by [`crate::global::deltat`]
/// and [`crate::global::deltat_ex`], or resume computed values.
///
/// `Some(dt)` pins Delta T to `dt` days; `None` clears the override by
/// passing the native [`crate::constants::SE_DELTAT_AUTOMATIC`] sentinel. While an
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
/// use swisseph_bindings::native::{global::deltat, global::set_delta_t_userdef};
///
/// set_delta_t_userdef(Some(65.0 / 86400.0))?;
/// assert!((deltat(2451545.0)? - 65.0 / 86400.0).abs() < 1e-12);
/// set_delta_t_userdef(None)?;
/// # Ok::<(), swisseph_bindings::native::Error>(())
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
        None => crate::constants::SE_DELTAT_AUTOMATIC,
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
fn checked_ayanamsha(function: &'static str, value: f64) -> Result<f64, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::native(format!(
            "{function}: non-finite native ayanamsha; check sidereal epoch and Delta-T configuration"
        )).with_native_channels(function, None, Some(value), None))
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
/// use swisseph_bindings::native::{SIDM_LAHIRI, global::get_ayanamsa_ut, global::set_sid_mode};
///
/// set_sid_mode(SIDM_LAHIRI, 0.0, 0.0)?;
/// let ayan = get_ayanamsa_ut(2451545.0)?;
/// assert!((23.0..25.0).contains(&ayan));
/// # Ok::<(), swisseph_bindings::native::Error>(())
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
/// use swisseph_bindings::native::{SIDM_LAHIRI, global::get_ayanamsa, global::set_sid_mode};
///
/// set_sid_mode(SIDM_LAHIRI, 0.0, 0.0)?;
/// let ayan = get_ayanamsa(2451545.0)?;
/// assert!((23.0..25.0).contains(&ayan));
/// # Ok::<(), swisseph_bindings::native::Error>(())
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
    with_native_access(|| get_ayanamsa_locked(jd_et))
}

/// Execute [`crate::global::get_ayanamsa`] with native access already held.
/// Preserve its input domains, units, native status/flags and diagnostics;
/// apply no configuration, fallback or alternate numerical method. Errors
/// follow the public operation. Caller owns the lock for validation and C read.
pub(crate) fn get_ayanamsa_locked(jd_et: f64) -> Result<f64, Error> {
    if !jd_et.is_finite() {
        return Err(Error::invalid_input(format!(
            "get_ayanamsa: Julian Day must be finite ({jd_et})"
        )));
    }
    crate::domain::check_et("get_ayanamsa", jd_et)?;
    check_sidereal_epoch_locked("get_ayanamsa: reference epoch", -1)?;
    // SAFETY: pure value call like [`get_ayanamsa_ut`], in the ET
    // scale the native entry point expects.
    let value = unsafe { ffi::swe_get_ayanamsa(jd_et as c_double) };
    checked_ayanamsha("get_ayanamsa", value)
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
/// use swisseph_bindings::native::{FLG_SWIEPH, SIDM_LAHIRI, global::get_ayanamsa_ex, global::set_sid_mode};
///
/// set_sid_mode(SIDM_LAHIRI, 0.0, 0.0)?;
/// let ayan = get_ayanamsa_ex(2451545.0, FLG_SWIEPH)?;
/// assert!((23.0..25.0).contains(&ayan.value));
/// # Ok::<(), swisseph_bindings::native::Error>(())
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
    with_native_access(|| get_ayanamsa_ex_locked(jd_et, flags))
}

/// Execute [`crate::global::get_ayanamsa_ex`] with native access already held.
/// Preserve its input domains, units, native status/flags and diagnostics;
/// apply no configuration, fallback or alternate numerical method. Errors
/// follow the public operation. Caller owns the lock for validation and C read.
pub(crate) fn get_ayanamsa_ex_locked(jd_et: f64, flags: i32) -> Result<Ayanamsha, Error> {
    if !jd_et.is_finite() {
        return Err(Error::invalid_input(format!(
            "get_ayanamsa_ex: Julian Day must be finite ({jd_et})"
        )));
    }
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
        )).with_native_channels("get_ayanamsa_ex", Some(ret), None, Some(&diagnostic)));
    }
    Ok(Ayanamsha {
        value: daya,
        returned_flags: ret,
        diagnostic,
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
/// use swisseph_bindings::native::{FLG_SWIEPH, SIDM_LAHIRI, global::get_ayanamsa_ex_ut, global::set_sid_mode};
///
/// set_sid_mode(SIDM_LAHIRI, 0.0, 0.0)?;
/// let ayan = get_ayanamsa_ex_ut(2451545.0, FLG_SWIEPH)?;
/// assert!((23.0..25.0).contains(&ayan.value));
/// # Ok::<(), swisseph_bindings::native::Error>(())
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
    with_native_access(|| get_ayanamsa_ex_ut_locked(jd_ut, flags))
}

/// Execute [`crate::global::get_ayanamsa_ex_ut`] with native access already held.
/// Preserve its input domains, units, native status/flags and diagnostics;
/// apply no configuration, fallback or alternate numerical method. Errors
/// follow the public operation. Caller owns the lock for validation and C read.
pub(crate) fn get_ayanamsa_ex_ut_locked(jd_ut: f64, flags: i32) -> Result<Ayanamsha, Error> {
    if !jd_ut.is_finite() {
        return Err(Error::invalid_input(format!(
            "get_ayanamsa_ex_ut: Julian Day must be finite ({jd_ut})"
        )));
    }
    crate::domain::check_ut_locked("get_ayanamsa_ex_ut", jd_ut, flags)?;
    check_sidereal_epoch_locked("get_ayanamsa_ex_ut: reference epoch", flags)?;
    let mut daya = 0.0;
    let mut serr = [0 as c_char; SERR_LEN];
    // SAFETY: same buffer contract as in [`get_ayanamsa_ex`].
    let ret = unsafe { ffi::swe_get_ayanamsa_ex_ut(jd_ut, flags, &mut daya, serr.as_mut_ptr()) };
    // SAFETY: same bounded read as in [`get_ayanamsa_ex`].
    let diagnostic = unsafe { read_native_text(serr.as_ptr(), SERR_LEN) };
    if ret < 0 || !daya.is_finite() {
        return Err(Error::native(format!(
            "get_ayanamsa_ex_ut: native computation failed (jd_ut={jd_ut}, flags={flags}): {diagnostic}"
        )).with_native_channels("get_ayanamsa_ex_ut", Some(ret), None, Some(&diagnostic)));
    }
    Ok(Ayanamsha {
        value: daya,
        returned_flags: ret,
        diagnostic,
    })
}

/// Short native name of a sidereal (ayanamsha) mode.
///
/// `mode` is one of the `SIDM_*` constants (for example
/// [`crate::constants::SIDM_LAHIRI`] yields `"Lahiri"`). Modes without a predefined
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
/// use swisseph_bindings::native::{SIDM_FAGAN_BRADLEY, SIDM_LAHIRI, native::get_ayanamsa_name};
///
/// assert_eq!(get_ayanamsa_name(SIDM_LAHIRI)?, "Lahiri");
/// assert_eq!(get_ayanamsa_name(SIDM_FAGAN_BRADLEY)?, "Fagan/Bradley");
/// # Ok::<(), swisseph_bindings::native::Error>(())
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
/// The pinned native close resets sidereal and observer state, but retains
/// the search path and explicit Delta-T override. This API clears all private
/// validation metadata, requiring path installation at the next configured
/// read. Use [`close_preserving_path`] to retain native path reuse instead.
/// Dropping a Rust value never closes anything.
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

/// Close native files and reset calculation state while retaining path reuse.
///
/// Run the same native close as [`close`], without re-installing an unchanged
/// search path at the next configured read. Native close retains that path;
/// re-installing it would eagerly read the modern Moon file and can change
/// automatic Delta-T selection for a subsequent date outside that file.
/// No file or calculated result is retained. Other path owners, environment,
/// relevant working-directory and native-thread changes still require ordinary
/// path installation. Native sidereal, observer and tidal state is reset;
/// the explicit Delta-T override is retained, matching the C operation.
///
/// This process-wide operation has no physical input or output units. Native
/// lock failures propagate without retry. Raw C writes outside the binding's
/// locking contract are not tracked. The existing [`close`] policy is unchanged.
pub fn close_preserving_path() -> Result<(), Error> {
    with_native_access(|| {
        let mut settings = VALIDATION_SETTINGS
            .lock()
            .map_err(|_| Error::lock_poisoned())?;
        // SAFETY: native access is held; the no-argument C operation drops
        // files/caches and resets mode/site/tidal state, retaining path and
        // the explicit Delta-T override. No caller pointer is exposed.
        unsafe { ffi::swe_close() };
        let previous = std::mem::take(&mut *settings);
        settings.delta_t = previous.delta_t;
        settings.ephe_path = previous.ephe_path;
        settings.ephe_path_directory = previous.ephe_path_directory;
        settings.ephe_path_thread = previous.ephe_path_thread;
        settings.ephe_path_automatic_tidal = settings.ephe_path.is_some();
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
    let mut data = None;
    capture_current_file_data_locked(ifno, &mut data);
    data
}

/// Replace one owned snapshot with a fresh native file-slot observation.
///
/// The caller holds native access. Every call queries the requested slot and
/// copies its current path bytes and scalar fields; a null path clears the
/// snapshot. Existing path capacity is reused, without retaining a native
/// pointer, skipping a query or comparing against earlier field values.
pub(crate) fn capture_current_file_data_locked(ifno: i32, target: &mut Option<CurrentFileData>) {
    let mut tfstart: c_double = 0.0;
    let mut tfend: c_double = 0.0;
    let mut denum: std::ffi::c_int = 0;
    // SAFETY: each out variable has the native declared type and remains live
    // under the caller's lock. A non-null path is NUL-terminated static native
    // storage, copied immediately before any subsequent native call.
    unsafe {
        let ptr = ffi::swe_get_current_file_data(
            ifno as std::ffi::c_int,
            &mut tfstart,
            &mut tfend,
            &mut denum,
        );
        if ptr.is_null() {
            *target = None;
            return;
        }
        let path = std::ffi::CStr::from_ptr(ptr).to_string_lossy();
        match target {
            Some(data) => {
                data.path.clear();
                data.path.push_str(&path);
                data.tfstart = tfstart;
                data.tfend = tfend;
                data.denum = denum;
            }
            None => {
                *target = Some(CurrentFileData {
                    path: path.into_owned(),
                    tfstart,
                    tfend,
                    denum,
                });
            }
        }
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
/// file-based computation (for example [`crate::global::calc_ut`] with
/// [`crate::constants::FLG_SWIEPH`]) must run first; the query itself holds the
/// native lock with its read.
///
/// # Examples
///
/// ```
/// // The query always succeeds; whether a file is loaded depends on
/// // the process-global data configuration and computation history.
/// let _report = swisseph_bindings::global::get_current_file_data(0)?;
/// # Ok::<(), swisseph_bindings::native::Error>(())
/// ```
pub fn get_current_file_data(ifno: i32) -> Result<Option<CurrentFileData>, Error> {
    with_native_access(|| Ok(get_current_file_data_locked(ifno)))
}

#[cfg(all(test, unix))]
mod path_directory_tests {
    use super::*;
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    #[test]
    fn effective_path_keeps_native_precedence_and_separator_rules() {
        let absolute = CString::new("/data/ephe").unwrap();
        let relative = CString::new(".").unwrap();
        let empty = CString::new("").unwrap();
        for path in [None, Some(&empty), Some(&relative)] {
            assert!(ephe_path_depends_on_directory(path, None));
        }
        for text in ["/a", "/a:/b", "/a;/b", "/a:/b;/c"] {
            let path = CString::new(text).unwrap();
            assert!(!ephe_path_depends_on_directory(Some(&path), None));
            assert!(!ephe_path_depends_on_directory(
                Some(&relative),
                Some(OsStr::new(text))
            ));
        }
        for text in [".", "relative", "/a:.", "/a;relative", ":/a", "/a;"] {
            let path = CString::new(text).unwrap();
            assert!(ephe_path_depends_on_directory(Some(&path), None));
            assert!(ephe_path_depends_on_directory(
                Some(&absolute),
                Some(OsStr::new(text))
            ));
        }
        for environment in [
            OsStr::new(""),
            OsStr::new(&"r".repeat(MAX_EPHE_PATH_LEN + 1)),
        ] {
            assert!(!ephe_path_depends_on_directory(
                Some(&absolute),
                Some(environment)
            ));
            assert!(ephe_path_depends_on_directory(None, Some(environment)));
        }
        // Native Unix filenames are bytes, not necessarily UTF-8.
        let environment = OsStr::from_bytes(b"/data/\xff:/other");
        assert!(!ephe_path_depends_on_directory(
            Some(&relative),
            Some(environment)
        ));
    }
}

/// Retain the original SDK sidereal-setting reuse policy under native access.
pub(crate) fn ensure_sid_mode_locked(mode: i32, t0: f64, ayan_t0: f64) -> Result<(), Error> {
    let reusable = {
        let settings = VALIDATION_SETTINGS
            .lock()
            .map_err(|_| Error::lock_poisoned())?;
        settings.sidereal_thread == Some(configuration_thread_id())
            && settings
                .sidereal_mode
                .is_some_and(|(saved_mode, saved_t0, saved_ayan)| {
                    saved_mode == mode
                        && saved_t0.to_bits() == t0.to_bits()
                        && saved_ayan.to_bits() == ayan_t0.to_bits()
                })
    };
    if reusable {
        Ok(())
    } else {
        apply_sid_mode_locked(mode, t0, ayan_t0)
    }
}
