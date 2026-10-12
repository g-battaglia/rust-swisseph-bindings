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
    native_status: Option<i32>,
    native_operation: Option<&'static str>,
    native_diagnostic: Option<String>,
}

impl Error {
    /// Classify a native failure, keeping the native diagnostic text.
    pub(crate) fn native(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Native,
            message: message.into(),
            native_status: None,
            native_operation: None,
            native_diagnostic: None,
        }
    }

    /// Classify input rejected before the dependent native operation.
    pub(crate) fn invalid_input(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::InvalidInput,
            message: message.into(),
            native_status: None,
            native_operation: None,
            native_diagnostic: None,
        }
    }

    /// Classify a poisoned native lock; see [`ErrorKind::LockPoisoned`].
    pub(crate) fn lock_poisoned() -> Self {
        Self {
            kind: ErrorKind::LockPoisoned,
            native_status: None,
            native_operation: None,
            native_diagnostic: None,
            message: String::from(
                "native state lock is poisoned; configuration may be inconsistent",
            ),
        }
    }

    /// Attach an actual negative C status and original diagnostic to a failure.
    ///
    /// Called only after the named position entry returns a negative status;
    /// copy its initialized diagnostic buffer without parsing the display text.
    /// Preserve the existing contextual message. No status is inferred for a
    /// Rust preflight, non-finite output or another operation lacking transport.
    pub(crate) fn with_native_details(
        mut self,
        operation: &'static str,
        status: i32,
        diagnostic: &str,
    ) -> Self {
        self.native_status = Some(status);
        self.native_operation = Some(operation);
        self.native_diagnostic = Some(diagnostic.to_owned());
        self
    }

    /// Original negative status from a completed position C call, when retained.
    ///
    /// Dimensionless C return code, independent of the error display message.
    /// None means no transported status, including Rust-side admission errors;
    /// it must not be interpreted as zero or success. This getter performs no C
    /// call and does not infer status from diagnostic wording.
    pub fn native_status(&self) -> Option<i32> {
        self.native_status
    }

    /// Native position entry that produced the transported negative status.
    ///
    /// Return calc or calc_ut for their completed failing C calls; otherwise
    /// None. The static name carries no scientific or data-source guarantee.
    /// Pure metadata access never acquires or changes native state.
    pub fn native_operation(&self) -> Option<&'static str> {
        self.native_operation
    }

    /// Original C diagnostic text, independently of contextual display detail.
    ///
    /// Return the copied, bounded, decoded serr buffer for a failing position
    /// entry, including an empty string if C supplied one. None means this
    /// failure has no transported C diagnostic. No suffix is stripped or text
    /// reconstructed; the borrowed string uses this error's lifetime.
    pub fn native_diagnostic(&self) -> Option<&str> {
        self.native_diagnostic.as_deref()
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
