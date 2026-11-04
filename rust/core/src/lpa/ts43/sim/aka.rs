//! EAP-AKA authentication challenge, response, and FIPS-186-2 PRF key derivation.
//!
//! Ported from `sim/aka.go` / `sim/aka.dart`.

use core::fmt;
use std::error::Error;

use crate::crypto::{base64_decode, base64_encode, hmac_sha1, sha1};

const EAP_CODE_REQUEST: u8 = 0x01;
const EAP_CODE_RESPONSE: u8 = 0x02;
const EAP_TYPE_AKA: u8 = 0x17;

const AKA_SUBTYPE_CHALLENGE: u8 = 0x01;
const AKA_SUBTYPE_SYNC_FAILURE: u8 = 0x04;

const ATTRIBUTE_RAND: u8 = 0x01;
const ATTRIBUTE_AUTN: u8 = 0x02;
const ATTRIBUTE_RES: u8 = 0x03;
const ATTRIBUTE_AUTS: u8 = 0x04;
const ATTRIBUTE_MAC: u8 = 0x0B;

const SECURITY_CONTEXT_SUCCESS: u8 = 0xDB;
const SECURITY_CONTEXT_SYNC_FAILURE: u8 = 0xDC;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AkaError {
    InvalidLength {
        expected: usize,
        actual: usize,
        field: &'static str,
    },
    EmptySecurityContext,
    UnknownSecurityContext(u8),
    ChallengeTooShort(usize),
    UnexpectedEapCode(u8),
    PacketLengthMismatch {
        header: usize,
        actual: usize,
    },
    UnexpectedEapType(u8),
    UnexpectedAkaSubtype(u8),
    TruncatedAttribute,
    InvalidAttributeLength(usize),
    MissingAttribute(&'static str),
    MissingRequiredField(&'static str),
    Base64Decode(String),
}

impl fmt::Display for AkaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength {
                expected,
                actual,
                field,
            } => {
                write!(f, "{field} length {actual}, want {expected}")
            }
            Self::EmptySecurityContext => write!(f, "security context is empty"),
            Self::UnknownSecurityContext(tag) => {
                write!(f, "unknown security context tag 0x{tag:02X}")
            }
            Self::ChallengeTooShort(len) => write!(f, "EAP-AKA challenge too short ({len} bytes)"),
            Self::UnexpectedEapCode(code) => write!(f, "unexpected EAP code 0x{code:02X}"),
            Self::PacketLengthMismatch { header, actual } => {
                write!(
                    f,
                    "EAP length {header} does not match packet length {actual}"
                )
            }
            Self::UnexpectedEapType(typ) => write!(f, "unexpected EAP type 0x{typ:02X}"),
            Self::UnexpectedAkaSubtype(sub) => write!(f, "unexpected EAP-AKA subtype 0x{sub:02X}"),
            Self::TruncatedAttribute => write!(f, "truncated EAP-AKA attribute"),
            Self::InvalidAttributeLength(len) => {
                write!(f, "invalid EAP-AKA attribute length {len}")
            }
            Self::MissingAttribute(name) => write!(f, "{name} is missing"),
            Self::MissingRequiredField(name) => write!(f, "{name} is required"),
            Self::Base64Decode(e) => write!(f, "base64 decoding failed: {e}"),
        }
    }
}

impl Error for AkaError {}

/// 3G AUTHENTICATE request (RAND + AUTN) for the USIM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AkaRequest {
    pub rand: [u8; 16],
    pub autn: [u8; 16],
}

impl AkaRequest {
    #[must_use]
    pub fn new(rand: [u8; 16], autn: [u8; 16]) -> Self {
        Self { rand, autn }
    }

    /// Builds the data field for the AUTHENTICATE APDU:
    /// `len(RAND) || RAND || len(AUTN) || AUTN`.
    #[must_use]
    pub fn marshal(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + 16 + 1 + 16);
        out.push(16);
        out.extend_from_slice(&self.rand);
        out.push(16);
        out.extend_from_slice(&self.autn);
        out
    }
}

/// 3G AUTHENTICATE response parsed from the USIM security context.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AkaResponse {
    pub res: Option<Vec<u8>>,
    pub ck: Option<Vec<u8>>,
    pub ik: Option<Vec<u8>>,
    pub auts: Option<Vec<u8>>,
}

impl AkaResponse {
    #[must_use]
    pub fn is_sync_failure(&self) -> bool {
        self.auts.as_ref().is_some_and(|a| !a.is_empty())
    }

    /// Parses the raw security context bytes returned from USIM AUTHENTICATE.
    pub fn from_bytes(data: &[u8]) -> Result<Self, AkaError> {
        if data.is_empty() {
            return Err(AkaError::EmptySecurityContext);
        }
        match data[0] {
            SECURITY_CONTEXT_SUCCESS => {
                let mut off = 1;
                let (res, next) = read_len_value(data, off, "RES")?;
                off = next;
                let (ck, next) = read_len_value(data, off, "CK")?;
                off = next;
                let (ik, _) = read_len_value(data, off, "IK")?;
                Ok(Self {
                    res: Some(res),
                    ck: Some(ck),
                    ik: Some(ik),
                    auts: None,
                })
            }
            SECURITY_CONTEXT_SYNC_FAILURE => {
                let (auts, _) = read_len_value(data, 1, "AUTS")?;
                Ok(Self {
                    res: None,
                    ck: None,
                    ik: None,
                    auts: Some(auts),
                })
            }
            other => Err(AkaError::UnknownSecurityContext(other)),
        }
    }
}

fn read_len_value(
    data: &[u8],
    mut off: usize,
    field: &'static str,
) -> Result<(Vec<u8>, usize), AkaError> {
    if off >= data.len() {
        return Err(AkaError::MissingRequiredField(field));
    }
    let n = usize::from(data[off]);
    off += 1;
    if n > data.len() - off {
        return Err(AkaError::InvalidLength {
            expected: n,
            actual: data.len() - off,
            field,
        });
    }
    let value = data[off..off + n].to_vec();
    Ok((value, off + n))
}

/// A parsed EAP-AKA challenge packet (AT_RAND + AT_AUTN).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AkaChallenge {
    pub identifier: u8,
    pub rand: [u8; 16],
    pub autn: [u8; 16],
}

impl AkaChallenge {
    pub fn from_base64(text: &str) -> Result<Self, AkaError> {
        let bytes =
            base64_decode(text.trim()).map_err(|e| AkaError::Base64Decode(e.to_string()))?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, AkaError> {
        if data.len() < 8 {
            return Err(AkaError::ChallengeTooShort(data.len()));
        }
        if data[0] != EAP_CODE_REQUEST {
            return Err(AkaError::UnexpectedEapCode(data[0]));
        }
        let packet_len = usize::from(u16::from_be_bytes([data[2], data[3]]));
        if packet_len != data.len() {
            return Err(AkaError::PacketLengthMismatch {
                header: packet_len,
                actual: data.len(),
            });
        }
        if data[4] != EAP_TYPE_AKA {
            return Err(AkaError::UnexpectedEapType(data[4]));
        }
        if data[5] != AKA_SUBTYPE_CHALLENGE {
            return Err(AkaError::UnexpectedAkaSubtype(data[5]));
        }

        let mut rand = None;
        let mut autn = None;
        let mut off = 8;

        while off < data.len() {
            if data.len() - off < 2 {
                return Err(AkaError::TruncatedAttribute);
            }
            let typ = data[off];
            let attr_len = usize::from(data[off + 1]) * 4;
            if attr_len < 4 {
                return Err(AkaError::InvalidAttributeLength(attr_len));
            }
            if attr_len > data.len() - off {
                return Err(AkaError::TruncatedAttribute);
            }
            let val = &data[off + 4..off + attr_len];
            match typ {
                ATTRIBUTE_RAND => {
                    if val.len() != 16 {
                        return Err(AkaError::InvalidLength {
                            expected: 16,
                            actual: val.len(),
                            field: "AT_RAND",
                        });
                    }
                    let mut r = [0u8; 16];
                    r.copy_from_slice(val);
                    rand = Some(r);
                }
                ATTRIBUTE_AUTN => {
                    if val.len() != 16 {
                        return Err(AkaError::InvalidLength {
                            expected: 16,
                            actual: val.len(),
                            field: "AT_AUTN",
                        });
                    }
                    let mut a = [0u8; 16];
                    a.copy_from_slice(val);
                    autn = Some(a);
                }
                _ => {}
            }
            off += attr_len;
        }

        let rand = rand.ok_or(AkaError::MissingAttribute("AT_RAND"))?;
        let autn = autn.ok_or(AkaError::MissingAttribute("AT_AUTN"))?;

        Ok(Self {
            identifier: data[1],
            rand,
            autn,
        })
    }

    #[must_use]
    pub fn authentication_request(&self) -> AkaRequest {
        AkaRequest {
            rand: self.rand,
            autn: self.autn,
        }
    }
}

/// Builds the 3GPP EAP identity NAI: `0<IMSI>@<realm>.mncXXX.mccYYY.3gppnetwork.org`.
pub fn imsi_eap_identity(mcc_mnc: &str, imsi: &str, realm: &str) -> Result<String, AkaError> {
    if mcc_mnc.len() < 5 {
        return Err(AkaError::InvalidLength {
            expected: 5,
            actual: mcc_mnc.len(),
            field: "MCCMNC",
        });
    }
    if imsi.is_empty() {
        return Err(AkaError::MissingRequiredField("IMSI"));
    }
    if realm.is_empty() {
        return Err(AkaError::MissingRequiredField("realm"));
    }

    let mcc = &mcc_mnc[..3];
    let mnc = &mcc_mnc[3..];
    let padded_mnc = if mnc.len() == 2 {
        format!("0{mnc}")
    } else {
        mnc.to_string()
    };

    Ok(format!(
        "0{imsi}@{realm}.mnc{padded_mnc}.mcc{mcc}.3gppnetwork.org"
    ))
}

/// Builds the base64-encoded EAP-AKA response packet.
pub fn respond_to_challenge_base64(
    challenge: &AkaChallenge,
    response: &AkaResponse,
    identity: &str,
) -> Result<String, AkaError> {
    let packet = respond_to_challenge(challenge, response, identity)?;
    Ok(base64_encode(&packet))
}

/// Builds the EAP-AKA response packet for the challenge.
pub fn respond_to_challenge(
    challenge: &AkaChallenge,
    response: &AkaResponse,
    identity: &str,
) -> Result<Vec<u8>, AkaError> {
    if let Some(auts) = &response.auts {
        if !auts.is_empty() {
            if auts.len() != 14 {
                return Err(AkaError::InvalidLength {
                    expected: 14,
                    actual: auts.len(),
                    field: "AUTS",
                });
            }
            return Ok(sync_failure_response(challenge.identifier, auts));
        }
    }

    let res = response
        .res
        .as_ref()
        .filter(|r| !r.is_empty())
        .ok_or(AkaError::MissingRequiredField("RES"))?;
    let ck = response
        .ck
        .as_ref()
        .filter(|c| !c.is_empty())
        .ok_or(AkaError::MissingRequiredField("CK"))?;
    let ik = response
        .ik
        .as_ref()
        .filter(|i| !i.is_empty())
        .ok_or(AkaError::MissingRequiredField("IK"))?;

    let k_aut = derive_k_aut(identity, ik, ck)?;
    Ok(challenge_response(challenge.identifier, res, &k_aut))
}

fn padded_len(n: usize) -> usize {
    if n.is_multiple_of(4) {
        n
    } else {
        n + 4 - (n % 4)
    }
}

fn challenge_response(identifier: u8, res: &[u8], k_aut: &[u8; 16]) -> Vec<u8> {
    let res_pad = padded_len(res.len());
    let at_res_len = 4 + res_pad;
    let total_len = 8 + at_res_len + 20;

    let mut msg = vec![0u8; total_len];
    msg[0] = EAP_CODE_RESPONSE;
    msg[1] = identifier;
    msg[2] = ((total_len >> 8) & 0xFF) as u8;
    msg[3] = (total_len & 0xFF) as u8;
    msg[4] = EAP_TYPE_AKA;
    msg[5] = AKA_SUBTYPE_CHALLENGE;

    let mut off = 8;
    msg[off] = ATTRIBUTE_RES;
    msg[off + 1] = (at_res_len / 4) as u8;
    let res_bits = (res.len() * 8) as u16;
    msg[off + 2] = ((res_bits >> 8) & 0xFF) as u8;
    msg[off + 3] = (res_bits & 0xFF) as u8;
    msg[off + 4..off + 4 + res.len()].copy_from_slice(res);
    off += at_res_len;

    msg[off] = ATTRIBUTE_MAC;
    msg[off + 1] = 0x05;
    // Compute HMAC-SHA1 over the whole packet with zeroed MAC field
    let mac = hmac_sha1(k_aut, &msg);
    msg[off + 4..off + 20].copy_from_slice(&mac[..16]);

    msg
}

fn sync_failure_response(identifier: u8, auts: &[u8]) -> Vec<u8> {
    let total_len = 10 + auts.len();
    let mut msg = vec![0u8; total_len];
    msg[0] = EAP_CODE_RESPONSE;
    msg[1] = identifier;
    msg[2] = ((total_len >> 8) & 0xFF) as u8;
    msg[3] = (total_len & 0xFF) as u8;
    msg[4] = EAP_TYPE_AKA;
    msg[5] = AKA_SUBTYPE_SYNC_FAILURE;
    msg[8] = ATTRIBUTE_AUTS;
    msg[9] = 0x04;
    msg[10..10 + auts.len()].copy_from_slice(auts);
    msg
}

/// Derives the 16-byte K_aut MAC key using FIPS-186-2 PRF over SHA-1.
pub fn derive_k_aut(identity: &str, ik: &[u8], ck: &[u8]) -> Result<[u8; 16], AkaError> {
    if identity.is_empty() {
        return Err(AkaError::MissingRequiredField("identity"));
    }
    let mut seed_input = Vec::with_capacity(identity.len() + ik.len() + ck.len());
    seed_input.extend_from_slice(identity.as_bytes());
    seed_input.extend_from_slice(ik);
    seed_input.extend_from_slice(ck);

    let seed = sha1(&seed_input);
    let key = fips186_2_prf(&seed, 160);

    let mut k_aut = [0u8; 16];
    k_aut.copy_from_slice(&key[16..32]);
    Ok(k_aut)
}

fn fips186_2_prf(seed: &[u8; 20], length: usize) -> Vec<u8> {
    let mut xkey = *seed;
    let mut out = Vec::with_capacity(length);
    let state = [
        0x6745_2301u32,
        0xEFCD_AB89u32,
        0x98BA_DCFEu32,
        0x1032_5476u32,
        0xC3D2_E1F0u32,
    ];

    while out.len() < length {
        for _ in 0..2 {
            let w = sha1_g(&state, &xkey);
            out.extend_from_slice(&w);
            add_mod_160(&mut xkey, &w);
        }
    }

    out.truncate(length);
    out
}

fn add_mod_160(xkey: &mut [u8; 20], w: &[u8; 20]) {
    let mut carry: u32 = 1;
    for i in (0..20).rev() {
        carry += u32::from(xkey[i]) + u32::from(w[i]);
        xkey[i] = (carry & 0xFF) as u8;
        carry >>= 8;
    }
}

fn sha1_g(state: &[u32; 5], xkey: &[u8; 20]) -> [u8; 20] {
    let mut block = [0u8; 64];
    block[..20].copy_from_slice(xkey);
    let mut h = *state;
    compress_sha1_block(&mut h, &block);

    let mut out = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn compress_sha1_block(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    for (i, chunk) in block.as_chunks::<4>().0.iter().enumerate() {
        w[i] = u32::from_be_bytes(*chunk);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }

    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];

    for (i, &w_i) in w.iter().enumerate() {
        let (f, k) = match i {
            0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
            20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
            _ => (b ^ c ^ d, 0xCA62_C1D6),
        };
        let temp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(w_i);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = temp;
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}
