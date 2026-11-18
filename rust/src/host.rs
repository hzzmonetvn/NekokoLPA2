//! Host bridge abstraction (`NativeHostBridge.callHost`) and typed RPC wrappers.

use core::fmt;
use std::collections::BTreeMap;
use std::error::Error;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Error returned by the Android `NativeHostBridge` or mock host transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCallError {
    pub code: String,
    pub message: String,
}

impl HostCallError {
    /// Constructs a new [`HostCallError`].
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Returns `true` when Android OMAPI signaled a corrupted/wedged Secure Element session.
    #[must_use]
    pub fn is_omapi_session_corrupted(&self) -> bool {
        self.code == "OMAPI_SESSION_CORRUPTED" || self.message.contains("OMAPI_SESSION_CORRUPTED")
    }
}

impl fmt::Display for HostCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.code.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "[{}] {}", self.code, self.message)
        }
    }
}

impl Error for HostCallError {}

#[derive(Deserialize)]
struct RawHostResponse {
    ok: bool,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// Abstract synchronous host bridge matching Kotlin `NativeHostBridge`.
pub trait HostTransport: Send + Sync {
    /// Sends `{"op": "...", "args": {...}}` JSON to the host and returns raw response JSON.
    fn call_host_raw(&self, request_json: &str) -> Result<String, HostCallError>;

    /// Notifies the host that persisted state or background status updated asynchronously.
    fn notify_state_updated(&self) {}

    /// Invokes a structured host operation and extracts `data` or structured error.
    fn call_op(&self, op: &str, args: Value) -> Result<Value, HostCallError> {
        let req = json!({
            "op": op,
            "args": args,
        });
        let req_str = serde_json::to_string(&req)
            .map_err(|e| HostCallError::new("SERIALIZE_ERROR", e.to_string()))?;
        let raw = self.call_host_raw(&req_str)?;
        let parsed: RawHostResponse = serde_json::from_str(&raw)
            .map_err(|e| HostCallError::new("INVALID_HOST_JSON", e.to_string()))?;
        if parsed.ok {
            Ok(parsed.data)
        } else {
            Err(HostCallError::new(
                parsed.code.unwrap_or_else(|| "HOST_ERROR".to_string()),
                parsed
                    .message
                    .unwrap_or_else(|| "Host operation failed".to_string()),
            ))
        }
    }
}

/// Result of `omapi.connect`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmapiConnectResult {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub atr: String,
}

/// Result of `omapi.openChannel`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmapiOpenChannelResult {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub aid: String,
    #[serde(default)]
    pub select_response: Option<String>,
}

/// Logical port descriptor inside [`TelephonySlot`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelephonyPort {
    #[serde(default)]
    pub port_index: i32,
    #[serde(default)]
    pub logical_slot_index: i32,
}

/// Slot descriptor returned by `telephony.listSlots`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelephonySlot {
    #[serde(default)]
    pub slot_index: i32,
    #[serde(default)]
    pub is_euicc: bool,
    #[serde(default)]
    pub card_id: String,
    #[serde(default)]
    pub is_removable: bool,
    #[serde(default)]
    pub ports: Vec<TelephonyPort>,
    #[serde(default)]
    pub sim_state: i32,
}

/// Channel open response from `telephony.openChannel`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelephonyOpenChannelResult {
    #[serde(default)]
    pub channel: i32,
    #[serde(default)]
    pub select_response: Option<String>,
}

/// Slot returned by `nbridge.listSlots` / `adb.listSlots`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeSlot {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub display_name: String,
}

/// Channel response from `nbridge.connectLogicalChannel` / `adb.connectLogicalChannel`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeLogicalChannel {
    #[serde(default)]
    pub connection_id: String,
    #[serde(default)]
    pub selected_aid: String,
    #[serde(default)]
    pub channel_number: i32,
    #[serde(default)]
    pub response_apdu: String,
}

/// Received QRTR packet from `qrtr.receive`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrtrDatagram {
    #[serde(default)]
    pub node: u32,
    #[serde(default)]
    pub port: u32,
    #[serde(default)]
    pub payload_hex: String,
}

/// Host certificate hashes & supported ABIs returned by `cert.getInfo`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertInfoResult {
    #[serde(default)]
    pub sha256: Vec<String>,
    #[serde(default)]
    pub sha1: Vec<String>,
    #[serde(default)]
    pub aram_sha256: Vec<String>,
    #[serde(default)]
    pub aram_sha1: Vec<String>,
    #[serde(default)]
    pub abis: Vec<String>,
}

/// Request arguments passed to `http.request`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestArgs {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    pub timeout_ms: u64,
    pub insecure_tls: bool,
}

/// Response returned by `http.request`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseData {
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub set_cookies: Vec<String>,
    #[serde(default)]
    pub body_base64: String,
}

/// Typed host bridge client wrapping a [`HostTransport`].
pub struct HostClient<'a> {
    transport: &'a dyn HostTransport,
}

impl<'a> HostClient<'a> {
    /// Creates a typed wrapper around a [`HostTransport`].
    #[must_use]
    pub fn new(transport: &'a dyn HostTransport) -> Self {
        Self { transport }
    }

    pub fn omapi_list_readers(&self) -> Result<Vec<String>, HostCallError> {
        let v = self.transport.call_op("omapi.listReaders", json!({}))?;
        Ok(parse_string_vec(&v))
    }

    pub fn omapi_connect(&self, reader: &str) -> Result<OmapiConnectResult, HostCallError> {
        let v = self
            .transport
            .call_op("omapi.connect", json!({ "reader": reader }))?;
        serde_json::from_value(v).map_err(|e| HostCallError::new("DECODE_ERROR", e.to_string()))
    }

    pub fn omapi_disconnect(&self, reader: &str) -> Result<(), HostCallError> {
        self.transport
            .call_op("omapi.disconnect", json!({ "reader": reader }))?;
        Ok(())
    }

    pub fn omapi_reset(&self) -> Result<(), HostCallError> {
        self.transport.call_op("omapi.reset", json!({}))?;
        Ok(())
    }

    pub fn omapi_open_channel(
        &self,
        reader: &str,
        aids: &[String],
    ) -> Result<OmapiOpenChannelResult, HostCallError> {
        let v = self.transport.call_op(
            "omapi.openChannel",
            json!({ "reader": reader, "aids": aids }),
        )?;
        serde_json::from_value(v).map_err(|e| HostCallError::new("DECODE_ERROR", e.to_string()))
    }

    pub fn omapi_close_channel(&self, reader: &str, aid: &str) -> Result<(), HostCallError> {
        self.transport.call_op(
            "omapi.closeChannel",
            json!({ "reader": reader, "aid": aid }),
        )?;
        Ok(())
    }

    pub fn omapi_close_channels(&self, reader: &str) -> Result<(), HostCallError> {
        self.transport
            .call_op("omapi.closeChannels", json!({ "reader": reader }))?;
        Ok(())
    }

    pub fn omapi_transmit(&self, reader: &str, apdu: &str) -> Result<String, HostCallError> {
        let v = self
            .transport
            .call_op("omapi.transmit", json!({ "reader": reader, "apdu": apdu }))?;
        parse_hex_response(&v)
    }

    pub fn omapi_transmit_on_channel(
        &self,
        reader: &str,
        aid: &str,
        apdu: &str,
    ) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "omapi.transmitOnChannel",
            json!({ "reader": reader, "aid": aid, "apdu": apdu }),
        )?;
        parse_hex_response(&v)
    }

    pub fn omapi_set_profile_switch_in_progress(&self, value: bool) -> Result<(), HostCallError> {
        self.transport.call_op(
            "omapi.setProfileSwitchInProgress",
            json!({ "value": value }),
        )?;
        Ok(())
    }

    pub fn telephony_has_privileged_permission(&self) -> Result<bool, HostCallError> {
        let v = self
            .transport
            .call_op("telephony.hasPrivilegedPermission", json!({}))?;
        Ok(parse_bool(&v, &["granted", "hasPermission", "privileged"]))
    }

    pub fn telephony_list_slots(&self) -> Result<Vec<TelephonySlot>, HostCallError> {
        let v = self.transport.call_op("telephony.listSlots", json!({}))?;
        serde_json::from_value(v).map_err(|e| HostCallError::new("DECODE_ERROR", e.to_string()))
    }

    pub fn telephony_open_channel(
        &self,
        slot_index: i32,
        port_index: i32,
        aid: &str,
    ) -> Result<TelephonyOpenChannelResult, HostCallError> {
        let v = self.transport.call_op(
            "telephony.openChannel",
            json!({ "slotIndex": slot_index, "portIndex": port_index, "aid": aid }),
        )?;
        serde_json::from_value(v).map_err(|e| HostCallError::new("DECODE_ERROR", e.to_string()))
    }

    pub fn telephony_transmit(
        &self,
        slot_index: i32,
        port_index: i32,
        channel: i32,
        apdu: &str,
    ) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "telephony.transmit",
            json!({
                "slotIndex": slot_index,
                "portIndex": port_index,
                "channel": channel,
                "apdu": apdu,
            }),
        )?;
        parse_hex_response(&v)
    }

    pub fn telephony_close_channel(
        &self,
        slot_index: i32,
        port_index: i32,
        channel: i32,
    ) -> Result<(), HostCallError> {
        self.transport.call_op(
            "telephony.closeChannel",
            json!({ "slotIndex": slot_index, "portIndex": port_index, "channel": channel }),
        )?;
        Ok(())
    }

    pub fn telephony_cleanup_all_channels(
        &self,
        slot_index: i32,
        port_index: i32,
    ) -> Result<(), HostCallError> {
        self.transport.call_op(
            "telephony.cleanupAllChannels",
            json!({ "slotIndex": slot_index, "portIndex": port_index }),
        )?;
        Ok(())
    }

    pub fn telephony_send_terminal_capabilities(
        &self,
        slot_index: i32,
        port_index: i32,
    ) -> Result<(), HostCallError> {
        self.transport.call_op(
            "telephony.sendTerminalCapabilities",
            json!({ "slotIndex": slot_index, "portIndex": port_index }),
        )?;
        Ok(())
    }

    pub fn nbridge_is_available(&self) -> Result<bool, HostCallError> {
        let v = self.transport.call_op("nbridge.isAvailable", json!({}))?;
        Ok(parse_bool(&v, &["available"]))
    }

    pub fn nbridge_is_rooted(&self) -> Result<bool, HostCallError> {
        let v = self.transport.call_op("nbridge.isRooted", json!({}))?;
        Ok(parse_bool(&v, &["rooted"]))
    }

    pub fn nbridge_list_slots(&self) -> Result<Vec<BridgeSlot>, HostCallError> {
        let v = self.transport.call_op("nbridge.listSlots", json!({}))?;
        serde_json::from_value(v).map_err(|e| HostCallError::new("DECODE_ERROR", e.to_string()))
    }

    pub fn nbridge_connect_logical_channel(
        &self,
        slot_id: &str,
        aids: &[String],
    ) -> Result<BridgeLogicalChannel, HostCallError> {
        let v = self.transport.call_op(
            "nbridge.connectLogicalChannel",
            json!({ "slotId": slot_id, "aids": aids }),
        )?;
        serde_json::from_value(v).map_err(|e| HostCallError::new("DECODE_ERROR", e.to_string()))
    }

    pub fn nbridge_transmit_logical(
        &self,
        connection_id: &str,
        apdu: &str,
    ) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "nbridge.transmitLogical",
            json!({ "connectionId": connection_id, "apdu": apdu }),
        )?;
        parse_hex_response(&v)
    }

    pub fn nbridge_transmit_basic(
        &self,
        slot_id: &str,
        apdu: &str,
    ) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "nbridge.transmitBasic",
            json!({ "slotId": slot_id, "apdu": apdu }),
        )?;
        parse_hex_response(&v)
    }

    pub fn nbridge_close_logical_channel(&self, connection_id: &str) -> Result<(), HostCallError> {
        self.transport.call_op(
            "nbridge.closeLogicalChannel",
            json!({ "connectionId": connection_id }),
        )?;
        Ok(())
    }

    pub fn adb_get_connect_command(&self) -> Result<String, HostCallError> {
        let v = self.transport.call_op("adb.getConnectCommand", json!({}))?;
        Ok(v.as_str()
            .or_else(|| v.get("command").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string())
    }

    pub fn adb_is_connected(&self) -> Result<bool, HostCallError> {
        let v = self.transport.call_op("adb.isConnected", json!({}))?;
        Ok(parse_bool(&v, &["connected"]))
    }

    pub fn adb_begin_connect(&self, timeout_ms: u64) -> Result<(), HostCallError> {
        self.transport
            .call_op("adb.beginConnect", json!({ "timeoutMs": timeout_ms }))?;
        Ok(())
    }

    pub fn adb_cancel_connect(&self) -> Result<(), HostCallError> {
        self.transport.call_op("adb.cancelConnect", json!({}))?;
        Ok(())
    }

    pub fn adb_disconnect(&self) -> Result<(), HostCallError> {
        self.transport.call_op("adb.disconnect", json!({}))?;
        Ok(())
    }

    pub fn adb_list_slots(&self) -> Result<Vec<BridgeSlot>, HostCallError> {
        let v = self.transport.call_op("adb.listSlots", json!({}))?;
        serde_json::from_value(v).map_err(|e| HostCallError::new("DECODE_ERROR", e.to_string()))
    }

    pub fn adb_connect_logical_channel(
        &self,
        slot_id: &str,
        aids: &[String],
    ) -> Result<BridgeLogicalChannel, HostCallError> {
        let v = self.transport.call_op(
            "adb.connectLogicalChannel",
            json!({ "slotId": slot_id, "aids": aids }),
        )?;
        serde_json::from_value(v).map_err(|e| HostCallError::new("DECODE_ERROR", e.to_string()))
    }

    pub fn adb_transmit_logical(
        &self,
        connection_id: &str,
        apdu: &str,
    ) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "adb.transmitLogical",
            json!({ "connectionId": connection_id, "apdu": apdu }),
        )?;
        parse_hex_response(&v)
    }

    pub fn adb_transmit_basic(&self, slot_id: &str, apdu: &str) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "adb.transmitBasic",
            json!({ "slotId": slot_id, "apdu": apdu }),
        )?;
        parse_hex_response(&v)
    }

    pub fn adb_close_logical_channel(&self, connection_id: &str) -> Result<(), HostCallError> {
        self.transport.call_op(
            "adb.closeLogicalChannel",
            json!({ "connectionId": connection_id }),
        )?;
        Ok(())
    }

    pub fn qrtr_is_supported(&self) -> Result<bool, HostCallError> {
        let v = self.transport.call_op("qrtr.isSupported", json!({}))?;
        Ok(parse_bool(&v, &["supported"]))
    }

    pub fn qrtr_is_available(&self) -> Result<bool, HostCallError> {
        let v = self.transport.call_op("qrtr.isAvailable", json!({}))?;
        Ok(parse_bool(&v, &["available"]))
    }

    pub fn qrtr_has_permission(&self) -> Result<bool, HostCallError> {
        let v = self.transport.call_op("qrtr.hasPermission", json!({}))?;
        Ok(parse_bool(&v, &["granted", "hasPermission"]))
    }

    pub fn qrtr_request_permission(&self) -> Result<bool, HostCallError> {
        let v = self
            .transport
            .call_op("qrtr.requestPermission", json!({}))?;
        Ok(parse_bool(&v, &["granted"]))
    }

    pub fn qrtr_open_bus(&self) -> Result<i64, HostCallError> {
        let v = self.transport.call_op("qrtr.openBus", json!({}))?;
        Ok(parse_i64(&v, &["fd", "bus", "node"]).unwrap_or(0))
    }

    pub fn qrtr_bus_node(&self) -> Result<u32, HostCallError> {
        let v = self.transport.call_op("qrtr.busNode", json!({}))?;
        Ok(parse_i64(&v, &["node"]).unwrap_or(0) as u32)
    }

    pub fn qrtr_send(&self, node: u32, port: u32, payload_hex: &str) -> Result<(), HostCallError> {
        self.transport.call_op(
            "qrtr.send",
            json!({ "node": node, "port": port, "payloadHex": payload_hex }),
        )?;
        Ok(())
    }

    pub fn qrtr_receive(&self, timeout_ms: u64) -> Result<Option<QrtrDatagram>, HostCallError> {
        let v = self
            .transport
            .call_op("qrtr.receive", json!({ "timeoutMs": timeout_ms }))?;
        if v.is_null() {
            return Ok(None);
        }
        let pkt: QrtrDatagram = serde_json::from_value(v)
            .map_err(|e| HostCallError::new("DECODE_ERROR", e.to_string()))?;
        Ok(Some(pkt))
    }

    pub fn qrtr_close_bus(&self) -> Result<(), HostCallError> {
        self.transport.call_op("qrtr.closeBus", json!({}))?;
        Ok(())
    }

    pub fn se_hal_invalidate_sessions(&self) -> Result<(), HostCallError> {
        self.transport
            .call_op("seHal.invalidateSessions", json!({}))?;
        Ok(())
    }

    pub fn se_hal_use_borrowed(&self, slot: i32) -> Result<(), HostCallError> {
        self.transport
            .call_op("seHal.useBorrowed", json!({ "slot": slot }))?;
        Ok(())
    }

    pub fn se_hal_open_owned(&self, slot: i32) -> Result<(), HostCallError> {
        self.transport
            .call_op("seHal.openOwned", json!({ "slot": slot }))?;
        Ok(())
    }

    pub fn se_hal_close_owned(&self, slot: i32) -> Result<(), HostCallError> {
        self.transport
            .call_op("seHal.closeOwned", json!({ "slot": slot }))?;
        Ok(())
    }

    pub fn se_hal_atr(&self, slot: i32) -> Result<String, HostCallError> {
        let v = self
            .transport
            .call_op("seHal.atr", json!({ "slot": slot }))?;
        parse_hex_response(&v)
    }

    pub fn se_hal_read(&self, slot: i32, apdu_hex: &str) -> Result<String, HostCallError> {
        let v = self
            .transport
            .call_op("seHal.read", json!({ "slot": slot, "apduHex": apdu_hex }))?;
        parse_hex_response(&v)
    }

    pub fn se_hal_begin_download(&self, slot: i32) -> Result<(), HostCallError> {
        self.transport
            .call_op("seHal.beginDownload", json!({ "slot": slot }))?;
        Ok(())
    }

    pub fn se_hal_end_download(&self, slot: i32) -> Result<(), HostCallError> {
        self.transport
            .call_op("seHal.endDownload", json!({ "slot": slot }))?;
        Ok(())
    }

    pub fn se_hal_download(
        &self,
        slot: i32,
        segments_hex: &[String],
    ) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "seHal.download",
            json!({ "slot": slot, "segmentsHex": segments_hex }),
        )?;
        parse_hex_response(&v)
    }

    pub fn se_hal_switch_profile(
        &self,
        slot: i32,
        iccid_hex: &str,
        enable: bool,
        eid_hex: Option<&str>,
    ) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "seHal.switchProfile",
            json!({
                "slot": slot,
                "iccidHex": iccid_hex,
                "enable": enable,
                "eidHex": eid_hex,
            }),
        )?;
        parse_hex_response(&v)
    }

    pub fn se_hal_rename_profile(
        &self,
        slot: i32,
        iccid_hex: &str,
        nickname: &str,
        eid_hex: Option<&str>,
    ) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "seHal.renameProfile",
            json!({
                "slot": slot,
                "iccidHex": iccid_hex,
                "nickname": nickname,
                "eidHex": eid_hex,
            }),
        )?;
        parse_hex_response(&v)
    }

    pub fn se_hal_delete_profile(
        &self,
        slot: i32,
        iccid_hex: &str,
        eid_hex: Option<&str>,
    ) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "seHal.deleteProfile",
            json!({
                "slot": slot,
                "iccidHex": iccid_hex,
                "eidHex": eid_hex,
            }),
        )?;
        parse_hex_response(&v)
    }

    pub fn se_hal_remove_notification(
        &self,
        slot: i32,
        seq_number: u32,
        eid_hex: Option<&str>,
    ) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "seHal.removeNotification",
            json!({
                "slot": slot,
                "seqNumber": seq_number,
                "eidHex": eid_hex,
            }),
        )?;
        parse_hex_response(&v)
    }

    pub fn ccid_list_readers(&self) -> Result<Vec<String>, HostCallError> {
        let v = self.transport.call_op("ccid.listReaders", json!({}))?;
        Ok(parse_string_vec(&v))
    }

    pub fn ccid_connect(&self, reader: &str) -> Result<String, HostCallError> {
        let v = self
            .transport
            .call_op("ccid.connect", json!({ "reader": reader }))?;
        parse_hex_response(&v)
    }

    pub fn ccid_request_permission(&self, reader: &str) -> Result<bool, HostCallError> {
        let v = self
            .transport
            .call_op("ccid.requestPermission", json!({ "reader": reader }))?;
        Ok(parse_bool(&v, &["granted"]))
    }

    pub fn ccid_transceive(&self, reader: &str, apdu_hex: &str) -> Result<String, HostCallError> {
        let v = self.transport.call_op(
            "ccid.transceive",
            json!({ "reader": reader, "apduHex": apdu_hex }),
        )?;
        parse_hex_response(&v)
    }

    pub fn ccid_power_cycle(&self, reader: &str) -> Result<String, HostCallError> {
        let v = self
            .transport
            .call_op("ccid.powerCycle", json!({ "reader": reader }))?;
        parse_hex_response(&v)
    }

    pub fn ccid_disconnect(&self, reader: &str) -> Result<(), HostCallError> {
        self.transport
            .call_op("ccid.disconnect", json!({ "reader": reader }))?;
        Ok(())
    }

    pub fn cert_get_info(&self) -> Result<CertInfoResult, HostCallError> {
        let v = self.transport.call_op("cert.getInfo", json!({}))?;
        serde_json::from_value(v).map_err(|e| HostCallError::new("DECODE_ERROR", e.to_string()))
    }

    pub fn http_request(&self, args: &HttpRequestArgs) -> Result<HttpResponseData, HostCallError> {
        let v = self.transport.call_op(
            "http.request",
            serde_json::to_value(args)
                .map_err(|e| HostCallError::new("SERIALIZE_ERROR", e.to_string()))?,
        )?;
        serde_json::from_value(v).map_err(|e| HostCallError::new("DECODE_ERROR", e.to_string()))
    }

    pub fn ui_copy_clipboard(&self, text: &str) -> Result<(), HostCallError> {
        self.transport
            .call_op("ui.copyClipboard", json!({ "text": text }))?;
        Ok(())
    }

    pub fn ui_read_clipboard(&self) -> Result<String, HostCallError> {
        let v = self.transport.call_op("ui.readClipboard", json!({}))?;
        Ok(v.get("text")
            .and_then(Value::as_str)
            .or_else(|| v.as_str())
            .unwrap_or_default()
            .to_string())
    }

    pub fn ui_open_url(&self, url: &str) -> Result<(), HostCallError> {
        self.transport
            .call_op("ui.openUrl", json!({ "url": url }))?;
        Ok(())
    }

    pub fn ui_pick_wallpaper(&self) -> Result<(), HostCallError> {
        self.transport.call_op("ui.pickWallpaper", json!({}))?;
        Ok(())
    }

    pub fn ui_clear_wallpaper(&self) -> Result<(), HostCallError> {
        self.transport.call_op("ui.clearWallpaper", json!({}))?;
        Ok(())
    }

    pub fn ui_export_state(&self, state_json: &str) -> Result<(), HostCallError> {
        self.transport
            .call_op("ui.exportState", json!({ "json": state_json }))?;
        Ok(())
    }

    pub fn ui_import_state(&self) -> Result<(), HostCallError> {
        self.transport.call_op("ui.importState", json!({}))?;
        Ok(())
    }

    pub fn ui_open_websheet(&self, url: &str, post_data: &str) -> Result<(), HostCallError> {
        self.transport.call_op(
            "ui.openWebsheet",
            json!({ "url": url, "postData": post_data }),
        )?;
        Ok(())
    }

    pub fn ui_scan_qr(&self) -> Result<(), HostCallError> {
        self.transport.call_op("ui.scanQr", json!({}))?;
        Ok(())
    }
}

fn parse_string_vec(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_hex_response(value: &Value) -> Result<String, HostCallError> {
    if let Some(s) = value.as_str() {
        return Ok(s.to_string());
    }
    for key in ["apdu", "hex", "response", "atr", "data"] {
        if let Some(s) = value.get(key).and_then(Value::as_str) {
            return Ok(s.to_string());
        }
    }
    if value.is_null() {
        return Ok(String::new());
    }
    Err(HostCallError::new(
        "DECODE_ERROR",
        "Expected hex string in host response",
    ))
}

fn parse_bool(value: &Value, keys: &[&str]) -> bool {
    if let Some(b) = value.as_bool() {
        return b;
    }
    for key in keys {
        if let Some(b) = value.get(*key).and_then(Value::as_bool) {
            return b;
        }
    }
    false
}

fn parse_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    for key in keys {
        if let Some(n) = value.get(*key).and_then(Value::as_i64) {
            return Some(n);
        }
    }
    None
}
