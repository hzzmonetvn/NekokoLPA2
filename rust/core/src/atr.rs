//! ISO/IEC 7816-3 Answer to Reset (ATR) parser and manipulator.
//!
//! Handles direct-convention ATRs (`TS = 0x3B`), interface byte traversal
//! (T0, TA, TB, TC, TD), protocol T=15 global interface byte detection,
//! GSMA SGP.22 eUICC capability bit inspection/toggling, and TCK checksum
//! recalculation enforcing `XOR(T0..TCK) == 0`.

use std::fmt;

use crate::hex::{bytes_to_hex, hex_to_bytes};

/// Standard default ATR advertised by eUICC smartcards and CCID readers.
pub const DEFAULT_EUICC_ATR: &str = "3B9F96801F038031E073FE21136702434F537EC101FF";

/// Errors encountered while parsing or manipulating an ISO/IEC 7816-3 ATR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtrError {
    InvalidHex(String),
    InvalidLength(usize),
    UnsupportedConvention(u8),
    MalformedInterfaceBytes,
    MissingT15Protocol,
    MissingGlobalInterfaceByteAfterT15,
    AtrTooLong(usize),
}

impl fmt::Display for AtrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHex(err) => write!(f, "Invalid ATR hex encoding: {err}"),
            Self::InvalidLength(len) => {
                write!(f, "Invalid ATR length: {len} bytes (minimum is 2)")
            }
            Self::UnsupportedConvention(ts) => {
                write!(
                    f,
                    "Unsupported ATR initial character 0x{ts:02X} (expected direct convention 0x3B)"
                )
            }
            Self::MalformedInterfaceBytes => write!(f, "Malformed ATR interface bytes"),
            Self::MissingT15Protocol => write!(f, "ATR does not advertise protocol T=15"),
            Self::MissingGlobalInterfaceByteAfterT15 => {
                write!(f, "ATR has no global interface byte after T=15")
            }
            Self::AtrTooLong(len) => write!(f, "ATR is too long: {len} bytes (maximum is 255)"),
        }
    }
}

impl std::error::Error for AtrError {}

/// Represents an ISO/IEC 7816-3 Answer to Reset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atr {
    raw: Vec<u8>,
}

impl Atr {
    /// Parses an ATR from raw bytes, verifying direct convention (`0x3B`) and length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AtrError> {
        if bytes.len() < 2 {
            return Err(AtrError::InvalidLength(bytes.len()));
        }
        if bytes[0] != 0x3B {
            return Err(AtrError::UnsupportedConvention(bytes[0]));
        }
        if bytes.len() > 255 {
            return Err(AtrError::AtrTooLong(bytes.len()));
        }
        Ok(Self {
            raw: bytes.to_vec(),
        })
    }

    /// Parses an ATR from a hexadecimal string, stripping whitespace and validating format.
    pub fn from_hex(hex_str: &str) -> Result<Self, AtrError> {
        let normalized = hex_str.split_whitespace().collect::<String>();
        if normalized.len() < 4 || normalized.len() % 2 != 0 {
            return Err(AtrError::InvalidHex(
                "ATR hex string must have even length and at least 4 characters".to_string(),
            ));
        }
        let bytes = hex_to_bytes(&normalized).map_err(|e| AtrError::InvalidHex(e.to_string()))?;
        Self::from_bytes(&bytes)
    }

    /// Returns the raw ATR bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw
    }

    /// Consumes the ATR and returns its owned byte vector.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.raw
    }

    /// Formats the ATR as an uppercase hexadecimal string.
    #[must_use]
    pub fn to_hex(&self) -> String {
        bytes_to_hex(&self.raw).to_ascii_uppercase()
    }

    /// Returns the initial character TS (`0x3B` for direct convention).
    #[must_use]
    pub fn ts(&self) -> u8 {
        self.raw[0]
    }

    /// Returns the format byte T0.
    #[must_use]
    pub fn t0(&self) -> u8 {
        self.raw[1]
    }

    /// Returns the number of historical bytes K (bits 4..=1 of T0).
    #[must_use]
    pub fn historical_bytes_count(&self) -> usize {
        (self.t0() & 0x0F) as usize
    }

    /// Finds the byte index of the first global interface byte following `T=15`.
    ///
    /// ISO/IEC 7816-3 specifies that when a TD byte indicates protocol `T=15`,
    /// the subsequent interface bytes are global interface bytes.
    pub fn global_interface_byte_index(&self) -> Result<usize, AtrError> {
        let bytes = &self.raw;
        let mut index = 2;
        let mut presence = bytes[1] >> 4;

        loop {
            if (presence & 0x01) != 0 {
                index += 1;
            }
            if (presence & 0x02) != 0 {
                index += 1;
            }
            if (presence & 0x04) != 0 {
                index += 1;
            }
            if (presence & 0x08) == 0 {
                break;
            }
            if index >= bytes.len().saturating_sub(1) {
                return Err(AtrError::MalformedInterfaceBytes);
            }

            let td = bytes[index];
            index += 1;
            presence = td >> 4;

            if (td & 0x0F) == 0x0F {
                if presence == 0 || index >= bytes.len().saturating_sub(1) {
                    return Err(AtrError::MissingGlobalInterfaceByteAfterT15);
                }
                return Ok(index);
            }
        }

        Err(AtrError::MissingT15Protocol)
    }

    /// Checks if the eUICC capability bit (bit 1, `0x02`) is set in the T=15 global interface byte.
    pub fn euicc_capability_enabled(&self) -> Result<bool, AtrError> {
        let index = self.global_interface_byte_index()?;
        Ok((self.raw[index] & 0x02) != 0)
    }

    /// Sets or clears the eUICC capability bit (bit 1, `0x02`) in the T=15 global interface byte,
    /// and recalculates the trailing ISO/IEC 7816-3 TCK checksum byte so that `XOR(T0..TCK) == 0`.
    pub fn set_euicc_capability(&mut self, enabled: bool) -> Result<(), AtrError> {
        let index = self.global_interface_byte_index()?;
        if enabled {
            self.raw[index] |= 0x02;
        } else {
            self.raw[index] &= !0x02;
        }

        self.recalculate_tck();
        Ok(())
    }

    /// Recalculates the trailing TCK checksum byte enforcing `XOR(T0..TCK) == 0`.
    ///
    /// Per ISO/IEC 7816-3, when any protocol other than T=0 alone is indicated (or T=15 is present),
    /// the last byte is TCK, which is the XOR sum of all bytes from T0 (byte 1) up to TCK-1.
    pub fn recalculate_tck(&mut self) {
        if self.raw.len() < 2 {
            return;
        }
        let last_idx = self.raw.len() - 1;
        let mut checksum = 0u8;
        for &b in &self.raw[1..last_idx] {
            checksum ^= b;
        }
        self.raw[last_idx] = checksum;
    }
}
