//! TS.43 error definitions and HTTP error handling.
//!
//! Ported from `errors.go` / `errors.dart`.

use core::fmt;
use std::collections::HashMap;
use std::error::Error;

use super::enums::Operation;

/// Carrier rejected a TS.43 operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarrierDeniedError {
    pub operation: Operation,
    pub general_error_text: String,
}

impl CarrierDeniedError {
    #[must_use]
    pub fn new(operation: Operation, general_error_text: impl Into<String>) -> Self {
        Self {
            operation,
            general_error_text: general_error_text.into(),
        }
    }
}

impl fmt::Display for CarrierDeniedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.general_error_text.is_empty() {
            write!(
                f,
                "{} denied: {}",
                self.operation.as_str(),
                self.general_error_text
            )
        } else {
            write!(f, "{} denied", self.operation.as_str())
        }
    }
}

impl Error for CarrierDeniedError {}

/// HTTP error carrying the status code and optional body/headers, used for 511 retry handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ts43HttpError {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Ts43HttpError {
    #[must_use]
    pub fn new(status_code: u16, headers: HashMap<String, String>, body: Vec<u8>) -> Self {
        Self {
            status_code,
            headers,
            body,
        }
    }
}

impl fmt::Display for Ts43HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TS.43 HTTP status {}", self.status_code)
    }
}

impl Error for Ts43HttpError {}

/// Entitlement discovery error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitlementException(pub String);

impl fmt::Display for EntitlementException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for EntitlementException {}

pub const ENTITLEMENT_NOT_FOUND: &str = "entitlement config not found";
pub const ENTITLEMENT_AMBIGUOUS: &str = "entitlement config is ambiguous";
pub const ENTITLEMENT_UNSUPPORTED: &str = "entitlement transfer is unsupported";

/// Websheet flow control error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebsheetException(pub String);

impl fmt::Display for WebsheetException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for WebsheetException {}

pub const WEBSHEET_DISMISSED: &str = "websheet dismissed";
pub const WEBSHEET_DEFAULT_SMDP_UNSUPPORTED: &str =
    "websheet default SM-DP+ callback is unsupported";

#[must_use]
pub fn is_http_status(err: &(dyn Error + 'static), status: u16) -> bool {
    if let Some(http_err) = err.downcast_ref::<Ts43HttpError>() {
        http_err.status_code == status
    } else {
        false
    }
}
