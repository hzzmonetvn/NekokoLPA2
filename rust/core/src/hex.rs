//! Fast, allocation-conscious hexadecimal encoding and decoding for APDUs and SIM payloads.

use core::fmt;
use std::error::Error;

const LOWER_HEX: &[u8; 16] = b"0123456789abcdef";

/// Error returned when decoding an invalid hexadecimal string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HexError {
    /// Found an invalid character at the given byte position.
    InvalidCharacter { char: char, index: usize },
}

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCharacter { char, index } => {
                write!(f, "invalid hex character '{char}' at index {index}")
            }
        }
    }
}

impl Error for HexError {}

/// Encodes binary bytes to a lowercase hexadecimal string without separators.
#[must_use]
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = Vec::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(LOWER_HEX[(byte >> 4) as usize]);
        out.push(LOWER_HEX[(byte & 0x0f) as usize]);
    }
    // SAFETY: LOWER_HEX only contains ASCII characters 0-9 and a-f.
    unsafe { String::from_utf8_unchecked(out) }
}

/// Parses a hex string into owned bytes, ignoring ASCII space characters (`' '`).
///
/// An odd number of hex digits is read as though the string were left-padded with a zero.
///
/// # Errors
/// Returns [`HexError::InvalidCharacter`] if any non-space, non-hex character is encountered.
pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, HexError> {
    let mut nibbles: Vec<u8> = Vec::with_capacity(hex.len());
    for (idx, b) in hex.bytes().enumerate() {
        if b == b' ' {
            continue;
        }
        let nibble = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => {
                return Err(HexError::InvalidCharacter {
                    char: b as char,
                    index: idx,
                });
            }
        };
        nibbles.push(nibble);
    }

    let mut result = Vec::with_capacity(nibbles.len().div_ceil(2));
    let mut iter = nibbles.into_iter();

    if result.capacity() * 2 > iter.len() {
        // Odd number of nibbles: first byte takes implicit leading zero
        if let Some(first) = iter.next() {
            result.push(first);
        }
    }

    while let (Some(hi), Some(lo)) = (iter.next(), iter.next()) {
        result.push((hi << 4) | lo);
    }

    Ok(result)
}

/// Swaps adjacent nibbles in a hex string, ensuring even length by prepending 'F' if odd.
#[must_use]
pub fn swap_nibbles(input: &str) -> String {
    let mut s = String::with_capacity(input.len() + 1);
    if !input.len().is_multiple_of(2) {
        s.push('F');
    }
    s.push_str(input);

    let bytes = s.as_bytes();
    let (chunks, _) = bytes.as_chunks::<2>();
    let mut out = Vec::with_capacity(bytes.len());
    for chunk in chunks {
        out.push(chunk[1]);
        out.push(chunk[0]);
    }
    if s.is_ascii() {
        // SAFETY: `s` is verified ASCII, so swapping adjacent bytes yields only valid ASCII bytes.
        unsafe { String::from_utf8_unchecked(out) }
    } else {
        String::from_utf8(out).unwrap_or_else(|_| input.to_string())
    }
}
