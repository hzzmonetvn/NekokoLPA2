//! SIM file parsing operations bridged to Flutter.

use flutter_rust_bridge::frb;
use nlpa_core::asn1::fcp::{parse_application_record, parse_fcp};
use nlpa_core::lpa::ts43::sim::{decode_swapped_bcd, Imsi};

#[frb]
pub struct SimFileInfoDto {
    pub file_structure: u8,
    pub record_size: u32,
    pub record_count: u32,
    pub file_size: u32,
}

#[frb]
pub struct SimApplicationDto {
    pub aid: Vec<u8>,
    pub label: String,
}

#[frb]
pub fn decode_sim_iccid(data: Vec<u8>) -> String {
    decode_swapped_bcd(&data, true)
}

#[frb]
pub fn decode_sim_imsi(data: Vec<u8>) -> Result<String, String> {
    Imsi::from_bytes(&data)
        .map(|imsi| imsi.to_string())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_iccid_like_dart_swapped_bcd_path() {
        assert_eq!(decode_sim_iccid(vec![0x98, 0x10, 0xF0]), "89010");
        assert_eq!(decode_sim_iccid(vec![0x21, 0xF3]), "123");
        assert_eq!(decode_sim_iccid(vec![0xF1, 0x23]), "1");
        assert_eq!(decode_sim_iccid(vec![0xF9]), "");
        assert_eq!(decode_sim_iccid(Vec::new()), "");
    }

    #[test]
    fn decodes_length_prefixed_imsi() {
        assert_eq!(
            decode_sim_imsi(vec![0x08, 0x29, 0x43, 0x01, 0x87, 0x31, 0x42, 0x70, 0x97]),
            Ok("234107813240779".to_string())
        );
        assert_eq!(
            decode_sim_imsi(Vec::new()),
            Err("reading EF_IMSI: empty payload".to_string())
        );
    }
}

#[frb]
pub fn parse_sim_fcp(data: Vec<u8>) -> Result<SimFileInfoDto, String> {
    let info = parse_fcp(&data).map_err(|error| error.to_string())?;
    Ok(SimFileInfoDto {
        file_structure: info.file_structure,
        record_size: u32::try_from(info.record_size).map_err(|error| error.to_string())?,
        record_count: u32::try_from(info.record_count).map_err(|error| error.to_string())?,
        file_size: u32::try_from(info.file_size).map_err(|error| error.to_string())?,
    })
}

#[frb]
pub fn parse_sim_application_record(data: Vec<u8>) -> Result<Option<SimApplicationDto>, String> {
    parse_application_record(&data)
        .map(|application| application.map(|(aid, label)| SimApplicationDto { aid, label }))
        .map_err(|error| error.to_string())
}
