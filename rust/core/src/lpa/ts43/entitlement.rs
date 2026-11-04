//! GSMA TS.43 Carrier Entitlement Discovery and Configuration Database.
//!
//! Ported from `entitlement.go` / `entitlement.dart`.

use core::time::Duration;
use serde::{Deserialize, Serialize};

use super::enums::{EntitlementChannel, SimType};
use super::errors::{
    EntitlementException, ENTITLEMENT_AMBIGUOUS, ENTITLEMENT_NOT_FOUND, ENTITLEMENT_UNSUPPORTED,
};
use super::sim::Identity;
use super::types::Entitlement;

pub const DEFAULT_ENTITLEMENT_TIMEOUT: Duration = Duration::from_secs(30);

/// One bundled carrier entitlement record.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EntitlementRecord {
    #[serde(rename = "mccmnc", default)]
    pub mcc_mnc: String,
    #[serde(default)]
    pub gid1: Option<String>,
    #[serde(rename = "carrierName", default)]
    pub carrier_name: String,
    #[serde(default)]
    pub channel: String,
    #[serde(rename = "entitlementServer", default)]
    pub entitlement_server: String,
    #[serde(default)]
    pub app: String,
    #[serde(rename = "appName", default)]
    pub app_name: String,
    #[serde(rename = "entitlementVersion", default)]
    pub entitlement_version: String,
    #[serde(rename = "provisioningVersion", default)]
    pub provisioning_version: String,
    #[serde(rename = "requestVersion", default)]
    pub request_version: String,
    #[serde(rename = "timeoutSeconds", default)]
    pub timeout_seconds: u32,
    #[serde(rename = "transferTypePsim", default)]
    pub transfer_type_psim: i32,
    #[serde(rename = "transferTypeEsim", default)]
    pub transfer_type_esim: i32,
    #[serde(rename = "acquireConfigurationPollingIntervalMinutes", default)]
    pub polling_interval_minutes: u32,
    #[serde(rename = "acquireConfigurationPollingTimeoutMinutes", default)]
    pub polling_timeout_minutes: u32,
    #[serde(rename = "useHttpPost", default)]
    pub use_http_post: bool,
    #[serde(rename = "supportPsimToEsimTransfer", default)]
    pub support_psim_to_esim: bool,
    #[serde(rename = "supportEsimToEsimTransfer", default)]
    pub support_esim_to_esim: bool,
    #[serde(rename = "useSmdsDelayedDownload", default)]
    pub use_smds_delayed_download: bool,
    #[serde(rename = "useTempTokenConversion", default)]
    pub use_temp_token_conversion: bool,
    #[serde(rename = "setTargetTerminalFields", default)]
    pub set_target_terminal_fields: bool,
    #[serde(rename = "restrictOldTerminalFields", default)]
    pub restrict_old_terminal_fields: bool,
}

impl EntitlementRecord {
    #[must_use]
    pub fn to_entitlement(&self, source_sim_type: SimType, transfer_type: i32) -> Entitlement {
        let channel = normalize_channel(self.channel.parse().unwrap_or_default());
        let mut e = Entitlement {
            server: self.entitlement_server.clone(),
            channel,
            use_post: self.use_http_post,
            timeout: if self.timeout_seconds > 0 {
                Duration::from_secs(u64::from(self.timeout_seconds))
            } else {
                DEFAULT_ENTITLEMENT_TIMEOUT
            },
            source_sim_type,
            transfer_type,
            use_temp_token_conversion: self.use_temp_token_conversion,
            set_target_terminal_fields: self.set_target_terminal_fields,
            restrict_old_terminal_fields: self.restrict_old_terminal_fields,
            use_smds_delayed_download: self.use_smds_delayed_download,
            poll_interval: Duration::from_secs(u64::from(self.polling_interval_minutes) * 60),
            poll_timeout: Duration::from_secs(u64::from(self.polling_timeout_minutes) * 60),
            auto_retry_511_limit: 1,
            app: self.app.clone(),
            app_name: self.app_name.clone(),
            provisioning_version: self.provisioning_version.clone(),
            request_version: self.request_version.clone(),
            version: self.entitlement_version.clone(),
        };
        if self.timeout_seconds > 0 {
            e.timeout = Duration::from_secs(u64::from(self.timeout_seconds));
        }
        e
    }

    pub fn entitlement(
        &self,
        source_sim_type: SimType,
    ) -> Result<Entitlement, EntitlementException> {
        match source_sim_type {
            SimType::Esim => {
                if !self.support_esim_to_esim {
                    return Err(EntitlementException(format!(
                        "{ENTITLEMENT_UNSUPPORTED} for MCCMNC {} eSIM source",
                        self.mcc_mnc
                    )));
                }
                Ok(self.to_entitlement(source_sim_type, self.transfer_type_esim))
            }
            SimType::Psim => {
                if !self.support_psim_to_esim {
                    return Err(EntitlementException(format!(
                        "{ENTITLEMENT_UNSUPPORTED} for MCCMNC {} pSIM source",
                        self.mcc_mnc
                    )));
                }
                Ok(self.to_entitlement(source_sim_type, self.transfer_type_psim))
            }
        }
    }
}

#[must_use]
pub fn normalize_channel(channel: EntitlementChannel) -> EntitlementChannel {
    if channel == EntitlementChannel::None {
        EntitlementChannel::Google
    } else {
        channel
    }
}

pub fn parse_entitlement_records(
    json_text: &str,
) -> Result<Vec<EntitlementRecord>, serde_json::Error> {
    serde_json::from_str(json_text)
}

/// Discovers a carrier entitlement config for the requested channel (or auto google -> samsung).
pub fn discover_entitlement_for_channel(
    records: &[EntitlementRecord],
    identity: &Identity,
    source_sim_type: SimType,
    channel: EntitlementChannel,
) -> Result<Entitlement, EntitlementException> {
    if identity.mcc.is_empty() || identity.mnc.is_empty() {
        return Err(EntitlementException(
            "source MCC/MNC is required".to_string(),
        ));
    }
    if channel == EntitlementChannel::None {
        return discover_auto(records, identity, source_sim_type);
    }
    let record = select_record(
        records,
        &identity.mcc_mnc(),
        &identity.gid1,
        normalize_channel(channel),
    )?;
    record.entitlement(source_sim_type)
}

pub fn discover_entitlement(
    records: &[EntitlementRecord],
    identity: &Identity,
    source_sim_type: SimType,
) -> Result<Entitlement, EntitlementException> {
    discover_entitlement_for_channel(records, identity, source_sim_type, EntitlementChannel::None)
}

pub fn discover_auto(
    records: &[EntitlementRecord],
    identity: &Identity,
    source_sim_type: SimType,
) -> Result<Entitlement, EntitlementException> {
    let mut unsupported: Option<EntitlementException> = None;
    for channel in [EntitlementChannel::Google, EntitlementChannel::Samsung] {
        let record = match select_record(records, &identity.mcc_mnc(), &identity.gid1, channel) {
            Ok(r) => r,
            Err(e) => {
                if e.0.starts_with(ENTITLEMENT_NOT_FOUND) {
                    continue;
                }
                return Err(e);
            }
        };

        match record.entitlement(source_sim_type) {
            Ok(ent) => return Ok(ent),
            Err(e) => {
                if e.0.starts_with(ENTITLEMENT_UNSUPPORTED) {
                    unsupported = Some(e);
                    continue;
                }
                return Err(e);
            }
        }
    }

    if let Some(err) = unsupported {
        return Err(err);
    }
    Err(EntitlementException(format!(
        "{ENTITLEMENT_NOT_FOUND} for MCCMNC {}",
        identity.mcc_mnc()
    )))
}

fn select_record(
    records: &[EntitlementRecord],
    mcc_mnc: &str,
    gid1: &str,
    channel: EntitlementChannel,
) -> Result<EntitlementRecord, EntitlementException> {
    let mut fallback = Vec::new();
    let mut gid_matches = Vec::new();

    for record in records {
        if record.mcc_mnc != mcc_mnc {
            continue;
        }
        if normalize_channel(record.channel.parse().unwrap_or_default()) != channel {
            continue;
        }
        if record.gid1.is_none() {
            fallback.push(record.clone());
            continue;
        }
        if let Some(r_gid1) = &record.gid1 {
            if !gid1.is_empty() && r_gid1.eq_ignore_ascii_case(gid1) {
                gid_matches.push(record.clone());
            }
        }
    }

    if gid_matches.len() == 1 {
        return Ok(gid_matches.remove(0));
    }
    if gid_matches.len() > 1 {
        return Err(EntitlementException(format!(
            "{ENTITLEMENT_AMBIGUOUS} for MCCMNC {mcc_mnc} GID1 {gid1}"
        )));
    }
    if fallback.len() == 1 {
        return Ok(fallback.remove(0));
    }
    if fallback.len() > 1 {
        return Err(EntitlementException(format!(
            "{ENTITLEMENT_AMBIGUOUS} for MCCMNC {mcc_mnc}"
        )));
    }
    Err(EntitlementException(format!(
        "{ENTITLEMENT_NOT_FOUND} for MCCMNC {mcc_mnc}"
    )))
}
