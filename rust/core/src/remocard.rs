//! RemoCard V2 remote smart card reader protocol models, JSON builders,
//! and optional AES-256-CBC payload encryption/decryption codecs.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::crypto::{
    base64_decode, base64_encode, decrypt_aes_256_cbc_with_key, encrypt_aes_256_cbc_with_key,
    key_from_password, sha256, CryptoError,
};
use crate::hex::{hex_to_bytes, HexError};

/// Configured RemoCard V2 remote reader endpoint.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoCardServer {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub token: Option<String>,
    pub aes_key_hex: Option<String>,
    pub enabled: bool,
}

/// Slot descriptor returned by `GET /listSlots`.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RemoCardSlot {
    pub name: String,
    pub eid: Option<String>,
}

/// Errors returned while parsing or decrypting RemoCard V2 messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoCardError {
    InvalidJson(String),
    Crypto(CryptoError),
    Hex(HexError),
    InvalidUtf8,
    ServerError(String),
    MissingField(&'static str),
    ChannelOpenFailed,
}

impl fmt::Display for RemoCardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(err) => write!(f, "invalid RemoCard JSON: {err}"),
            Self::Crypto(err) => write!(f, "RemoCard crypto error: {err}"),
            Self::Hex(err) => write!(f, "RemoCard hex error: {err}"),
            Self::InvalidUtf8 => write!(f, "RemoCard decrypted payload is not valid UTF-8"),
            Self::ServerError(msg) => write!(f, "RemoCard server error: {msg}"),
            Self::MissingField(field) => write!(f, "missing RemoCard response field '{field}'"),
            Self::ChannelOpenFailed => write!(f, "RemoCard failed to open logical channel"),
        }
    }
}

impl std::error::Error for RemoCardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Crypto(err) => Some(err),
            Self::Hex(err) => Some(err),
            _ => None,
        }
    }
}

impl From<CryptoError> for RemoCardError {
    fn from(err: CryptoError) -> Self {
        Self::Crypto(err)
    }
}

impl From<HexError> for RemoCardError {
    fn from(err: HexError) -> Self {
        Self::Hex(err)
    }
}

/// Extracts optional embedded password from `http://[user:]password@host[:port]` URLs
/// and returns `(clean_url, embedded_password)`.
#[must_use]
pub fn clean_remocard_url(base_url: &str) -> (String, Option<String>) {
    let trimmed = base_url.trim();
    let Some(scheme_end) = trimmed.find("://") else {
        return (trimmed.to_string(), None);
    };
    let scheme = &trimmed[..scheme_end + 3];
    let rest = &trimmed[scheme_end + 3..];
    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, ""),
    };

    let Some((userinfo, host)) = authority.rsplit_once('@') else {
        return (trimmed.to_string(), None);
    };

    let password = match userinfo.split_once(':') {
        Some((_, pass)) => pass,
        None => userinfo,
    };
    let clean = format!("{scheme}{host}{path}");
    let extracted = (!password.is_empty()).then(|| password.to_string());
    (clean, extracted)
}

/// Resolves the 32-byte AES-256 key from `aes_key_hex`, `token`, or embedded URL credentials.
#[must_use]
pub fn resolve_remocard_key(server: &RemoCardServer) -> Option<[u8; 32]> {
    if let Some(ref hex_key) = server.aes_key_hex {
        let trimmed = hex_key.trim();
        if !trimmed.is_empty() {
            if let Ok(bytes) = hex_to_bytes(trimmed) {
                if bytes.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&bytes);
                    return Some(key);
                }
            }
        }
    }

    if let Some(ref token) = server.token {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Some(key_from_password(trimmed));
        }
    }

    let (_, url_password) = clean_remocard_url(&server.base_url);
    url_password.map(|p| key_from_password(&p))
}

/// Builds HTTP headers for RemoCard V2 requests (`X-Remocard-Version: 2`, `Content-Type`,
/// and `Authorization: Bearer <token>` when a token or URL password is configured).
#[must_use]
pub fn build_remocard_headers(server: &RemoCardServer) -> Vec<(String, String)> {
    let mut headers = vec![
        ("Content-Type".to_string(), "application/json".to_string()),
        ("X-Remocard-Version".to_string(), "2".to_string()),
    ];

    let bearer = server
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| clean_remocard_url(&server.base_url).1);

    if let Some(token) = bearer {
        headers.push(("Authorization".to_string(), format!("Bearer {token}")));
    }

    headers
}

/// Builds the JSON request body for `POST /openChannel`.
#[must_use]
pub fn build_open_channel_request(reader: &str, aids: &[String]) -> String {
    serde_json::json!({
        "reader": reader,
        "aids": aids,
    })
    .to_string()
}

/// Builds the JSON request body for `POST /sendApdu`.
#[must_use]
pub fn build_send_apdu_request(reader: &str, aid: &str, apdu_hex: &str) -> String {
    serde_json::json!({
        "reader": reader,
        "aid": aid,
        "apdu": apdu_hex,
    })
    .to_string()
}

/// Builds the JSON request body for `POST /sendRawApdu`.
#[must_use]
pub fn build_send_raw_apdu_request(reader: &str, apdu_hex: &str) -> String {
    serde_json::json!({
        "reader": reader,
        "apdu": apdu_hex,
    })
    .to_string()
}

/// Builds the JSON request body for `POST /closeChannel`.
#[must_use]
pub fn build_close_channel_request(reader: &str, aids: &[String]) -> String {
    serde_json::json!({
        "reader": reader,
        "aids": aids,
    })
    .to_string()
}

/// Wraps a RemoCard V2 JSON payload in `{"data": "<base64(iv + ciphertext)>"}` using a fresh IV.
#[must_use]
pub fn encrypt_remocard_payload(plaintext_json: &str, key: &[u8; 32]) -> String {
    let iv = generate_random_iv(plaintext_json.as_bytes());
    encrypt_remocard_payload_with_iv(plaintext_json, key, &iv)
}

/// Deterministic variant of [`encrypt_remocard_payload`] using an explicit 16-byte IV.
#[must_use]
pub fn encrypt_remocard_payload_with_iv(
    plaintext_json: &str,
    key: &[u8; 32],
    iv: &[u8; 16],
) -> String {
    let encrypted = encrypt_aes_256_cbc_with_key(plaintext_json.as_bytes(), key, iv);
    let b64 = base64_encode(&encrypted);
    serde_json::json!({ "data": b64 }).to_string()
}

/// Unwraps and decrypts a RemoCard V2 `{"data": "..."}` JSON body if encrypted,
/// or returns the plaintext JSON unchanged if the server sent unencrypted JSON.
///
/// # Errors
/// Returns [`RemoCardError`] if JSON parsing, base64 decoding, AES decryption, or UTF-8
/// validation fails.
pub fn decrypt_remocard_payload(body: &str, key: &[u8; 32]) -> Result<String, RemoCardError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| RemoCardError::InvalidJson(e.to_string()))?;

    if let Some(encrypted_b64) = value.get("data").and_then(serde_json::Value::as_str) {
        let raw = base64_decode(encrypted_b64)?;
        let decrypted = decrypt_aes_256_cbc_with_key(&raw, key)?;
        return String::from_utf8(decrypted).map_err(|_| RemoCardError::InvalidUtf8);
    }

    Ok(body.to_string())
}

/// Parses a RemoCard V2 `GET /listSlots` JSON response (decrypting if `server` has a key).
///
/// # Errors
/// Returns [`RemoCardError`] on malformed JSON, server error, or decryption failure.
pub fn parse_list_slots_response(
    body: &str,
    server: Option<&RemoCardServer>,
) -> Result<Vec<RemoCardSlot>, RemoCardError> {
    let decoded_body = maybe_decrypt(body, server)?;
    let root: serde_json::Value = serde_json::from_str(&decoded_body)
        .map_err(|e| RemoCardError::InvalidJson(e.to_string()))?;

    if let Some(err) = root.get("error").and_then(serde_json::Value::as_str) {
        return Err(RemoCardError::ServerError(err.to_string()));
    }

    let slots = root
        .get("slots")
        .and_then(serde_json::Value::as_array)
        .ok_or(RemoCardError::MissingField("slots"))?;

    let mut out = Vec::with_capacity(slots.len());
    for slot in slots {
        if let Some(name) = slot.get("name").and_then(serde_json::Value::as_str) {
            let eid = slot
                .get("eid")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            out.push(RemoCardSlot {
                name: name.to_string(),
                eid,
            });
        }
    }
    Ok(out)
}

/// Parses a RemoCard V2 `POST /openChannel` JSON response, returning the matched AID.
///
/// # Errors
/// Returns [`RemoCardError`] if opening failed, JSON is malformed, or `aid` is missing.
pub fn parse_open_channel_response(
    body: &str,
    server: Option<&RemoCardServer>,
) -> Result<String, RemoCardError> {
    let decoded_body = maybe_decrypt(body, server)?;
    let root: serde_json::Value = serde_json::from_str(&decoded_body)
        .map_err(|e| RemoCardError::InvalidJson(e.to_string()))?;

    if let Some(err) = root.get("error").and_then(serde_json::Value::as_str) {
        return Err(RemoCardError::ServerError(err.to_string()));
    }
    if root.get("success").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(RemoCardError::ChannelOpenFailed);
    }

    root.get("aid")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or(RemoCardError::MissingField("aid"))
}

/// Parses a RemoCard V2 `POST /sendApdu` or `/sendRawApdu` JSON response into APDU bytes.
///
/// # Errors
/// Returns [`RemoCardError`] if JSON is invalid, `response` is missing, or hex decoding fails.
pub fn parse_send_apdu_response(
    body: &str,
    server: Option<&RemoCardServer>,
) -> Result<Vec<u8>, RemoCardError> {
    let decoded_body = maybe_decrypt(body, server)?;
    let root: serde_json::Value = serde_json::from_str(&decoded_body)
        .map_err(|e| RemoCardError::InvalidJson(e.to_string()))?;

    if let Some(err) = root.get("error").and_then(serde_json::Value::as_str) {
        return Err(RemoCardError::ServerError(err.to_string()));
    }

    let hex_str = root
        .get("response")
        .and_then(serde_json::Value::as_str)
        .ok_or(RemoCardError::MissingField("response"))?;

    Ok(hex_to_bytes(hex_str)?)
}

fn maybe_decrypt(body: &str, server: Option<&RemoCardServer>) -> Result<String, RemoCardError> {
    let Some(srv) = server else {
        return Ok(body.to_string());
    };
    let Some(key) = resolve_remocard_key(srv) else {
        return Ok(body.to_string());
    };
    decrypt_remocard_payload(body, &key)
}

fn generate_random_iv(context: &[u8]) -> [u8; 16] {
    let mut iv = [0u8; 16];
    if let Ok(mut f) = File::open("/dev/urandom") {
        if f.read_exact(&mut iv).is_ok() {
            return iv;
        }
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut seed = Vec::with_capacity(context.len() + 16);
    seed.extend_from_slice(context);
    seed.extend_from_slice(&nanos.to_le_bytes());
    let digest = sha256(&seed);
    iv.copy_from_slice(&digest[..16]);
    iv
}
