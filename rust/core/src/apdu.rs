//! APDU status words, response codes, and validation according to ISO/IEC 7816 and GSMA SGP.22.

use core::fmt;
use std::error::Error;

/// Specific APDU status failure or warning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApduError {
    /// Conditions of use not satisfied (Profile state change ongoing, 0x6985).
    ConditionOfUseNotSatisfied,
    /// Card busy - session unavailable (0x6881).
    CardBusy,
    /// Incorrect/invalid data field encoding (not a DER data object, 0x6A80).
    InvalidDataEncoding,
    /// Reference data not found (unsupported/unknown command request, 0x6A88).
    ReferenceDataNotFound,
    /// Proactive command pending / CAT refresh (SW1=0x91 / SW1=0x92 / SW1=0x9F).
    ProactiveRefresh { sw1: u8, sw2: u8 },
    /// Generic APDU error with raw status words.
    Failure { sw1: u8, sw2: u8 },
}

impl fmt::Display for ApduError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConditionOfUseNotSatisfied => {
                write!(
                    f,
                    "Conditions of use not satisfied (Profile state change ongoing) [6985]"
                )
            }
            Self::CardBusy => write!(f, "Card busy - session unavailable [6881]"),
            Self::InvalidDataEncoding => {
                write!(
                    f,
                    "Incorrect/invalid data field encoding (not a DER data object) [6A80]"
                )
            }
            Self::ReferenceDataNotFound => {
                write!(
                    f,
                    "Reference data not found (unsupported/unknown command request) [6A88]"
                )
            }
            Self::ProactiveRefresh { sw1, sw2 } => {
                write!(f, "Proactive command pending (SW={sw1:02X}{sw2:02X})")
            }
            Self::Failure { sw1, sw2 } => {
                write!(f, "APDU failed (SW={sw1:02X}{sw2:02X})")
            }
        }
    }
}

impl Error for ApduError {}

/// Validates APDU status bytes SW1 and SW2.
///
/// Returns `Ok(())` on standard success (0x9000).
///
/// # Errors
/// Returns an [`ApduError`] matching the ISO 7816 or GSMA SGP.22 error condition.
pub fn validate_apdu_sw(sw1: u8, sw2: u8) -> Result<(), ApduError> {
    if sw1 == 0x90 && sw2 == 0x00 {
        return Ok(());
    }

    match (sw1, sw2) {
        (0x69, 0x85) => Err(ApduError::ConditionOfUseNotSatisfied),
        (0x68, 0x81) => Err(ApduError::CardBusy),
        (0x6A, 0x80) => Err(ApduError::InvalidDataEncoding),
        (0x6A, 0x88) => Err(ApduError::ReferenceDataNotFound),
        (0x91, _) | (0x92, _) | (0x9F, _) => Err(ApduError::ProactiveRefresh { sw1, sw2 }),
        _ => Err(ApduError::Failure { sw1, sw2 }),
    }
}
