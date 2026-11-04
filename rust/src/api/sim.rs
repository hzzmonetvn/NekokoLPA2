//! SIM file parsing operations bridged to Flutter.

use flutter_rust_bridge::frb;
use nlpa_core::asn1::fcp::{parse_application_record, parse_fcp};

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
