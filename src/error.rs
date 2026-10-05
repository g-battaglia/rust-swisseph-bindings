//! Classified errors returned by the safe binding API.
//!
//! A native failure (unknown body, missing data, polar house failure) is
//! distinct from invalid Rust-side input (interior NUL in a path) and from
//! a poisoned native lock. The native diagnostic text is preserved in the
//! message whenever the C call provides one; a valid zero or an empty
//! diagnostic is never treated as an error on its own.

use std::fmt;

/// Machine-readable classification of a binding error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The native library reported a failure, rejected the request, or
    /// produced non-finite components in a finite-result operation.
    /// The message carries the native diagnostic (`serr`) when available.
    Native,
    /// A Rust-side argument cannot be passed to the native library
    /// (interior NUL byte, overlong path, unsupported layout or arithmetic
    /// domain). Safe native preflights may run, but the dependent native
    /// operation is not invoked.
    InvalidInput,
    /// The process-wide native lock was poisoned by a panic that happened
    /// while native state was held. Configuration may be inconsistent, so
    /// no further native call is attempted.
    LockPoisoned,
}

/// Error returned by fallible binding functions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    /// Classify a native failure, keeping the native diagnostic text.
    pub(crate) fn native(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Native,
            message: message.into(),
        }
    }

    /// Classify input rejected before the dependent native operation.
    pub(crate) fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::InvalidInput,
            message: message.into(),
        }
    }

    /// Classify a poisoned native lock; see [`ErrorKind::LockPoisoned`].
    pub(crate) fn lock_poisoned() -> Self {
        Self {
            kind: ErrorKind::LockPoisoned,
            message: String::from(
                "native state lock is poisoned; configuration may be inconsistent",
            ),
        }
    }

    /// Machine-readable classification of this error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Human-readable detail, including the native diagnostic when present.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let origin = match self.kind {
            ErrorKind::Native => "native error",
            ErrorKind::InvalidInput => "invalid input",
            ErrorKind::LockPoisoned => "native lock poisoned",
        };
        write!(f, "{origin}: {}", self.message)
    }
}

impl std::error::Error for Error {}
