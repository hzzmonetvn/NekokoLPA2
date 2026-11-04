//! Low-level ASN.1 DER primitives, Tag-Length-Value encoding and decoding.

use core::fmt;
use std::error::Error;

/// Error returned when encoding or decoding an ASN.1 TLV stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asn1Error {
    /// Buffer ended unexpectedly while reading tag, length, or value.
    UnexpectedEof { offset: usize, needed: usize },
    /// Length encoding exceeds supported bounds.
    UnsupportedLength { num_bytes: usize },
    /// Invalid integer encoding.
    InvalidInteger,
    /// Invalid UTF-8 string encoding.
    InvalidUtf8,
    /// Expected a specific tag but encountered another.
    UnexpectedTag { expected: u32, actual: u32 },
    /// Missing mandatory child field in sequence.
    MissingField { tag: u32, name: &'static str },
}

impl fmt::Display for Asn1Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof { offset, needed } => {
                write!(
                    f,
                    "unexpected EOF at offset {offset}, needed {needed} bytes"
                )
            }
            Self::UnsupportedLength { num_bytes } => {
                write!(f, "unsupported ASN.1 length with {num_bytes} length octets")
            }
            Self::InvalidInteger => write!(f, "invalid ASN.1 integer encoding"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 string in ASN.1 payload"),
            Self::UnexpectedTag { expected, actual } => {
                write!(f, "expected tag 0x{expected:X}, found 0x{actual:X}")
            }
            Self::MissingField { tag, name } => {
                write!(f, "missing mandatory ASN.1 field '{name}' (tag 0x{tag:X})")
            }
        }
    }
}

impl Error for Asn1Error {}

/// Encodes an ASN.1 DER length field.
#[must_use]
pub fn encode_length(length: usize) -> Vec<u8> {
    if length < 0x80 {
        vec![length as u8]
    } else {
        let mut octets = Vec::new();
        let mut temp = length;
        while temp > 0 {
            octets.push((temp & 0xFF) as u8);
            temp >>= 8;
        }
        octets.reverse();
        let mut result = Vec::with_capacity(1 + octets.len());
        result.push(0x80 | (octets.len() as u8));
        result.extend_from_slice(&octets);
        result
    }
}

/// Encodes an ASN.1 DER multi-byte or single-byte tag.
#[must_use]
pub fn encode_tag(tag: u32) -> Vec<u8> {
    if tag <= 0xFF {
        vec![tag as u8]
    } else if tag <= 0xFFFF {
        vec![(tag >> 8) as u8, (tag & 0xFF) as u8]
    } else if tag <= 0xFF_FFFF {
        vec![
            (tag >> 16) as u8,
            ((tag >> 8) & 0xFF) as u8,
            (tag & 0xFF) as u8,
        ]
    } else {
        vec![
            (tag >> 24) as u8,
            ((tag >> 16) & 0xFF) as u8,
            ((tag >> 8) & 0xFF) as u8,
            (tag & 0xFF) as u8,
        ]
    }
}

/// Encodes a complete ASN.1 TLV (Tag-Length-Value) packet.
#[must_use]
pub fn encode_tlv(tag: u32, value: &[u8]) -> Vec<u8> {
    let tag_bytes = encode_tag(tag);
    let len_bytes = encode_length(value.len());
    let mut out = Vec::with_capacity(tag_bytes.len() + len_bytes.len() + value.len());
    out.extend_from_slice(&tag_bytes);
    out.extend_from_slice(&len_bytes);
    out.extend_from_slice(value);
    out
}

/// Encodes an ASN.1 INTEGER (Tag 0x02).
#[must_use]
pub fn encode_integer(val: i64) -> Vec<u8> {
    if val == 0 {
        return encode_tlv(0x02, &[0x00]);
    }
    let mut bytes = val.to_be_bytes().to_vec();
    // Strip leading 0x00 or 0xFF that are not necessary for two's complement sign
    while bytes.len() > 1 {
        if (bytes[0] == 0x00 && (bytes[1] & 0x80) == 0)
            || (bytes[0] == 0xFF && (bytes[1] & 0x80) != 0)
        {
            bytes.remove(0);
        } else {
            break;
        }
    }
    encode_tlv(0x02, &bytes)
}

/// Decodes an ASN.1 INTEGER from value bytes.
///
/// # Errors
/// Returns [`Asn1Error::InvalidInteger`] if bytes are empty or exceed 8 bytes.
pub fn decode_integer(bytes: &[u8]) -> Result<i64, Asn1Error> {
    if bytes.is_empty() || bytes.len() > 8 {
        return Err(Asn1Error::InvalidInteger);
    }
    let mut sign_extended = if (bytes[0] & 0x80) != 0 {
        [0xFF; 8]
    } else {
        [0x00; 8]
    };
    sign_extended[8 - bytes.len()..].copy_from_slice(bytes);
    Ok(i64::from_be_bytes(sign_extended))
}

/// Encodes an ASN.1 OCTET STRING (Tag 0x04).
#[must_use]
pub fn encode_octet_string(bytes: &[u8]) -> Vec<u8> {
    encode_tlv(0x04, bytes)
}

/// Encodes an ASN.1 UTF8String (Tag 0x0C).
#[must_use]
pub fn encode_utf8_string(s: &str) -> Vec<u8> {
    encode_tlv(0x0C, s.as_bytes())
}

/// Decodes a UTF8String from value bytes.
///
/// # Errors
/// Returns [`Asn1Error::InvalidUtf8`] if the slice contains invalid UTF-8.
pub fn decode_utf8_string(bytes: &[u8]) -> Result<String, Asn1Error> {
    String::from_utf8(bytes.to_vec()).map_err(|_| Asn1Error::InvalidUtf8)
}

/// Encodes an ASN.1 SEQUENCE (Tag 0x30) containing multiple encoded TLV elements.
#[must_use]
pub fn encode_sequence(elements: &[Vec<u8>]) -> Vec<u8> {
    let total_len: usize = elements.iter().map(Vec::len).sum();
    let mut payload = Vec::with_capacity(total_len);
    for el in elements {
        payload.extend_from_slice(el);
    }
    encode_tlv(0x30, &payload)
}

/// An ASN.1 Tag-Length-Value node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asn1Node {
    pub tag: u32,
    pub value: Vec<u8>,
    pub raw_bytes: Vec<u8>,
}

impl Asn1Node {
    /// Creates a new `Asn1Node`.
    #[must_use]
    pub fn new(tag: u32, value: Vec<u8>, raw_bytes: Vec<u8>) -> Self {
        Self {
            tag,
            value,
            raw_bytes,
        }
    }

    /// Checks if this node matches the specified tag.
    #[must_use]
    pub fn is_tag(&self, tag: u32) -> bool {
        self.tag == tag
    }

    /// Recursively decodes child nodes if the value represents an ASN.1 constructed sequence.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] if child parsing encounters an invalid TLV stream.
    pub fn children(&self) -> Result<Vec<Self>, Asn1Error> {
        let mut reader = Asn1Reader::new(&self.value);
        reader.read_all()
    }

    /// Finds the first child node matching a specific tag.
    #[must_use]
    pub fn find_child(&self, tag: u32) -> Option<Self> {
        self.children().ok()?.into_iter().find(|c| c.is_tag(tag))
    }

    /// Returns the child node matching `tag`, or an error if missing.
    ///
    /// # Errors
    /// Returns [`Asn1Error::MissingField`] if the child tag is not found.
    pub fn require_child(&self, tag: u32, name: &'static str) -> Result<Self, Asn1Error> {
        self.find_child(tag)
            .ok_or(Asn1Error::MissingField { tag, name })
    }
}

/// Streaming reader for ASN.1 DER byte buffers.
pub struct Asn1Reader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Asn1Reader<'a> {
    /// Creates a new `Asn1Reader` over a byte slice.
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    /// Returns `true` if more bytes remain in the stream.
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.offset < self.data.len()
    }

    /// Current read offset.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Reads all TLV nodes from the current position to the end of the buffer.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] on malformed TLV structure.
    pub fn read_all(&mut self) -> Result<Vec<Asn1Node>, Asn1Error> {
        let mut nodes = Vec::new();
        while self.has_more() {
            if let Some(node) = self.read_next()? {
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    /// Reads the next ASN.1 TLV node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] on malformed TLV structure.
    pub fn read_next(&mut self) -> Result<Option<Asn1Node>, Asn1Error> {
        if self.offset >= self.data.len() {
            return Ok(None);
        }

        let start_offset = self.offset;

        // 1. Read Tag
        let first_byte = self.data[self.offset];
        self.offset += 1;
        let mut tag_value = first_byte as u32;

        // Extended multi-byte tag
        if (first_byte & 0x1F) == 0x1F {
            while self.offset < self.data.len() {
                let b = self.data[self.offset];
                self.offset += 1;
                tag_value = (tag_value << 8) | (b as u32);
                if (b & 0x80) == 0 {
                    break;
                }
            }
        }

        // 2. Read Length
        if self.offset >= self.data.len() {
            return Err(Asn1Error::UnexpectedEof {
                offset: self.offset,
                needed: 1,
            });
        }

        let length_byte = self.data[self.offset];
        self.offset += 1;

        let length: usize = if (length_byte & 0x80) == 0 {
            length_byte as usize
        } else {
            let num_bytes = (length_byte & 0x7F) as usize;
            if num_bytes == 0 {
                return Err(Asn1Error::UnsupportedLength { num_bytes });
            }
            if num_bytes > 4 {
                return Err(Asn1Error::UnsupportedLength { num_bytes });
            }
            if self.offset + num_bytes > self.data.len() {
                return Err(Asn1Error::UnexpectedEof {
                    offset: self.offset,
                    needed: num_bytes,
                });
            }
            let mut len = 0usize;
            for _ in 0..num_bytes {
                len = (len << 8) | (self.data[self.offset] as usize);
                self.offset += 1;
            }
            len
        };

        // 3. Read Value
        if self.offset + length > self.data.len() {
            return Err(Asn1Error::UnexpectedEof {
                offset: self.offset,
                needed: length,
            });
        }

        let value = self.data[self.offset..self.offset + length].to_vec();
        self.offset += length;

        let raw_bytes = self.data[start_offset..self.offset].to_vec();

        Ok(Some(Asn1Node {
            tag: tag_value,
            value,
            raw_bytes,
        }))
    }
}
