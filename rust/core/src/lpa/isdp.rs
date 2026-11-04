//! Issuer Security Domain - Profile (ISD-P) security domain lifecycle management.
//!
//! Implemented per GSMA SGP.22 for managing on-card profile containers,
//! ISD-P AID addressing, state transitions, and personalization command construction.

use std::fmt;

use crate::hex::{bytes_to_hex, hex_to_bytes, HexError};

/// Standard GSMA SGP.22 ISD-P Application Identifier (AID) prefix: `A0000005591010`.
pub const ISDP_AID_PREFIX: &str = "A0000005591010";

/// Errors occurring during ISD-P operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsdpError {
    InvalidAidLength(usize),
    InvalidHex(String),
    InvalidStateTransition { from: IsdpState, to: IsdpState },
    PayloadTooLong(usize),
}

impl fmt::Display for IsdpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAidLength(len) => {
                write!(f, "Invalid ISD-P AID length: {len} bytes (expected 5..=16)")
            }
            Self::InvalidHex(err) => write!(f, "Invalid hex encoding: {err}"),
            Self::InvalidStateTransition { from, to } => {
                write!(f, "Invalid ISD-P state transition from {from:?} to {to:?}")
            }
            Self::PayloadTooLong(len) => write!(f, "ISD-P payload exceeds limit: {len} bytes"),
        }
    }
}

impl std::error::Error for IsdpError {}

impl From<HexError> for IsdpError {
    fn from(err: HexError) -> Self {
        Self::InvalidHex(err.to_string())
    }
}

/// Lifecycle states of an on-card ISD-P container per GSMA SGP.22.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IsdpState {
    /// Initial allocated state on eUICC
    #[default]
    Created,
    /// Profile Elements and cryptographic parameters written
    Personalized,
    /// Fully installed and verified profile container
    Installed,
    /// Profile currently enabled and active on telecom interface
    Operating,
    /// Profile removed from card storage
    Deleted,
}

impl IsdpState {
    /// Validates whether transitioning to `next` is permitted by SGP.22 rules.
    pub fn can_transition_to(self, next: Self) -> bool {
        match (self, next) {
            (Self::Created, Self::Personalized | Self::Deleted) => true,
            (Self::Personalized, Self::Installed | Self::Deleted) => true,
            (Self::Installed, Self::Operating | Self::Deleted) => true,
            (Self::Operating, Self::Installed | Self::Deleted) => true,
            (Self::Deleted, _) => false,
            _ => false,
        }
    }
}

/// Strongly typed Application Identifier (AID) representing an ISD-P.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsdpAid {
    raw: Vec<u8>,
}

impl IsdpAid {
    /// Creates an `IsdpAid` from raw bytes (length must be 5 to 16 bytes per ISO/IEC 7816-4).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IsdpError> {
        if bytes.len() < 5 || bytes.len() > 16 {
            return Err(IsdpError::InvalidAidLength(bytes.len()));
        }
        Ok(Self {
            raw: bytes.to_vec(),
        })
    }

    /// Creates an `IsdpAid` from a hexadecimal string.
    pub fn from_hex(hex_str: &str) -> Result<Self, IsdpError> {
        let bytes = hex_to_bytes(hex_str)?;
        Self::from_bytes(&bytes)
    }

    /// Returns the raw AID bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw
    }

    /// Formats the AID as an uppercase hexadecimal string.
    #[must_use]
    pub fn to_hex(&self) -> String {
        bytes_to_hex(&self.raw).to_ascii_uppercase()
    }

    /// Checks if this AID begins with the standard GSMA SGP.22 prefix (`A0000005591010`).
    #[must_use]
    pub fn has_standard_sgp22_prefix(&self) -> bool {
        let prefix = match hex_to_bytes(ISDP_AID_PREFIX) {
            Ok(p) => p,
            Err(_) => return false,
        };
        self.raw.starts_with(&prefix)
    }
}

/// Represents an on-card ISD-P security domain container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsdpContainer {
    aid: IsdpAid,
    state: IsdpState,
    iccid: Option<String>,
}

impl IsdpContainer {
    /// Creates a new ISD-P container descriptor in the `Created` state.
    #[must_use]
    pub fn new(aid: IsdpAid) -> Self {
        Self {
            aid,
            state: IsdpState::Created,
            iccid: None,
        }
    }

    /// Returns the container's AID.
    #[must_use]
    pub fn aid(&self) -> &IsdpAid {
        &self.aid
    }

    /// Returns the current lifecycle state.
    #[must_use]
    pub fn state(&self) -> IsdpState {
        self.state
    }

    /// Returns the associated ICCID, if bound.
    #[must_use]
    pub fn iccid(&self) -> Option<&str> {
        self.iccid.as_deref()
    }

    /// Sets the associated ICCID for this container.
    pub fn set_iccid(&mut self, iccid: String) {
        self.iccid = Some(iccid);
    }

    /// Transitions the ISD-P to a new state if allowed.
    pub fn transition_to(&mut self, new_state: IsdpState) -> Result<(), IsdpError> {
        if !self.state.can_transition_to(new_state) {
            return Err(IsdpError::InvalidStateTransition {
                from: self.state,
                to: new_state,
            });
        }
        self.state = new_state;
        Ok(())
    }
}

/// Builds the ISO 7816-4 `SELECT [AID]` APDU targeting an ISD-P.
///
/// APDU format: `00 A4 04 00 <AID_LEN> <AID>`
pub fn build_select_isdp_apdu(aid: &IsdpAid) -> Vec<u8> {
    let bytes = aid.as_bytes();
    let mut apdu = Vec::with_capacity(5 + bytes.len());
    apdu.extend_from_slice(&[0x00, 0xA4, 0x04, 0x00, bytes.len() as u8]);
    apdu.extend_from_slice(bytes);
    apdu
}

/// Builds GlobalPlatform `DELETE [AID]` APDU command to delete an ISD-P.
///
/// APDU format: `80 E4 00 00 <LEN> 4F <AID_LEN> <AID>`
pub fn build_delete_isdp_apdu(aid: &IsdpAid) -> Vec<u8> {
    let bytes = aid.as_bytes();
    let tag_4f_len = 2 + bytes.len();
    let mut apdu = Vec::with_capacity(5 + tag_4f_len);
    apdu.extend_from_slice(&[
        0x80,
        0xE4,
        0x00,
        0x00,
        tag_4f_len as u8,
        0x4F,
        bytes.len() as u8,
    ]);
    apdu.extend_from_slice(bytes);
    apdu
}
