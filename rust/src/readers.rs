//! Smart-card reader discovery, EID deduplication, OMAPI safety latch, and logical channels.

use core::fmt;
use core::pin::Pin;
use core::time::Duration;
use std::collections::BTreeMap;
use std::error::Error;
use std::future::Future;
use std::sync::{Arc, Mutex};

use nlpa_core::apdu::validate_apdu_sw;
use nlpa_core::atr::is_likely_euicc_atr;
use nlpa_core::channel::{
    chain_apdu_request, map_logical_channel_cla, OmapiSafetyLatch, ReassemblyAction,
    ResponseReassembler, DEFAULT_STORE_DATA_MSS,
};
use nlpa_core::crypto::{base64_decode, base64_encode};
use nlpa_core::hex::{bytes_to_hex, hex_to_bytes};
use nlpa_core::lpa::es10::{build_select_isdr_apdu, ISDR_AID};
use nlpa_core::modem::{ModemSession, QrtrSocketAddress, QrtrSocketDatagram, TransportSocket};
use nlpa_core::plugins::{estk_extra_aids, ESTK_ATR_MARKER};
use nlpa_core::remocard::{
    build_close_channel_request, build_open_channel_request, build_remocard_headers,
    build_send_apdu_request, clean_remocard_url, encrypt_remocard_payload,
    parse_list_slots_response, parse_open_channel_response, parse_send_apdu_response,
    resolve_remocard_key, RemoCardServer,
};
use nlpa_core::storage::PersistedState;

use crate::host::{HostCallError, HostClient, HostTransport, HttpRequestArgs};
use crate::runtime;

/// Error returned while discovering readers or executing card APDU transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReaderError {
    OmapiCorrupted(String),
    PermissionRequired(String),
    ChannelUnavailable(String),
    ApduFailed { sw1: u8, sw2: u8 },
    Host(HostCallError),
    Protocol(String),
}

impl fmt::Display for ReaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OmapiCorrupted(msg) => write!(f, "OMAPI_SESSION_CORRUPTED: {msg}"),
            Self::PermissionRequired(msg) => write!(f, "Permission required: {msg}"),
            Self::ChannelUnavailable(msg) => write!(f, "Reader unavailable: {msg}"),
            Self::ApduFailed { sw1, sw2 } => write!(f, "Card returned SW={sw1:02X}{sw2:02X}"),
            Self::Host(e) => write!(f, "{e}"),
            Self::Protocol(msg) => write!(f, "{msg}"),
        }
    }
}

impl Error for ReaderError {}

impl From<HostCallError> for ReaderError {
    fn from(err: HostCallError) -> Self {
        if err.is_omapi_session_corrupted() {
            Self::OmapiCorrupted(err.message)
        } else {
            Self::Host(err)
        }
    }
}

impl ReaderError {
    /// Returns `true` if the card status word is ISO 7816-4 `0x6881` (logical channel not supported/active).
    #[must_use]
    pub const fn is_sw6881(&self) -> bool {
        matches!(
            self,
            Self::ApduFailed {
                sw1: 0x68,
                sw2: 0x81
            }
        )
    }
}

/// Supported eUICC reader backend types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportKind {
    NBridgeTmapi,
    NBridgeOmapi,
    Telephony,
    Omapi,
    NBridgeOther,
    AdbBridge,
    Qrtr,
    SeHal,
    Ccid,
    Remote,
}

impl TransportKind {
    /// Short badge string shown on reader selector chips.
    #[must_use]
    pub const fn badge(self) -> &'static str {
        match self {
            Self::NBridgeTmapi | Self::NBridgeOmapi | Self::NBridgeOther => "OTBridge",
            Self::Telephony => "TMAPI",
            Self::Omapi => "OMAPI",
            Self::AdbBridge => "ADB",
            Self::Qrtr => "QRTR",
            Self::SeHal => "HAL",
            Self::Ccid => "CCID",
            Self::Remote => "Remote",
        }
    }

    /// Deduplication rank when multiple transports expose the same eUICC EID.
    ///
    /// Order matches `#20`: `NBridge tmapi > NBridge omapi > Telephony > Omapi > AdbBridge`.
    #[must_use]
    pub const fn dedup_priority(self) -> u8 {
        match self {
            Self::NBridgeTmapi => 40,
            Self::NBridgeOmapi => 35,
            Self::Telephony => 30,
            Self::Qrtr => 25,
            Self::Omapi => 20,
            Self::Ccid => 15,
            Self::NBridgeOther => 10,
            Self::SeHal => 5,
            Self::Remote => 2,
            Self::AdbBridge => 0,
        }
    }
}

/// One discovered card reader or SIM slot entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredReader {
    pub id: String,
    pub label: String,
    pub kind: TransportKind,
    pub slot_label: String,
    pub raw_name: String,
    pub slot_index: Option<i32>,
    pub port_index: Option<i32>,
    pub is_removable: bool,
    pub needs_permission: bool,
    pub atr_hex: Option<String>,
    pub eid: Option<String>,
    pub remocard_server: Option<RemoCardServer>,
}

impl DiscoveredReader {
    /// Returns `true` if ATR bytes or name match eSTK smart-card markers.
    #[must_use]
    pub fn looks_like_estk(&self) -> bool {
        if let Some(atr) = &self.atr_hex {
            if let Ok(bytes) = hex_to_bytes(atr) {
                if bytes.windows(ESTK_ATR_MARKER.len()).any(|w| w == ESTK_ATR_MARKER) {
                    return true;
                }
            }
        }
        self.label.to_ascii_lowercase().contains("estk")
    }

    /// Returns `true` if this CCID/removable reader matches 9eSIM USB management patterns.
    #[must_use]
    pub fn looks_like_9esim(&self) -> bool {
        let lower = self.label.to_ascii_lowercase();
        self.kind == TransportKind::Ccid
            || lower.contains("9esim")
            || lower.contains("esim writer")
    }
}

/// Output of a single-flight reader scan.
#[derive(Debug, Clone, Default)]
pub struct ReaderDiscoveryReport {
    pub readers: Vec<DiscoveredReader>,
    pub omapi_euicc_warning: bool,
    pub omapi_corrupted_banner: Option<String>,
    pub logs: Vec<String>,
}

/// Builds the ordered list of ISD-R AIDs to attempt when opening an eUICC channel.
#[must_use]
pub fn candidate_isdr_aids(custom_aids: &[String], atr_hex: Option<&str>) -> Vec<String> {
    let mut aids = Vec::new();
    let default_hex = bytes_to_hex(&ISDR_AID);
    for custom in custom_aids {
        let clean = custom.trim().to_ascii_uppercase();
        if !clean.is_empty() && !aids.contains(&clean) {
            aids.push(clean);
        }
    }
    if let Some(atr) = atr_hex {
        for extra in estk_extra_aids(atr) {
            let clean = extra.trim().to_ascii_uppercase();
            if !clean.is_empty() && !aids.contains(&clean) {
                aids.push(clean);
            }
        }
    }
    if !aids.contains(&default_hex) {
        aids.push(default_hex);
    }
    aids
}

/// Discovers enabled readers across all configured backends and deduplicates by EID priority.
///
/// When `foreground_card_session_open` is `true`, EID probing is skipped to avoid
/// concurrent logical channel interference (`SW=6881`) during active downloads/switches.
pub fn discover_readers(
    host: Arc<dyn HostTransport>,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    foreground_card_session_open: bool,
    cached_eids: &BTreeMap<String, String>,
) -> ReaderDiscoveryReport {
    let client = HostClient::new(host.as_ref());
    let mut report = ReaderDiscoveryReport::default();
    let mut raw_readers = Vec::new();

    if state.enable_nbridge {
        collect_nbridge_readers(&client, &mut raw_readers, &mut report.logs);
    }
    if state.enable_tmapi {
        collect_telephony_readers(&client, &mut raw_readers, &mut report.logs);
    }
    if state.enable_omapi {
        collect_omapi_readers(&client, omapi_latch, &mut raw_readers, &mut report);
    }
    if state.enable_adb_bridge {
        collect_adb_readers(&client, &mut raw_readers, &mut report.logs);
    }
    if state.enable_qrtr {
        collect_qrtr_readers(host.clone(), &mut raw_readers, &mut report.logs);
    }
    if state.enable_se_hal {
        collect_se_hal_readers(&client, &mut raw_readers, &mut report.logs);
    }
    if state.enable_ccid {
        collect_ccid_readers(&client, &mut raw_readers, &mut report.logs);
    }
    if state.enable_remote {
        collect_remote_readers(&client, state, &mut raw_readers, &mut report.logs);
    }

    // Attach cached EIDs or probe when no foreground session holds the card.
    for reader in &mut raw_readers {
        if let Some(cached) = cached_eids.get(&reader.id) {
            reader.eid = Some(cached.clone());
            continue;
        }
        if foreground_card_session_open || reader.needs_permission {
            continue;
        }
        if let Ok(eid) = probe_reader_eid(host.clone(), reader, state, omapi_latch) {
            if !eid.is_empty() {
                reader.eid = Some(eid);
            }
        }
    }

    report.readers = deduplicate_by_eid(raw_readers);
    report
}

fn collect_nbridge_readers(
    client: &HostClient<'_>,
    out: &mut Vec<DiscoveredReader>,
    logs: &mut Vec<String>,
) {
    let available = match client.nbridge_is_available() {
        Ok(v) => v,
        Err(e) => {
            logs.push(format!("NBridge availability check failed: {e}"));
            return;
        }
    };
    if !available {
        return;
    }
    match client.nbridge_list_slots() {
        Ok(slots) => {
            for slot in slots {
                let lower_id = slot.id.to_ascii_lowercase();
                let lower_name = slot.display_name.to_ascii_lowercase();
                let kind = if lower_id.contains("tmapi") || lower_name.contains("tmapi") {
                    TransportKind::NBridgeTmapi
                } else if lower_id.contains("omapi") || lower_name.contains("omapi") {
                    TransportKind::NBridgeOmapi
                } else {
                    TransportKind::NBridgeOther
                };
                out.push(DiscoveredReader {
                    id: format!("nbridge:{}", slot.id),
                    label: if slot.display_name.is_empty() {
                        slot.id.clone()
                    } else {
                        slot.display_name.clone()
                    },
                    kind,
                    slot_label: slot.id.clone(),
                    raw_name: slot.id,
                    slot_index: None,
                    port_index: None,
                    is_removable: false,
                    needs_permission: false,
                    atr_hex: None,
                    eid: None,
                    remocard_server: None,
                });
            }
        }
        Err(e) => logs.push(format!("NBridge listSlots failed: {e}")),
    }
}

fn collect_telephony_readers(
    client: &HostClient<'_>,
    out: &mut Vec<DiscoveredReader>,
    logs: &mut Vec<String>,
) {
    let has_perm = client.telephony_has_privileged_permission().unwrap_or(false);
    if !has_perm {
        return;
    }
    match client.telephony_list_slots() {
        Ok(slots) => {
            for slot in slots {
                if !slot.is_euicc && slot.ports.is_empty() {
                    continue;
                }
                let port_idx = slot.ports.first().map_or(0, |p| p.port_index);
                out.push(DiscoveredReader {
                    id: format!("tmapi:slot{}:port{}", slot.slot_index, port_idx),
                    label: format!("SIM{} (Telephony)", slot.slot_index + 1),
                    kind: TransportKind::Telephony,
                    slot_label: format!("Slot {}", slot.slot_index + 1),
                    raw_name: slot.card_id,
                    slot_index: Some(slot.slot_index),
                    port_index: Some(port_idx),
                    is_removable: slot.is_removable,
                    needs_permission: false,
                    atr_hex: None,
                    eid: None,
                    remocard_server: None,
                });
            }
        }
        Err(e) => logs.push(format!("Telephony listSlots failed: {e}")),
    }
}

fn collect_omapi_readers(
    client: &HostClient<'_>,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    out: &mut Vec<DiscoveredReader>,
    report: &mut ReaderDiscoveryReport,
) {
    if let Ok(guard) = omapi_latch.lock() {
        if guard.is_poisoned() {
            report.omapi_corrupted_banner =
                Some("OMAPI session was marked corrupted; reconnect or restart device before retrying OMAPI.".into());
            return;
        }
    }
    let names = match client.omapi_list_readers() {
        Ok(list) => list,
        Err(e) => {
            if e.is_omapi_session_corrupted() {
                if let Ok(mut guard) = omapi_latch.lock() {
                    guard.poison(&e.message);
                }
                report.omapi_corrupted_banner = Some(e.to_string());
            } else {
                report.logs.push(format!("OMAPI listReaders failed: {e}"));
            }
            return;
        }
    };
    for name in names {
        if !name.to_ascii_uppercase().starts_with("SIM")
            && !name.to_ascii_uppercase().starts_with("ESE")
        {
            continue;
        }
        match client.omapi_connect(&name) {
            Ok(conn) if conn.success => {
                let is_likely = hex_to_bytes(&conn.atr)
                    .map(|b| is_likely_euicc_atr(&b))
                    .unwrap_or(false);
                if conn.atr.is_empty() && name.to_ascii_uppercase().contains("SIM") {
                    report.omapi_euicc_warning = true;
                }
                let _ = client.omapi_disconnect(&name);
                out.push(DiscoveredReader {
                    id: format!("omapi:{name}"),
                    label: name.clone(),
                    kind: TransportKind::Omapi,
                    slot_label: name.clone(),
                    raw_name: name,
                    slot_index: None,
                    port_index: None,
                    is_removable: true,
                    needs_permission: false,
                    atr_hex: if conn.atr.is_empty() {
                        None
                    } else {
                        Some(conn.atr)
                    },
                    eid: None,
                    remocard_server: None,
                });
                if !is_likely && out.last().is_some_and(|r| r.atr_hex.is_none()) {
                    report.omapi_euicc_warning = true;
                }
            }
            Ok(conn) => {
                let is_likely = hex_to_bytes(&conn.atr)
                    .map(|b| is_likely_euicc_atr(&b))
                    .unwrap_or(false);
                if is_likely || name.to_ascii_uppercase().starts_with("SIM") {
                    report.omapi_euicc_warning = true;
                }
            }
            Err(e) => {
                if e.is_omapi_session_corrupted() {
                    if let Ok(mut guard) = omapi_latch.lock() {
                        guard.poison(&e.message);
                    }
                    report.omapi_corrupted_banner = Some(e.to_string());
                    return;
                }
                report.logs.push(format!("OMAPI connect {name}: {e}"));
            }
        }
    }
}

fn collect_adb_readers(
    client: &HostClient<'_>,
    out: &mut Vec<DiscoveredReader>,
    logs: &mut Vec<String>,
) {
    if !client.adb_is_connected().unwrap_or(false) {
        return;
    }
    match client.adb_list_slots() {
        Ok(slots) => {
            for slot in slots {
                out.push(DiscoveredReader {
                    id: format!("adb:{}", slot.id),
                    label: if slot.display_name.is_empty() {
                        format!("ADB {}", slot.id)
                    } else {
                        slot.display_name.clone()
                    },
                    kind: TransportKind::AdbBridge,
                    slot_label: slot.id.clone(),
                    raw_name: slot.id,
                    slot_index: None,
                    port_index: None,
                    is_removable: false,
                    needs_permission: false,
                    atr_hex: None,
                    eid: None,
                    remocard_server: None,
                });
            }
        }
        Err(e) => logs.push(format!("ADB listSlots failed: {e}")),
    }
}

fn collect_qrtr_readers(
    host: Arc<dyn HostTransport>,
    out: &mut Vec<DiscoveredReader>,
    logs: &mut Vec<String>,
) {
    let client = HostClient::new(host.as_ref());
    if !client.qrtr_is_supported().unwrap_or(false)
        || !client.qrtr_is_available().unwrap_or(false)
    {
        return;
    }
    if !client.qrtr_has_permission().unwrap_or(false) {
        return;
    }
    if let Err(e) = client.qrtr_open_bus() {
        logs.push(format!("QRTR openBus failed: {e}"));
        return;
    }
    let mut session = ModemSession::new(Box::new(QrtrModemTransport {
        host: Arc::clone(&host),
    }));
    let slots_res = runtime::block_on(session.state());
    let _ = client.qrtr_close_bus();

    match slots_res {
        Ok(slots) => {
            for slot in slots {
                if !slot.present || !slot.ready {
                    continue;
                }
                let atr_hex = slot.atr.as_deref().map(bytes_to_hex);
                out.push(DiscoveredReader {
                    id: format!("qrtr:slot{}", slot.slot),
                    label: format!("Modem Slot {} (QRTR)", slot.slot),
                    kind: TransportKind::Qrtr,
                    slot_label: format!("Slot {}", slot.slot),
                    raw_name: format!("slot{}", slot.slot),
                    slot_index: Some(i32::from(slot.slot)),
                    port_index: None,
                    is_removable: false,
                    needs_permission: false,
                    atr_hex,
                    eid: None,
                    remocard_server: None,
                });
            }
        }
        Err(e) => logs.push(format!("QRTR ModemSession::state failed: {e}")),
    }
}

fn collect_se_hal_readers(
    client: &HostClient<'_>,
    out: &mut Vec<DiscoveredReader>,
    logs: &mut Vec<String>,
) {
    for slot in [1_i32, 2_i32] {
        match client.se_hal_atr(slot) {
            Ok(atr) if !atr.is_empty() => {
                out.push(DiscoveredReader {
                    id: format!("sehal:slot{slot}"),
                    label: format!("SE HAL Slot {slot}"),
                    kind: TransportKind::SeHal,
                    slot_label: format!("Slot {slot}"),
                    raw_name: format!("slot{slot}"),
                    slot_index: Some(slot),
                    port_index: None,
                    is_removable: false,
                    needs_permission: false,
                    atr_hex: Some(atr),
                    eid: None,
                    remocard_server: None,
                });
            }
            Ok(_) => {}
            Err(e) => logs.push(format!("SE HAL slot {slot} skipped: {e}")),
        }
    }
}

fn collect_ccid_readers(
    client: &HostClient<'_>,
    out: &mut Vec<DiscoveredReader>,
    logs: &mut Vec<String>,
) {
    match client.ccid_list_readers() {
        Ok(list) => {
            for name in list {
                let mut needs_permission = false;
                let mut atr_hex = None;
                match client.ccid_connect(&name) {
                    Ok(atr) => {
                        if !atr.is_empty() {
                            atr_hex = Some(atr);
                        }
                        let _ = client.ccid_disconnect(&name);
                    }
                    Err(e) => {
                        let lower = e.message.to_ascii_lowercase();
                        if lower.contains("permission") || e.code == "PERMISSION_DENIED" {
                            needs_permission = true;
                        } else {
                            logs.push(format!("CCID connect {name}: {e}"));
                        }
                    }
                }
                out.push(DiscoveredReader {
                    id: format!("ccid:{name}"),
                    label: name.clone(),
                    kind: TransportKind::Ccid,
                    slot_label: "USB".into(),
                    raw_name: name,
                    slot_index: None,
                    port_index: None,
                    is_removable: true,
                    needs_permission,
                    atr_hex,
                    eid: None,
                    remocard_server: None,
                });
            }
        }
        Err(e) => logs.push(format!("CCID listReaders failed: {e}")),
    }
}

fn collect_remote_readers(
    client: &HostClient<'_>,
    state: &PersistedState,
    out: &mut Vec<DiscoveredReader>,
    logs: &mut Vec<String>,
) {
    for server in &state.remocard_servers {
        if !server.enabled || server.base_url.trim().is_empty() {
            continue;
        }
        let mut headers = BTreeMap::new();
        for (k, v) in build_remocard_headers(server) {
            headers.insert(k, v);
        }
        let (clean_url, _) = clean_remocard_url(&server.base_url);
        let url = format!("{}/listSlots", clean_url.trim_end_matches('/'));
        let resp = client.http_request(&HttpRequestArgs {
            method: "GET".into(),
            url,
            headers,
            body_base64: None,
            proxy_url: state.proxy_url.clone(),
            timeout_ms: 8_000,
            insecure_tls: false,
        });
        match resp {
            Ok(http) if (200..300).contains(&http.status) => {
                if let Ok(bytes) = base64_decode(&http.body_base64) {
                    let body = String::from_utf8_lossy(&bytes);
                    match parse_list_slots_response(&body, Some(server)) {
                        Ok(slots) => {
                            for slot in slots {
                                if slot.name.is_empty() {
                                    continue;
                                }
                                out.push(DiscoveredReader {
                                    id: format!("remote:{}:{}", server.id, slot.name),
                                    label: format!("{} ({})", slot.name, server.name),
                                    kind: TransportKind::Remote,
                                    slot_label: server.name.clone(),
                                    raw_name: slot.name,
                                    slot_index: None,
                                    port_index: None,
                                    is_removable: true,
                                    needs_permission: false,
                                    atr_hex: None,
                                    eid: slot.eid,
                                    remocard_server: Some(server.clone()),
                                });
                            }
                        }
                        Err(e) => logs.push(format!("RemoCard {} parse: {e}", server.name)),
                    }
                }
            }
            Ok(http) => logs.push(format!("RemoCard {} HTTP {}", server.name, http.status)),
            Err(e) => logs.push(format!("RemoCard {} error: {e}", server.name)),
        }
    }
}

/// Deduplicates discovered readers sharing the same EID using [`TransportKind::dedup_priority`].
#[must_use]
pub fn deduplicate_by_eid(readers: Vec<DiscoveredReader>) -> Vec<DiscoveredReader> {
    let mut best_by_eid: BTreeMap<String, DiscoveredReader> = BTreeMap::new();
    let mut unprobed = Vec::new();

    for reader in readers {
        let Some(ref eid) = reader.eid else {
            unprobed.push(reader);
            continue;
        };
        let norm_eid = eid.trim().to_ascii_uppercase();
        if norm_eid.is_empty() {
            unprobed.push(reader);
            continue;
        }
        match best_by_eid.get(&norm_eid) {
            Some(existing) if existing.kind.dedup_priority() >= reader.kind.dedup_priority() => {}
            _ => {
                best_by_eid.insert(norm_eid, reader);
            }
        }
    }

    let mut combined: Vec<DiscoveredReader> = best_by_eid.into_values().collect();
    combined.extend(unprobed);
    combined.sort_by(|a, b| {
        b.kind
            .dedup_priority()
            .cmp(&a.kind.dedup_priority())
            .then_with(|| a.id.cmp(&b.id))
    });
    combined
}

/// Probes only the EID (`ES10c.GetEuiccData` Tag `0xBF3E`) for discovery deduplication.
fn probe_reader_eid(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
) -> Result<String, ReaderError> {
    let aids = candidate_isdr_aids(&state.custom_aids, reader.atr_hex.as_deref());
    let mss = usize::from(state.mss.max(32));
    let mut channel = CardChannel::open(
        host,
        reader,
        &aids,
        mss,
        omapi_latch,
        match reader.slot_index {
            Some(2) => state.se_hal_borrowed_slot2,
            _ => state.se_hal_borrowed_slot1,
        },
        state.proxy_url.clone(),
    )?;
    let get_eid_tlv = [0xBF, 0x3E, 0x03, 0x5C, 0x01, 0x5A];
    let resp = channel.transmit_es10_tlv(&get_eid_tlv);
    let _ = channel.close();
    let data = resp?;
    parse_eid_from_get_euicc_data(&data)
        .ok_or_else(|| ReaderError::Protocol("EID missing in GetEuiccData response".into()))
}

/// Parses EID hex from `GetEuiccData` TLV response (`0xBF3E` -> `0x5A`).
#[must_use]
pub fn parse_eid_from_get_euicc_data(der: &[u8]) -> Option<String> {
    let nodes = nlpa_core::asn1::Asn1Reader::new(der).read_all().ok()?;
    for node in nodes {
        if node.is_tag(0xBF3E) {
            if let Some(eid) = node.find_child(0x5A) {
                return Some(bytes_to_hex(&eid.value));
            }
        } else if node.is_tag(0x5A) {
            return Some(bytes_to_hex(&node.value));
        }
    }
    None
}

/// Active [`nlpa_core::modem::TransportSocket`] implementation backed by `qrtr.*` host calls.
pub struct QrtrModemTransport {
    pub host: Arc<dyn HostTransport>,
}

impl TransportSocket for QrtrModemTransport {
    fn address(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<QrtrSocketAddress, String>> + Send + '_>> {
        Box::pin(async move {
            let client = HostClient::new(self.host.as_ref());
            let node = client.qrtr_bus_node().map_err(|e| e.to_string())?;
            Ok(QrtrSocketAddress { node, port: 0 })
        })
    }

    fn send(
        &mut self,
        to: QrtrSocketAddress,
        datagram: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move {
            let client = HostClient::new(self.host.as_ref());
            let hex = bytes_to_hex(&datagram);
            client
                .qrtr_send(to.node, to.port, &hex)
                .map_err(|e| e.to_string())
        })
    }

    fn receive(
        &mut self,
        timeout: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Option<QrtrSocketDatagram>, String>> + Send + '_>> {
        Box::pin(async move {
            let client = HostClient::new(self.host.as_ref());
            let ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
            let Some(pkt) = client.qrtr_receive(ms).map_err(|e| e.to_string())? else {
                return Ok(None);
            };
            let data = hex_to_bytes(&pkt.payload_hex).map_err(|e| e.to_string())?;
            Ok(Some(QrtrSocketDatagram {
                node: pkt.node,
                port: pkt.port,
                data,
            }))
        })
    }
}

enum ChannelBackend {
    Omapi {
        reader: String,
        aid: String,
    },
    Telephony {
        slot_index: i32,
        port_index: i32,
        channel: i32,
    },
    NBridge {
        connection_id: String,
    },
    Adb {
        connection_id: String,
    },
    Qrtr {
        slot: u8,
        channel: u8,
        session: Box<ModemSession>,
    },
    SeHal {
        slot: i32,
        borrowed: bool,
    },
    Ccid {
        reader: String,
        channel: u8,
    },
    Remote {
        server: RemoCardServer,
        reader_name: String,
        aid: String,
        proxy_url: Option<String>,
    },
}

/// Active logical eUICC card session abstracting all 10 transports uniformly.
pub struct CardChannel {
    host: Arc<dyn HostTransport>,
    backend: Option<ChannelBackend>,
    mss: usize,
    select_response: Vec<u8>,
    omapi_latch: Arc<Mutex<OmapiSafetyLatch>>,
    pub apdu_log: Vec<(String, String)>,
}

impl CardChannel {
    /// Opens an ISD-R (or requested AID) logical channel on `reader`.
    pub fn open(
        host: Arc<dyn HostTransport>,
        reader: &DiscoveredReader,
        aids: &[String],
        mss: usize,
        omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
        se_hal_borrowed: bool,
        proxy_url: Option<String>,
    ) -> Result<Self, ReaderError> {
        let effective_mss = if mss < 32 { DEFAULT_STORE_DATA_MSS } else { mss };
        let client = HostClient::new(host.as_ref());

        match reader.kind {
            TransportKind::Omapi => {
                if let Ok(guard) = omapi_latch.lock() {
                    guard
                        .check_safety()
                        .map_err(|e| ReaderError::OmapiCorrupted(e.to_string()))?;
                }
                let conn = client.omapi_connect(&reader.raw_name).map_err(|e| {
                    if e.is_omapi_session_corrupted() {
                        if let Ok(mut g) = omapi_latch.lock() {
                            g.poison(&e.message);
                        }
                    }
                    ReaderError::from(e)
                })?;
                if !conn.success {
                    return Err(ReaderError::ChannelUnavailable(format!(
                        "OMAPI connect refused for {}",
                        reader.raw_name
                    )));
                }
                let open_res = client
                    .omapi_open_channel(&reader.raw_name, aids)
                    .map_err(|e| {
                        if e.is_omapi_session_corrupted() {
                            if let Ok(mut g) = omapi_latch.lock() {
                                g.poison(&e.message);
                            }
                        }
                        let _ = client.omapi_disconnect(&reader.raw_name);
                        ReaderError::from(e)
                    })?;
                if !open_res.success {
                    let _ = client.omapi_disconnect(&reader.raw_name);
                    return Err(ReaderError::ChannelUnavailable(
                        "OMAPI openChannel did not select an eUICC AID".into(),
                    ));
                }
                let sel_bytes = open_res
                    .select_response
                    .as_deref()
                    .and_then(|s| hex_to_bytes(s).ok())
                    .unwrap_or_default();
                Ok(Self {
                    host,
                    backend: Some(ChannelBackend::Omapi {
                        reader: reader.raw_name.clone(),
                        aid: open_res.aid,
                    }),
                    mss: effective_mss,
                    select_response: sel_bytes,
                    omapi_latch: Arc::clone(omapi_latch),
                    apdu_log: Vec::new(),
                })
            }
            TransportKind::Telephony => {
                let slot_index = reader.slot_index.unwrap_or(0);
                let port_index = reader.port_index.unwrap_or(0);
                let _ = client.telephony_send_terminal_capabilities(slot_index, port_index);
                let mut last_err = ReaderError::ChannelUnavailable("No AID selected".into());
                for aid in aids {
                    match client.telephony_open_channel(slot_index, port_index, aid) {
                        Ok(res) if res.channel > 0 => {
                            let sel_bytes = res
                                .select_response
                                .as_deref()
                                .and_then(|s| hex_to_bytes(s).ok())
                                .unwrap_or_default();
                            return Ok(Self {
                                host,
                                backend: Some(ChannelBackend::Telephony {
                                    slot_index,
                                    port_index,
                                    channel: res.channel,
                                }),
                                mss: effective_mss,
                                select_response: sel_bytes,
                                omapi_latch: Arc::clone(omapi_latch),
                                apdu_log: Vec::new(),
                            });
                        }
                        Ok(_) => {}
                        Err(e) => last_err = ReaderError::from(e),
                    }
                }
                Err(last_err)
            }
            TransportKind::NBridgeTmapi
            | TransportKind::NBridgeOmapi
            | TransportKind::NBridgeOther => {
                let res = client.nbridge_connect_logical_channel(&reader.raw_name, aids)?;
                let sel_bytes = hex_to_bytes(&res.response_apdu).unwrap_or_default();
                Ok(Self {
                    host,
                    backend: Some(ChannelBackend::NBridge {
                        connection_id: res.connection_id,
                    }),
                    mss: effective_mss,
                    select_response: sel_bytes,
                    omapi_latch: Arc::clone(omapi_latch),
                    apdu_log: Vec::new(),
                })
            }
            TransportKind::AdbBridge => {
                let res = client.adb_connect_logical_channel(&reader.raw_name, aids)?;
                let sel_bytes = hex_to_bytes(&res.response_apdu).unwrap_or_default();
                Ok(Self {
                    host,
                    backend: Some(ChannelBackend::Adb {
                        connection_id: res.connection_id,
                    }),
                    mss: effective_mss,
                    select_response: sel_bytes,
                    omapi_latch: Arc::clone(omapi_latch),
                    apdu_log: Vec::new(),
                })
            }
            TransportKind::Qrtr => {
                let slot = u8::try_from(reader.slot_index.unwrap_or(0)).unwrap_or(0);
                client.qrtr_open_bus()?;
                let mut session = ModemSession::new(Box::new(QrtrModemTransport {
                    host: Arc::clone(&host),
                }));
                let aid_bytes = aids
                    .first()
                    .and_then(|h| hex_to_bytes(h).ok())
                    .unwrap_or_else(|| ISDR_AID.to_vec());
                let ch = runtime::block_on(session.open_channel(slot, aid_bytes)).map_err(|e| {
                    let _ = client.qrtr_close_bus();
                    ReaderError::Protocol(e)
                })?;
                Ok(Self {
                    host,
                    backend: Some(ChannelBackend::Qrtr {
                        slot,
                        channel: ch,
                        session: Box::new(session),
                    }),
                    mss: effective_mss,
                    select_response: Vec::new(),
                    omapi_latch: Arc::clone(omapi_latch),
                    apdu_log: Vec::new(),
                })
            }
            TransportKind::SeHal => {
                let slot = reader.slot_index.unwrap_or(1);
                if se_hal_borrowed {
                    client.se_hal_use_borrowed(slot)?;
                } else {
                    client.se_hal_open_owned(slot)?;
                }
                Ok(Self {
                    host,
                    backend: Some(ChannelBackend::SeHal {
                        slot,
                        borrowed: se_hal_borrowed,
                    }),
                    mss: effective_mss,
                    select_response: Vec::new(),
                    omapi_latch: Arc::clone(omapi_latch),
                    apdu_log: Vec::new(),
                })
            }
            TransportKind::Ccid => {
                if reader.needs_permission && !client.ccid_request_permission(&reader.raw_name)? {
                    return Err(ReaderError::PermissionRequired(reader.label.clone()));
                }
                client.ccid_connect(&reader.raw_name)?;
                // Try MANAGE CHANNEL OPEN; fallback to basic channel 0 if reader restricts logical channels.
                let open_hex = bytes_to_hex(&nlpa_core::channel::build_manage_channel_open_apdu());
                let open_resp_hex = client.ccid_transceive(&reader.raw_name, &open_hex)?;
                let open_resp = hex_to_bytes(&open_resp_hex).unwrap_or_default();
                let ch = if open_resp.len() >= 3
                    && validate_apdu_sw(
                        open_resp[open_resp.len() - 2],
                        open_resp[open_resp.len() - 1],
                    )
                    .is_ok()
                {
                    open_resp[0]
                } else {
                    0
                };
                let mut select_apdu = build_select_isdr_apdu();
                if ch > 0 {
                    select_apdu[0] = map_logical_channel_cla(select_apdu[0], ch)
                        .map_err(|e| ReaderError::Protocol(e.to_string()))?;
                }
                let sel_hex = client.ccid_transceive(&reader.raw_name, &bytes_to_hex(&select_apdu))?;
                let sel_bytes = hex_to_bytes(&sel_hex)
                    .map_err(|e| ReaderError::Protocol(e.to_string()))?;
                Ok(Self {
                    host,
                    backend: Some(ChannelBackend::Ccid {
                        reader: reader.raw_name.clone(),
                        channel: ch,
                    }),
                    mss: effective_mss,
                    select_response: sel_bytes,
                    omapi_latch: Arc::clone(omapi_latch),
                    apdu_log: Vec::new(),
                })
            }
            TransportKind::Remote => {
                let server = reader.remocard_server.clone().ok_or_else(|| {
                    ReaderError::ChannelUnavailable("Missing RemoCard server config".into())
                })?;
                let req_json = build_open_channel_request(&reader.raw_name, aids);
                let resp_body = post_remocard_json(
                    &client,
                    &server,
                    "openChannel",
                    &req_json,
                    proxy_url.as_deref(),
                )?;
                let selected_aid = parse_open_channel_response(&resp_body, Some(&server))
                    .map_err(|e| ReaderError::ChannelUnavailable(e.to_string()))?;
                Ok(Self {
                    host,
                    backend: Some(ChannelBackend::Remote {
                        server,
                        reader_name: reader.raw_name.clone(),
                        aid: selected_aid,
                        proxy_url,
                    }),
                    mss: effective_mss,
                    select_response: Vec::new(),
                    omapi_latch: Arc::clone(omapi_latch),
                    apdu_log: Vec::new(),
                })
            }
        }
    }

    /// Returns the `SELECT ISD-R` response FCI bytes if reported on open.
    #[must_use]
    pub fn select_response(&self) -> &[u8] {
        &self.select_response
    }

    /// Returns `true` when operating over the specialized SE HAL transport.
    #[must_use]
    pub fn is_se_hal(&self) -> bool {
        matches!(self.backend, Some(ChannelBackend::SeHal { .. }))
    }

    /// Returns the SE HAL slot index if this channel is [`TransportKind::SeHal`].
    #[must_use]
    pub fn se_hal_slot(&self) -> Option<i32> {
        match self.backend {
            Some(ChannelBackend::SeHal { slot, .. }) => Some(slot),
            _ => None,
        }
    }

    /// Transmits an SGP.22 ES10 TLV command (`STORE DATA`), segmenting when exceeding MSS.
    pub fn transmit_es10_tlv(&mut self, tlv_der: &[u8]) -> Result<Vec<u8>, ReaderError> {
        if let Some(ChannelBackend::SeHal { slot, .. }) = &self.backend {
            let client = HostClient::new(self.host.as_ref());
            let full_apdu = nlpa_core::lpa::es10::wrap_store_data_apdu(tlv_der, true, true);
            let req_hex = bytes_to_hex(&full_apdu);
            let resp_hex = client.se_hal_read(*slot, &req_hex)?;
            self.apdu_log.push((req_hex, resp_hex.clone()));
            let raw = hex_to_bytes(&resp_hex).map_err(|e| ReaderError::Protocol(e.to_string()))?;
            return unwrap_se_hal_or_sw(&raw);
        }

        let frames = if tlv_der.len() <= self.mss {
            vec![nlpa_core::lpa::es10::wrap_store_data_apdu(tlv_der, true, true)]
        } else {
            let mut pkts =
                chain_apdu_request(0x80, 0xE2, 0x91, 0x00, tlv_der, None, self.mss);
            if let Some(last) = pkts.last_mut() {
                last.push(0x00);
            }
            pkts
        };

        let mut last_response = Vec::new();
        for frame in frames {
            last_response = self.transmit_raw(&frame)?;
        }
        strip_and_validate_sw(&last_response)
    }

    /// Transmits an arbitrary ISO 7816-4 APDU, automatically handling channel CLA and `61xx`/`6Cxx`.
    pub fn transmit_raw(&mut self, apdu: &[u8]) -> Result<Vec<u8>, ReaderError> {
        let mut current = apdu.to_vec();
        if let Some(ch) = self.raw_cla_channel() {
            if !current.is_empty() && ch > 0 {
                current[0] = map_logical_channel_cla(current[0], ch)
                    .map_err(|e| ReaderError::Protocol(e.to_string()))?;
            }
        }

        let mut reassembler = ResponseReassembler::default();
        loop {
            let chunk = self.transmit_single_apdu(&current)?;
            match reassembler
                .process_chunk(&current, &chunk)
                .map_err(|e| ReaderError::Protocol(e.to_string()))?
            {
                ReassemblyAction::Complete(full) => return Ok(full),
                ReassemblyAction::SendGetResponse(mut get_resp) => {
                    if let Some(ch) = self.raw_cla_channel() {
                        if ch > 0 {
                            get_resp[0] = map_logical_channel_cla(get_resp[0], ch)
                                .map_err(|e| ReaderError::Protocol(e.to_string()))?;
                        }
                    }
                    current = get_resp.to_vec();
                }
                ReassemblyAction::ReissueWithLe(le) => {
                    if current.len() >= 5 {
                        if let Some(last) = current.last_mut() {
                            *last = le;
                        }
                    }
                }
            }
        }
    }

    fn raw_cla_channel(&self) -> Option<u8> {
        match self.backend {
            Some(ChannelBackend::Telephony { channel, .. }) => u8::try_from(channel).ok(),
            Some(ChannelBackend::Ccid { channel, .. }) => Some(channel),
            _ => None,
        }
    }

    fn transmit_single_apdu(&mut self, apdu: &[u8]) -> Result<Vec<u8>, ReaderError> {
        let tx_hex = bytes_to_hex(apdu);
        let client = HostClient::new(self.host.as_ref());

        let rx_hex = match &mut self.backend {
            Some(ChannelBackend::Omapi { reader, aid }) => {
                if let Ok(guard) = self.omapi_latch.lock() {
                    guard
                        .check_safety()
                        .map_err(|e| ReaderError::OmapiCorrupted(e.to_string()))?;
                }
                match client.omapi_transmit_on_channel(reader, aid, &tx_hex) {
                    Ok(h) => h,
                    Err(e) => {
                        if e.is_omapi_session_corrupted() {
                            if let Ok(mut guard) = self.omapi_latch.lock() {
                                guard.poison(&e.message);
                            }
                        }
                        return Err(ReaderError::from(e));
                    }
                }
            }
            Some(ChannelBackend::Telephony {
                slot_index,
                port_index,
                channel,
            }) => client.telephony_transmit(*slot_index, *port_index, *channel, &tx_hex)?,
            Some(ChannelBackend::NBridge { connection_id }) => {
                client.nbridge_transmit_logical(connection_id, &tx_hex)?
            }
            Some(ChannelBackend::Adb { connection_id }) => {
                client.adb_transmit_logical(connection_id, &tx_hex)?
            }
            Some(ChannelBackend::Qrtr {
                slot,
                channel,
                session,
            }) => {
                let raw = runtime::block_on(session.transmit(*slot, *channel, apdu.to_vec()))
                    .map_err(ReaderError::Protocol)?;
                bytes_to_hex(&raw)
            }
            Some(ChannelBackend::SeHal { slot, .. }) => client.se_hal_read(*slot, &tx_hex)?,
            Some(ChannelBackend::Ccid { reader, .. }) => {
                client.ccid_transceive(reader, &tx_hex)?
            }
            Some(ChannelBackend::Remote {
                server,
                reader_name,
                aid,
                proxy_url,
            }) => {
                let req_json = build_send_apdu_request(reader_name, aid, &tx_hex);
                let resp_body = post_remocard_json(
                    &client,
                    server,
                    "sendApdu",
                    &req_json,
                    proxy_url.as_deref(),
                )?;
                let bytes = parse_send_apdu_response(&resp_body, Some(server))
                    .map_err(|e| ReaderError::Protocol(e.to_string()))?;
                bytes_to_hex(&bytes)
            }
            None => {
                return Err(ReaderError::ChannelUnavailable(
                    "Channel already closed".into(),
                ));
            }
        };

        self.apdu_log.push((tx_hex, rx_hex.clone()));
        hex_to_bytes(&rx_hex).map_err(|e| ReaderError::Protocol(e.to_string()))
    }

    /// Closes the underlying logical channel and disconnects cleanly.
    pub fn close(mut self) -> Result<(), ReaderError> {
        self.close_inner()
    }

    fn close_inner(&mut self) -> Result<(), ReaderError> {
        let Some(backend) = self.backend.take() else {
            return Ok(());
        };
        let client = HostClient::new(self.host.as_ref());
        match backend {
            ChannelBackend::Omapi { reader, aid } => {
                let _ = client.omapi_close_channel(&reader, &aid);
                let _ = client.omapi_disconnect(&reader);
            }
            ChannelBackend::Telephony {
                slot_index,
                port_index,
                channel,
            } => {
                let _ = client.telephony_close_channel(slot_index, port_index, channel);
            }
            ChannelBackend::NBridge { connection_id } => {
                let _ = client.nbridge_close_logical_channel(&connection_id);
            }
            ChannelBackend::Adb { connection_id } => {
                let _ = client.adb_close_logical_channel(&connection_id);
            }
            ChannelBackend::Qrtr {
                slot,
                channel,
                mut session,
            } => {
                let _ = runtime::block_on(session.close_channel(slot, channel));
                let _ = client.qrtr_close_bus();
            }
            ChannelBackend::SeHal { slot, borrowed } => {
                if !borrowed {
                    let _ = client.se_hal_close_owned(slot);
                }
            }
            ChannelBackend::Ccid { reader, channel } => {
                if channel > 0 {
                    if let Ok(close_apdu) = nlpa_core::channel::build_manage_channel_close_apdu(channel) {
                        let _ = client.ccid_transceive(&reader, &bytes_to_hex(&close_apdu));
                    }
                }
                let _ = client.ccid_disconnect(&reader);
            }
            ChannelBackend::Remote {
                server,
                reader_name,
                aid,
                proxy_url,
            } => {
                let req_json = build_close_channel_request(&reader_name, &[aid]);
                let _ = post_remocard_json(
                    &client,
                    &server,
                    "closeChannel",
                    &req_json,
                    proxy_url.as_deref(),
                );
            }
        }
        Ok(())
    }
}

fn post_remocard_json(
    client: &HostClient<'_>,
    server: &RemoCardServer,
    endpoint: &str,
    plaintext_json: &str,
    proxy_url: Option<&str>,
) -> Result<String, ReaderError> {
    let mut headers = BTreeMap::new();
    for (k, v) in build_remocard_headers(server) {
        headers.insert(k, v);
    }
    let payload_str = match resolve_remocard_key(server) {
        Some(key) => encrypt_remocard_payload(plaintext_json, &key),
        None => plaintext_json.to_string(),
    };
    let (clean_url, _) = clean_remocard_url(&server.base_url);
    let url = format!("{}/{endpoint}", clean_url.trim_end_matches('/'));
    let resp = client.http_request(&HttpRequestArgs {
        method: "POST".into(),
        url,
        headers,
        body_base64: Some(base64_encode(payload_str.as_bytes())),
        proxy_url: proxy_url.map(ToString::to_string),
        timeout_ms: 15_000,
        insecure_tls: false,
    })?;
    let bytes =
        base64_decode(&resp.body_base64).map_err(|e| ReaderError::Protocol(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| ReaderError::Protocol(e.to_string()))
}

impl Drop for CardChannel {
    fn drop(&mut self) {
        let _ = self.close_inner();
    }
}

fn strip_and_validate_sw(raw: &[u8]) -> Result<Vec<u8>, ReaderError> {
    if raw.len() < 2 {
        return Err(ReaderError::ApduFailed {
            sw1: 0x6F,
            sw2: 0x00,
        });
    }
    let sw1 = raw[raw.len() - 2];
    let sw2 = raw[raw.len() - 1];
    if let Err(_e) = validate_apdu_sw(sw1, sw2) {
        return Err(ReaderError::ApduFailed { sw1, sw2 });
    }
    Ok(raw[..raw.len() - 2].to_vec())
}

fn unwrap_se_hal_or_sw(raw: &[u8]) -> Result<Vec<u8>, ReaderError> {
    if raw.len() >= 2 {
        let sw1 = raw[raw.len() - 2];
        let sw2 = raw[raw.len() - 1];
        if (sw1 == 0x90 && sw2 == 0x00) || sw1 == 0x91 {
            return Ok(raw[..raw.len() - 2].to_vec());
        }
        if matches!(sw1, 0x61..=0x6F) && raw.len() == 2 {
            return Err(ReaderError::ApduFailed { sw1, sw2 });
        }
    }
    Ok(raw.to_vec())
}
