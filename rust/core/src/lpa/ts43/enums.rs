//! TS.43 enumerations and status codes.
//!
//! Ported from `constants.go` / `enums.dart`.

use core::fmt;

/// ODSA operation names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Operation {
    CheckEligibility,
    AcquireTemporaryToken,
    ManageSubscription,
    AcquireConfiguration,
    #[default]
    None,
}

impl Operation {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CheckEligibility => "CheckEligibility",
            Self::AcquireTemporaryToken => "AcquireTemporaryToken",
            Self::ManageSubscription => "ManageSubscription",
            Self::AcquireConfiguration => "AcquireConfiguration",
            Self::None => "",
        }
    }
}

impl core::str::FromStr for Operation {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "CheckEligibility" => Self::CheckEligibility,
            "AcquireTemporaryToken" => Self::AcquireTemporaryToken,
            "ManageSubscription" => Self::ManageSubscription,
            "AcquireConfiguration" => Self::AcquireConfiguration,
            _ => Self::None,
        })
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Operation type query values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OperationType {
    #[default]
    Omit,
    SamsungD2DTransfer,
    TransferSubscription,
    UpdateSubscription,
}

impl OperationType {
    #[must_use]
    pub fn to_i32(self) -> i32 {
        match self {
            Self::Omit => -1,
            Self::SamsungD2DTransfer => 0,
            Self::TransferSubscription => 3,
            Self::UpdateSubscription => 4,
        }
    }

    #[must_use]
    pub fn text(self) -> String {
        self.to_i32().to_string()
    }
}

/// SIM card form factor / type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SimType {
    #[default]
    Esim,
    Psim,
}

/// Carrier entitlement channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EntitlementChannel {
    Google,
    Samsung,
    #[default]
    None,
}

impl EntitlementChannel {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Samsung => "samsung",
            Self::None => "",
        }
    }
}

impl core::str::FromStr for EntitlementChannel {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "google" => Self::Google,
            "samsung" => Self::Samsung,
            _ => Self::None,
        })
    }
}

impl fmt::Display for EntitlementChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// TS.43 transfer types.
pub struct TransferType;

impl TransferType {
    pub const DISABLED: i32 = 0;
    pub const TEMPORARY_TOKEN: i32 = 2;
    pub const REUSABLE_ACTIVATION_CODE: i32 = 3;
    pub const TS43_V8: i32 = 6;
    pub const OLD_TERMINAL_IDS: i32 = 7;
    pub const TEMPORARY_TOKEN_OLD_TERMINAL_IDS: i32 = 9;
    pub const TEMPORARY_TOKEN_VARIANT11: i32 = 11;

    #[must_use]
    pub fn is_source_credential(t: i32) -> bool {
        t == Self::REUSABLE_ACTIVATION_CODE || t == Self::TS43_V8
    }

    #[must_use]
    pub fn text(t: i32) -> String {
        if t == Self::DISABLED {
            String::new()
        } else {
            t.to_string()
        }
    }

    #[must_use]
    pub fn uses_temporary_token(t: i32) -> bool {
        matches!(
            t,
            Self::TEMPORARY_TOKEN
                | Self::TEMPORARY_TOKEN_OLD_TERMINAL_IDS
                | Self::TEMPORARY_TOKEN_VARIANT11
        )
    }
}

pub struct OperationResult;
impl OperationResult {
    pub const UNKNOWN: i32 = 0;
    pub const SUCCESS: i32 = 1;

    #[must_use]
    pub fn parse(text: Option<&str>) -> i32 {
        parse_enum_int(text, Self::UNKNOWN)
    }
}

pub struct AppEligibility;
impl AppEligibility {
    pub const UNKNOWN: i32 = -1;
    pub const ENABLED: i32 = 1;

    #[must_use]
    pub fn parse(text: Option<&str>) -> i32 {
        parse_enum_int(text, Self::UNKNOWN)
    }
}

pub struct SubscriptionResult;
impl SubscriptionResult {
    pub const UNKNOWN: i32 = 0;
    pub const CONTINUE_TO_WEBSHEET: i32 = 1;
    pub const DOWNLOAD_PROFILE: i32 = 2;
    pub const DONE: i32 = 3;
    pub const DELAYED_DOWNLOAD: i32 = 4;
    pub const DISMISS: i32 = 5;
    pub const DELETE_PROFILE_IN_USE: i32 = 6;
    pub const REDOWNLOADABLE_PROFILE_MANDATORY: i32 = 7;
    pub const REQUIRES_USER_INPUT: i32 = 8;

    #[must_use]
    pub fn parse(text: Option<&str>) -> i32 {
        parse_enum_int(text, Self::UNKNOWN)
    }
}

pub struct ServiceStatus;
impl ServiceStatus {
    pub const UNKNOWN: i32 = 0;
    pub const ACTIVATED: i32 = 1;
    pub const ACTIVATING: i32 = 2;
    pub const DEACTIVATED: i32 = 3;
    pub const DEACTIVATED_NO_REUSE: i32 = 4;

    #[must_use]
    pub fn parse(text: Option<&str>) -> i32 {
        parse_enum_int(text, Self::UNKNOWN)
    }
}

pub struct MessageButton;
impl MessageButton {
    pub const UNKNOWN: i32 = -1;
    pub const REJECTED: i32 = 0;
    pub const ACCEPTED: i32 = 1;

    #[must_use]
    pub fn text(b: i32) -> String {
        if b == Self::UNKNOWN {
            String::new()
        } else {
            b.to_string()
        }
    }
}

fn parse_enum_int(text: Option<&str>, fallback: i32) -> i32 {
    let Some(s) = text else { return fallback };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return fallback;
    }
    trimmed.parse::<i32>().unwrap_or(fallback)
}
