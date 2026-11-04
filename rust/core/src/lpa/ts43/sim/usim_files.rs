//! USIM file identifiers and APDU constants.
//!
//! Ported from `sim/usim.go` / `sim/usim_files.dart`.

/// USIM ADF AID prefix (3GPP, Telecom).
pub const USIM_AID_PREFIX: [u8; 7] = [0xA0, 0x00, 0x00, 0x00, 0x87, 0x10, 0x02];

// File identifiers (FIDs)
pub const MASTER_FILE: u16 = 0x3F00;
pub const EF_DIR: u16 = 0x2F00;
pub const EF_ICCID: u16 = 0x2FE2;
pub const EF_IMSI: u16 = 0x6F07;
pub const EF_AD: u16 = 0x6FAD;
pub const EF_GID1: u16 = 0x6F3E;

// EF read lengths used by identity reads
pub const ICCID_LENGTH: usize = 10;
pub const IMSI_LENGTH: usize = 9;
pub const AD_LENGTH: usize = 4;

// AUTHENTICATE (3G) command header: CLA=00 INS=88 P1=00 P2=81
pub const AUTHENTICATE_CLA: u8 = 0x00;
pub const AUTHENTICATE_INS: u8 = 0x88;
pub const AUTHENTICATE_P1: u8 = 0x00;
pub const AUTHENTICATE_P2: u8 = 0x81;

// SELECT by FID: CLA=00 INS=A4 P1=00 P2=04 (return FCP template)
pub const SELECT_CLA: u8 = 0x00;
pub const SELECT_INS: u8 = 0xA4;
pub const SELECT_BY_ID_P1: u8 = 0x00;
pub const SELECT_BY_NAME_P1: u8 = 0x04;
pub const SELECT_P2: u8 = 0x04;

// READ BINARY: CLA=00 INS=B0. READ RECORD: CLA=00 INS=B2 (P2=04 absolute)
pub const READ_BINARY_CLA: u8 = 0x00;
pub const READ_BINARY_INS: u8 = 0xB0;
pub const READ_RECORD_CLA: u8 = 0x00;
pub const READ_RECORD_INS: u8 = 0xB2;
pub const READ_RECORD_P2: u8 = 0x04;

/// Encodes a 2-byte big-endian file identifier.
#[must_use]
pub fn fid(id: u16) -> [u8; 2] {
    id.to_be_bytes()
}
