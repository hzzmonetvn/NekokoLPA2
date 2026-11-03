//! GSMA SGP.22 ES9+ protocol endpoints, HTTP headers, request formatters,
//! and response payload parsers.

use core::fmt;
use std::collections::BTreeMap;
use std::error::Error;

use crate::crypto::{base64_decode, CryptoError};

pub const ENDPOINT_INITIATE_AUTHENTICATION: &str = "/gsma/rsp2/es9plus/initiateAuthentication";
pub const ENDPOINT_AUTHENTICATE_CLIENT: &str = "/gsma/rsp2/es9plus/authenticateClient";
pub const ENDPOINT_GET_BOUND_PROFILE_PACKAGE: &str = "/gsma/rsp2/es9plus/getBoundProfilePackage";
pub const ENDPOINT_CANCEL_SESSION: &str = "/gsma/rsp2/es9plus/cancelSession";
pub const ENDPOINT_HANDLE_NOTIFICATION: &str = "/gsma/rsp2/es9plus/handleNotification";

/// Returns standard GSMA SGP.22 ES9+ HTTP request headers.
#[must_use]
pub fn build_es9_headers() -> [(&'static str, &'static str); 3] {
    [
        ("User-Agent", "gsma-rsp-lpad"),
        ("X-Admin-Protocol", "gsma/rsp/v2.2.0"),
        ("Content-Type", "application/json"),
    ]
}

/// Errors occurring during GSMA SGP.22 ES9+ protocol processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Es9Error {
    /// JSON syntax error during decoding.
    InvalidJson(String),
    /// Response missing the required GSMA ES9+ `header` object.
    MissingHeader,
    /// Response missing mandatory data field.
    MissingField(&'static str),
    /// SM-DP+ returned a failed execution status with SGP.22 subject and reason codes.
    ExecutionFailed {
        subject_code: String,
        reason_code: String,
        subject_identifier: Option<String>,
        message: Option<String>,
        status_code: Option<u16>,
    },
    /// Base64 decoding failed on ASN.1 payload data.
    Base64Decode(String),
    /// HTTP transport failure.
    HttpError { status: u16, message: String },
}

impl fmt::Display for Es9Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(err) => write!(f, "invalid ES9+ JSON: {err}"),
            Self::MissingHeader => write!(f, "missing ES9+ response header"),
            Self::MissingField(name) => write!(f, "missing mandatory ES9+ field: '{name}'"),
            Self::ExecutionFailed {
                subject_code,
                reason_code,
                subject_identifier,
                message,
                status_code,
            } => {
                write!(f, "SM-DP+ error ({subject_code}, {reason_code})")?;
                if let Some(code) = status_code {
                    write!(f, " [HTTP {code}]")?;
                }
                if let Some(id) = subject_identifier {
                    write!(f, " Identifier: {id}")?;
                }
                if let Some(msg) = message {
                    write!(f, " Message: {msg}")?;
                }
                Ok(())
            }
            Self::Base64Decode(err) => write!(f, "base64 decode failed: {err}"),
            Self::HttpError { status, message } => {
                write!(f, "HTTP error {status}: {message}")
            }
        }
    }
}

impl Error for Es9Error {}

impl From<CryptoError> for Es9Error {
    fn from(err: CryptoError) -> Self {
        Self::Base64Decode(err.to_string())
    }
}

// ---------------------------------------------------------------------------
// Lightweight Zero-Dependency JSON Parser
// ---------------------------------------------------------------------------

/// Simple JSON value tree for ES9+ payloads.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.as_object().and_then(|map| map.get(key))
    }

    #[must_use]
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(JsonValue::as_str)
    }
}

struct JsonParser<'a> {
    chars: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.as_bytes(),
            pos: 0,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len()
            && matches!(self.chars[self.pos], b' ' | b'\t' | b'\r' | b'\n')
        {
            self.pos += 1;
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, Es9Error> {
        self.skip_whitespace();
        if self.pos >= self.chars.len() {
            return Err(Es9Error::InvalidJson("unexpected end of input".into()));
        }

        match self.chars[self.pos] {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => self.parse_string().map(JsonValue::String),
            b't' | b'f' => self.parse_bool(),
            b'n' => self.parse_null(),
            b'-' | b'0'..=b'9' => self.parse_number(),
            other => Err(Es9Error::InvalidJson(format!(
                "unexpected character: '{}' at index {}",
                other as char, self.pos
            ))),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, Es9Error> {
        self.pos += 1; // skip '{'
        let mut map = BTreeMap::new();
        self.skip_whitespace();

        if self.pos < self.chars.len() && self.chars[self.pos] == b'}' {
            self.pos += 1;
            return Ok(JsonValue::Object(map));
        }

        loop {
            self.skip_whitespace();
            if self.pos >= self.chars.len() || self.chars[self.pos] != b'"' {
                return Err(Es9Error::InvalidJson(
                    "expected string key in object".into(),
                ));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();

            if self.pos >= self.chars.len() || self.chars[self.pos] != b':' {
                return Err(Es9Error::InvalidJson(
                    "expected ':' after object key".into(),
                ));
            }
            self.pos += 1; // skip ':'

            let value = self.parse_value()?;
            map.insert(key, value);

            self.skip_whitespace();
            if self.pos >= self.chars.len() {
                return Err(Es9Error::InvalidJson("unclosed object".into()));
            }

            if self.chars[self.pos] == b'}' {
                self.pos += 1;
                break;
            } else if self.chars[self.pos] == b',' {
                self.pos += 1;
            } else {
                return Err(Es9Error::InvalidJson(
                    "expected ',' or '}' in object".into(),
                ));
            }
        }

        Ok(JsonValue::Object(map))
    }

    fn parse_array(&mut self) -> Result<JsonValue, Es9Error> {
        self.pos += 1; // skip '['
        let mut list = Vec::new();
        self.skip_whitespace();

        if self.pos < self.chars.len() && self.chars[self.pos] == b']' {
            self.pos += 1;
            return Ok(JsonValue::Array(list));
        }

        loop {
            let val = self.parse_value()?;
            list.push(val);
            self.skip_whitespace();

            if self.pos >= self.chars.len() {
                return Err(Es9Error::InvalidJson("unclosed array".into()));
            }

            if self.chars[self.pos] == b']' {
                self.pos += 1;
                break;
            } else if self.chars[self.pos] == b',' {
                self.pos += 1;
            } else {
                return Err(Es9Error::InvalidJson("expected ',' or ']' in array".into()));
            }
        }

        Ok(JsonValue::Array(list))
    }

    fn parse_string(&mut self) -> Result<String, Es9Error> {
        self.pos += 1; // skip initial '"'
        let mut s = String::new();

        while self.pos < self.chars.len() {
            let b = self.chars[self.pos];
            if b == b'"' {
                self.pos += 1;
                return Ok(s);
            }
            if b == b'\\' {
                self.pos += 1;
                if self.pos >= self.chars.len() {
                    return Err(Es9Error::InvalidJson("trailing escape in string".into()));
                }
                match self.chars[self.pos] {
                    b'"' => s.push('"'),
                    b'\\' => s.push('\\'),
                    b'/' => s.push('/'),
                    b'b' => s.push('\x08'),
                    b'f' => s.push('\x0C'),
                    b'n' => s.push('\n'),
                    b'r' => s.push('\r'),
                    b't' => s.push('\t'),
                    other => {
                        return Err(Es9Error::InvalidJson(format!(
                            "unsupported escape character: {other}"
                        )))
                    }
                }
                self.pos += 1;
            } else {
                s.push(b as char);
                self.pos += 1;
            }
        }

        Err(Es9Error::InvalidJson("unterminated string".into()))
    }

    fn parse_bool(&mut self) -> Result<JsonValue, Es9Error> {
        if self.chars[self.pos..].starts_with(b"true") {
            self.pos += 4;
            Ok(JsonValue::Bool(true))
        } else if self.chars[self.pos..].starts_with(b"false") {
            self.pos += 5;
            Ok(JsonValue::Bool(false))
        } else {
            Err(Es9Error::InvalidJson("invalid boolean literal".into()))
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, Es9Error> {
        if self.chars[self.pos..].starts_with(b"null") {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err(Es9Error::InvalidJson("invalid null literal".into()))
        }
    }

    fn parse_number(&mut self) -> Result<JsonValue, Es9Error> {
        let start = self.pos;
        if self.chars[self.pos] == b'-' {
            self.pos += 1;
        }
        while self.pos < self.chars.len()
            && matches!(
                self.chars[self.pos],
                b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-'
            )
        {
            self.pos += 1;
        }
        let s = std::str::from_utf8(&self.chars[start..self.pos])
            .map_err(|e| Es9Error::InvalidJson(e.to_string()))?;
        let num: f64 = s
            .parse()
            .map_err(|e: std::num::ParseFloatError| Es9Error::InvalidJson(e.to_string()))?;
        Ok(JsonValue::Number(num))
    }
}

/// Parses an arbitrary JSON string into a [`JsonValue`].
///
/// # Errors
/// Returns [`Es9Error::InvalidJson`] on malformed JSON input.
pub fn parse_json(input: &str) -> Result<JsonValue, Es9Error> {
    let mut parser = JsonParser::new(input);
    let val = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.pos < parser.chars.len() {
        return Err(Es9Error::InvalidJson(
            "trailing data after top-level JSON value".into(),
        ));
    }
    Ok(val)
}

// ---------------------------------------------------------------------------
// ES9+ Request Payloads
// ---------------------------------------------------------------------------

/// Builds an ES9+ `InitiateAuthentication` JSON request body.
#[must_use]
pub fn build_initiate_authentication_body(
    smdp_address: &str,
    euicc_challenge_b64: &str,
    euicc_info1_b64: &str,
) -> String {
    format!(
        r#"{{"euiccChallenge":"{euicc_challenge_b64}","euiccInfo1":"{euicc_info1_b64}","smdpAddress":"{smdp_address}"}}"#
    )
}

/// Builds an ES9+ `AuthenticateClient` JSON request body.
#[must_use]
pub fn build_authenticate_client_body(
    transaction_id: &str,
    authenticate_server_response_b64: &str,
) -> String {
    format!(
        r#"{{"transactionId":"{transaction_id}","authenticateServerResponse":"{authenticate_server_response_b64}"}}"#
    )
}

/// Builds an ES9+ `GetBoundProfilePackage` JSON request body.
#[must_use]
pub fn build_get_bound_profile_package_body(
    transaction_id: &str,
    prepare_download_response_b64: &str,
) -> String {
    format!(
        r#"{{"transactionId":"{transaction_id}","prepareDownloadResponse":"{prepare_download_response_b64}"}}"#
    )
}

/// Builds an ES9+ `CancelSession` JSON request body.
#[must_use]
pub fn build_cancel_session_body(
    transaction_id: &str,
    cancel_session_response_b64: &str,
) -> String {
    format!(
        r#"{{"transactionId":"{transaction_id}","cancelSessionResponse":"{cancel_session_response_b64}"}}"#
    )
}

/// Builds an ES9+ `HandleNotification` JSON request body.
#[must_use]
pub fn build_handle_notification_body(pending_notification_b64: &str) -> String {
    format!(r#"{{"pendingNotification":"{pending_notification_b64}"}}"#)
}

// ---------------------------------------------------------------------------
// ES9+ Response Verification and Payload Parsers
// ---------------------------------------------------------------------------

/// Verifies standard GSMA SGP.22 function execution status in response header.
///
/// # Errors
/// Returns [`Es9Error::ExecutionFailed`] if the status is `"Failed"` or status codes indicate failure.
pub fn verify_es9_header(root: &JsonValue, http_status: Option<u16>) -> Result<(), Es9Error> {
    if let Some(header) = root.get("header") {
        if let Some(status_obj) = header.get("functionExecutionStatus") {
            let status = status_obj.get_str("status").unwrap_or("");
            if status.eq_ignore_ascii_case("Failed") || status.eq_ignore_ascii_case("Expired") {
                let code_data = status_obj.get("statusCodeData");
                let subject_code = code_data
                    .and_then(|c| c.get_str("subjectCode"))
                    .unwrap_or("N/A")
                    .to_string();
                let reason_code = code_data
                    .and_then(|c| c.get_str("reasonCode"))
                    .unwrap_or("N/A")
                    .to_string();
                let subject_identifier = code_data
                    .and_then(|c| c.get_str("subjectIdentifier"))
                    .map(ToString::to_string);
                let message = code_data
                    .and_then(|c| c.get_str("message"))
                    .map(ToString::to_string);

                return Err(Es9Error::ExecutionFailed {
                    subject_code,
                    reason_code,
                    subject_identifier,
                    message,
                    status_code: http_status,
                });
            }
        }
    }
    Ok(())
}

/// Decoded payload from an ES9+ `InitiateAuthentication` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitiateAuthenticationResponse {
    pub transaction_id: String,
    pub server_signed1: Vec<u8>,
    pub server_signature1: Vec<u8>,
    pub euicc_ci_pk_id_to_be_used: Option<Vec<u8>>,
    pub server_certificate: Vec<u8>,
}

/// Decoded payload from an ES9+ `AuthenticateClient` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticateClientResponse {
    pub transaction_id: String,
    pub profile_metadata: Option<Vec<u8>>,
    pub smdp_signed2: Option<Vec<u8>>,
    pub smdp_signature2: Option<Vec<u8>>,
    pub smdp_certificate: Option<Vec<u8>>,
}

/// Decoded payload from an ES9+ `GetBoundProfilePackage` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetBoundProfilePackageResponse {
    pub transaction_id: String,
    pub bound_profile_package: Vec<u8>,
}

/// Parses and validates an ES9+ `InitiateAuthentication` JSON response string.
///
/// # Errors
/// Returns [`Es9Error`] if status check fails or mandatory fields are missing.
pub fn parse_initiate_authentication_response(
    json_str: &str,
    http_status: Option<u16>,
) -> Result<InitiateAuthenticationResponse, Es9Error> {
    let root = parse_json(json_str)?;
    verify_es9_header(&root, http_status)?;

    let transaction_id = root
        .get_str("transactionId")
        .ok_or(Es9Error::MissingField("transactionId"))?
        .to_string();

    let server_signed1_b64 = root
        .get_str("serverSigned1")
        .ok_or(Es9Error::MissingField("serverSigned1"))?;
    let server_signature1_b64 = root
        .get_str("serverSignature1")
        .ok_or(Es9Error::MissingField("serverSignature1"))?;
    let server_cert_b64 = root
        .get_str("serverCertificate")
        .ok_or(Es9Error::MissingField("serverCertificate"))?;

    let server_signed1 = base64_decode(server_signed1_b64)?;
    let server_signature1 = base64_decode(server_signature1_b64)?;
    let server_certificate = base64_decode(server_cert_b64)?;

    let euicc_ci_pk_id_to_be_used = if let Some(ci_b64) = root.get_str("euiccCiPKIdToBeUsed") {
        Some(base64_decode(ci_b64)?)
    } else {
        None
    };

    Ok(InitiateAuthenticationResponse {
        transaction_id,
        server_signed1,
        server_signature1,
        euicc_ci_pk_id_to_be_used,
        server_certificate,
    })
}

/// Parses and validates an ES9+ `AuthenticateClient` JSON response string.
///
/// # Errors
/// Returns [`Es9Error`] if status check fails or mandatory fields are missing.
pub fn parse_authenticate_client_response(
    json_str: &str,
    http_status: Option<u16>,
) -> Result<AuthenticateClientResponse, Es9Error> {
    let root = parse_json(json_str)?;
    verify_es9_header(&root, http_status)?;

    let transaction_id = root
        .get_str("transactionId")
        .ok_or(Es9Error::MissingField("transactionId"))?
        .to_string();

    let profile_metadata = if let Some(b64) = root.get_str("profileMetadata") {
        Some(base64_decode(b64)?)
    } else {
        None
    };

    let smdp_signed2 = if let Some(b64) = root.get_str("smdpSigned2") {
        Some(base64_decode(b64)?)
    } else {
        None
    };

    let smdp_signature2 = if let Some(b64) = root.get_str("smdpSignature2") {
        Some(base64_decode(b64)?)
    } else {
        None
    };

    let smdp_certificate = if let Some(b64) = root.get_str("smdpCertificate") {
        Some(base64_decode(b64)?)
    } else {
        None
    };

    Ok(AuthenticateClientResponse {
        transaction_id,
        profile_metadata,
        smdp_signed2,
        smdp_signature2,
        smdp_certificate,
    })
}

/// Parses and validates an ES9+ `GetBoundProfilePackage` JSON response string.
///
/// # Errors
/// Returns [`Es9Error`] if status check fails or mandatory fields are missing.
pub fn parse_get_bound_profile_package_response(
    json_str: &str,
    http_status: Option<u16>,
) -> Result<GetBoundProfilePackageResponse, Es9Error> {
    let root = parse_json(json_str)?;
    verify_es9_header(&root, http_status)?;

    let transaction_id = root
        .get_str("transactionId")
        .ok_or(Es9Error::MissingField("transactionId"))?
        .to_string();

    let bpp_b64 = root
        .get_str("boundProfilePackage")
        .or_else(|| root.get_str("bpp"))
        .ok_or(Es9Error::MissingField("boundProfilePackage"))?;

    let bound_profile_package = base64_decode(bpp_b64)?;

    Ok(GetBoundProfilePackageResponse {
        transaction_id,
        bound_profile_package,
    })
}
