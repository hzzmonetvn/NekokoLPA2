//! GSMA SGP.22 Profile Element (PE) models and BPP (Bound Profile Package) stream parser.

use crate::asn1::der::{Asn1Error, Asn1Reader};

/// Supported Profile Element types in GSMA SGP.22 / TCA Profile Package specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeType {
    Header,
    GenericFileManagement,
    PinCodes,
    PukCodes,
    AkaParameter,
    SecurityDomain,
    Application,
    RfU(u32),
    End,
}

impl PeType {
    #[must_use]
    pub fn from_tag(tag: u32) -> Self {
        match tag {
            0xBF20 | 0xBF27 => Self::Header,
            0xBF21 | 0xBF28 => Self::GenericFileManagement,
            0xBF22 | 0xBF29 => Self::PinCodes,
            0xBF23 | 0xBF2A => Self::PukCodes,
            0xBF24 | 0xBF2B => Self::AkaParameter,
            0xBF25 | 0xBF2C => Self::SecurityDomain,
            0xBF26 | 0xBF2D => Self::Application,
            0xBF2E | 0xBF2F => Self::End,
            other => Self::RfU(other),
        }
    }

    #[must_use]
    pub fn is_end(self) -> bool {
        matches!(self, Self::End)
    }
}

/// A parsed Profile Element inside a Bound Profile Package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileElement {
    pub pe_type: PeType,
    pub tag: u32,
    pub payload: Vec<u8>,
    pub raw_bytes: Vec<u8>,
}

impl ProfileElement {
    #[must_use]
    pub fn new(tag: u32, payload: Vec<u8>, raw_bytes: Vec<u8>) -> Self {
        Self {
            pe_type: PeType::from_tag(tag),
            tag,
            payload,
            raw_bytes,
        }
    }
}

/// Parses a Bound Profile Package byte stream into discrete Profile Elements.
///
/// # Errors
/// Returns [`Asn1Error`] if the stream contains malformed ASN.1 TLV structures.
pub fn parse_profile_package(bpp_bytes: &[u8]) -> Result<Vec<ProfileElement>, Asn1Error> {
    let mut reader = Asn1Reader::new(bpp_bytes);
    let nodes = reader.read_all()?;
    let mut elements = Vec::with_capacity(nodes.len());

    for node in nodes {
        elements.push(ProfileElement::new(node.tag, node.value, node.raw_bytes));
    }

    Ok(elements)
}

/// Validates that a profile package has a valid structure (starts with Header, finishes with End).
#[must_use]
pub fn validate_profile_elements(elements: &[ProfileElement]) -> bool {
    if elements.is_empty() {
        return false;
    }
    // Must begin with Header
    if elements[0].pe_type != PeType::Header {
        return false;
    }
    // Must terminate with End
    if elements.last().map(|e| e.pe_type) != Some(PeType::End) {
        return false;
    }
    true
}
