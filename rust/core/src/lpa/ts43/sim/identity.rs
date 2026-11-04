//! SIM identity representation (ICCID, IMSI, MCC, MNC, GID1).
//!
//! Ported from `sim/identity.go` / `sim/identity.dart`.

use super::imsi::{decode_mnc_length, format_mnc, Imsi, ImsiError};

/// Source SIM identity fields.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Identity {
    pub iccid: String,
    pub imsi: String,
    pub mcc: String,
    pub mnc: String,
    pub gid1: String,
}

impl Identity {
    #[must_use]
    pub fn new(
        iccid: impl Into<String>,
        imsi: impl Into<String>,
        mcc: impl Into<String>,
        mnc: impl Into<String>,
        gid1: impl Into<String>,
    ) -> Self {
        Self {
            iccid: iccid.into(),
            imsi: imsi.into(),
            mcc: mcc.into(),
            mnc: mnc.into(),
            gid1: gid1.into(),
        }
    }

    #[must_use]
    pub fn mcc_mnc(&self) -> String {
        format!("{}{}", self.mcc, self.mnc)
    }

    /// Builds an `Identity` from raw EF reads, deriving MCC from IMSI and MNC from EF_AD.
    pub fn derive(
        iccid: impl Into<String>,
        imsi: impl Into<String>,
        gid1: impl Into<String>,
        administrative_data: &[u8],
    ) -> Result<Self, ImsiError> {
        let imsi_str = imsi.into();
        if imsi_str.is_empty() {
            return Err(ImsiError::EmptyPayload);
        }
        let parsed_imsi = Imsi::new(&imsi_str);
        let mcc = parsed_imsi.mcc()?.to_string();
        let mnc_len = decode_mnc_length(administrative_data)?;
        let mnc = format_mnc(&imsi_str, mnc_len)?.to_string();

        Ok(Self {
            iccid: iccid.into(),
            imsi: imsi_str,
            mcc,
            mnc,
            gid1: gid1.into(),
        })
    }
}
