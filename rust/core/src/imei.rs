//! IMEI / TAC codec and Luhn checksum implementation according to 3GPP and GSMA specs.

use core::fmt;
use std::error::Error;

/// Error returned when parsing or validating an IMEI or TAC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeiError {
    /// String length is invalid for TAC (must be 8 digits) or IMEI (must be 14 or 15 digits).
    InvalidLength {
        expected: &'static str,
        actual: usize,
    },
    /// Contains non-digit ASCII characters.
    NonDigitCharacter { char: char },
    /// Stored IMEI byte length must be exactly 8 bytes.
    InvalidByteLength { actual: usize },
    /// Invalid nibble value in stored BCD representation.
    InvalidNibble { nibble: u8 },
}

impl fmt::Display for ImeiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { expected, actual } => {
                write!(f, "invalid length: expected {expected}, got {actual}")
            }
            Self::NonDigitCharacter { char } => {
                write!(f, "non-digit character encountered: '{char}'")
            }
            Self::InvalidByteLength { actual } => {
                write!(f, "invalid stored IMEI byte length: {actual} (expected 8)")
            }
            Self::InvalidNibble { nibble } => {
                write!(f, "invalid BCD digit nibble: 0x{nibble:X}")
            }
        }
    }
}

impl Error for ImeiError {}

/// Calculates the Luhn checksum check digit for a 14-digit IMEI body.
///
/// # Errors
/// Returns [`ImeiError`] if the input does not contain exactly 14 ASCII digits.
pub fn calculate_imei_luhn_checksum(body_digits: &str) -> Result<u8, ImeiError> {
    let clean: String = body_digits.chars().filter(|c| c.is_ascii_digit()).collect();
    if clean.len() != 14 {
        return Err(ImeiError::InvalidLength {
            expected: "14 digits",
            actual: clean.len(),
        });
    }

    let mut sum: u32 = 0;
    for (idx, b) in clean.bytes().enumerate() {
        let mut digit = (b - b'0') as u32;
        // In IMEI Luhn (1-indexed odd/even, here 0-indexed): odd indices are multiplied by 2
        if idx % 2 != 0 {
            digit *= 2;
            if digit > 9 {
                digit = (digit % 10) + 1;
            }
        }
        sum += digit;
    }

    let rem = sum % 10;
    let check = (10 - rem) % 10;
    Ok(check as u8)
}

/// Reads a 15-digit IMEI string from stored 8-byte BCD representation.
///
/// # Errors
/// Returns [`ImeiError`] if the slice length is not 8 or contains non-BCD nibbles.
pub fn imei_digits_from_stored_bytes(imei_bytes: &[u8]) -> Result<String, ImeiError> {
    if imei_bytes.len() != 8 {
        return Err(ImeiError::InvalidByteLength {
            actual: imei_bytes.len(),
        });
    }

    let mut digits = String::with_capacity(15);
    for (index, &byte) in imei_bytes.iter().enumerate() {
        let hi = (byte >> 4) & 0x0f;
        let lo = byte & 0x0f;

        if hi > 9 {
            return Err(ImeiError::InvalidNibble { nibble: hi });
        }
        digits.push((b'0' + hi) as char);

        // The final nibble (index 7, lo) is BCD filler
        if index == 7 {
            break;
        }

        if lo > 9 {
            return Err(ImeiError::InvalidNibble { nibble: lo });
        }
        digits.push((b'0' + lo) as char);
    }

    Ok(digits)
}

/// Encodes a 15-digit IMEI string into the 8-byte stored BCD format.
///
/// # Errors
/// Returns [`ImeiError`] if `digits` does not contain exactly 15 digits.
pub fn encode_stored_imei_bytes(digits: &str) -> Result<[u8; 8], ImeiError> {
    let clean: Vec<u8> = digits
        .bytes()
        .filter(|b| b.is_ascii_digit())
        .map(|b| b - b'0')
        .collect();

    if clean.len() != 15 {
        return Err(ImeiError::InvalidLength {
            expected: "15 digits",
            actual: clean.len(),
        });
    }

    let mut out = [0u8; 8];
    for i in 0..7 {
        out[i] = (clean[i * 2] << 4) | clean[i * 2 + 1];
    }
    out[7] = clean[14] << 4;

    Ok(out)
}
