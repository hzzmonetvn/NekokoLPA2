//! GSMA SGP.22 RSP (Remote SIM Provisioning) ASN.1 data types and encoders/decoders.

use crate::asn1::der::{
    decode_integer, decode_utf8_string, encode_integer, encode_tlv, encode_utf8_string, Asn1Error,
    Asn1Node,
};
use crate::hex::{bytes_to_hex, hex_to_bytes, swap_nibbles};
use crate::iccid::iccid_for_display;

// =========================================================================
// GSMA SGP.22 ASN.1 Tag Constants
// =========================================================================

pub const TAG_GET_CONFIGURED_ADDRESSES: u32 = 0xBF3C;
pub const TAG_GET_EUICC_INFO1: u32 = 0xBF20;
pub const TAG_GET_EUICC_INFO2: u32 = 0xBF22;
pub const TAG_GET_EUICC_CHALLENGE: u32 = 0xBF2E;
pub const TAG_GET_PROFILES_INFO: u32 = 0xBF2D;
pub const TAG_PROFILE_INFO_LIST: u32 = 0xA0;
pub const TAG_PROFILE_INFO: u32 = 0xE3;
pub const TAG_AUTHENTICATE_SERVER: u32 = 0xBF38;
pub const TAG_PREPARE_DOWNLOAD: u32 = 0xBF21;
pub const TAG_LOAD_BOUND_PROFILE_PACKAGE: u32 = 0xBF36;
pub const TAG_INITIALISE_SECURE_CHANNEL: u32 = 0xBF23;
pub const TAG_CONFIGURE_ISDP: u32 = 0xBF24;
pub const TAG_STORE_METADATA: u32 = 0xBF25;
pub const TAG_SET_NICKNAME: u32 = 0xBF29;
pub const TAG_ENABLE_PROFILE: u32 = 0xBF31;
pub const TAG_DISABLE_PROFILE: u32 = 0xBF32;
pub const TAG_DELETE_PROFILE: u32 = 0xBF33;
pub const TAG_HANDLE_NOTIFICATION: u32 = 0xBF37;
pub const TAG_NOTIFICATION_METADATA: u32 = 0xBF2F;
pub const TAG_LIST_NOTIFICATION: u32 = 0xBF28;
pub const TAG_PROCESS_NOTIFICATION: u32 = 0xBF30;
pub const TAG_REMOVE_NOTIFICATION: u32 = 0xBF30;
pub const TAG_EUICC_MEMORY_RESET: u32 = 0xBF34;
pub const TAG_GET_RAT: u32 = 0xBF43;

// Field-specific tags
pub const TAG_ICCID: u32 = 0x5A;
pub const TAG_ISDP_AID: u32 = 0x4F;
pub const TAG_PROFILE_STATE: u32 = 0x9F70;
pub const TAG_NICKNAME: u32 = 0x90;
pub const TAG_SERVICE_PROVIDER_NAME: u32 = 0x91;
pub const TAG_PROFILE_NAME: u32 = 0x92;
pub const TAG_ICON_TYPE: u32 = 0x93;
pub const TAG_ICON: u32 = 0x94;
pub const TAG_PROFILE_CLASS: u32 = 0x95;
pub const TAG_NOTIFICATION_ADDRESS: u32 = 0x0C;
pub const TAG_SEQ_NUMBER: u32 = 0x80;
pub const TAG_PROFILE_MGMT_OP: u32 = 0x81;
pub const TAG_CHALLENGE: u32 = 0x82;
pub const TAG_DEFAULT_SMDP: u32 = 0x80;
pub const TAG_ROOT_SMDS: u32 = 0x81;
pub const TAG_SVN: u32 = 0x82;
pub const TAG_FW_VER: u32 = 0x83;
pub const TAG_EXT_CARD_RES: u32 = 0x84;
pub const TAG_FREE_NVRAM: u32 = 0x82;
pub const TAG_FREE_RAM: u32 = 0x83;
pub const TAG_REFRESH_FLAG: u32 = 0x81;
pub const TAG_RESET_OPTIONS: u32 = 0x82;

// =========================================================================
// Helpers
// =========================================================================

/// Encodes an ICCID into ITU-T E.118 / SGP.22 swapped-nibble BCD bytes.
///
/// If length is odd (e.g. 19 digits), standard telecom padding `'F'` is appended
/// before swapping each octet's nibbles.
#[must_use]
pub fn encode_iccid_bytes(iccid: &str) -> Vec<u8> {
    let mut s: String = iccid.chars().filter(|c| !c.is_whitespace()).collect();
    if !s.len().is_multiple_of(2) {
        s.push('F');
    }
    let swapped = swap_nibbles(&s);
    hex_to_bytes(&swapped).unwrap_or_default()
}

fn decode_iccid_bytes(bytes: &[u8]) -> String {
    let hex = bytes_to_hex(bytes);
    iccid_for_display(&swap_nibbles(&hex)).to_string()
}

fn decode_profile_id_iccid(node: &Asn1Node) -> Result<String, Asn1Error> {
    if let Some(choice) = node.find_child(0xA0) {
        if let Some(iccid_child) = choice.find_child(TAG_ICCID) {
            return Ok(decode_iccid_bytes(&iccid_child.value));
        }
    }
    let iccid_child = node.require_child(TAG_ICCID, "iccid")?;
    Ok(decode_iccid_bytes(&iccid_child.value))
}

fn encode_profile_refresh_request(outer_tag: u32, iccid: &str, refresh_flag: bool) -> Vec<u8> {
    let iccid_bytes = encode_iccid_bytes(iccid);
    let profile_id = encode_tlv(0xA0, &encode_tlv(TAG_ICCID, &iccid_bytes));
    let mut refresh_byte = 0x00;
    if refresh_flag {
        refresh_byte = 0xFF;
    }
    let refresh_tlv = encode_tlv(TAG_REFRESH_FLAG, &[refresh_byte]);

    let mut payload = Vec::with_capacity(profile_id.len() + refresh_tlv.len());
    payload.extend_from_slice(&profile_id);
    payload.extend_from_slice(&refresh_tlv);
    encode_tlv(outer_tag, &payload)
}

pub(crate) fn encode_implicit_integer(tag: u32, val: i64) -> Vec<u8> {
    if val == 0 {
        return encode_tlv(tag, &[0x00]);
    }
    let bytes = val.to_be_bytes();
    let mut start = 0usize;
    while start + 1 < bytes.len() {
        let cur = bytes[start];
        let next = bytes[start + 1];
        if (cur == 0x00 && (next & 0x80) == 0) || (cur == 0xFF && (next & 0x80) != 0) {
            start += 1;
        } else {
            break;
        }
    }
    encode_tlv(tag, &bytes[start..])
}

pub(crate) fn encode_mgmt_op_bit_string(op: u8) -> Vec<u8> {
    if op == 0 {
        return vec![0x00];
    }
    let msb = op.reverse_bits();
    vec![msb.trailing_zeros() as u8, msb]
}

fn decode_mgmt_op_bit_string(val: &[u8]) -> Result<u8, Asn1Error> {
    if val.len() >= 2 && val[0] <= 7 {
        return Ok(val[1].reverse_bits());
    }
    val.first().copied().ok_or(Asn1Error::UnexpectedEof {
        offset: 0,
        needed: 1,
    })
}

// =========================================================================
// Data Structures
// =========================================================================

/// eUICC Configured Addresses Response (GSMA SGP.22 ES10b.GetEuiccConfiguredAddresses).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EuiccConfiguredAddresses {
    pub default_smdp_address: Option<String>,
    pub root_smds_address: Option<String>,
}

impl EuiccConfiguredAddresses {
    /// Encodes to ASN.1 DER Tag 0xBF3C.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut fields = Vec::new();
        if let Some(ref smdp) = self.default_smdp_address {
            fields.push(encode_tlv(TAG_DEFAULT_SMDP, smdp.as_bytes()));
        }
        if let Some(ref smds) = self.root_smds_address {
            fields.push(encode_tlv(TAG_ROOT_SMDS, smds.as_bytes()));
        }
        let total_len: usize = fields.iter().map(Vec::len).sum();
        let mut payload = Vec::with_capacity(total_len);
        for f in fields {
            payload.extend_from_slice(&f);
        }
        encode_tlv(TAG_GET_CONFIGURED_ADDRESSES, &payload)
    }

    /// Decodes from an ASN.1 node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] if tag does not match or payload is malformed.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_GET_CONFIGURED_ADDRESSES) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_GET_CONFIGURED_ADDRESSES,
                actual: node.tag,
            });
        }
        let mut default_smdp = None;
        let mut root_smds = None;

        for child in node.children()? {
            match child.tag {
                TAG_DEFAULT_SMDP => {
                    default_smdp = Some(decode_utf8_string(&child.value)?);
                }
                TAG_ROOT_SMDS => {
                    root_smds = Some(decode_utf8_string(&child.value)?);
                }
                _ => {}
            }
        }

        Ok(Self {
            default_smdp_address: default_smdp,
            root_smds_address: root_smds,
        })
    }
}

/// eUICC Info1 (GSMA SGP.22 ES10b.GetEuiccInfo1).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EuiccInfo1 {
    pub svn: Vec<u8>,
    pub euicc_firmware_ver: Vec<u8>,
    pub ext_card_res: Vec<u8>,
}

impl EuiccInfo1 {
    /// Encodes to ASN.1 DER Tag 0xBF20.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut fields = Vec::new();
        if !self.svn.is_empty() {
            fields.push(encode_tlv(TAG_SVN, &self.svn));
        }
        if !self.euicc_firmware_ver.is_empty() {
            fields.push(encode_tlv(TAG_FW_VER, &self.euicc_firmware_ver));
        }
        if !self.ext_card_res.is_empty() {
            fields.push(encode_tlv(TAG_EXT_CARD_RES, &self.ext_card_res));
        }
        let total_len: usize = fields.iter().map(Vec::len).sum();
        let mut payload = Vec::with_capacity(total_len);
        for f in fields {
            payload.extend_from_slice(&f);
        }
        encode_tlv(TAG_GET_EUICC_INFO1, &payload)
    }

    /// Decodes from an ASN.1 node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] if tag does not match or payload is malformed.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_GET_EUICC_INFO1) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_GET_EUICC_INFO1,
                actual: node.tag,
            });
        }
        let mut svn = Vec::new();
        let mut fw = Vec::new();
        let mut ext = Vec::new();

        for child in node.children()? {
            match child.tag {
                TAG_SVN => svn = child.value,
                TAG_FW_VER => fw = child.value,
                TAG_EXT_CARD_RES => ext = child.value,
                _ => {}
            }
        }

        Ok(Self {
            svn,
            euicc_firmware_ver: fw,
            ext_card_res: ext,
        })
    }
}

/// eUICC Info2 (GSMA SGP.22 ES10b.GetEuiccInfo2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EuiccInfo2 {
    pub profile_version: Vec<u8>,
    pub svn: Vec<u8>,
    pub euicc_firmware_ver: Vec<u8>,
    pub ext_card_res: Vec<u8>,
    pub free_nvram: u32,
    pub free_ram: u32,
}

impl EuiccInfo2 {
    /// Encodes to ASN.1 DER Tag 0xBF22.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut fields = Vec::new();
        if !self.profile_version.is_empty() {
            fields.push(encode_tlv(0x81, &self.profile_version));
        }
        if !self.svn.is_empty() {
            fields.push(encode_tlv(TAG_SVN, &self.svn));
        }
        if !self.euicc_firmware_ver.is_empty() {
            fields.push(encode_tlv(TAG_FW_VER, &self.euicc_firmware_ver));
        }
        if !self.ext_card_res.is_empty() {
            fields.push(encode_tlv(TAG_EXT_CARD_RES, &self.ext_card_res));
        } else if self.free_nvram > 0 || self.free_ram > 0 {
            let mut ext_fields = Vec::new();
            if self.free_nvram > 0 {
                ext_fields.push(encode_tlv(TAG_FREE_NVRAM, &self.free_nvram.to_be_bytes()));
            }
            if self.free_ram > 0 {
                ext_fields.push(encode_tlv(TAG_FREE_RAM, &self.free_ram.to_be_bytes()));
            }
            let ext_len: usize = ext_fields.iter().map(Vec::len).sum();
            let mut ext_payload = Vec::with_capacity(ext_len);
            for f in ext_fields {
                ext_payload.extend_from_slice(&f);
            }
            fields.push(encode_tlv(TAG_EXT_CARD_RES, &ext_payload));
        }

        let total_len: usize = fields.iter().map(Vec::len).sum();
        let mut payload = Vec::with_capacity(total_len);
        for f in fields {
            payload.extend_from_slice(&f);
        }
        encode_tlv(TAG_GET_EUICC_INFO2, &payload)
    }

    /// Decodes from an ASN.1 node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] if tag does not match or payload is malformed.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_GET_EUICC_INFO2) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_GET_EUICC_INFO2,
                actual: node.tag,
            });
        }
        let mut profile_version = Vec::new();
        let mut svn = Vec::new();
        let mut fw = Vec::new();
        let mut ext = Vec::new();
        let mut free_nvram = 0u32;
        let mut free_ram = 0u32;

        for child in node.children()? {
            match child.tag {
                0x81 => profile_version = child.value,
                TAG_SVN => svn = child.value,
                TAG_FW_VER => fw = child.value,
                TAG_EXT_CARD_RES => {
                    ext = child.value.clone();
                    if let Ok(ext_children) = child.children() {
                        for ec in ext_children {
                            if ec.tag == TAG_FREE_NVRAM {
                                if let Ok(v) = decode_integer(&ec.value) {
                                    free_nvram = v as u32;
                                }
                            } else if ec.tag == TAG_FREE_RAM {
                                if let Ok(v) = decode_integer(&ec.value) {
                                    free_ram = v as u32;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            profile_version,
            svn,
            euicc_firmware_ver: fw,
            ext_card_res: ext,
            free_nvram,
            free_ram,
        })
    }
}

/// eUICC Challenge (GSMA SGP.22 ES10b.GetEuiccChallenge).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EuiccChallenge {
    pub challenge: [u8; 16],
}

impl EuiccChallenge {
    #[must_use]
    pub fn new(challenge: [u8; 16]) -> Self {
        Self { challenge }
    }

    /// Encodes to ASN.1 DER Tag 0xBF2E.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let inner = encode_tlv(TAG_CHALLENGE, &self.challenge);
        encode_tlv(TAG_GET_EUICC_CHALLENGE, &inner)
    }

    /// Decodes from an ASN.1 node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] on invalid challenge format or length.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_GET_EUICC_CHALLENGE) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_GET_EUICC_CHALLENGE,
                actual: node.tag,
            });
        }
        let child = node.require_child(TAG_CHALLENGE, "euiccChallenge")?;
        if child.value.len() != 16 {
            return Err(Asn1Error::UnexpectedEof {
                offset: 0,
                needed: 16,
            });
        }
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&child.value);
        Ok(Self { challenge: arr })
    }
}

/// Detailed profile metadata parsed from eUICC response (Tag 0xE3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RspProfileInfo {
    pub iccid: String,
    pub isdp_aid: Option<Vec<u8>>,
    pub profile_state: u8,
    pub nickname: Option<String>,
    pub service_provider_name: Option<String>,
    pub profile_name: Option<String>,
    pub profile_class: u8,
}

impl RspProfileInfo {
    /// Encodes to ASN.1 DER Tag 0xE3.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut fields = Vec::new();

        // 1. ICCID (BCD encoded octet string)
        let iccid_bytes = encode_iccid_bytes(&self.iccid);
        if !iccid_bytes.is_empty() {
            fields.push(encode_tlv(TAG_ICCID, &iccid_bytes));
        }

        // 2. ISDP AID
        if let Some(ref aid) = self.isdp_aid {
            fields.push(encode_tlv(TAG_ISDP_AID, aid));
        }

        // 3. Profile state
        fields.push(encode_tlv(TAG_PROFILE_STATE, &[self.profile_state]));

        // 4. Nickname
        if let Some(ref nick) = self.nickname {
            fields.push(encode_tlv(TAG_NICKNAME, nick.as_bytes()));
        }

        // 5. Service Provider Name
        if let Some(ref spn) = self.service_provider_name {
            fields.push(encode_tlv(TAG_SERVICE_PROVIDER_NAME, spn.as_bytes()));
        }

        // 6. Profile Name
        if let Some(ref name) = self.profile_name {
            fields.push(encode_tlv(TAG_PROFILE_NAME, name.as_bytes()));
        }

        // 7. Profile Class
        fields.push(encode_tlv(TAG_PROFILE_CLASS, &[self.profile_class]));

        let total_len: usize = fields.iter().map(Vec::len).sum();
        let mut payload = Vec::with_capacity(total_len);
        for f in fields {
            payload.extend_from_slice(&f);
        }
        encode_tlv(TAG_PROFILE_INFO, &payload)
    }

    /// Decodes a profile info node from Tag 0xE3.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] if tag does not match or child decoding fails.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_PROFILE_INFO) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_PROFILE_INFO,
                actual: node.tag,
            });
        }

        let mut iccid = String::new();
        let mut isdp_aid = None;
        let mut profile_state = 0u8;
        let mut nickname = None;
        let mut service_provider_name = None;
        let mut profile_name = None;
        let mut profile_class = 2u8; // default operational

        for child in node.children()? {
            match child.tag {
                TAG_ICCID => {
                    iccid = decode_iccid_bytes(&child.value);
                }
                TAG_ISDP_AID => {
                    isdp_aid = Some(child.value);
                }
                TAG_PROFILE_STATE => {
                    if let Some(&b) = child.value.first() {
                        profile_state = b;
                    }
                }
                TAG_NICKNAME => {
                    nickname = Some(decode_utf8_string(&child.value)?);
                }
                TAG_SERVICE_PROVIDER_NAME => {
                    service_provider_name = Some(decode_utf8_string(&child.value)?);
                }
                TAG_PROFILE_NAME => {
                    profile_name = Some(decode_utf8_string(&child.value)?);
                }
                TAG_PROFILE_CLASS => {
                    if let Some(&b) = child.value.first() {
                        profile_class = b;
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            iccid,
            isdp_aid,
            profile_state,
            nickname,
            service_provider_name,
            profile_name,
            profile_class,
        })
    }
}

/// SGP.22 ES10c.SetNickname Request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetNicknameRequest {
    pub iccid: String,
    pub nickname: String,
}

impl SetNicknameRequest {
    #[must_use]
    pub fn new(iccid: String, nickname: String) -> Self {
        Self { iccid, nickname }
    }

    /// Encodes to ASN.1 DER Tag 0xBF29.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let iccid_bytes = encode_iccid_bytes(&self.iccid);
        let fields = vec![
            encode_tlv(TAG_ICCID, &iccid_bytes),
            encode_tlv(TAG_NICKNAME, self.nickname.as_bytes()),
        ];

        let total_len: usize = fields.iter().map(Vec::len).sum();
        let mut payload = Vec::with_capacity(total_len);
        for f in fields {
            payload.extend_from_slice(&f);
        }
        encode_tlv(TAG_SET_NICKNAME, &payload)
    }

    /// Decodes from an ASN.1 node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] if tag does not match or fields are missing.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_SET_NICKNAME) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_SET_NICKNAME,
                actual: node.tag,
            });
        }
        let iccid_child = node.require_child(TAG_ICCID, "iccid")?;
        let nick_child = node.require_child(TAG_NICKNAME, "nickname")?;

        let iccid = decode_iccid_bytes(&iccid_child.value);
        let nickname = decode_utf8_string(&nick_child.value)?;

        Ok(Self { iccid, nickname })
    }
}

/// SGP.22 ES10c.EnableProfile Request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnableProfileRequest {
    pub iccid: String,
    pub refresh_flag: bool,
}

impl EnableProfileRequest {
    #[must_use]
    pub fn new(iccid: String, refresh_flag: bool) -> Self {
        Self {
            iccid,
            refresh_flag,
        }
    }

    /// Encodes to ASN.1 DER Tag 0xBF31.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        encode_profile_refresh_request(TAG_ENABLE_PROFILE, &self.iccid, self.refresh_flag)
    }

    /// Decodes from an ASN.1 node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] on tag mismatch or missing mandatory ICCID.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_ENABLE_PROFILE) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_ENABLE_PROFILE,
                actual: node.tag,
            });
        }
        let iccid = decode_profile_id_iccid(node)?;
        let refresh_flag = node
            .find_child(TAG_REFRESH_FLAG)
            .and_then(|c| c.value.first().copied())
            != Some(0);

        Ok(Self {
            iccid,
            refresh_flag,
        })
    }
}

/// SGP.22 ES10c.DisableProfile Request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisableProfileRequest {
    pub iccid: String,
    pub refresh_flag: bool,
}

impl DisableProfileRequest {
    #[must_use]
    pub fn new(iccid: String, refresh_flag: bool) -> Self {
        Self {
            iccid,
            refresh_flag,
        }
    }

    /// Encodes to ASN.1 DER Tag 0xBF32.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        encode_profile_refresh_request(TAG_DISABLE_PROFILE, &self.iccid, self.refresh_flag)
    }

    /// Decodes from an ASN.1 node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] on tag mismatch or missing mandatory ICCID.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_DISABLE_PROFILE) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_DISABLE_PROFILE,
                actual: node.tag,
            });
        }
        let iccid = decode_profile_id_iccid(node)?;
        let refresh_flag = node
            .find_child(TAG_REFRESH_FLAG)
            .and_then(|c| c.value.first().copied())
            != Some(0);

        Ok(Self {
            iccid,
            refresh_flag,
        })
    }
}

/// SGP.22 ES10c.DeleteProfile Request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteProfileRequest {
    pub iccid: String,
}

impl DeleteProfileRequest {
    #[must_use]
    pub fn new(iccid: String) -> Self {
        Self { iccid }
    }

    /// Encodes to ASN.1 DER Tag 0xBF33.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let iccid_bytes = encode_iccid_bytes(&self.iccid);
        let inner = encode_tlv(TAG_ICCID, &iccid_bytes);
        encode_tlv(TAG_DELETE_PROFILE, &inner)
    }

    /// Decodes from an ASN.1 node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] on tag mismatch or missing ICCID.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_DELETE_PROFILE) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_DELETE_PROFILE,
                actual: node.tag,
            });
        }
        let iccid_child = node.require_child(TAG_ICCID, "iccid")?;
        let iccid = decode_iccid_bytes(&iccid_child.value);

        Ok(Self { iccid })
    }
}

/// SGP.22 ES10b.NotificationMetadata (Tag 0xBF2F).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NotificationMetadata {
    pub seq_number: u32,
    pub profile_management_operation: u8,
    pub notification_address: String,
    pub iccid: Option<String>,
}

impl NotificationMetadata {
    /// Encodes to ASN.1 DER Tag 0xBF2F.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut fields = Vec::new();
        fields.push(encode_implicit_integer(
            TAG_SEQ_NUMBER,
            i64::from(self.seq_number),
        ));
        let op_bytes = encode_mgmt_op_bit_string(self.profile_management_operation);
        fields.push(encode_tlv(TAG_PROFILE_MGMT_OP, &op_bytes));
        fields.push(encode_utf8_string(&self.notification_address));
        if let Some(ref iccid) = self.iccid {
            let bytes = encode_iccid_bytes(iccid);
            if !bytes.is_empty() {
                fields.push(encode_tlv(TAG_ICCID, &bytes));
            }
        }

        let total_len: usize = fields.iter().map(Vec::len).sum();
        let mut payload = Vec::with_capacity(total_len);
        for f in fields {
            payload.extend_from_slice(&f);
        }
        encode_tlv(TAG_NOTIFICATION_METADATA, &payload)
    }

    /// Decodes from an ASN.1 node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] on tag mismatch or missing required notification fields.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_NOTIFICATION_METADATA) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_NOTIFICATION_METADATA,
                actual: node.tag,
            });
        }

        let seq_child = match node.find_child(TAG_SEQ_NUMBER) {
            Some(c) => c,
            None => node.require_child(0x02, "seqNumber")?,
        };
        let raw_seq = decode_integer(&seq_child.value)?;
        let seq_number = u32::try_from(raw_seq).map_err(|_| Asn1Error::InvalidInteger)?;

        let op_child = node.require_child(TAG_PROFILE_MGMT_OP, "profileManagementOperation")?;
        let profile_management_operation = decode_mgmt_op_bit_string(&op_child.value)?;

        let addr_child = node.require_child(0x0C, "notificationAddress")?;
        let notification_address = decode_utf8_string(&addr_child.value)?;

        let mut iccid = None;
        if let Some(child) = node.find_child(TAG_ICCID) {
            iccid = Some(decode_iccid_bytes(&child.value));
        }

        Ok(Self {
            seq_number,
            profile_management_operation,
            notification_address,
            iccid,
        })
    }
}

/// SGP.22 ES10c.EuiccMemoryReset Request (Tag 0xBF34).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EuiccMemoryResetRequest {
    pub reset_options: u8,
}

impl EuiccMemoryResetRequest {
    #[must_use]
    pub fn new(reset_options: u8) -> Self {
        Self { reset_options }
    }

    /// Encodes to ASN.1 DER Tag 0xBF34.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let inner = encode_tlv(TAG_RESET_OPTIONS, &[self.reset_options]);
        encode_tlv(TAG_EUICC_MEMORY_RESET, &inner)
    }

    /// Decodes from an ASN.1 node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] on tag mismatch or missing reset options.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_EUICC_MEMORY_RESET) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_EUICC_MEMORY_RESET,
                actual: node.tag,
            });
        }
        let child = node.require_child(TAG_RESET_OPTIONS, "resetOptions")?;
        let opt = *child.value.first().ok_or(Asn1Error::UnexpectedEof {
            offset: 0,
            needed: 1,
        })?;
        Ok(Self { reset_options: opt })
    }
}

/// Status code returned by eUICC after an ES10b.RemoveNotificationFromList operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteNotificationStatus {
    Ok,
    NothingToDelete,
    UndefinedError,
    Other(i64),
}

impl From<i64> for DeleteNotificationStatus {
    fn from(code: i64) -> Self {
        match code {
            0 => Self::Ok,
            1 => Self::NothingToDelete,
            127 => Self::UndefinedError,
            other => Self::Other(other),
        }
    }
}

impl DeleteNotificationStatus {
    #[must_use]
    pub const fn to_code(self) -> i64 {
        match self {
            Self::Ok => 0,
            Self::NothingToDelete => 1,
            Self::UndefinedError => 127,
            Self::Other(c) => c,
        }
    }

    #[must_use]
    pub const fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }
}

/// SGP.22 ES10b.NotificationSentResponse (Tag 0xBF30).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationSentResponse {
    pub delete_notification_status: DeleteNotificationStatus,
}

impl NotificationSentResponse {
    #[must_use]
    pub fn new(status: DeleteNotificationStatus) -> Self {
        Self {
            delete_notification_status: status,
        }
    }

    /// Encodes to ASN.1 DER Tag 0xBF30.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let status_bytes = encode_integer(self.delete_notification_status.to_code());
        encode_tlv(TAG_REMOVE_NOTIFICATION, &status_bytes)
    }

    /// Decodes from an ASN.1 node.
    ///
    /// # Errors
    /// Returns [`Asn1Error`] on tag mismatch or missing deleteNotificationStatus integer.
    pub fn decode(node: &Asn1Node) -> Result<Self, Asn1Error> {
        if !node.is_tag(TAG_REMOVE_NOTIFICATION) {
            return Err(Asn1Error::UnexpectedTag {
                expected: TAG_REMOVE_NOTIFICATION,
                actual: node.tag,
            });
        }

        let status_code = if let Some(child) = node.find_child(0x02) {
            decode_integer(&child.value)?
        } else if !node.value.is_empty() {
            decode_integer(&node.value)?
        } else {
            return Err(Asn1Error::MissingField {
                tag: 0x02,
                name: "deleteNotificationStatus",
            });
        };

        Ok(Self {
            delete_notification_status: DeleteNotificationStatus::from(status_code),
        })
    }
}
