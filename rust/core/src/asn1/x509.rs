//! X.509 PKIX certificate parser and GSMA SGP.22 Certificate Issuer (CI) constraint verification.
//!
//! Provides a zero-dependency, safe Rust implementation for:
//! - Parsing DER-encoded X.509 v1/v2/v3 certificates (`RFC 5280`)
//! - Extracting Subject Key Identifier (SKI, OID `2.5.29.14`) and Authority Key Identifier (AKI, OID `2.5.29.35`)
//! - RFC 5280 Method 1 fallback SKI computation via 160-bit SHA-1 (`SHA1(subjectPublicKey)`)
//! - Parsing Subject and Issuer Distinguished Names (Common Name `CN`, Organization `O`, Country `C`)
//! - Parsing Basic Constraints (CA flag) and Key Usage
//! - GSMA SGP.22 CI public key constraint verification against `euiccCiPKIdListForVerification`

use std::fmt;

use crate::asn1::der::{decode_integer, decode_utf8_string, Asn1Error, Asn1Node, Asn1Reader};
use crate::crypto::{base64_decode, sha1};
use crate::hex::{bytes_to_hex, hex_to_bytes};

// Standard X.509 OID byte constants
const OID_COMMON_NAME: &[u8] = &[0x55, 0x04, 0x03]; // 2.5.4.3
const OID_COUNTRY: &[u8] = &[0x55, 0x04, 0x06]; // 2.5.4.6
const OID_ORGANIZATION: &[u8] = &[0x55, 0x04, 0x0A]; // 2.5.4.10

const OID_SUBJECT_KEY_IDENTIFIER: &[u8] = &[0x55, 0x1D, 0x0E]; // 2.5.29.14
const OID_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x0F]; // 2.5.29.15
const OID_BASIC_CONSTRAINTS: &[u8] = &[0x55, 0x1D, 0x13]; // 2.5.29.19
const OID_AUTHORITY_KEY_IDENTIFIER: &[u8] = &[0x55, 0x1D, 0x23]; // 2.5.29.35

/// Errors encountered while parsing or validating an X.509 PKIX certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum X509Error {
    Asn1(Asn1Error),
    InvalidCertificateStructure(&'static str),
    UnsupportedVersion(u32),
    MissingField(&'static str),
    InvalidPem,
    EmptyCertificate,
}

impl fmt::Display for X509Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Asn1(err) => write!(f, "ASN.1 parsing error: {err}"),
            Self::InvalidCertificateStructure(msg) => {
                write!(f, "Invalid X.509 certificate structure: {msg}")
            }
            Self::UnsupportedVersion(v) => write!(f, "Unsupported X.509 version: {v}"),
            Self::MissingField(name) => write!(f, "Missing mandatory X.509 field: {name}"),
            Self::InvalidPem => write!(f, "Invalid PEM-encoded certificate"),
            Self::EmptyCertificate => write!(f, "Empty certificate payload"),
        }
    }
}

impl std::error::Error for X509Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Asn1(err) => Some(err),
            _ => None,
        }
    }
}

impl From<Asn1Error> for X509Error {
    fn from(err: Asn1Error) -> Self {
        Self::Asn1(err)
    }
}

/// Converts an ASN.1 OBJECT IDENTIFIER byte slice into a dotted string (e.g. "2.5.29.14").
#[must_use]
pub fn oid_bytes_to_string(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    let first = bytes[0];
    let b1 = first / 40;
    let b2 = first % 40;

    let mut parts = vec![b1.to_string(), b2.to_string()];
    let mut current_val: u64 = 0;

    for &b in &bytes[1..] {
        current_val = (current_val << 7) | u64::from(b & 0x7F);
        if (b & 0x80) == 0 {
            parts.push(current_val.to_string());
            current_val = 0;
        }
    }

    parts.join(".")
}

/// Distinguished Name (DN) attribute components extracted from an X.509 certificate.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct X509Name {
    pub common_name: Option<String>,
    pub organization: Option<String>,
    pub country: Option<String>,
    pub raw_bytes: Vec<u8>,
}

impl X509Name {
    /// Parses an ASN.1 RDNSequence into an `X509Name`.
    pub fn parse(node: &Asn1Node) -> Result<Self, X509Error> {
        let mut common_name = None;
        let mut organization = None;
        let mut country = None;

        let rdn_sets = node.children()?;
        for rdn_set in rdn_sets {
            let attributes = rdn_set.children().unwrap_or_default();
            for attr in attributes {
                let parts = attr.children().unwrap_or_default();
                if parts.len() >= 2 {
                    let type_oid = &parts[0].value;
                    let val_node = &parts[1];
                    let text = decode_utf8_string(&val_node.value).ok();

                    if type_oid == OID_COMMON_NAME && common_name.is_none() {
                        common_name = text;
                    } else if type_oid == OID_ORGANIZATION && organization.is_none() {
                        organization = text;
                    } else if type_oid == OID_COUNTRY && country.is_none() {
                        country = text;
                    }
                }
            }
        }

        Ok(Self {
            common_name,
            organization,
            country,
            raw_bytes: node.raw_bytes.clone(),
        })
    }
}

/// Certificate validity period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validity {
    pub not_before: String,
    pub not_after: String,
}

impl Validity {
    /// Parses the Validity sequence (`notBefore` and `notAfter`).
    pub fn parse(node: &Asn1Node) -> Result<Self, X509Error> {
        let children = node.children()?;
        if children.len() < 2 {
            return Err(X509Error::InvalidCertificateStructure(
                "Validity sequence requires notBefore and notAfter",
            ));
        }
        let not_before = decode_utf8_string(&children[0].value)
            .unwrap_or_else(|_| bytes_to_hex(&children[0].value));
        let not_after = decode_utf8_string(&children[1].value)
            .unwrap_or_else(|_| bytes_to_hex(&children[1].value));
        Ok(Self {
            not_before,
            not_after,
        })
    }
}

/// Subject Public Key Info (`AlgorithmIdentifier` + `BIT STRING` public key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectPublicKeyInfo {
    pub algorithm_oid: String,
    pub public_key_bytes: Vec<u8>,
    pub raw_bytes: Vec<u8>,
}

impl SubjectPublicKeyInfo {
    /// Parses the SubjectPublicKeyInfo ASN.1 sequence.
    pub fn parse(node: &Asn1Node) -> Result<Self, X509Error> {
        let children = node.children()?;
        if children.len() < 2 {
            return Err(X509Error::InvalidCertificateStructure(
                "SubjectPublicKeyInfo requires algorithm and subjectPublicKey",
            ));
        }

        // 1. AlgorithmIdentifier
        let alg_children = children[0].children().unwrap_or_default();
        let algorithm_oid = if !alg_children.is_empty() {
            oid_bytes_to_string(&alg_children[0].value)
        } else {
            String::new()
        };

        // 2. subjectPublicKey (BIT STRING: first byte is number of unused bits, followed by raw key)
        let bit_string_val = &children[1].value;
        let public_key_bytes = if !bit_string_val.is_empty() {
            bit_string_val[1..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Self {
            algorithm_oid,
            public_key_bytes,
            raw_bytes: node.raw_bytes.clone(),
        })
    }
}

/// An X.509 extension (OID, critical flag, and raw DER octets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X509Extension {
    pub oid: String,
    pub critical: bool,
    pub value: Vec<u8>,
}

/// A parsed X.509 PKIX certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X509Certificate {
    pub version: u32,
    pub serial_number: Vec<u8>,
    pub signature_algorithm: String,
    pub issuer: X509Name,
    pub validity: Validity,
    pub subject: X509Name,
    pub subject_public_key_info: SubjectPublicKeyInfo,
    pub subject_key_identifier: Option<Vec<u8>>,
    pub authority_key_identifier: Option<Vec<u8>>,
    pub is_ca: bool,
    pub key_usage: Option<u16>,
    pub raw_tbs: Vec<u8>,
    pub signature_value: Vec<u8>,
}

impl X509Certificate {
    /// Parses an X.509 certificate from raw DER-encoded bytes.
    pub fn from_der(der_bytes: &[u8]) -> Result<Self, X509Error> {
        if der_bytes.is_empty() {
            return Err(X509Error::EmptyCertificate);
        }

        let mut root_reader = Asn1Reader::new(der_bytes);
        let root_node = root_reader
            .read_next()?
            .ok_or(X509Error::EmptyCertificate)?;

        // Root must be SEQUENCE (Tag 0x30)
        if !root_node.is_tag(0x30) {
            return Err(X509Error::InvalidCertificateStructure(
                "Certificate root must be an ASN.1 SEQUENCE",
            ));
        }

        let cert_parts = root_node.children()?;
        if cert_parts.len() < 3 {
            return Err(X509Error::InvalidCertificateStructure(
                "Certificate requires tbsCertificate, signatureAlgorithm, and signatureValue",
            ));
        }

        let tbs_node = &cert_parts[0];
        let sig_alg_node = &cert_parts[1];
        let sig_val_node = &cert_parts[2];

        // 1. Signature Algorithm OID
        let sig_alg_children = sig_alg_node.children().unwrap_or_default();
        let signature_algorithm = if !sig_alg_children.is_empty() {
            oid_bytes_to_string(&sig_alg_children[0].value)
        } else {
            String::new()
        };

        // 2. Signature Value
        let signature_value = if !sig_val_node.value.is_empty() {
            sig_val_node.value[1..].to_vec() // strip unused bits
        } else {
            Vec::new()
        };

        // 3. Parse TBSCertificate fields
        let tbs_children = tbs_node.children()?;
        let mut idx = 0;

        // Version (optional [0] EXPLICIT, default v1=0)
        let mut version = 1;
        if idx < tbs_children.len() && (tbs_children[idx].tag == 0xA0) {
            let ver_children = tbs_children[idx].children().unwrap_or_default();
            if !ver_children.is_empty() {
                version = (decode_integer(&ver_children[0].value).unwrap_or(0) + 1) as u32;
            }
            idx += 1;
        }

        // Serial Number
        if idx >= tbs_children.len() {
            return Err(X509Error::MissingField("serialNumber"));
        }
        let serial_number = tbs_children[idx].value.clone();
        idx += 1;

        // Signature Algorithm inside TBS
        if idx >= tbs_children.len() {
            return Err(X509Error::MissingField("signature"));
        }
        idx += 1;

        // Issuer
        if idx >= tbs_children.len() {
            return Err(X509Error::MissingField("issuer"));
        }
        let issuer = X509Name::parse(&tbs_children[idx])?;
        idx += 1;

        // Validity
        if idx >= tbs_children.len() {
            return Err(X509Error::MissingField("validity"));
        }
        let validity = Validity::parse(&tbs_children[idx])?;
        idx += 1;

        // Subject
        if idx >= tbs_children.len() {
            return Err(X509Error::MissingField("subject"));
        }
        let subject = X509Name::parse(&tbs_children[idx])?;
        idx += 1;

        // SubjectPublicKeyInfo
        if idx >= tbs_children.len() {
            return Err(X509Error::MissingField("subjectPublicKeyInfo"));
        }
        let subject_public_key_info = SubjectPublicKeyInfo::parse(&tbs_children[idx])?;
        idx += 1;

        // Optional Extensions ([3] EXPLICIT Tag 0xA3)
        let mut subject_key_identifier = None;
        let mut authority_key_identifier = None;
        let mut is_ca = false;
        let mut key_usage = None;

        for item in &tbs_children[idx..] {
            if item.tag == 0xA3 {
                // Extensions sequence
                let ext_seq = item.children().unwrap_or_default();
                for ext_group in ext_seq {
                    let extensions = ext_group.children().unwrap_or_default();
                    for ext in extensions {
                        let ext_parts = ext.children().unwrap_or_default();
                        if ext_parts.is_empty() {
                            continue;
                        }

                        let oid = &ext_parts[0].value;
                        let val_node = if ext_parts.len() >= 3 {
                            &ext_parts[2] // [oid, critical_bool, value]
                        } else if ext_parts.len() >= 2 {
                            &ext_parts[1] // [oid, value]
                        } else {
                            continue;
                        };

                        if oid == OID_SUBJECT_KEY_IDENTIFIER {
                            // OCTET STRING containing OCTET STRING (or direct key id)
                            let mut inner_reader = Asn1Reader::new(&val_node.value);
                            if let Ok(Some(inner_node)) = inner_reader.read_next() {
                                subject_key_identifier = Some(inner_node.value);
                            } else {
                                subject_key_identifier = Some(val_node.value.clone());
                            }
                        } else if oid == OID_AUTHORITY_KEY_IDENTIFIER {
                            // SEQUENCE containing [0] IMPLICIT KeyIdentifier
                            let mut inner_reader = Asn1Reader::new(&val_node.value);
                            if let Ok(Some(aki_seq)) = inner_reader.read_next() {
                                if let Ok(aki_children) = aki_seq.children() {
                                    for child in aki_children {
                                        if child.tag == 0x80 {
                                            authority_key_identifier = Some(child.value);
                                            break;
                                        }
                                    }
                                }
                            }
                        } else if oid == OID_BASIC_CONSTRAINTS {
                            let mut inner_reader = Asn1Reader::new(&val_node.value);
                            if let Ok(Some(bc_seq)) = inner_reader.read_next() {
                                if let Ok(bc_children) = bc_seq.children() {
                                    for child in bc_children {
                                        if child.tag == 0x01 && !child.value.is_empty() {
                                            is_ca = child.value[0] != 0;
                                        }
                                    }
                                }
                            }
                        } else if oid == OID_KEY_USAGE {
                            let mut inner_reader = Asn1Reader::new(&val_node.value);
                            if let Ok(Some(ku_node)) = inner_reader.read_next() {
                                if ku_node.value.len() >= 2 {
                                    key_usage = Some(u16::from(ku_node.value[1]));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            version,
            serial_number,
            signature_algorithm,
            issuer,
            validity,
            subject,
            subject_public_key_info,
            subject_key_identifier,
            authority_key_identifier,
            is_ca,
            key_usage,
            raw_tbs: tbs_node.raw_bytes.clone(),
            signature_value,
        })
    }

    /// Parses an X.509 certificate from a PEM-encoded string.
    pub fn from_pem(pem_str: &str) -> Result<Self, X509Error> {
        let lines: Vec<&str> = pem_str.lines().collect();
        let mut b64 = String::new();
        let mut in_cert = false;

        for line in lines {
            let trimmed = line.trim();
            if trimmed == "-----BEGIN CERTIFICATE-----" {
                in_cert = true;
                continue;
            }
            if trimmed == "-----END CERTIFICATE-----" {
                break;
            }
            if in_cert {
                b64.push_str(trimmed);
            }
        }

        if b64.is_empty() {
            return Err(X509Error::InvalidPem);
        }

        let der = base64_decode(&b64).map_err(|_| X509Error::InvalidPem)?;
        Self::from_der(&der)
    }

    /// Returns the 20-byte Subject Key Identifier (SKI).
    ///
    /// If the `SubjectKeyIdentifier` extension is present and exactly 20 bytes long,
    /// it is returned directly. Otherwise, it is computed via RFC 5280 Method 1:
    /// `SHA-1(subjectPublicKeyInfo.public_key_bytes)`.
    #[must_use]
    pub fn ski(&self) -> [u8; 20] {
        if let Some(ref ext_ski) = self.subject_key_identifier {
            if ext_ski.len() == 20 {
                let mut out = [0u8; 20];
                out.copy_from_slice(ext_ski);
                return out;
            }
        }

        // Fallback: 160-bit SHA-1 digest of subjectPublicKeyInfo
        sha1(&self.subject_public_key_info.public_key_bytes)
    }

    /// Formats the serial number as an uppercase hexadecimal string.
    #[must_use]
    pub fn serial_number_hex(&self) -> String {
        bytes_to_hex(&self.serial_number).to_ascii_uppercase()
    }
}

/// Verifies whether the certificate's Authority Key Identifier (AKI) or Subject Key Identifier (SKI)
/// matches any Certificate Issuer (CI) PKID permitted by the eUICC's `euiccCiPKIdListForVerification`.
///
/// Implemented per GSMA SGP.22 Section 2.8.2 and Section 5.6.3 for SM-DP+ and SM-DS authentication.
#[must_use]
pub fn verify_ci_public_key_constraint(
    cert: &X509Certificate,
    allowed_ci_pkids: &[Vec<u8>],
) -> bool {
    if allowed_ci_pkids.is_empty() {
        return false;
    }

    // 1. Check Authority Key Identifier (AKI)
    if let Some(ref aki) = cert.authority_key_identifier {
        if allowed_ci_pkids.iter().any(|allowed| allowed == aki) {
            return true;
        }
    }

    // 2. Check Subject Key Identifier (SKI) (for self-signed CI root or direct match)
    let cert_ski = cert.ski();
    allowed_ci_pkids
        .iter()
        .any(|allowed| allowed.as_slice() == cert_ski)
}

/// Finds and returns the first matching CI PKID from the allowed list, if verified.
#[must_use]
pub fn find_matching_ci_pkid<'a>(
    cert: &X509Certificate,
    allowed_ci_pkids: &'a [Vec<u8>],
) -> Option<&'a [u8]> {
    if let Some(ref aki) = cert.authority_key_identifier {
        if let Some(m) = allowed_ci_pkids.iter().find(|allowed| *allowed == aki) {
            return Some(m.as_slice());
        }
    }

    let cert_ski = cert.ski();
    allowed_ci_pkids
        .iter()
        .find(|allowed| allowed.as_slice() == cert_ski)
        .map(Vec::as_slice)
}

/// Formats a 20-byte CI PKID (Subject Key Identifier) as an uppercase hexadecimal string.
#[must_use]
pub fn format_ci_pkid(ski: &[u8]) -> String {
    bytes_to_hex(ski).to_ascii_uppercase()
}

/// Parses a list of hex-encoded CI PKIDs into raw 20-byte vectors.
pub fn parse_ci_pkid_hex_list(hex_list: &[&str]) -> Result<Vec<Vec<u8>>, X509Error> {
    let mut out = Vec::with_capacity(hex_list.len());
    for s in hex_list {
        let bytes = hex_to_bytes(s)
            .map_err(|_| X509Error::InvalidCertificateStructure("Invalid hex in CI PKID list"))?;
        if bytes.len() != 20 {
            return Err(X509Error::InvalidCertificateStructure(
                "CI PKID must be exactly 20 bytes (40 hex characters)",
            ));
        }
        out.push(bytes);
    }
    Ok(out)
}
