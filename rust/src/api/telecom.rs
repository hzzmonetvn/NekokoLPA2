//! Telecommunications, APDU, and SIM identifier helpers bridged to Flutter.

use flutter_rust_bridge::frb;
use nlpa_core::{
    bytes_to_hex as core_bytes_to_hex, calculate_imei_luhn_checksum as core_calculate_luhn,
    encode_stored_imei_bytes as core_encode_imei, format_iccid_grouped as core_format_iccid,
    hex_to_bytes as core_hex_to_bytes, iccid_for_display as core_iccid_for_display,
    imei_digits_from_stored_bytes as core_decode_imei, swap_nibbles as core_swap_nibbles,
    validate_apdu_sw as core_validate_apdu_sw, ActivationCode as CoreActivationCode,
    Asn1Reader as CoreAsn1Reader, EuiccProfile as CoreEuiccProfile,
};

#[frb]
pub struct ActivationCodeDto {
    pub format: String,
    pub smdp_address: String,
    pub matching_id: String,
    pub smdp_oid: Option<String>,
    pub confirmation_code: Option<String>,
    pub confirmation_code_required: bool,
}

#[frb]
pub struct EuiccProfileDto {
    pub iccid: String,
    pub raw_iccid: String,
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub service_provider_name: Option<String>,
    pub enabled: bool,
    pub profile_class: u8,
    pub isdp_aid: Option<String>,
    pub smdp_address: Option<String>,
}

#[frb]
pub struct Asn1NodeDto {
    pub tag: u32,
    pub value: Vec<u8>,
    pub raw_bytes: Vec<u8>,
}

#[frb]
pub fn parse_activation_code(code: String) -> Result<ActivationCodeDto, String> {
    let parsed = CoreActivationCode::parse(&code).map_err(|e| e.to_string())?;
    Ok(ActivationCodeDto {
        format: parsed.format,
        smdp_address: parsed.smdp_address,
        matching_id: parsed.matching_id,
        smdp_oid: parsed.smdp_oid,
        confirmation_code: parsed.confirmation_code,
        confirmation_code_required: parsed.confirmation_code_required,
    })
}

#[frb]
pub fn extract_activation_code(raw_text: String) -> Option<ActivationCodeDto> {
    CoreActivationCode::extract_from_text(&raw_text).map(|parsed| ActivationCodeDto {
        format: parsed.format,
        smdp_address: parsed.smdp_address,
        matching_id: parsed.matching_id,
        smdp_oid: parsed.smdp_oid,
        confirmation_code: parsed.confirmation_code,
        confirmation_code_required: parsed.confirmation_code_required,
    })
}

#[frb]
pub fn format_activation_code(dto: ActivationCodeDto) -> String {
    let ac = CoreActivationCode::new(
        dto.smdp_address,
        dto.matching_id,
        dto.smdp_oid,
        dto.confirmation_code,
    );
    ac.to_string()
}

#[frb]
pub fn hex_encode(bytes: Vec<u8>) -> String {
    core_bytes_to_hex(&bytes)
}

#[frb]
pub fn hex_decode(hex: String) -> Result<Vec<u8>, String> {
    core_hex_to_bytes(&hex).map_err(|e| e.to_string())
}

#[frb]
pub fn nibble_swap(input: String) -> String {
    core_swap_nibbles(&input)
}

#[frb]
pub fn format_iccid(iccid: String) -> String {
    core_format_iccid(&iccid)
}

#[frb]
pub fn clean_iccid(iccid: String) -> String {
    core_iccid_for_display(&iccid).to_string()
}

#[frb]
pub fn validate_apdu(sw1: u8, sw2: u8) -> Result<(), String> {
    core_validate_apdu_sw(sw1, sw2).map_err(|e| e.to_string())
}

#[frb]
pub fn calculate_imei_luhn(body_digits: String) -> Result<u8, String> {
    core_calculate_luhn(&body_digits).map_err(|e| e.to_string())
}

#[frb]
pub fn decode_stored_imei(bytes: Vec<u8>) -> Result<String, String> {
    core_decode_imei(&bytes).map_err(|e| e.to_string())
}

#[frb]
pub fn encode_stored_imei(digits: String) -> Result<Vec<u8>, String> {
    let arr = core_encode_imei(&digits).map_err(|e| e.to_string())?;
    Ok(arr.to_vec())
}

#[frb]
pub fn parse_asn1_tlvs(der_bytes: Vec<u8>) -> Result<Vec<Asn1NodeDto>, String> {
    let mut reader = CoreAsn1Reader::new(&der_bytes);
    let nodes = reader.read_all().map_err(|e| e.to_string())?;
    Ok(nodes
        .into_iter()
        .map(|n| Asn1NodeDto {
            tag: n.tag,
            value: n.value,
            raw_bytes: n.raw_bytes,
        })
        .collect())
}

#[frb]
pub fn create_euicc_profile(iccid: String, enabled: bool) -> EuiccProfileDto {
    let p = CoreEuiccProfile::new(iccid, enabled);
    EuiccProfileDto {
        iccid: p.iccid,
        raw_iccid: p.raw_iccid,
        name: p.name,
        nickname: p.nickname,
        service_provider_name: p.service_provider_name,
        enabled: p.enabled,
        profile_class: p.profile_class,
        isdp_aid: p.isdp_aid,
        smdp_address: p.smdp_address,
    }
}
