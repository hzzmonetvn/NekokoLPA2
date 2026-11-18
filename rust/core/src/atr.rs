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

/// Checks ETSI TS 102 221 V17.1.0 §6.3.3 / Table 6.7 eUICC capability advertisement on a raw ATR.
///
/// Validates TS (`0x3B` direct or `0x3F` inverse convention), maximum ATR length (33 bytes),
/// interface byte chain structure, historical byte length, and TCK XOR checksum whenever any
/// protocol other than `T=0` is indicated. Returns `Some(true)` only when the first global `TB`
/// byte (`group > 2`) after a `TD` indicating `T=15` has both `b8` and `b2` (`0x82`) set,
/// `Some(false)` when structurally valid without that flag, and `None` when malformed or truncated.
#[must_use]
pub fn supports_euicc(atr_bytes: &[u8]) -> Option<bool> {
    parse_atr_euicc_flags(atr_bytes).map(|flags| flags.etsi_tb_euicc)
}

/// Returns `true` if `atr_bytes` is a structurally valid ISO/IEC 7816-3 ATR that advertises eUICC
/// capabilities either via ETSI TS 102 221 `TB(i > 2)` (`(tb & 0x82) == 0x82`) or via the
/// 9eSIM/known `T=15` global interface capability byte (`TA` `0x02`/`0x03` when `TB` is absent).
#[must_use]
pub fn is_likely_euicc_atr(atr_bytes: &[u8]) -> bool {
    let Some(flags) = parse_atr_euicc_flags(atr_bytes) else {
        return false;
    };
    flags.etsi_tb_euicc || flags.legacy_global_ta_euicc
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedAtrEuiccFlags {
    etsi_tb_euicc: bool,
    legacy_global_ta_euicc: bool,
}

fn parse_atr_euicc_flags(bytes: &[u8]) -> Option<ParsedAtrEuiccFlags> {
    if bytes.len() < 2 || bytes.len() > 33 {
        return None;
    }
    if bytes[0] != 0x3B && bytes[0] != 0x3F {
        return None;
    }

    let historical_count = usize::from(bytes[1] & 0x0F);
    let mut present = bytes[1] >> 4;
    let mut offset = 2usize;
    let mut group = 1usize;
    let mut protocol = 0u8;
    let mut needs_checksum = false;
    let mut global_euicc: Option<bool> = None;
    let mut first_t15_ta: Option<u8> = None;
    let mut saw_t15_tb = false;

    loop {
        if (present & 1) != 0 {
            let ta = *bytes.get(offset)?;
            offset += 1;
            if protocol == 15 && group > 2 && first_t15_ta.is_none() {
                first_t15_ta = Some(ta);
            }
        }
        if (present & 2) != 0 {
            let tb = *bytes.get(offset)?;
            offset += 1;
            if protocol == 15 && group > 2 && global_euicc.is_none() {
                saw_t15_tb = true;
                global_euicc = Some((tb & 0x82) == 0x82);
            }
        }
        if (present & 4) != 0 {
            if offset >= bytes.len() {
                return None;
            }
            offset += 1;
        }
        if (present & 8) == 0 {
            break;
        }
        let td = *bytes.get(offset)?;
        offset += 1;
        protocol = td & 0x0F;
        needs_checksum |= protocol != 0;
        present = td >> 4;
        group += 1;
    }

    let expected_len = offset + historical_count + usize::from(needs_checksum);
    if expected_len != bytes.len() {
        return None;
    }
    if needs_checksum {
        let xor_sum = bytes[1..].iter().fold(0u8, |acc, &b| acc ^ b);
        if xor_sum != 0 {
            return None;
        }
    }

    let legacy_global_ta_euicc = !saw_t15_tb && matches!(first_t15_ta, Some(0x02 | 0x03));

    Some(ParsedAtrEuiccFlags {
        etsi_tb_euicc: global_euicc.unwrap_or(false),
        legacy_global_ta_euicc,
    })
}
