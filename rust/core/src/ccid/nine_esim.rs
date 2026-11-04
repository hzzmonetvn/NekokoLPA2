//! 9eSIM CCID USB custom management protocol implementation.
//!
//! Handles CRC-24 password generation, CCID vendor command construction,
//! and ISO/IEC 7816-3 ATR programming commands for 9eSIM smart card readers.

use std::fmt;

use crate::atr::{Atr, AtrError, DEFAULT_EUICC_ATR};
use crate::hex::{bytes_to_hex, hex_to_bytes};

/// Default ATR advertised by 9eSIM devices.
pub const DEFAULT_9ESIM_ATR: &str = DEFAULT_EUICC_ATR;

/// Errors arising during 9eSIM protocol operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NineEsimError {
    InvalidDeviceId(String),
    Atr(AtrError),
    AtrTooLong(usize),
}

impl fmt::Display for NineEsimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDeviceId(id) => {
                write!(
                    f,
                    "Device ID must be exactly 6 hexadecimal characters, got: {id}"
                )
            }
            Self::Atr(err) => write!(f, "ATR error: {err}"),
            Self::AtrTooLong(len) => write!(f, "ATR length {len} exceeds 255 bytes"),
        }
    }
}

impl std::error::Error for NineEsimError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Atr(err) => Some(err),
            _ => None,
        }
    }
}

impl From<AtrError> for NineEsimError {
    fn from(err: AtrError) -> Self {
        Self::Atr(err)
    }
}

/// Computes the 9eSIM CRC-24 checksum with polynomial `0x1864CFB` and seed `0xB704CE`.
#[must_use]
pub fn calculate_crc24(data: &[u8]) -> u32 {
    let mut crc: u32 = 0x00B7_04CE;
    for &byte in data {
        crc ^= u32::from(byte) << 16;
        for _ in 0..8 {
            crc <<= 1;
            if (crc & 0x0100_0000) != 0 {
                crc ^= 0x0186_4CFB;
            }
        }
    }
    crc & 0x00FF_FFFF
}

/// Derives the 6-character uppercase hex password string.
///
/// If the input is already a 6-character hexadecimal string, it is normalized to uppercase.
/// Otherwise, its UTF-8 bytes are hashed with the 9eSIM CRC-24 algorithm.
#[must_use]
pub fn password_hex(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() == 6 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return trimmed.to_ascii_uppercase();
    }

    let crc = calculate_crc24(trimmed.as_bytes());
    format!("{crc:06X}")
}

/// Constructs the CCID vendor command to set device password.
///
/// Device name must be exactly 6 hexadecimal characters.
pub fn set_password_command(device_name: &str, password: &str) -> Result<Vec<u8>, NineEsimError> {
    let name = device_name.trim().to_ascii_uppercase();
    if name.len() != 6 || !name.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(NineEsimError::InvalidDeviceId(name));
    }

    let pwd_hex = password_hex(password);
    let payload_hex = format!("3027000009111111{name}{pwd_hex}");
    hex_to_bytes(&payload_hex)
        .map_err(|e| NineEsimError::InvalidDeviceId(format!("Failed to encode command: {e}")))
}

/// Constructs the CCID command to enable ATR programming on the reader: `30 97 43 18 00`.
#[must_use]
pub fn enable_atr_programming_command() -> [u8; 5] {
    [0x30, 0x97, 0x43, 0x18, 0x00]
}

/// Constructs the CCID vendor command to write a new ATR to the reader.
///
/// Format: `30 38 00 00 <LEN> <ATR_LEN_WITHOUT_TS> <ATR_WITHOUT_TS...>`
pub fn set_atr_command(atr_bytes: &[u8]) -> Result<Vec<u8>, NineEsimError> {
    if atr_bytes.is_empty() || atr_bytes.len() > 255 {
        return Err(NineEsimError::AtrTooLong(atr_bytes.len()));
    }

    // The reader expects DATA = (ATR length - 1) followed by ATR without TS (3B).
    let data_len = atr_bytes.len();
    let mut command = Vec::with_capacity(5 + data_len);
    command.extend_from_slice(&[0x30, 0x38, 0x00, 0x00, data_len as u8]);
    command.push((data_len - 1) as u8);
    command.extend_from_slice(&atr_bytes[1..]);
    Ok(command)
}

/// Checks if eUICC capability is enabled in an ATR hex string.
pub fn euicc_capability_enabled(atr_hex: &str) -> Result<bool, NineEsimError> {
    let atr = Atr::from_hex(atr_hex)?;
    Ok(atr.euicc_capability_enabled()?)
}

/// Sets or clears eUICC capability in an ATR hex string, recalculating TCK checksum.
pub fn set_euicc_capability(atr_hex: &str, enabled: bool) -> Result<String, NineEsimError> {
    let mut atr = Atr::from_hex(atr_hex)?;
    atr.set_euicc_capability(enabled)?;
    Ok(bytes_to_hex(atr.as_bytes()).to_ascii_uppercase())
}
