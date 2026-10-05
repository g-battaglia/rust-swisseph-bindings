//! Atomic configured operations over process-global native state.
//!
//! Free `set_*` functions and the calculation that depends on them are two
//! separate acquisitions of the process-wide native lock, so two threads
//! can interleave (`set_topo(A)` → another thread's `set_topo(B)` → first
//! thread's `calc_ut` sees `B`). [`Session`] closes that gap: it owns a
//! validated configuration and every computation applies that configuration and
//! runs one dependent computation inside a single acquisition of the same
//! lock used by the free functions.
//!
//! # Scope
//!
//! A session covers every writable configuration knob of the binding:
//! ephemeris path, JPL file selection, observer position, sidereal mode
//! (including `SIDM_USER` parameters), tidal acceleration, Delta-T
//! override and lapse rate. Each computation method documents which knobs
//! it reads. Unset knobs are inherited from whatever the process-global
//! state currently holds; there is no snapshot/restore because the native
//! library offers no getters for most settings. After a session call the
//! applied values stay installed globally (last writer wins); the atomicity
//! guarantee is that the session's own computation observed its own
//! configuration, not that later free calls are isolated. Checks that require
//! the applied configuration run under the lock after setters; even if they
//! return `InvalidInput`, applied settings may remain installed. Rejection
//! preserves the session's file snapshot, not the previous global configuration.
//!
//! # Reset semantics
//!
//! - Ephemeris path: [`SessionBuilder::ephe_path`] selects a directory,
//!   [`SessionBuilder::default_ephe_path`] restores the compiled default
//!   (the native null-pointer behavior). Unset inherits.
//! - JPL file: [`SessionBuilder::jpl_file`] selects a file,
//!   [`SessionBuilder::close_jpl_file`] closes the open file by passing
//!   the native empty string. Unset inherits.
//! - Observer / sidereal / tidal / lapse rate: set methods install a
//!   value; unset inherits. Process defaults are the native fresh-process
//!   values (observer `0,0,0`, sidereal Fagan/Bradley, automatic tidal
//!   selection, compiled lapse rate): the session never invents getters,
//!   it only writes.
//! - Delta T: [`SessionBuilder::delta_t_override`] pins a value in days,
//!   [`SessionBuilder::clear_delta_t_override`] resumes computed values
//!   via the native automatic sentinel. Unset inherits.
//!
//! # Observer side effects
//!
//! Observer-taking searches ([`Session::rise_trans`]) install their
//! per-call observer as the process-global observer, exactly like the free
//! functions (established by probing the pinned build). A session applies
//! its configured observer first and then runs the search with the
//! search's own observer, so after such a call the global observer is the
//! search's observer, not the session's configured one. The search result
//! itself is still computed atomically from the search's observer.
//!
//! # Discipline
//!
//! - One lock: sessions use the same process-wide boundary as every free
//!   function, so [`crate::close`] and data-identity reads
//!   ([`Session::get_current_file_data`]) cannot interleave mid-sequence.
//! - No reentry: calling any binding entry (free function or another
//!   session method) from inside a session method on the same thread is
//!   rejected with `InvalidInput` instead of deadlocking. Cross-thread
//!   contention blocks on the mutex.
//! - No callbacks run under the lock and no `Drop` closes native state:
//!   sessions are plain owned Rust values (`Send`/`Sync` by auto-trait,
//!   no unsafe impls) and closing stays an explicit [`crate::close`]
//!   call.
//! - Free functions are preserved with their global-state semantics; they
//!   remain non-atomic across `set_*` + compute pairs by design.
//! - No isolation is promised against raw callers outside this binding's
//!   locking contract that touch the native library directly.

use std::ffi::CString;
use std::sync::{Arc, Mutex};

use crate::config::{
    MAX_EPHE_PATH_LEN, MAX_JPL_FILE_LEN, apply_delta_t_locked, apply_ephe_path_locked,
    apply_jpl_file_locked, apply_sid_mode_locked, apply_tid_acc_locked, apply_topo_locked,
    check_sidereal_request_locked, get_ayanamsa_ut_locked, get_current_file_data_locked,
};
use crate::error::Error;
use crate::houses::{Houses, check_houses_inputs, check_system, houses_ex_locked, houses_locked};
use crate::observer::{
    RiseTransitOutcome, apply_lapse_rate_locked, check_rise_inputs, rise_trans_locked,
};
use crate::positions::{Position, calc_locked, calc_ut_locked};
use crate::state::with_native_access;
use crate::time::deltat_locked;

/// Builder for [`Session`].
///
/// Each knob is optional: unset knobs inherit the process-global state at
/// call time. [`SessionBuilder::build`] validates owned inputs and checks
/// UT-based user sidereal epochs under native access without installing
/// the session configuration. A temporary Delta-T preflight restores the
/// inherited override before returning. Configuration-dependent checks
/// run again with the complete applied configuration at computation time.
#[derive(Debug, Clone, Default)]
pub struct SessionBuilder {
    ephe_path: Option<Option<String>>,
    jpl_file: Option<Option<String>>,
    topo: Option<(f64, f64, f64)>,
    sid_mode: Option<(i32, f64, f64)>,
    tid_acc: Option<f64>,
    delta_t: Option<Option<f64>>,
    lapse_rate: Option<f64>,
}

impl SessionBuilder {
    /// Start a session builder with every knob unset (fully inherited).
    pub fn new() -> Self {
        Self::default()
    }

    /// Select the ephemeris data directory for the session.
    pub fn ephe_path(&mut self, path: &str) -> &mut Self {
        self.ephe_path = Some(Some(path.to_string()));
        self
    }

    /// Restore the compiled default search path for the session.
    pub fn default_ephe_path(&mut self) -> &mut Self {
        self.ephe_path = Some(None);
        self
    }

    /// Select the JPL file for `FLG_JPLEPH` calculations in the session.
    pub fn jpl_file(&mut self, fname: &str) -> &mut Self {
        self.jpl_file = Some(Some(fname.to_string()));
        self
    }

    /// Close any open JPL file for the session (native empty string).
    pub fn close_jpl_file(&mut self) -> &mut Self {
        self.jpl_file = Some(None);
        self
    }

    /// Set the geographic observer (east-positive degrees, degrees north,
    /// meters) for the session. [`Self::build`] checks the arithmetic height
    /// limit documented by [`crate::set_topo`].
    pub fn topo(&mut self, longitude: f64, latitude: f64, altitude: f64) -> &mut Self {
        self.topo = Some((longitude, latitude, altitude));
        self
    }

    /// Select the sidereal mode for the session; `t0`/`ayan_t0`
    /// parameterize `SIDM_USER`. [`Self::build`] checks the reference epoch
    /// domain documented by [`crate::set_sid_mode`].
    pub fn sid_mode(&mut self, mode: i32, t0: f64, ayan_t0: f64) -> &mut Self {
        self.sid_mode = Some((mode, t0, ayan_t0));
        self
    }

    /// Set the tidal acceleration (arcsec/cy²) for the session.
    pub fn tid_acc(&mut self, acc: f64) -> &mut Self {
        self.tid_acc = Some(acc);
        self
    }

    /// Pin Delta T (TT minus UT, in days) for the session.
    pub fn delta_t_override(&mut self, dt: f64) -> &mut Self {
        self.delta_t = Some(Some(dt));
        self
    }

    /// Resume computed Delta T for the session.
    pub fn clear_delta_t_override(&mut self) -> &mut Self {
        self.delta_t = Some(None);
        self
    }

    /// Override the lapse rate (K/m) for the session.
    pub fn lapse_rate(&mut self, rate: f64) -> &mut Self {
        self.lapse_rate = Some(rate);
        self
    }

    /// Validate the builder without installing its configuration.
    ///
    /// UT-based user sidereal epochs include explicit or inherited Delta-T.
    /// Changes to the computed model/data selection are validated with the
    /// complete configuration when a dependent session operation runs.
    /// Any temporary override used for this preflight is restored while
    /// holding the native lock, including on rejection.
    ///
    /// # Errors
    ///
    /// Interior NUL bytes, overlong paths/names, non-finite numerics
    /// and unrepresentable user reference epochs/effective shifts are
    /// rejected with `InvalidInput`.
    pub fn build(&self) -> Result<Session, Error> {
        let ephe_path = match &self.ephe_path {
            None => None,
            Some(None) => Some(None),
            Some(Some(text)) => {
                if text.as_bytes().contains(&0) {
                    return Err(Error::invalid_input(
                        "SessionBuilder: ephemeris path contains an interior NUL byte",
                    ));
                }
                if text.len() > MAX_EPHE_PATH_LEN {
                    return Err(Error::invalid_input(format!(
                        "SessionBuilder: ephemeris path is {} bytes, longer than the {MAX_EPHE_PATH_LEN}-byte native limit",
                        text.len()
                    )));
                }
                Some(Some(CString::new(text.as_str()).map_err(|_| {
                    Error::invalid_input(
                        "SessionBuilder: ephemeris path contains an interior NUL byte",
                    )
                })?))
            }
        };
        let jpl_file = match &self.jpl_file {
            None => None,
            Some(None) => Some(None),
            Some(Some(text)) => {
                if text.as_bytes().contains(&0) {
                    return Err(Error::invalid_input(
                        "SessionBuilder: JPL file name contains an interior NUL byte",
                    ));
                }
                if text.len() > MAX_JPL_FILE_LEN {
                    return Err(Error::invalid_input(format!(
                        "SessionBuilder: JPL file name is {} bytes, longer than the {MAX_JPL_FILE_LEN}-byte native limit",
                        text.len()
                    )));
                }
                Some(Some(CString::new(text.as_str()).map_err(|_| {
                    Error::invalid_input(
                        "SessionBuilder: JPL file name contains an interior NUL byte",
                    )
                })?))
            }
        };
        if let Some((lon, lat, alt)) = self.topo
            && (!lon.is_finite() || !lat.is_finite() || !alt.is_finite())
        {
            return Err(Error::invalid_input(
                "SessionBuilder: observer longitude, latitude and altitude must be finite",
            ));
        }
        if let Some((_, t0, ayan_t0)) = self.sid_mode
            && (!t0.is_finite() || !ayan_t0.is_finite())
        {
            return Err(Error::invalid_input(
                "SessionBuilder: sidereal t0 and ayan_t0 must be finite",
            ));
        }
        if let Some(acc) = self.tid_acc
            && !acc.is_finite()
        {
            return Err(Error::invalid_input(
                "SessionBuilder: tidal acceleration must be finite",
            ));
        }
        if let Some((mode, t0, _)) = self.sid_mode
            && mode & 255 == crate::SIDM_USER
        {
            crate::domain::check_et("SessionBuilder: user reference epoch", t0)?;
        }
        if let Some((_, _, altitude)) = self.topo {
            crate::domain::check_observer_height("SessionBuilder", altitude)?;
        }
        if let Some(Some(dt)) = self.delta_t
            && !dt.is_finite()
        {
            return Err(Error::invalid_input(
                "SessionBuilder: Delta-T override must be finite",
            ));
        }
        if let Some(rate) = self.lapse_rate
            && !rate.is_finite()
        {
            return Err(Error::invalid_input(
                "SessionBuilder: lapse rate must be finite",
            ));
        }
        if let Some((mode, t0, _)) = self.sid_mode {
            with_native_access(|| {
                check_sidereal_request_locked(
                    "SessionBuilder: user reference epoch",
                    mode,
                    t0,
                    self.delta_t,
                    self.tid_acc.is_some() || self.ephe_path.is_some() || self.jpl_file.is_some(),
                )
            })?;
        }
        Ok(Session {
            ephe_path,
            jpl_file,
            topo: self.topo,
            sid_mode: self.sid_mode,
            tid_acc: self.tid_acc,
            delta_t: self.delta_t,
            lapse_rate: self.lapse_rate,
            file_data: Arc::new(Mutex::new(std::array::from_fn(|_| None))),
        })
    }
}

/// An owned, validated native configuration applied atomically with each
/// computation.
///
/// Build with [`SessionBuilder::build`]. Every computation validates its
/// configuration-independent input checks before acquiring the lock, then
/// applies the whole session configuration and checks dependent domains before
/// native computation in the same acquisition. A rejection after configuration
/// may leave applied settings installed globally; no transactional rollback is
/// promised. See the module documentation for inheritance, reset and observer
/// side-effect semantics.
/// File metadata is captured before releasing native access and can be
/// queried later without changing native state. Clones share this history;
/// separately built sessions have independent histories.
#[derive(Debug, Clone)]
pub struct Session {
    ephe_path: Option<Option<CString>>,
    jpl_file: Option<Option<CString>>,
    topo: Option<(f64, f64, f64)>,
    sid_mode: Option<(i32, f64, f64)>,
    tid_acc: Option<f64>,
    delta_t: Option<Option<f64>>,
    lapse_rate: Option<f64>,
    /// Owned copies of slots 0..=4, the range accepted by the pinned
    /// `swe_get_current_file_data`. Only accessed under the native lock;
    /// the inner mutex provides safe interior mutability for `&self`.
    file_data: Arc<Mutex<[Option<crate::CurrentFileData>; 5]>>,
}

impl Session {
    /// Apply configuration, compute and capture native file slots under
    /// one acquisition. No user-provided callback reaches this helper.
    /// Lock order is always native access, then the metadata mutex.
    fn compute<R>(&self, operation: impl FnOnce() -> Result<R, Error>) -> Result<R, Error> {
        with_native_access(|| {
            self.apply_locked()?;
            let result = operation();
            if result
                .as_ref()
                .is_err_and(|error| error.kind() == crate::ErrorKind::InvalidInput)
            {
                return result;
            }
            let files = std::array::from_fn(|slot| get_current_file_data_locked(slot as i32));
            *self.file_data.lock().map_err(|_| Error::lock_poisoned())? = files;
            result
        })
    }

    /// Apply the owned configuration. The caller must hold the native
    /// lock (`compute` establishes one native acquisition wrapping this,
    /// the computation and the metadata snapshot).
    fn apply_locked(&self) -> Result<(), Error> {
        if let Some(path) = &self.ephe_path {
            apply_ephe_path_locked(path.as_ref());
        }
        if let Some(jpl) = &self.jpl_file {
            match jpl {
                Some(name) => apply_jpl_file_locked(name),
                // SAFETY: an empty `CString` is a valid empty C string;
                // the native setter treats it as "close the JPL file".
                None => apply_jpl_file_locked(&CString::new("").unwrap_or_default()),
            }
        }
        if let Some((lon, lat, alt)) = self.topo {
            apply_topo_locked(lon, lat, alt);
        }
        if let Some((mode, t0, ayan_t0)) = self.sid_mode {
            apply_sid_mode_locked(mode, t0, ayan_t0)?;
        }
        if let Some(acc) = self.tid_acc {
            apply_tid_acc_locked(acc)?;
        }
        if let Some(dt) = self.delta_t {
            apply_delta_t_locked(dt.unwrap_or(crate::SE_DELTAT_AUTOMATIC))?;
        }
        if let Some(rate) = self.lapse_rate {
            apply_lapse_rate_locked(rate);
        }
        Ok(())
    }

    /// Body position for a Universal Time Julian Day.
    ///
    /// Reads the session observer, sidereal mode, data path/JPL file,
    /// tidal and Delta-T state atomically with the computation. `body`
    /// and `flags` follow [`crate::calc_ut`].
    pub fn calc_ut(&self, jd_ut: f64, body: i32, flags: i32) -> Result<Position, Error> {
        if !jd_ut.is_finite() {
            return Err(Error::invalid_input(format!(
                "Session::calc_ut: Julian Day must be finite ({jd_ut})"
            )));
        }
        self.compute(|| calc_ut_locked(jd_ut, body, flags))
    }

    /// Body position for an Ephemeris Time Julian Day.
    ///
    /// Same configuration dependence as [`Session::calc_ut`] through the
    /// native `swe_calc` entry point.
    pub fn calc(&self, jd_et: f64, body: i32, flags: i32) -> Result<Position, Error> {
        if !jd_et.is_finite() {
            return Err(Error::invalid_input(format!(
                "Session::calc: Julian Day must be finite ({jd_et})"
            )));
        }
        self.compute(|| calc_locked(jd_et, body, flags))
    }

    /// House cusps and angles for a Universal Time Julian Day.
    ///
    /// Reads the session data path (house systems with an internal solar
    /// position) atomically with the computation. Arguments follow
    /// [`crate::houses()`].
    pub fn houses(
        &self,
        jd_ut: f64,
        latitude: f64,
        longitude: f64,
        hsys: u8,
    ) -> Result<Houses, Error> {
        check_system(hsys)?;
        check_houses_inputs("Session::houses", jd_ut, latitude, longitude)?;
        self.compute(|| houses_locked(jd_ut, latitude, longitude, hsys))
    }

    /// House cusps and angles with ephemeris flags.
    ///
    /// Same configuration dependence as [`Session::houses`]; arguments
    /// follow [`crate::houses_ex`].
    pub fn houses_ex(
        &self,
        jd_ut: f64,
        flags: i32,
        latitude: f64,
        longitude: f64,
        hsys: u8,
    ) -> Result<Houses, Error> {
        check_system(hsys)?;
        check_houses_inputs("Session::houses_ex", jd_ut, latitude, longitude)?;
        self.compute(|| houses_ex_locked(jd_ut, flags, latitude, longitude, hsys))
    }

    /// Ayanamsha (tropical-sidereal offset) in degrees for a UT instant.
    ///
    /// Reads the session sidereal mode and data path atomically with the
    /// computation.
    pub fn get_ayanamsa_ut(&self, jd_ut: f64) -> Result<f64, Error> {
        if !jd_ut.is_finite() {
            return Err(Error::invalid_input(format!(
                "Session::get_ayanamsa_ut: Julian Day must be finite ({jd_ut})"
            )));
        }
        self.compute(|| get_ayanamsa_ut_locked(jd_ut))
    }

    /// Delta T (TT minus UT) in days for a UT instant.
    ///
    /// Reads the session tidal acceleration, Delta-T override and data
    /// path atomically with the computation.
    pub fn deltat(&self, jd_ut: f64) -> Result<f64, Error> {
        if !jd_ut.is_finite() {
            return Err(Error::invalid_input(format!(
                "Session::deltat: Julian Day must be finite ({jd_ut})"
            )));
        }
        self.compute(|| Ok(deltat_locked(jd_ut)))
    }

    /// Next rise/set/transit after a UT instant, seen from an explicit
    /// observer.
    ///
    /// The session configuration (data path for file-based sources) is
    /// applied atomically with the search; the search's own observer is
    /// passed per call and — like the free function — becomes the
    /// process-global observer afterwards. Arguments follow
    /// [`crate::rise_trans`].
    #[allow(clippy::too_many_arguments)]
    pub fn rise_trans(
        &self,
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
            "Session::rise_trans",
            jd_ut,
            star_name,
            longitude,
            latitude,
            altitude,
            atpress,
            attemp,
            None,
        )?;
        self.compute(|| {
            rise_trans_locked(
                jd_ut, body, star_name, epheflag, rsmi, longitude, latitude, altitude, atpress,
                attemp,
            )
        })
    }

    /// Report a native file slot captured after this session's last
    /// computation attempt.
    ///
    /// The snapshot is copied under the computation's native lock, before
    /// another call can replace the file state. This query does not apply
    /// configuration or read current native slots. Other sessions, free
    /// calls and [`crate::close`] therefore cannot change the snapshot.
    /// Clones of this session share it; the latest completed computation
    /// on any clone wins, including a native failure. Inputs rejected
    /// before native access leave it untouched.
    ///
    /// Slots and fields follow [`crate::get_current_file_data`]. Returns
    /// `None` before the first computation, for an empty slot, or for an
    /// ID outside `0..=4`. The report describes native slots at the end of
    /// the operation, not a complete list of sources used by the result;
    /// use returned source flags to detect fallback.
    pub fn get_current_file_data(
        &self,
        ifno: i32,
    ) -> Result<Option<crate::CurrentFileData>, Error> {
        with_native_access(|| {
            let files = self.file_data.lock().map_err(|_| Error::lock_poisoned())?;
            Ok(usize::try_from(ifno)
                .ok()
                .and_then(|slot| files.get(slot))
                .cloned()
                .flatten())
        })
    }
}
