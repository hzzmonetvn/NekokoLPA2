//! SIM primitives, identifiers, IMSI, identity, USIM files, and EAP-AKA.

pub mod aka;
pub mod identifier;
pub mod identity;
pub mod imsi;
pub mod usim_files;

pub use aka::{
    imsi_eap_identity, respond_to_challenge, respond_to_challenge_base64, AkaChallenge, AkaError,
    AkaRequest, AkaResponse,
};
pub use identifier::{
    decode_bcd, decode_swapped_bcd, encode_bcd, group_identifier_string, IdentifierError,
};
pub use identity::Identity;
pub use imsi::{decode_mnc_length, format_mnc, Imsi, ImsiError};
pub use usim_files::{fid, USIM_AID_PREFIX};

// Re-export struct for USIM file constants
pub struct UsimFiles;

impl UsimFiles {
    pub const AID_PREFIX: [u8; 7] = usim_files::USIM_AID_PREFIX;
    pub const MASTER_FILE: u16 = usim_files::MASTER_FILE;
    pub const EF_DIR: u16 = usim_files::EF_DIR;
    pub const EF_ICCID: u16 = usim_files::EF_ICCID;
    pub const EF_IMSI: u16 = usim_files::EF_IMSI;
    pub const EF_AD: u16 = usim_files::EF_AD;
    pub const EF_GID1: u16 = usim_files::EF_GID1;
}
