//! Application-level error codes and exception formatting.

use core::fmt;

/// Errors reported by application operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppErrorCode {
    ErrorOmapiNotSupported,
    ErrorOmapiPermissionDenied,
    ErrorOmapiSecurityexception,
    ErrorOmapiReaderNotAvailable,
    ErrorOmapiChannelOpenFailed,
    ErrorOmapiTransmitFailed,
    ErrorOmapiSessionCorrupted,
    ErrorRemoteConnectionFailed,
    ErrorRemoteTimeout,
    ErrorRemoteAuthFailed,
    ErrorRemoteWrongPassword,
    ErrorCardNotPresent,
    ErrorCardUnsupported,
    ErrorConditionOfUseNotSatisfied,
    ErrorApplicationNotFound,
    ErrorEstkMaxNotFound,
    ErrorTransactionFailed,
    ErrorChannelOpenFailed,
    ErrorNblesimSimMode,
    ErrorUnknown,
}

impl fmt::Display for AppErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::ErrorOmapiNotSupported => "ERROR_OMAPI_NOT_SUPPORTED",
            Self::ErrorOmapiPermissionDenied => "ERROR_OMAPI_PERMISSION_DENIED",
            Self::ErrorOmapiSecurityexception => "ERROR_OMAPI_SECURITYEXCEPTION",
            Self::ErrorOmapiReaderNotAvailable => "ERROR_OMAPI_READER_NOT_AVAILABLE",
            Self::ErrorOmapiChannelOpenFailed => "ERROR_OMAPI_CHANNEL_OPEN_FAILED",
            Self::ErrorOmapiTransmitFailed => "ERROR_OMAPI_TRANSMIT_FAILED",
            Self::ErrorOmapiSessionCorrupted => "ERROR_OMAPI_SESSION_CORRUPTED",
            Self::ErrorRemoteConnectionFailed => "ERROR_REMOTE_CONNECTION_FAILED",
            Self::ErrorRemoteTimeout => "ERROR_REMOTE_TIMEOUT",
            Self::ErrorRemoteAuthFailed => "ERROR_REMOTE_AUTH_FAILED",
            Self::ErrorRemoteWrongPassword => "ERROR_REMOTE_WRONG_PASSWORD",
            Self::ErrorCardNotPresent => "ERROR_CARD_NOT_PRESENT",
            Self::ErrorCardUnsupported => "ERROR_CARD_UNSUPPORTED",
            Self::ErrorConditionOfUseNotSatisfied => "ERROR_CONDITION_OF_USE_NOT_SATISFIED",
            Self::ErrorApplicationNotFound => "ERROR_APPLICATION_NOT_FOUND",
            Self::ErrorEstkMaxNotFound => "ERROR_ESTK_MAX_NOT_FOUND",
            Self::ErrorTransactionFailed => "ERROR_TRANSACTION_FAILED",
            Self::ErrorChannelOpenFailed => "ERROR_CHANNEL_OPEN_FAILED",
            Self::ErrorNblesimSimMode => "ERROR_NBLESIM_SIM_MODE",
            Self::ErrorUnknown => "ERROR_UNKNOWN",
        };
        f.write_str(name)
    }
}

/// An application error with optional message and original-error text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppException {
    pub code: AppErrorCode,
    pub message: Option<String>,
    pub original_error: Option<String>,
}

impl AppException {
    /// Creates an exception with optional message and original-error text.
    #[must_use]
    pub fn new(
        code: AppErrorCode,
        message: Option<String>,
        original_error: Option<String>,
    ) -> Self {
        Self {
            code,
            message,
            original_error,
        }
    }
}

impl fmt::Display for AppException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AppException: {}", self.code)?;
        if let Some(message) = &self.message {
            write!(f, " ({message})")?;
        }
        if let Some(original_error) = &self.original_error {
            write!(f, "\nOriginal Error: {original_error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for AppException {}
