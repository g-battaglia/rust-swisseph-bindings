//! Library identity: native version and executable/library pathname.
//!
//! [`version`] reports the version of the linked native library, which must
//! correspond to the pinned submodule build. [`library_path`]
//! exposes the native executable/library pathname for diagnostics.

use std::ffi::c_char;

use crate::ffi::{self, LIBRARY_PATH_LEN, TEXT_BUF_LEN, copy_returned_string};
use crate::state::with_native_access;

/// Return the version string of the linked native Swiss Ephemeris library.
///
/// The value comes from the same pinned sources compiled by `build.rs`,
/// so it identifies the actual calculation engine rather than this crate's
/// own version. It is typically of the form `"2.10.03"`, without any
/// `"v"` prefix.
///
/// # Examples
///
/// ```
/// let version = swisseph_bindings::version()?;
/// assert!(!version.is_empty());
/// # Ok::<(), swisseph_bindings::Error>(())
/// ```
pub fn version() -> Result<String, crate::Error> {
    with_native_access(|| {
        let mut buffer = [0 as c_char; TEXT_BUF_LEN];
        // SAFETY: `buffer` owns TEXT_BUF_LEN writable bytes for this call.
        // The returned pointer aliases `buffer` (or is null on unexpected
        // failure); the contents are copied into an owned `String` before
        // the lock is released and no pointer escapes.
        unsafe {
            let returned = ffi::swe_version(buffer.as_mut_ptr());
            Ok(copy_returned_string(
                returned.cast_const(),
                &buffer.map(|b| b as u8),
            ))
        }
    })
}

/// Return the executable/library pathname reported by the native library.
///
/// With this static native build, the pathname identifies the executable.
/// It is independent of [`crate::set_ephe_path`]. The pinned native helper
/// may truncate it to 256 bytes; some native build variants return an empty
/// string. Invalid UTF-8 at a truncation boundary is decoded lossily.
/// The Rust result owns its contents and no native pointer escapes.
pub fn library_path() -> Result<String, crate::Error> {
    with_native_access(|| {
        let mut buffer = [0 as c_char; LIBRARY_PATH_LEN];
        // SAFETY: the pinned path helper writes at most 256 pathname
        // bytes plus its terminator. All 257 bytes are owned, initialized
        // and remain alive through the bounded copy under native access.
        unsafe {
            let returned = ffi::swe_get_library_path(buffer.as_mut_ptr());
            Ok(copy_returned_string(
                returned.cast_const(),
                &buffer.map(|b| b as u8),
            ))
        }
    })
}
