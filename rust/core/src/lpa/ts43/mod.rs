//! GSMA TS.43 Service Management and Entitlement Configuration protocol.

pub mod client;
pub mod constants;
pub mod entitlement;
pub mod enums;
pub mod errors;
pub mod request;
pub mod response;
pub mod sim;
pub mod types;
pub mod websheet;

use core::fmt;
use std::error::Error;

pub use client::{
    SourceSimProvider, Ts43Client, Ts43Config, Ts43HttpRequest, Ts43HttpResponse, Ts43Method,
    Ts43Transport,
};
pub use constants::Ts43Constants;
pub use entitlement::{
    discover_auto, discover_entitlement, discover_entitlement_for_channel, EntitlementRecord,
    DEFAULT_ENTITLEMENT_TIMEOUT,
};
pub use enums::{
    AppEligibility, EntitlementChannel, MessageButton, Operation, OperationResult, OperationType,
    ServiceStatus, SimType, SubscriptionResult, TransferType,
};
pub use errors::{
    is_http_status, CarrierDeniedError, EntitlementException, Ts43HttpError, WebsheetException,
    ENTITLEMENT_AMBIGUOUS, ENTITLEMENT_NOT_FOUND, ENTITLEMENT_UNSUPPORTED,
    WEBSHEET_DEFAULT_SMDP_UNSUPPORTED, WEBSHEET_DISMISSED,
};
pub use request::{
    acquire_configuration_operation, acquire_configuration_request_device,
    activation_configuration_operation, build_prepared_request, check_eligibility_operation,
    manage_subscription_operation, manage_subscription_request_device, temporary_token_operation,
    update_subscription_operation, OdsaOperation, PreparedRequest, ServiceEntitlementRequest,
};
pub use response::OdsaResponse;
pub use sim::{
    decode_bcd, decode_mnc_length, decode_swapped_bcd, encode_bcd, fid, format_mnc,
    group_identifier_string, imsi_eap_identity, respond_to_challenge, respond_to_challenge_base64,
    AkaChallenge, AkaError, AkaRequest, AkaResponse, IdentifierError, Identity, Imsi, ImsiError,
    UsimFiles,
};
pub use types::{
    first_non_empty, merge_cookie_headers, ActivationResult, AuthToken, ContinueRequest, Device,
    DownloadConfig, Endpoint, Entitlement, Message, TemporaryToken, TransferOffer, TransferSession,
    TransferTokenResult, Ts43Event, Ts43Result, Ts43State, UserInputResult, Websheet,
    WebsheetCallbackEvent, WebsheetResult,
};
pub use websheet::parse_websheet_result;

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
