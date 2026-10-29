//! Primary error with the original native error retained as its source.
use super::ErrorKind;
use std::{fmt, sync::Arc};

/// Classified operation failure retaining all available native error channels.
///
/// The native cause is owned rather than reconstructed from display text.
/// Status and scalar channels are distinct; an absent channel remains None.
#[derive(Clone, Debug)]
pub struct Error {
    kind: ErrorKind,
    operation: Option<String>,
    status: Option<i32>,
    scalar: Option<f64>,
    diagnostic: Option<String>,
    source: Option<Arc<crate::error::Error>>,
    /// A primary admission reason, separate from absent native channels.
    message: Option<String>,
}
impl Error {
    /// Construct an error from explicitly supplied channels, without a cause.
    ///
    /// # Arguments
    ///
    /// * `kind` - Structured failure class.
    /// * `operation` - Producing operation, when known.
    /// * `native_status` - Actual native integer result, when supplied.
    /// * `native_scalar` - Actual native scalar result, when supplied.
    /// * `diagnostic` - Original diagnostic text, when supplied.
    ///
    /// # Returns
    ///
    /// An owned error; no status or diagnostic is inferred.
    pub fn new(
        kind: ErrorKind,
        operation: Option<String>,
        native_status: Option<i32>,
        native_scalar: Option<f64>,
        diagnostic: Option<String>,
    ) -> Self {
        Self {
            kind,
            operation,
            status: native_status,
            scalar: native_scalar,
            diagnostic,
            source: None,
            message: None,
        }
    }
    /// Structured failure class, independent of diagnostic wording.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
    /// Producing operation, when the producer supplied one.
    pub fn operation(&self) -> Option<&str> {
        self.operation.as_deref()
    }
    /// Actual integer return channel, when present.
    pub fn native_status(&self) -> Option<i32> {
        self.status
    }
    /// Actual floating-point return channel, when present.
    pub fn native_scalar(&self) -> Option<f64> {
        self.scalar
    }
    /// Original diagnostic channel, including an explicitly empty string.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// A request shape cannot supply a physical input required by the native operation.
    pub(super) fn unsupported(operation: &str, reason: &str) -> Self {
        let mut error = Self::new(
            ErrorKind::Unsupported,
            Some(operation.to_owned()),
            None,
            None,
            None,
        );
        error.message = Some(reason.to_owned());
        error
    }
    pub(super) fn invalid(operation: &str, reason: &str) -> Self {
        let cause = crate::error::Error::invalid_input(format!("{operation}: {reason}"));
        let mut error = Self::from_native(cause);
        error.operation = Some(operation.to_owned());
        error
    }
}
impl Error {
    pub(super) fn from_native(source: crate::error::Error) -> Self {
        let kind = match source.kind() {
            crate::error::ErrorKind::InvalidInput => ErrorKind::InvalidInput,
            crate::error::ErrorKind::Native => ErrorKind::Calculation,
            crate::error::ErrorKind::LockPoisoned => ErrorKind::Unavailable,
        };
        Self {
            kind,
            operation: source.native_operation().map(str::to_owned),
            status: source.native_status(),
            scalar: source.native_scalar_return(),
            diagnostic: source.native_diagnostic().map(str::to_owned),
            source: Some(Arc::new(source)),
            message: None,
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(message) = &self.message {
            if let Some(operation) = &self.operation {
                write!(f, "{operation}: ")?;
            }
            return f.write_str(message);
        }
        if let Some(source) = &self.source {
            return source.fmt(f);
        }
        if let Some(text) = &self.diagnostic {
            return f.write_str(text);
        }
        write!(f, "{:?}", self.kind)?;
        Ok(())
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn std::error::Error + 'static))
    }
}
