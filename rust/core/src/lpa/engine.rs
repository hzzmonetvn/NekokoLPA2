//! Complete LPA State Machine and eUICC lifecycle management engine.

use core::fmt;
use std::collections::HashMap;
use std::error::Error;

use crate::apdu::ApduError;
use crate::asn1::rsp::RspProfileInfo;
use crate::asn1::Asn1Error;
use crate::iccid::iccid_for_display;
use crate::profile::EuiccProfile;

/// Error returned by LPA state machine operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LpaError {
    ProfileNotFound { iccid: String },
    ProfileAlreadyEnabled { iccid: String },
    ProfileAlreadyDisabled { iccid: String },
    CannotDeleteActiveProfile { iccid: String },
    InvalidOperation { reason: String },
    ApduError(ApduError),
    Asn1Error(Asn1Error),
}

impl fmt::Display for LpaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProfileNotFound { iccid } => write!(f, "profile {iccid} not found on eUICC"),
            Self::ProfileAlreadyEnabled { iccid } => {
                write!(f, "profile {iccid} is already enabled")
            }
            Self::ProfileAlreadyDisabled { iccid } => {
                write!(f, "profile {iccid} is already disabled")
            }
            Self::CannotDeleteActiveProfile { iccid } => {
                write!(f, "cannot delete active profile {iccid}; disable it first")
            }
            Self::InvalidOperation { reason } => write!(f, "invalid LPA operation: {reason}"),
            Self::ApduError(e) => write!(f, "APDU communication failed: {e}"),
            Self::Asn1Error(e) => write!(f, "ASN.1 payload parsing failed: {e}"),
        }
    }
}

impl Error for LpaError {}

impl From<ApduError> for LpaError {
    fn from(e: ApduError) -> Self {
        Self::ApduError(e)
    }
}

impl From<Asn1Error> for LpaError {
    fn from(e: Asn1Error) -> Self {
        Self::Asn1Error(e)
    }
}

/// eUICC operational state and profile inventory manager.
#[derive(Debug, Clone, Default)]
pub struct LpaEngine {
    profiles: HashMap<String, EuiccProfile>,
    active_iccid: Option<String>,
}

impl LpaEngine {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads an initial set of profiles into the engine.
    pub fn load_profiles(&mut self, profiles: Vec<EuiccProfile>) {
        self.profiles.clear();
        self.active_iccid = None;

        for p in profiles {
            let clean = iccid_for_display(&p.iccid).to_string();
            if p.enabled {
                self.active_iccid = Some(clean.clone());
            }
            self.profiles.insert(clean, p);
        }
    }

    /// Loads raw SGP.22 `RspProfileInfo` entries into the engine.
    pub fn load_rsp_profiles(&mut self, rsp_profiles: Vec<RspProfileInfo>) {
        let mut list = Vec::with_capacity(rsp_profiles.len());
        for rsp in rsp_profiles {
            let clean = iccid_for_display(&rsp.iccid).to_string();
            let mut prof = EuiccProfile::new(clean, rsp.profile_state == 1);
            prof.nickname = rsp.nickname;
            prof.service_provider_name = rsp.service_provider_name;
            prof.name = rsp.profile_name;
            prof.profile_class = rsp.profile_class;
            if let Some(aid) = rsp.isdp_aid {
                prof.isdp_aid = Some(crate::hex::bytes_to_hex(&aid));
            }
            list.push(prof);
        }
        self.load_profiles(list);
    }

    /// Returns a list of all loaded profiles.
    #[must_use]
    pub fn list_profiles(&self) -> Vec<&EuiccProfile> {
        let mut list: Vec<&EuiccProfile> = self.profiles.values().collect();
        list.sort_by(|a, b| a.iccid.cmp(&b.iccid));
        list
    }

    /// Gets a profile by its clean or raw ICCID.
    #[must_use]
    pub fn get_profile(&self, iccid: &str) -> Option<&EuiccProfile> {
        let clean = iccid_for_display(iccid);
        self.profiles.get(clean)
    }

    /// Currently active profile ICCID.
    #[must_use]
    pub fn active_profile_iccid(&self) -> Option<&str> {
        self.active_iccid.as_deref()
    }

    /// Enables a profile, automatically disabling any currently active profile in single-enable mode.
    ///
    /// # Errors
    /// Returns [`LpaError::ProfileNotFound`] if the profile does not exist.
    /// Returns [`LpaError::ProfileAlreadyEnabled`] if the target profile is already active.
    pub fn enable_profile(&mut self, iccid: &str) -> Result<(), LpaError> {
        let clean = iccid_for_display(iccid).to_string();
        if !self.profiles.contains_key(&clean) {
            return Err(LpaError::ProfileNotFound { iccid: clean });
        }

        if let Some(ref active) = self.active_iccid {
            if active == &clean {
                return Err(LpaError::ProfileAlreadyEnabled { iccid: clean });
            }
            // Deactivate previous
            if let Some(prev) = self.profiles.get_mut(active) {
                prev.enabled = false;
            }
        }

        if let Some(target) = self.profiles.get_mut(&clean) {
            target.enabled = true;
        }
        self.active_iccid = Some(clean);
        Ok(())
    }

    /// Disables an active profile.
    ///
    /// # Errors
    /// Returns [`LpaError::ProfileNotFound`] if not found.
    /// Returns [`LpaError::ProfileAlreadyDisabled`] if the target profile is already inactive.
    pub fn disable_profile(&mut self, iccid: &str) -> Result<(), LpaError> {
        let clean = iccid_for_display(iccid).to_string();
        let prof = self
            .profiles
            .get_mut(&clean)
            .ok_or_else(|| LpaError::ProfileNotFound {
                iccid: clean.clone(),
            })?;

        if !prof.enabled {
            return Err(LpaError::ProfileAlreadyDisabled { iccid: clean });
        }

        prof.enabled = false;
        if self.active_iccid.as_deref() == Some(&clean) {
            self.active_iccid = None;
        }
        Ok(())
    }

    /// Deletes a profile from the eUICC inventory.
    ///
    /// # Errors
    /// Returns [`LpaError::CannotDeleteActiveProfile`] if the profile is enabled.
    /// Returns [`LpaError::ProfileNotFound`] if the ICCID is not in inventory.
    pub fn delete_profile(&mut self, iccid: &str) -> Result<EuiccProfile, LpaError> {
        let clean = iccid_for_display(iccid).to_string();
        let prof = self
            .profiles
            .get(&clean)
            .ok_or_else(|| LpaError::ProfileNotFound {
                iccid: clean.clone(),
            })?;

        if prof.enabled {
            return Err(LpaError::CannotDeleteActiveProfile { iccid: clean });
        }

        Ok(self.profiles.remove(&clean).expect("profile exists"))
    }

    /// Updates the nickname of an existing profile.
    ///
    /// # Errors
    /// Returns [`LpaError::ProfileNotFound`] if the profile is not found.
    pub fn set_nickname(&mut self, iccid: &str, nickname: String) -> Result<(), LpaError> {
        let clean = iccid_for_display(iccid).to_string();
        let prof = self
            .profiles
            .get_mut(&clean)
            .ok_or_else(|| LpaError::ProfileNotFound {
                iccid: clean.clone(),
            })?;

        prof.nickname = if nickname.trim().is_empty() {
            None
        } else {
            Some(nickname.trim().to_string())
        };
        Ok(())
    }
}
