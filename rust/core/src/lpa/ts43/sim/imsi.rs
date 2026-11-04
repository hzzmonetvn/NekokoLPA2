//! IMSI and EF_AD decoding utilities.
//!
//! Ported from `sim/imsi.go` / `sim/imsi.dart`.

use core::fmt;
use std::error::Error;

use super::identifier::decode_swapped_bcd;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImsiError {
    EmptyPayload,
    TruncatedPayload,
    PayloadTooShort,
    TooShortForMcc,
    TruncatedAdPayload,
    MncLengthUnavailable,
    InvalidMncLength(u8),
    ImsiTooShortForMnc { imsi_len: usize, needed: usize },
    UnsupportedMncLength(usize),
}

impl fmt::Display for ImsiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPayload => write!(f, "reading EF_IMSI: empty payload"),
            Self::TruncatedPayload => write!(f, "reading EF_IMSI: truncated payload"),
            Self::PayloadTooShort => write!(f, "reading EF_IMSI: decoded payload is too short"),
            Self::TooShortForMcc => write!(f, "IMSI is too short for MCC"),
            Self::TruncatedAdPayload => write!(f, "parsing EF_AD: truncated payload"),
            Self::MncLengthUnavailable => write!(f, "parsing EF_AD: MNC length is unavailable"),
            Self::InvalidMncLength(len) => write!(f, "parsing EF_AD: invalid MNC length {len}"),
            Self::ImsiTooShortForMnc { imsi_len, needed } => {
                write!(f, "IMSI length {imsi_len} is too short for MNC ({needed})")
            }
            Self::UnsupportedMncLength(len) => write!(f, "unsupported MNC length {len}"),
        }
    }
}

impl Error for ImsiError {}

/// Represents an International Mobile Subscriber Identity (IMSI).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Imsi {
    value: String,
}

impl Imsi {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Parses EF_IMSI binary content (length-prefixed swapped BCD).
    pub fn from_bytes(data: &[u8]) -> Result<Self, ImsiError> {
        if data.is_empty() {
            return Err(ImsiError::EmptyPayload);
        }
        let n = usize::from(data[0]);
        if data.len() < n + 1 {
            return Err(ImsiError::TruncatedPayload);
        }
        let digits = decode_swapped_bcd(&data[1..=n], true);
        if digits.len() < 6 {
            return Err(ImsiError::PayloadTooShort);
        }
        Ok(Self { value: digits })
    }

    /// Returns the 3-digit Mobile Country Code (MCC).
    pub fn mcc(&self) -> Result<&str, ImsiError> {
        if self.value.len() < 3 {
            return Err(ImsiError::TooShortForMcc);
        }
        Ok(&self.value[..3])
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for Imsi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// Decodes the MNC length from EF_AD content.
pub fn decode_mnc_length(data: &[u8]) -> Result<usize, ImsiError> {
    if data.len() < 4 {
        return Err(ImsiError::TruncatedAdPayload);
    }
    match data[3] & 0x0F {
        0x02 => Ok(2),
        0x03 => Ok(3),
        0x00 => Err(ImsiError::MncLengthUnavailable),
        other => Err(ImsiError::InvalidMncLength(other)),
    }
}

/// Extracts the MNC substring from the IMSI given the MNC length.
pub fn format_mnc(imsi: &str, n: usize) -> Result<&str, ImsiError> {
    if imsi.len() < 3 + n {
        return Err(ImsiError::ImsiTooShortForMnc {
            imsi_len: imsi.len(),
            needed: 3 + n,
        });
    }
    match n {
        2 => Ok(&imsi[3..5]),
        3 => Ok(&imsi[3..6]),
        _ => Err(ImsiError::UnsupportedMncLength(n)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imsi_and_mnc_parsing() {
        let imsi = Imsi::new("234107813240779");
        assert_eq!(imsi.mcc().unwrap(), "234");
        assert_eq!(format_mnc(imsi.as_str(), 2).unwrap(), "10");
        assert_eq!(format_mnc(imsi.as_str(), 3).unwrap(), "107");

        let ad = [0x00, 0x00, 0x00, 0x02];
        assert_eq!(decode_mnc_length(&ad).unwrap(), 2);
    }
}
