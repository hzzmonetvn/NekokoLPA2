//! GSMA SGP.22 ES10 (eUICC <-> LPA) APDU command builders and response parsers.

use crate::apdu::{validate_apdu_sw, ApduError};
use crate::asn1::encode_tlv;
use crate::asn1::rsp::{
    encode_implicit_integer, encode_mgmt_op_bit_string, DeleteProfileRequest,
    DisableProfileRequest, EnableProfileRequest, EuiccMemoryResetRequest, SetNicknameRequest,
    TAG_GET_CONFIGURED_ADDRESSES, TAG_GET_EUICC_CHALLENGE, TAG_GET_EUICC_INFO1,
    TAG_GET_EUICC_INFO2, TAG_GET_PROFILES_INFO, TAG_LIST_NOTIFICATION, TAG_PROFILE_MGMT_OP,
    TAG_REMOVE_NOTIFICATION, TAG_SEQ_NUMBER,
};

/// Standard ISD-R (Issuer Security Domain Root) Application Identifier.
pub const ISDR_AID: [u8; 16] = [
    0xA0, 0x00, 0x00, 0x05, 0x59, 0x10, 0x10, 0xFF, 0xFF, 0xFF, 0xFF, 0x89, 0x00, 0x00, 0x01, 0x00,
];

/// Encapsulates a command payload into a GSMA SGP.22 §5.7.2 `STORE DATA` APDU:
/// `CLA=0x80`, `INS=0xE2` (`STORE DATA`), `P1=0x91` (last block) or `0x11` (more blocks),
/// `P2=0x00` (block number), `Lc`, and `payload` (no trailing `Le` byte).
#[must_use]
pub fn wrap_store_data_apdu(payload: &[u8], _is_first: bool, is_last: bool) -> Vec<u8> {
    let p1 = if is_last { 0x91 } else { 0x11 };
    let mut apdu = Vec::with_capacity(5 + payload.len());
    apdu.push(0x80); // CLA (GlobalPlatform / SGP.22)
    apdu.push(0xE2); // INS (STORE DATA)
    apdu.push(p1); // P1 (SGP.22: 0x91 last block, 0x11 more blocks)
    apdu.push(0x00); // P2 (Block number)
    apdu.push(payload.len() as u8); // Lc
    apdu.extend_from_slice(payload);
    apdu
}

/// Builds an ISO 7816-4 SELECT APDU targeting the ISD-R application.
#[must_use]
pub fn build_select_isdr_apdu() -> Vec<u8> {
    let mut apdu = Vec::with_capacity(5 + ISDR_AID.len() + 1);
    apdu.push(0x00); // CLA
    apdu.push(0xA4); // INS (SELECT)
    apdu.push(0x04); // P1 (Select by DF name / AID)
    apdu.push(0x00); // P2 (First or only occurrence)
    apdu.push(ISDR_AID.len() as u8); // Lc
    apdu.extend_from_slice(&ISDR_AID);
    apdu.push(0x00); // Le
    apdu
}

/// Builds an ES10b.GetEuiccConfiguredAddresses APDU.
#[must_use]
pub fn build_get_euicc_configured_addresses_apdu() -> Vec<u8> {
    let tlv = encode_tlv(TAG_GET_CONFIGURED_ADDRESSES, &[]);
    wrap_store_data_apdu(&tlv, true, true)
}

/// Builds an ES10b.GetEuiccInfo1 APDU.
#[must_use]
pub fn build_get_euicc_info1_apdu() -> Vec<u8> {
    let tlv = encode_tlv(TAG_GET_EUICC_INFO1, &[]);
    wrap_store_data_apdu(&tlv, true, true)
}

/// Builds an ES10b.GetEuiccInfo2 APDU.
#[must_use]
pub fn build_get_euicc_info2_apdu() -> Vec<u8> {
    let tlv = encode_tlv(TAG_GET_EUICC_INFO2, &[]);
    wrap_store_data_apdu(&tlv, true, true)
}

/// Builds an ES10b.GetEuiccChallenge APDU.
#[must_use]
pub fn build_get_euicc_challenge_apdu() -> Vec<u8> {
    let tlv = encode_tlv(TAG_GET_EUICC_CHALLENGE, &[]);
    wrap_store_data_apdu(&tlv, true, true)
}

/// Builds an ES10c.GetProfilesInfo APDU.
#[must_use]
pub fn build_get_profiles_info_apdu() -> Vec<u8> {
    let tlv = encode_tlv(TAG_GET_PROFILES_INFO, &[]);
    wrap_store_data_apdu(&tlv, true, true)
}

/// Builds an ES10c.EnableProfile APDU.
#[must_use]
pub fn build_enable_profile_apdu(iccid: &str, refresh: bool) -> Vec<u8> {
    let req = EnableProfileRequest::new(iccid.to_string(), refresh);
    let payload = req.encode();
    wrap_store_data_apdu(&payload, true, true)
}

/// Builds an ES10c.DisableProfile APDU.
#[must_use]
pub fn build_disable_profile_apdu(iccid: &str, refresh: bool) -> Vec<u8> {
    let req = DisableProfileRequest::new(iccid.to_string(), refresh);
    let payload = req.encode();
    wrap_store_data_apdu(&payload, true, true)
}

/// Builds an ES10c.DeleteProfile APDU.
#[must_use]
pub fn build_delete_profile_apdu(iccid: &str) -> Vec<u8> {
    let req = DeleteProfileRequest::new(iccid.to_string());
    let payload = req.encode();
    wrap_store_data_apdu(&payload, true, true)
}

/// Builds an ES10c.SetNickname APDU.
#[must_use]
pub fn build_set_nickname_apdu(iccid: &str, nickname: &str) -> Vec<u8> {
    let req = SetNicknameRequest::new(iccid.to_string(), nickname.to_string());
    let payload = req.encode();
    wrap_store_data_apdu(&payload, true, true)
}

/// Builds an ES10b.ListNotification APDU (`0xBF28`), optionally filtering by
/// `profileManagementOperation` BIT STRING (`0x81`).
#[must_use]
pub fn build_list_notifications_apdu(seq: Option<u32>) -> Vec<u8> {
    let payload = match seq {
        Some(op) => {
            let bits = encode_mgmt_op_bit_string(op.to_le_bytes()[0]);
            encode_tlv(TAG_PROFILE_MGMT_OP, &bits)
        }
        None => Vec::new(),
    };
    let tlv = encode_tlv(TAG_LIST_NOTIFICATION, &payload);
    wrap_store_data_apdu(&tlv, true, true)
}

/// Builds an ES10b.RemoveNotificationFromList APDU (`NotificationSentRequest`, `0xBF30`).
#[must_use]
pub fn build_remove_notification_apdu(seq_number: u32) -> Vec<u8> {
    let payload = encode_implicit_integer(TAG_SEQ_NUMBER, i64::from(seq_number));
    let tlv = encode_tlv(TAG_REMOVE_NOTIFICATION, &payload);
    wrap_store_data_apdu(&tlv, true, true)
}

/// Builds an ES10c.EuiccMemoryReset APDU.
#[must_use]
pub fn build_euicc_memory_reset_apdu(options: u8) -> Vec<u8> {
    let req = EuiccMemoryResetRequest::new(options);
    let payload = req.encode();
    wrap_store_data_apdu(&payload, true, true)
}

/// Parses an APDU response buffer, validating status bytes SW1/SW2 and extracting payload.
///
/// # Errors
/// Returns [`ApduError`] if the status word indicates failure or data is truncated.
pub fn parse_es10_response(response: &[u8]) -> Result<Vec<u8>, ApduError> {
    if response.len() < 2 {
        return Err(ApduError::Failure {
            sw1: 0x6F,
            sw2: 0x00,
        });
    }
    let sw1 = response[response.len() - 2];
    let sw2 = response[response.len() - 1];

    validate_apdu_sw(sw1, sw2)?;

    let payload = &response[..response.len() - 2];
    Ok(payload.to_vec())
}
