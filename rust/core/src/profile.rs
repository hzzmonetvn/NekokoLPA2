//! GSMA SGP.22 eSIM profile models and metadata representation.

use crate::iccid::iccid_for_display;

/// An installed or available eSIM profile on an eUICC card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EuiccProfile {
    /// Formatted ICCID with BCD trailing padding removed for UI presentation.
    pub iccid: String,
    /// Raw un-stripped ICCID used for card command APDUs (e.g. enable, disable, delete).
    pub raw_iccid: String,
    /// Profile display name.
    pub name: Option<String>,
    /// User assigned nickname.
    pub nickname: Option<String>,
    /// Service provider / carrier name.
    pub service_provider_name: Option<String>,
    /// Profile state (true if enabled/active).
    pub enabled: bool,
    /// Profile class (0: test, 1: provisioning, 2: operational).
    pub profile_class: u8,
    /// ISD-P Application Identifier (AID).
    pub isdp_aid: Option<String>,
    /// Raw profile icon bytes.
    pub icon: Option<Vec<u8>>,
    /// Mobile Country Code (MCC).
    pub mcc: Option<String>,
    /// Mobile Network Code (MNC).
    pub mnc: Option<String>,
    /// Group Identifier level 1 (GID1).
    pub gid1: Option<String>,
    /// Group Identifier level 2 (GID2).
    pub gid2: Option<String>,
    /// SM-DP+ server address.
    pub smdp_address: Option<String>,
    /// Profile tags.
    pub tags: Vec<String>,
}

impl EuiccProfile {
    /// Constructs a new `EuiccProfile`.
    #[must_use]
    pub fn new(iccid: impl Into<String>, enabled: bool) -> Self {
        let raw = iccid.into();
        let display = iccid_for_display(&raw).to_string();
        Self {
            iccid: display,
            raw_iccid: raw,
            name: None,
            nickname: None,
            service_provider_name: None,
            enabled,
            profile_class: 2,
            isdp_aid: None,
            icon: None,
            mcc: None,
            mnc: None,
            gid1: None,
            gid2: None,
            smdp_address: None,
            tags: Vec::new(),
        }
    }

    /// Computes the best human-readable display name for this profile.
    #[must_use]
    pub fn display_name(&self) -> &str {
        if let Some(ref nick) = self.nickname {
            let trimmed = nick.trim();
            if !trimmed.is_empty() {
                return trimmed;
            }
        }
        if let Some(ref name) = self.name {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                return trimmed;
            }
        }
        if let Some(ref sp) = self.service_provider_name {
            let trimmed = sp.trim();
            if !trimmed.is_empty() {
                return trimmed;
            }
        }
        "Unnamed Profile"
    }
}
