//! ASN.1 DER Tag-Length-Value (TLV) reader and node hierarchy for parsing GSMA SGP.22 eSIM metadata.

use core::fmt;
use std::error::Error;

/// Error returned when decoding an ASN.1 TLV stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asn1Error {
    /// Buffer ended unexpectedly while reading tag, length, or value.
    UnexpectedEof { offset: usize, needed: usize },
    /// Length encoding exceeds supported bounds.
    UnsupportedLength { num_bytes: usize },
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
        }
    }
}

impl Error for Asn1Error {}

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
    pub fn find_child(&self, tag: u32) -> Option<Self> {
        self.children().ok()?.into_iter().find(|c| c.is_tag(tag))
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
