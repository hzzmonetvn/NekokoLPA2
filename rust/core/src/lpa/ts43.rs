//! GSMA TS.43 Service Management and Entitlement Configuration protocol types.

use core::fmt;
use std::error::Error;

/// Error returned during TS.43 operation parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ts43Error {
    InvalidServerUrl(String),
    MalformedPayload(String),
    AuthenticationFailed(String),
}

impl fmt::Display for Ts43Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidServerUrl(url) => write!(f, "invalid TS.43 server URL: {url}"),
            Self::MalformedPayload(msg) => write!(f, "malformed TS.43 payload: {msg}"),
            Self::AuthenticationFailed(msg) => write!(f, "TS.43 authentication failed: {msg}"),
        }
    }
}

impl Error for Ts43Error {}

/// Entitlement state for a specific device feature (VoLTE, VoWiFi, 5G Standalone, eSIM Transfer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitlementStatus {
    Disabled,
    Enabled,
    Incompatible,
    Provisioning,
}

impl EntitlementStatus {
    #[must_use]
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Enabled,
            2 => Self::Incompatible,
            3 => Self::Provisioning,
            _ => Self::Disabled,
        }
    }
}

/// A TS.43 Carrier Configuration and Service Discovery record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ts43CarrierConfig {
    pub server_url: String,
    pub app_id: String,
    pub vowifi_entitled: EntitlementStatus,
    pub volte_entitled: EntitlementStatus,
    pub esim_odsa_entitled: EntitlementStatus,
}

impl Ts43CarrierConfig {
    #[must_use]
    pub fn new(server_url: String, app_id: String) -> Self {
        Self {
            server_url,
            app_id,
            vowifi_entitled: EntitlementStatus::Disabled,
            volte_entitled: EntitlementStatus::Disabled,
            esim_odsa_entitled: EntitlementStatus::Disabled,
        }
    }

    /// Generates HTTP request headers required by GSMA TS.43 specifications.
    #[must_use]
    pub fn build_request_headers(&self, token: Option<&str>) -> Vec<(String, String)> {
        let mut headers = Vec::with_capacity(4);
        headers.push(("Accept".to_string(), "application/json".to_string()));
        headers.push((
            "User-Agent".to_string(),
            format!("NekokoLPA2-TS43/{}", self.app_id),
        ));
        headers.push((
            "Content-Type".to_string(),
            "application/vnd.gsma.eap-aka+json".to_string(),
        ));
        if let Some(tok) = token {
            headers.push(("Authorization".to_string(), format!("Bearer {tok}")));
        }
        headers
    }
}
