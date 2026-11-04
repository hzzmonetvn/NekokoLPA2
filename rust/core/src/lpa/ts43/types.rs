//! TS.43 data types, configurations, events, and results.
//!
//! Ported from `types.go` / `types.dart`.

use core::time::Duration;

use super::enums::{EntitlementChannel, Operation, SimType, TransferType};
use super::sim::Identity;

/// Device terminal identity fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Device {
    pub imei: String,
    pub imeis: Vec<String>,
    pub iccid: String,
    pub eid: String,
    pub vendor: String,
    pub model: String,
    pub software_version: String,
    pub serial_number: String,
    pub friendly_name: String,
    pub service: String,
    pub entitlement_protocol: i32,
}

/// Carrier TS.43 entitlement configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entitlement {
    pub server: String,
    pub channel: EntitlementChannel,
    pub use_post: bool,
    pub timeout: Duration,
    pub source_sim_type: SimType,
    pub transfer_type: i32,
    pub use_temp_token_conversion: bool,
    pub set_target_terminal_fields: bool,
    pub restrict_old_terminal_fields: bool,
    pub use_smds_delayed_download: bool,
    pub poll_interval: Duration,
    pub poll_timeout: Duration,
    pub auto_retry_511_limit: u32,
    pub app: String,
    pub app_name: String,
    pub provisioning_version: String,
    pub request_version: String,
    pub version: String,
}

impl Default for Entitlement {
    fn default() -> Self {
        Self {
            server: String::new(),
            channel: EntitlementChannel::None,
            use_post: false,
            timeout: Duration::ZERO,
            source_sim_type: SimType::Esim,
            transfer_type: TransferType::DISABLED,
            use_temp_token_conversion: false,
            set_target_terminal_fields: false,
            restrict_old_terminal_fields: false,
            use_smds_delayed_download: false,
            poll_interval: Duration::ZERO,
            poll_timeout: Duration::ZERO,
            auto_retry_511_limit: 0,
            app: String::new(),
            app_name: String::new(),
            provisioning_version: String::new(),
            request_version: String::new(),
            version: String::new(),
        }
    }
}

/// Terminal endpoint consisting of device metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Endpoint {
    pub device: Device,
}

/// TS.43 transfer flow states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Ts43State {
    #[default]
    Unknown,
    PendingUserInput,
    PendingWebsheet,
    PendingSourceProfileDeletion,
    PendingDownload,
    PendingActivation,
    Done,
    PendingSmdsDiscovery,
    Dismissed,
    PendingDelayedDownload,
}

/// Auth token sent as IMSI + token.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuthToken {
    pub value: String,
    pub expires_at: Option<u64>,
    pub cookies: Vec<String>,
}

/// Temporary token from `AcquireTemporaryToken`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TemporaryToken {
    pub value: String,
    pub expires_at: Option<u64>,
    pub cookies: Vec<String>,
    pub targets: Vec<Operation>,
}

/// User prompt message displayed in dialogs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Message {
    pub title: String,
    pub text: String,
    pub accept_button: bool,
    pub accept_button_label: String,
    pub reject_button: bool,
    pub reject_button_label: String,
    pub accept_free_text: bool,
    pub accept_free_text_hint: String,
    pub accept_free_text_validation: String,
    pub accept_free_text_validation_failed_error_text: String,
}

/// Websheet URL and payload configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Websheet {
    pub url: String,
    pub user_data: String,
    pub contents_type: String,
}

/// Profile download configuration returned by the TS.43 server.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DownloadConfig {
    pub activation_code: String,
    pub smdp_fqdn: String,
    pub matching_id: String,
    pub profile_iccid: String,
    pub imei: String,
}

/// Events returned by websheet callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WebsheetCallbackEvent {
    #[default]
    Unknown,
    ProfileReadyWithActivationCode,
    ProfileReadyWithDefaultSmdp,
    SelectionCompleted,
    DismissFlow,
    FinishFlow,
    DeleteToken,
    CheckProfileServiceStatus,
    DeleteProfileInUse,
    VoWiFiEntitlementChanged,
    VoWiFiDismissFlow,
}

/// Result of parsing a websheet callback.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WebsheetResult {
    pub event: WebsheetCallbackEvent,
    pub activation_code: String,
    pub smdp_fqdn: String,
    pub iccid: String,
    pub imei: String,
    pub next_action: Operation,
}

/// User input response from a prompt dialog.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserInputResult {
    pub response: String,
    pub button: i32,
}

/// Request to continue a paused transfer flow.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContinueRequest {
    pub user_input: Option<UserInputResult>,
    pub websheet: Option<WebsheetResult>,
    pub source_profile_deleted: bool,
}

/// Events emitted during the TS.43 flow lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ts43Event {
    UserInput(Message),
    Websheet(Websheet),
    SourceProfileDeletion(String),
    DownloadReady(DownloadConfig),
    SmdsDiscovery {
        source_iccid: String,
        target_eid: String,
        target_imei: String,
        subscription_result: i32,
    },
    DelayedDownload {
        source_iccid: String,
        target_eid: String,
        target_imei: String,
        subscription_result: i32,
    },
    ActivationPending {
        iccid: String,
        subscription_result: i32,
    },
    Dismiss(i32),
    Done,
}

/// Carried transfer offer session data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferOffer {
    pub source_identity: Identity,
    pub source_device: Device,
    pub target_device: Device,
    pub auth_token: AuthToken,
    pub temporary_token: TemporaryToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferSession {
    pub offer: TransferOffer,
    pub source_profile_iccid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActivationResult {
    pub iccid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransferTokenResult {
    pub auth: AuthToken,
    pub temporary: TemporaryToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ts43Result {
    pub state: Ts43State,
    pub event: Option<Ts43Event>,
    pub manage_subscription: Option<super::response::OdsaResponse>,
    pub update_subscription: Option<super::response::OdsaResponse>,
    pub configuration: Option<super::response::OdsaResponse>,
    pub update_configuration: Option<super::response::OdsaResponse>,
    pub download_config: DownloadConfig,
    pub session: Option<TransferSession>,
}

/// Returns the first non-empty string in a slice.
#[must_use]
pub fn first_non_empty(values: &[&str]) -> String {
    for &v in values {
        if !v.is_empty() {
            return v.to_string();
        }
    }
    String::new()
}

/// Replaces/merges `next` cookies into `current`, keyed by cookie name.
#[must_use]
pub fn merge_cookie_headers(current: &[String], next: &[String]) -> Vec<String> {
    if next.is_empty() {
        return current.to_vec();
    }
    let mut merged = current.to_vec();
    for cookie in next {
        if cookie.is_empty() {
            continue;
        }
        let name = cookie.split('=').next().unwrap_or(cookie);
        let mut replaced = false;
        for existing in &mut merged {
            let existing_name = existing.split('=').next().unwrap_or(existing);
            if existing_name == name {
                *existing = cookie.clone();
                replaced = true;
                break;
            }
        }
        if !replaced {
            merged.push(cookie.clone());
        }
    }
    merged
}
