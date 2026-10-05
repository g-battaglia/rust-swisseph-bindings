//! Process-wide discipline for native state access.
//!
//! The pinned native library keeps its configuration (ephemeris path,
//! observer position, sidereal mode, tidal acceleration, cached file data)
//! in process-global storage. On Apple targets the native build additionally
//! compiles without thread-local storage (`TLS` is empty on `__APPLE__` in
//! `sweodef.h`), so every thread observes the same native state.
//!
//! Every public binding therefore runs its complete native sequence —
//! configuration plus all dependent computation — while holding one private
//! process-wide mutex. Free `set_*` + `calc_*` pairs are each individually
//! serialized but remain two acquisitions: callers needing atomic
//! config-plus-computation must use [`crate::Session`], which holds this
//! same lock across its whole apply-then-compute sequence. Two Rust threads
//! can never interleave conflicting configurations mid-sequence. A lock on
//! a single Rust handle would not be sufficient because the sharing happens
//! inside the native library.
//!
//! Lock poisoning (a panic while native state was held) is reported as a
//! classified [`Error`] instead of continuing with possibly
//! inconsistent configuration. No `Drop` implementation calls the native
//! `swe_close`: closing is an explicit process-wide operation and one
//! handle must never invalidate another handle's data or configuration.
//!
//! Same-thread reentry is rejected without hanging: the underlying mutex
//! is not reentrant, so a nested `with_native_access` on the thread that
//! already holds native access returns [`ErrorKind::InvalidInput`](crate::ErrorKind)
//! instead of deadlocking. Cross-thread contention still blocks on the
//! mutex. No user callbacks run under the lock: the closure must be a
//! bounded native sequence built from already-locked helpers.
//!
//! No `Send`/`Sync` unsafe implementations are added here. The public API
//! consists of free functions plus [`crate::Session`], which are callable
//! from any thread precisely because each native sequence serializes
//! through this boundary.

use std::cell::Cell;
use std::sync::Mutex;

use crate::error::Error;

/// The single process-wide native access lock.
static NATIVE_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    /// Depth of `with_native_access` on this thread. Non-zero means this
    /// thread already holds the native lock; a nested acquisition would
    /// deadlock on the non-reentrant mutex, so it is rejected instead.
    static NATIVE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// Guard resetting the thread-local depth even when `operation` panics.
struct DepthGuard;

impl Drop for DepthGuard {
    fn drop(&mut self) {
        NATIVE_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// Run `operation` with exclusive access to the native library.
///
/// The closure must contain the whole dependent native sequence, from any
/// configuration writes to the final read of output buffers. A poisoned
/// lock short-circuits to [`Error`] without running the closure. A nested
/// call on the thread already holding access is rejected with
/// `InvalidInput` instead of deadlocking.
pub(crate) fn with_native_access<R>(
    operation: impl FnOnce() -> Result<R, Error>,
) -> Result<R, Error> {
    if NATIVE_DEPTH.with(|depth| depth.get()) > 0 {
        return Err(Error::invalid_input(
            "native access is already held on this thread; nested config-plus-computation is unsupported (use one Session call per atomic sequence)",
        ));
    }
    match NATIVE_LOCK.lock() {
        Ok(_guard) => {
            NATIVE_DEPTH.with(|depth| depth.set(depth.get().saturating_add(1)));
            let _depth = DepthGuard;
            operation()
        }
        Err(_) => Err(Error::lock_poisoned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    // NOTE: this test deliberately poisons the process-wide `NATIVE_LOCK`,
    // so it must remain the only unit test in the library test binary:
    // every `#[cfg(test)]` module in this crate shares one process, and a
    // poisoned static mutex stays poisoned for the rest of that process.
    // Integration tests and doctests run in separate processes and are
    // unaffected.
    #[test]
    fn poisoned_lock_is_reported_as_classified_error() {
        // Panic while the lock is held: `with_native_access` holds the
        // mutex guard across `operation`, so unwinding through the guard
        // poisons `NATIVE_LOCK`, modelling a panic during a native call.
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _: Result<(), Error> = with_native_access(|| {
                panic!("intentional poison probe");
            });
        }));
        assert!(
            caught.is_err(),
            "probe panic must unwind through the lock guard"
        );
        // Every later acquisition short-circuits without running the closure.
        let mut ran = false;
        let err = with_native_access(|| {
            ran = true;
            Ok::<(), Error>(())
        })
        .expect_err("poisoned lock must surface as an error");
        assert!(!ran, "closure must not run once the lock is poisoned");
        assert_eq!(err.kind(), ErrorKind::LockPoisoned);
        assert!(
            err.message().contains("poisoned"),
            "diagnostic must name the poisoned lock, got: {}",
            err.message()
        );
    }
}
