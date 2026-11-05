//! GSM swapped BCD encoding/decoding and SIM identifier utilities.
//!
//! Ported from `sim/identifier.go` / `sim/identifier.dart`.

use core::fmt;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifierError {
    InvalidBcd(String),
}

impl fmt::Display for IdentifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBcd(s) => write!(f, "invalid BCD value: {s}"),
        }
    }
}

impl Error for IdentifierError {}

/// Encodes a decimal/hex string as swapped GSM BCD.
///
/// Pads with a trailing 'F' nibble if the string length is odd.
pub fn encode_bcd(value: &str) -> Result<Vec<u8>, IdentifierError> {
    for ch in value.chars() {
        if !ch.is_ascii_hexdigit() {
            return Err(IdentifierError::InvalidBcd(value.to_string()));
        }
    }

    let mut chars: Vec<char> = value.chars().collect();
    if !chars.len().is_multiple_of(2) {
        chars.push('F');
    }

    let mut out = Vec::with_capacity(chars.len() / 2);
    for chunk in chars.as_chunks::<2>().0 {
        let hi = chunk[0].to_digit(16).unwrap_or(0) as u8;
        let lo = chunk[1].to_digit(16).unwrap_or(0) as u8;
        // Swap nibbles: GSM BCD stores least significant nibble first
        out.push(((lo << 4) & 0xF0) | (hi & 0x0F));
    }

    Ok(out)
}

/// Decodes swapped GSM BCD bytes to a decimal string, trimming trailing 'f' padding.
#[must_use]
pub fn decode_bcd(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 2);
    for &b in data {
        let lo = b & 0x0F;
        let hi = (b >> 4) & 0x0F;
        out.push(char::from_digit(u32::from(lo), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(hi), 16).unwrap_or('0'));
    }
    if data.last().is_some_and(|byte| byte >> 4 == 0x0F) {
        out.pop();
    }
    out
}

/// Decodes swapped BCD like the Dart SIM helper, optionally stripping a leading '9'.
#[must_use]
pub fn decode_swapped_bcd(data: &[u8], strip_leading_nine: bool) -> String {
    let mut out = String::with_capacity(data.len() * 2);
    for &byte in data {
        let low = byte & 0x0F;
        let high = (byte >> 4) & 0x0F;
        out.push(char::from_digit(u32::from(low), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(high), 16).unwrap_or('0'));
    }
    if let Some(index) = out.rfind('f') {
        out.truncate(index);
    }
    if strip_leading_nine && out.starts_with('9') {
        out.remove(0);
    }
    out
}

/// Renders an EF_GID value as an uppercase hex string with trailing `0xFF` padding trimmed.
#[must_use]
pub fn group_identifier_string(data: &[u8]) -> String {
    let mut end = data.len();
    while end > 0 && data[end - 1] == 0xFF {
        end -= 1;
    }
    if end == 0 {
        return String::new();
    }
    let mut out = String::with_capacity(end * 2);
    for &b in &data[..end] {
        use std::fmt::Write;
        let _ = write!(out, "{b:02X}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_bcd_roundtrip() {
        let raw = "234107813240779";
        let encoded = encode_bcd(raw).expect("encode");
        let decoded = decode_bcd(&encoded);
        assert_eq!(decoded, raw);
    }

    #[test]
    fn test_decode_bcd_trims_only_final_high_nibble_filler() {
        assert_eq!(decode_bcd(&[]), "");
        assert_eq!(decode_bcd(&[0x21, 0xF3]), "123");
        assert_eq!(decode_bcd(&[0xF1, 0x23]), "1f32");
        assert_eq!(decode_bcd(&[0x1F, 0x23]), "f132");
        assert_eq!(decode_bcd(&[0xF1]), "1");
    }

    #[test]
    fn test_decode_swapped_bcd_preserves_dart_trailing_filler_behavior() {
        assert_eq!(decode_swapped_bcd(&[0x21, 0xF3], false), "123");
        assert_eq!(decode_swapped_bcd(&[0x21, 0xF3], true), "123");
        assert_eq!(decode_swapped_bcd(&[0xF1, 0x23], false), "1");
        assert_eq!(decode_swapped_bcd(&[0x29], true), "2");
        assert_eq!(decode_swapped_bcd(&[], true), "");
    }

    #[test]
    fn test_group_identifier_string() {
        let data = [0x5A, 0x1B, 0xFF, 0xFF];
        assert_eq!(group_identifier_string(&data), "5A1B");
        assert_eq!(group_identifier_string(&[0xFF, 0xFF]), "");
    }
}
