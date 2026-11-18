//! Integration tests for `AppController`, safety invariants, EID dedup, SE HAL, and JNI logic.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use nlpa_core::asn1::encode_tlv;
use nlpa_core::asn1::rsp::{
    EuiccChallenge, EuiccConfiguredAddresses, EuiccInfo2, RspProfileInfo, TAG_GET_PROFILES_INFO,
    TAG_PROFILE_INFO_LIST,
};
use nlpa_core::crypto::base64_encode;
use nlpa_core::hex::{bytes_to_hex, hex_to_bytes};
use nlpa_core::i18n::LocaleChoice;
use nlpa_core::storage::ThemeStyle;
use nlpa_ui::RenderMeta;
use rust_lib_nlpa2::android_jni::{
    dispatch_action_json, handle_platform_event_json, init_controller, render_state_json,
    reset_global_controller,
};
use rust_lib_nlpa2::controller::{AppConfig, AppController};
use rust_lib_nlpa2::host::{HostCallError, HostTransport};
use rust_lib_nlpa2::readers::TransportKind;
use serde_json::{json, Value};

const TEST_EID: &str = "89049032001001234560000123456789";
const PROFILE_A_ICCID: &str = "8901260000000000001F";
const PROFILE_B_ICCID: &str = "8901260000000000002F";

#[derive(Default)]
struct MockHostState {
    calls: Vec<(String, Value)>,
    omapi_readers: Vec<String>,
    tmapi_available: bool,
    nbridge_slots: Vec<(String, String)>,
    adb_connected: bool,
    adb_slots: Vec<(String, String)>,
    se_hal_active: bool,
    eid_by_reader: BTreeMap<String, String>,
    fail_omapi_corrupted_on_transmit: bool,
    profile_a_enabled: bool,
    sw6881_on_next_switch: bool,
    se_hal_uncertain_on_switch: bool,
    enable_apdu_count: usize,
    se_hal_begin_download_count: usize,
    se_hal_end_download_count: usize,
    se_hal_download_open: bool,
    se_hal_download_tags: Vec<String>,
    http_responses: BTreeMap<String, (u16, String)>,
}

struct MockHostTransport {
    state: Mutex<MockHostState>,
}

impl MockHostTransport {
    fn new(init: MockHostState) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(init),
        })
    }
}

impl HostTransport for MockHostTransport {
    fn call_host_raw(&self, request_json: &str) -> Result<String, HostCallError> {
        let req: Value = serde_json::from_str(request_json).expect("valid host request JSON");
        let op = req["op"].as_str().unwrap_or("").to_string();
        let args = req["args"].clone();

        let mut st = self.state.lock().expect("mock lock");
        st.calls.push((op.clone(), args.clone()));

        let data: Value = match op.as_str() {
            "cert.getInfo" => json!({
                "sha256": ["AA:BB"],
                "sha1": ["c47350c7ba682b34a3e584a0d58463ea42b1ad73"],
                "aramSha256": [],
                "aramSha1": [],
                "abis": ["arm64-v8a"]
            }),
            "adb.isConnected" => json!(st.adb_connected),
            "adb.getConnectCommand" => json!("adb shell sh /data/local/tmp/start.sh"),
            "nbridge.isAvailable" => json!(!st.nbridge_slots.is_empty()),
            "nbridge.listSlots" => Value::Array(
                st.nbridge_slots
                    .iter()
                    .map(|(id, name)| json!({"id": id, "displayName": name}))
                    .collect(),
            ),
            "nbridge.connectLogicalChannel" => {
                let slot_id = args["slotId"].as_str().unwrap_or("");
                json!({
                    "connectionId": format!("conn:{slot_id}"),
                    "selectedAid": "A0000005591010FFFFFFFF8900000100",
                    "channelNumber": 1,
                    "responseApdu": "9000"
                })
            }
            "nbridge.transmitLogical" => {
                let conn_id = args["connectionId"].as_str().unwrap_or("");
                let apdu = args["apdu"].as_str().unwrap_or("");
                let reader_key = conn_id.strip_prefix("conn:").unwrap_or(conn_id);
                handle_card_apdu(&mut st, &format!("nbridge:{reader_key}"), apdu)?
            }
            "nbridge.closeLogicalChannel" => json!(null),
            "telephony.hasPrivilegedPermission" => json!(st.tmapi_available),
            "telephony.listSlots" => {
                if st.tmapi_available {
                    json!([{
                        "slotIndex": 0,
                        "isEuicc": true,
                        "cardId": "card-0",
                        "isRemovable": false,
                        "ports": [{"portIndex": 0, "logicalSlotIndex": 0}],
                        "simState": 10
                    }])
                } else {
                    json!([])
                }
            }
            "telephony.sendTerminalCapabilities" => json!(null),
            "telephony.openChannel" => json!({"channel": 2, "selectResponse": "9000"}),
            "telephony.transmit" => {
                let apdu = args["apdu"].as_str().unwrap_or("");
                handle_card_apdu(&mut st, "tmapi:slot0:port0", apdu)?
            }
            "telephony.closeChannel" => json!(null),
            "omapi.listReaders" => json!(st.omapi_readers),
            "omapi.connect" => json!({
                "success": true,
                "atr": "3B9F96801F038031E073FE21136702434F537EC101FF"
            }),
            "omapi.openChannel" => {
                let reader = args["reader"].as_str().unwrap_or("");
                json!({
                    "success": true,
                    "aid": "A0000005591010FFFFFFFF8900000100",
                    "selectResponse": format!("9000_{reader}")
                })
            }
            "omapi.transmitOnChannel" => {
                if st.fail_omapi_corrupted_on_transmit {
                    return Ok(json!({
                        "ok": false,
                        "code": "OMAPI_SESSION_CORRUPTED",
                        "message": "OMAPI session corrupted during transmit"
                    })
                    .to_string());
                }
                let reader = args["reader"].as_str().unwrap_or("");
                let apdu = args["apdu"].as_str().unwrap_or("");
                handle_card_apdu(&mut st, &format!("omapi:{reader}"), apdu)?
            }
            "omapi.closeChannel" | "omapi.disconnect" | "omapi.setProfileSwitchInProgress" => {
                json!(null)
            }
            "adb.listSlots" => Value::Array(
                st.adb_slots
                    .iter()
                    .map(|(id, name)| json!({"id": id, "displayName": name}))
                    .collect(),
            ),
            "adb.connectLogicalChannel" => {
                let slot_id = args["slotId"].as_str().unwrap_or("");
                json!({
                    "connectionId": format!("adbconn:{slot_id}"),
                    "selectedAid": "A0000005591010FFFFFFFF8900000100",
                    "channelNumber": 1,
                    "responseApdu": "9000"
                })
            }
            "adb.transmitLogical" => {
                let conn_id = args["connectionId"].as_str().unwrap_or("");
                let apdu = args["apdu"].as_str().unwrap_or("");
                let slot = conn_id.strip_prefix("adbconn:").unwrap_or(conn_id);
                handle_card_apdu(&mut st, &format!("adb:{slot}"), apdu)?
            }
            "adb.closeLogicalChannel" => json!(null),
            "qrtr.isSupported" | "qrtr.isAvailable" | "qrtr.hasPermission" => json!(false),
            "seHal.atr" => {
                if st.se_hal_active && args["slot"].as_i64() == Some(1) {
                    json!("3B9F96801F038031E073FE21136702434F537EC101FF")
                } else {
                    json!("")
                }
            }
            "seHal.openOwned" | "seHal.closeOwned" | "seHal.useBorrowed" => json!(null),
            "seHal.read" => {
                let apdu = args["apduHex"].as_str().unwrap_or("");
                assert_allowed_hal_read_apdu(apdu);
                handle_card_apdu(&mut st, "sehal:slot1", apdu)?
            }
            "seHal.beginDownload" => {
                st.se_hal_begin_download_count += 1;
                st.se_hal_download_open = true;
                json!(null)
            }
            "seHal.endDownload" => {
                st.se_hal_end_download_count += 1;
                st.se_hal_download_open = false;
                json!(null)
            }
            "seHal.download" => {
                assert!(
                    st.se_hal_download_open,
                    "seHal.download called without active seHal.beginDownload"
                );
                let segments = args["segmentsHex"].as_array().expect("segmentsHex array");
                assert!(!segments.is_empty(), "segmentsHex must not be empty");
                let mut payload = Vec::new();
                for seg in segments {
                    let s = seg.as_str().unwrap_or("");
                    payload.extend_from_slice(&hex_to_bytes(s).expect("valid segment hex"));
                }
                assert!(payload.len() >= 3 && payload[0] == 0xBF);
                let tag = payload[1];
                assert!(
                    matches!(tag, 0x2E | 0x38 | 0x21 | 0x36),
                    "unexpected ES10b tag BF{tag:02X} in seHal.download"
                );
                if tag != 0x36 {
                    assert_eq!(segments.len(), 1, "only BF36 allows multi-segment download");
                }
                st.se_hal_download_tags.push(format!("BF{tag:02X}"));
                let mut resp = match tag {
                    0x2E => EuiccChallenge::new([0x11; 16]).encode(),
                    0x38 => encode_tlv(0xBF38, &encode_tlv(0xA0, &[])),
                    0x21 => encode_tlv(0xBF21, &encode_tlv(0xA0, &[])),
                    0x36 => encode_tlv(0xBF37, &encode_tlv(0x80, &[0x00])),
                    _ => unreachable!(),
                };
                resp.extend_from_slice(&[0x90, 0x00]);
                json!(bytes_to_hex(&resp))
            }
            "seHal.switchProfile" => {
                let iccid_hex = args["iccidHex"].as_str().unwrap_or("");
                assert_se_hal_swapped_bcd_iccid(iccid_hex);
                st.profile_a_enabled = args["enable"].as_bool().unwrap_or(true);
                if st.se_hal_uncertain_on_switch {
                    return Ok(json!({
                        "ok": false,
                        "code": "HAL_SWITCH_UNCERTAIN",
                        "message": "reply uncertain after SIM refresh"
                    })
                    .to_string());
                }
                json!("BF31038001009000")
            }
            "seHal.renameProfile" | "seHal.deleteProfile" => {
                let iccid_hex = args["iccidHex"].as_str().unwrap_or("");
                assert_se_hal_swapped_bcd_iccid(iccid_hex);
                json!("BF29038001009000")
            }
            "seHal.removeNotification" => json!("BF30030201009000"),
            "ccid.listReaders" => json!([]),
            "http.request" => {
                let url = args["url"].as_str().unwrap_or("");
                let (status, body) = st
                    .http_responses
                    .get(url)
                    .cloned()
                    .unwrap_or((404, "{}".into()));
                json!({
                    "status": status,
                    "headers": {},
                    "setCookies": [],
                    "bodyBase64": base64_encode(body.as_bytes())
                })
            }
            "ui.copyClipboard" => json!(null),
            _ => json!(null),
        };

        Ok(json!({ "ok": true, "data": data }).to_string())
    }
}

fn handle_card_apdu(
    st: &mut MockHostState,
    reader_id: &str,
    apdu_hex: &str,
) -> Result<Value, HostCallError> {
    let upper = apdu_hex.to_ascii_uppercase();
    // GetEuiccData (EID: BF3E)
    if upper.contains("BF3E") {
        let eid = st
            .eid_by_reader
            .get(reader_id)
            .cloned()
            .unwrap_or_else(|| TEST_EID.to_string());
        let eid_bytes = nlpa_core::hex::hex_to_bytes(&eid).expect("valid EID hex");
        let mut tlv = encode_tlv(0xBF3E, &encode_tlv(0x5A, &eid_bytes));
        tlv.extend_from_slice(&[0x90, 0x00]);
        return Ok(json!(bytes_to_hex(&tlv)));
    }
    // GetConfiguredAddresses (BF3C)
    if upper.contains("BF3C") {
        let mut der = EuiccConfiguredAddresses {
            default_smdp_address: Some("rsp.example.com".into()),
            root_smds_address: Some("lpa.ds.gsma.com".into()),
        }
        .encode();
        der.extend_from_slice(&[0x90, 0x00]);
        return Ok(json!(bytes_to_hex(&der)));
    }
    // GetEuiccInfo2 (BF22)
    if upper.contains("BF22") {
        let mut der = EuiccInfo2 {
            profile_version: vec![2, 1, 0],
            svn: vec![2, 2, 0],
            euicc_firmware_ver: vec![4, 1, 0],
            ext_card_res: Vec::new(),
            free_nvram: 262_144,
            free_ram: 16_384,
        }
        .encode();
        der.extend_from_slice(&[0x90, 0x00]);
        return Ok(json!(bytes_to_hex(&der)));
    }
    // NXP proprietary GET DATA
    if upper.starts_with("80CA00FE") {
        return Ok(json!("6A88"));
    }
    // GetProfilesInfo (BF2D)
    if upper.contains("BF2D") {
        let p1 = RspProfileInfo {
            iccid: PROFILE_A_ICCID.into(),
            isdp_aid: Some(vec![0xA0, 0x00]),
            profile_state: u8::from(st.profile_a_enabled),
            nickname: Some("Alpha".into()),
            service_provider_name: Some("Carrier A".into()),
            profile_name: Some("Alpha Plan".into()),
            profile_class: 2,
        }
        .encode();
        let p2 = RspProfileInfo {
            iccid: PROFILE_B_ICCID.into(),
            isdp_aid: Some(vec![0xA0, 0x01]),
            profile_state: u8::from(!st.profile_a_enabled),
            nickname: Some("Beta".into()),
            service_provider_name: Some("Carrier B".into()),
            profile_name: Some("Beta Plan".into()),
            profile_class: 2,
        }
        .encode();
        let mut list_payload = Vec::new();
        list_payload.extend_from_slice(&p1);
        list_payload.extend_from_slice(&p2);
        let list_tlv = encode_tlv(TAG_PROFILE_INFO_LIST, &list_payload);
        let mut root = encode_tlv(TAG_GET_PROFILES_INFO, &list_tlv);
        root.extend_from_slice(&[0x90, 0x00]);
        return Ok(json!(bytes_to_hex(&root)));
    }
    // EnableProfile (BF31) / DisableProfile (BF32)
    if upper.contains("BF31") || upper.contains("BF32") {
        st.enable_apdu_count += 1;
        st.profile_a_enabled = upper.contains("BF31");
        if st.sw6881_on_next_switch {
            st.sw6881_on_next_switch = false;
            return Ok(json!("6881"));
        }
        let mut ok_tlv = if upper.contains("BF31") {
            encode_tlv(0xBF31, &encode_tlv(0x80, &[0x00]))
        } else {
            encode_tlv(0xBF32, &encode_tlv(0x80, &[0x00]))
        };
        ok_tlv.extend_from_slice(&[0x90, 0x00]);
        return Ok(json!(bytes_to_hex(&ok_tlv)));
    }
    // ListNotification / RetrieveNotificationsList
    if upper.contains("BF2B") || upper.contains("BF28") {
        let mut empty = encode_tlv(0xBF28, &encode_tlv(0xA0, &[]));
        empty.extend_from_slice(&[0x90, 0x00]);
        return Ok(json!(bytes_to_hex(&empty)));
    }
    Ok(json!("9000"))
}

fn temp_dir_for_test(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("nlpa2_{name}_{nanos}"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[test]
fn reader_eid_deduplication_follows_exact_priority_order() {
    let mut mock_state = MockHostState {
        nbridge_slots: vec![
            ("tmapi_0".into(), "NBridge TMAPI 0".into()),
            ("omapi_0".into(), "NBridge OMAPI 0".into()),
        ],
        tmapi_available: true,
        omapi_readers: vec!["SIM1".into()],
        adb_connected: true,
        adb_slots: vec![("0".into(), "ADB Slot 0".into())],
        ..MockHostState::default()
    };
    mock_state
        .eid_by_reader
        .insert("nbridge:tmapi_0".into(), TEST_EID.into());
    mock_state
        .eid_by_reader
        .insert("nbridge:omapi_0".into(), TEST_EID.into());
    mock_state
        .eid_by_reader
        .insert("tmapi:slot0:port0".into(), TEST_EID.into());
    mock_state
        .eid_by_reader
        .insert("omapi:SIM1".into(), TEST_EID.into());
    mock_state
        .eid_by_reader
        .insert("adb:0".into(), TEST_EID.into());

    let files_dir = temp_dir_for_test("dedup");
    let host = MockHostTransport::new(mock_state);
    let mut ctrl = AppController::new(
        host.clone(),
        AppConfig {
            files_dir: files_dir.clone(),
            app_version: "2.5.0".into(),
            version_code: 606,
            ..AppConfig::default()
        },
    );

    assert_eq!(ctrl.discovered_readers().len(), 1);
    assert_eq!(
        ctrl.discovered_readers()[0].kind,
        TransportKind::NBridgeTmapi
    );

    // Disable NBridge -> Telephony wins over OMAPI and ADB
    ctrl.dispatch_json(
        r#"{"action":"toggle_reader_type","reader_type":"nbridge","enabled":false}"#,
    );
    assert_eq!(ctrl.discovered_readers().len(), 1);
    assert_eq!(ctrl.discovered_readers()[0].kind, TransportKind::Telephony);

    // Disable Telephony -> OMAPI wins over ADB
    ctrl.dispatch_json(r#"{"action":"toggle_reader_type","reader_type":"tmapi","enabled":false}"#);
    assert_eq!(ctrl.discovered_readers().len(), 1);
    assert_eq!(ctrl.discovered_readers()[0].kind, TransportKind::Omapi);

    // Disable OMAPI -> ADB remains
    ctrl.dispatch_json(r#"{"action":"toggle_reader_type","reader_type":"omapi","enabled":false}"#);
    assert_eq!(ctrl.discovered_readers().len(), 1);
    assert_eq!(ctrl.discovered_readers()[0].kind, TransportKind::AdbBridge);

    let _ = std::fs::remove_dir_all(&files_dir);
}

#[test]
fn omapi_session_corrupted_poisons_safety_latch_and_stops_retries() {
    let mock_state = MockHostState {
        omapi_readers: vec!["SIM1".into()],
        fail_omapi_corrupted_on_transmit: true,
        ..MockHostState::default()
    };
    let host = MockHostTransport::new(mock_state);
    let files_dir = temp_dir_for_test("omapi_latch");
    let mut ctrl = AppController::new(
        host.clone(),
        AppConfig {
            files_dir: files_dir.clone(),
            ..AppConfig::default()
        },
    );

    let model_after_init = ctrl.build_ui_model();
    assert!(model_after_init.omapi_corrupted_banner.is_some());

    let calls_before = host.state.lock().expect("lock").calls.len();
    ctrl.dispatch_json(r#"{"action":"refresh_readers"}"#);
    let calls_after = host.state.lock().expect("lock").calls.clone();

    let omapi_calls_after: Vec<_> = calls_after[calls_before..]
        .iter()
        .filter(|(op, _)| op.starts_with("omapi."))
        .collect();
    assert!(
        omapi_calls_after.is_empty(),
        "poisoned OMAPI latch must suppress subsequent OMAPI calls, got {omapi_calls_after:?}"
    );

    let _ = std::fs::remove_dir_all(&files_dir);
}

#[test]
fn sw6881_on_profile_switch_never_replays_switch_apdu_and_converges_via_readback() {
    let mock_state = MockHostState {
        omapi_readers: vec!["SIM1".into()],
        profile_a_enabled: false,
        sw6881_on_next_switch: true,
        ..MockHostState::default()
    };
    let host = MockHostTransport::new(mock_state);
    let files_dir = temp_dir_for_test("sw6881");
    let mut ctrl = AppController::new(
        host.clone(),
        AppConfig {
            files_dir: files_dir.clone(),
            ..AppConfig::default()
        },
    );

    ctrl.dispatch_json(&format!(
        r#"{{"action":"confirm_switch_profile","iccid":"{PROFILE_A_ICCID}","enable":true}}"#
    ));

    let st = host.state.lock().expect("lock");
    assert_eq!(
        st.enable_apdu_count, 1,
        "ENABLE/DISABLE APDU must be transmitted once without replay on SW=6881"
    );
    drop(st);

    let snap = ctrl.active_snapshot().expect("snapshot present");
    let profile_a = snap
        .profiles
        .iter()
        .find(|p| p.iccid == nlpa_core::iccid::iccid_for_display(PROFILE_A_ICCID))
        .expect("profile A");
    assert!(
        profile_a.enabled,
        "read-back convergence must observe enabled state"
    );

    let _ = std::fs::remove_dir_all(&files_dir);
}

#[test]
fn se_hal_uncertain_switch_verifies_via_readback_without_double_switch() {
    let mock_state = MockHostState {
        se_hal_active: true,
        profile_a_enabled: false,
        se_hal_uncertain_on_switch: true,
        ..MockHostState::default()
    };
    let host = MockHostTransport::new(mock_state);
    let files_dir = temp_dir_for_test("se_hal_uncertain");
    let mut ctrl = AppController::new(
        host.clone(),
        AppConfig {
            files_dir: files_dir.clone(),
            ..AppConfig::default()
        },
    );
    ctrl.dispatch_json(r#"{"action":"toggle_reader_type","reader_type":"se_hal","enabled":true}"#);

    ctrl.dispatch_json(&format!(
        r#"{{"action":"confirm_switch_profile","iccid":"{PROFILE_A_ICCID}","enable":true}}"#
    ));

    let switch_calls = host
        .state
        .lock()
        .expect("lock")
        .calls
        .iter()
        .filter(|(op, _)| op == "seHal.switchProfile")
        .count();
    assert_eq!(
        switch_calls, 1,
        "SE HAL must not double-submit switchProfile"
    );
    let snap = ctrl.active_snapshot().expect("snapshot present");
    assert!(snap.profiles[0].enabled);

    let _ = std::fs::remove_dir_all(&files_dir);
}

#[test]
fn jni_lifecycle_wallpaper_opacity_vietnamese_roundtrip_and_atomic_persistence() {
    reset_global_controller();
    let files_dir = temp_dir_for_test("jni_roundtrip");
    let host = MockHostTransport::new(MockHostState::default());

    let doc = init_controller(
        host.clone(),
        AppConfig {
            files_dir: files_dir.clone(),
            app_version: "2.5.0".into(),
            version_code: 606,
            is_privileged: false,
            system_locale: "en".into(),
            ..AppConfig::default()
        },
    );
    assert!(doc.contains("<!DOCTYPE html>"));
    assert!(doc.contains("#000000"));

    // Toggle wallpaper + opacity + locale vi + switch to Settings tab
    handle_platform_event_json(concat!(
        r#"{"type":"wallpaper_changed","active":true,"#,
        r#""path":"/data/user/0/app/files/wallpaper.img"}"#
    ));
    dispatch_action_json(r#"{"action":"set_wallpaper_opacity","opacity":0.45}"#);
    dispatch_action_json(r#"{"action":"set_locale","locale":"vi"}"#);
    dispatch_action_json(r#"{"action":"set_theme_style","style":"amoled"}"#);
    let render_json = dispatch_action_json(r#"{"action":"switch_tab","tab":"settings"}"#);

    let meta: RenderMeta = serde_json::from_str(&render_json).expect("valid RenderMeta JSON");
    assert_eq!(meta.theme_style, "amoled");
    assert!(meta.dark);
    assert!(meta.wallpaper_active);
    assert!((meta.wallpaper_opacity - 0.45).abs() < 0.01);
    assert!(!meta.status_bar_black);
    assert!(meta.html.contains("nlpa-settings"));
    assert!(meta.html.contains("is-wallpaper"));
    assert!(meta.html.contains("Cài đặt"));
    assert!(meta.html.contains("Ngôn ngữ"));
    assert!(meta.html.contains("Tiếng Việt (vi)"));

    let state_file = files_dir.join("state.json");
    let tmp_file = files_dir.join("state.json.tmp");
    assert!(state_file.exists(), "atomic state.json must exist");
    assert!(!tmp_file.exists(), "temporary file must be renamed away");

    let current_json = render_state_json();
    assert_eq!(current_json, render_json);

    // Reload a fresh AppController from the persisted state directory
    let reloaded = AppController::new(
        host,
        AppConfig {
            files_dir: files_dir.clone(),
            ..AppConfig::default()
        },
    );
    assert_eq!(reloaded.persisted_state().theme_style, ThemeStyle::Amoled);
    assert_eq!(reloaded.persisted_state().locale, LocaleChoice::Vi);
    assert!(reloaded.persisted_state().wallpaper_enabled);
    assert!((reloaded.persisted_state().wallpaper_opacity - 0.45).abs() < 0.01);

    // Verify deep_link platform event switches to Download & populates LPA fields
    let dl_json = handle_platform_event_json(
        r#"{"type":"deep_link","url":"https://install.lpa.ee/#LPA:1$smdp.test$MATCH-42"}"#,
    );
    let dl_meta: RenderMeta = serde_json::from_str(&dl_json).expect("valid deep-link RenderMeta");
    assert!(dl_meta.html.contains("smdp.test"));
    assert!(dl_meta.html.contains("MATCH-42"));

    reset_global_controller();
    let _ = std::fs::remove_dir_all(&files_dir);
}

#[test]
fn notification_removal_requires_local_history_backup() {
    let host = MockHostTransport::new(MockHostState::default());
    let reader = rust_lib_nlpa2::readers::DiscoveredReader {
        id: "sehal:slot1".into(),
        label: "SE HAL Slot 1".into(),
        kind: TransportKind::SeHal,
        slot_label: "Slot 1".into(),
        raw_name: "slot1".into(),
        slot_index: Some(1),
        port_index: None,
        is_removable: false,
        needs_permission: false,
        atr_hex: None,
        eid: Some(TEST_EID.into()),
        remocard_server: None,
    };
    let state = nlpa_core::storage::PersistedState::default();
    let latch = Arc::new(Mutex::new(nlpa_core::channel::OmapiSafetyLatch::new()));

    let blocked = rust_lib_nlpa2::lpa_ops::remove_notification_safe(
        host.clone(),
        &reader,
        &state,
        &latch,
        99,
        false,
        Some(TEST_EID),
    );
    assert!(
        blocked.is_err(),
        "unbacked-up notification removal must fail safely"
    );

    let allowed = rust_lib_nlpa2::lpa_ops::remove_notification_safe(
        host,
        &reader,
        &state,
        &latch,
        99,
        true,
        Some(TEST_EID),
    );
    assert!(
        allowed.is_ok(),
        "backed-up notification removal must succeed"
    );
}

fn assert_allowed_hal_read_apdu(apdu_hex: &str) {
    let a = hex_to_bytes(apdu_hex).expect("valid seHal.read APDU hex");
    assert!(a.len() >= 5, "seHal.read APDU too short: {apdu_hex}");
    if a[1] == 0xC0 {
        assert!(
            a.len() == 5 && (a[0] == 0x01 || a[0] == 0x81) && a[2] == 0 && a[3] == 0,
            "invalid SE HAL GET RESPONSE APDU: {apdu_hex}"
        );
        return;
    }
    assert!(
        a.len() >= 8 && a[0] == 0x81 && a[1] == 0xE2 && a[2] == 0x91 && a[3] == 0x00,
        "SE HAL read APDU must use CLA=81 INS=E2 P1=91 P2=00, got {apdu_hex}"
    );
    let lc = usize::from(a[4]);
    assert!(
        a.len() == 5 + lc || a.len() == 6 + lc,
        "invalid Lc framing in seHal.read: {apdu_hex}"
    );
    assert!(
        a[5] == 0xBF && matches!(a[6], 0x20 | 0x22 | 0x28 | 0x2B | 0x2D | 0x3E),
        "disallowed ES10 tag in seHal.read: {apdu_hex}"
    );
    assert!(
        a[7] < 128 && lc == 3 + usize::from(a[7]),
        "SE HAL read only allows short-form TLVs: {apdu_hex}"
    );
}

fn assert_se_hal_swapped_bcd_iccid(iccid_hex: &str) {
    assert_eq!(
        iccid_hex.len(),
        20,
        "SE HAL expects 10-byte BCD iccidHex, got {iccid_hex}"
    );
    assert!(
        iccid_hex.starts_with("98"),
        "SE HAL expects swapped-BCD ICCID starting with 98, got {iccid_hex}"
    );
}

#[test]
fn se_hal_download_uses_single_begin_end_and_valid_read_apdus() {
    let mut http_responses = BTreeMap::new();
    let init_json = json!({
        "header": { "functionExecutionStatus": { "status": "Executed-Success" } },
        "transactionId": "0A1B2C3D",
        "serverSigned1": base64_encode(&encode_tlv(0x30, &encode_tlv(0x80, &[0x01]))),
        "serverSignature1": base64_encode(&encode_tlv(0x5F37, &[0xAA; 4])),
        "serverCertificate": base64_encode(&encode_tlv(0x30, &[0x02, 0x01, 0x00]))
    })
    .to_string();
    http_responses.insert(
        "https://rsp.example.com/gsma/rsp2/es9plus/initiateAuthentication".into(),
        (200, init_json),
    );

    let mut meta_children = Vec::new();
    let iccid_bcd = hex_to_bytes("981062000000000000F3").expect("iccid bcd");
    meta_children.extend_from_slice(&encode_tlv(0x5A, &iccid_bcd));
    meta_children.extend_from_slice(&encode_tlv(0x91, b"DemoTel"));
    meta_children.extend_from_slice(&encode_tlv(0x92, b"Demo Plan"));
    let auth_client_json = json!({
        "header": { "functionExecutionStatus": { "status": "Executed-Success" } },
        "transactionId": "0A1B2C3D",
        "profileMetadata": base64_encode(&encode_tlv(0xBF25, &meta_children)),
        "smdpSigned2": base64_encode(&encode_tlv(0x30, &encode_tlv(0x80, &[0x02]))),
        "smdpSignature2": base64_encode(&encode_tlv(0x5F37, &[0xBB; 4]))
    })
    .to_string();
    http_responses.insert(
        "https://rsp.example.com/gsma/rsp2/es9plus/authenticateClient".into(),
        (200, auth_client_json),
    );

    let mut bpp_inner = encode_tlv(0xBF23, &[0x80, 0x01, 0x01]);
    bpp_inner.extend_from_slice(&encode_tlv(0xA0, &encode_tlv(0x87, &[0x01])));
    let bpp_json = json!({
        "header": { "functionExecutionStatus": { "status": "Executed-Success" } },
        "transactionId": "0A1B2C3D",
        "boundProfilePackage": base64_encode(&encode_tlv(0xBF36, &bpp_inner))
    })
    .to_string();
    http_responses.insert(
        "https://rsp.example.com/gsma/rsp2/es9plus/getBoundProfilePackage".into(),
        (200, bpp_json),
    );

    let mock_state = MockHostState {
        se_hal_active: true,
        http_responses,
        ..MockHostState::default()
    };
    let host = MockHostTransport::new(mock_state);
    let files_dir = temp_dir_for_test("se_hal_download");
    let mut ctrl = AppController::new(
        host.clone(),
        AppConfig {
            files_dir: files_dir.clone(),
            ..AppConfig::default()
        },
    );
    ctrl.dispatch_json(r#"{"action":"toggle_reader_type","reader_type":"se_hal","enabled":true}"#);
    let reader = ctrl
        .discovered_readers()
        .first()
        .cloned()
        .expect("SE HAL reader discovered");
    assert_eq!(reader.kind, TransportKind::SeHal);

    let state = ctrl.persisted_state().clone();
    let latch = Arc::new(Mutex::new(nlpa_core::channel::OmapiSafetyLatch::new()));
    let catalog = nlpa_core::profile_size_predictor::ProfileSizeCatalog::default();

    let preview_out = rust_lib_nlpa2::lpa_ops::download_profile(
        host.clone(),
        &reader,
        &state,
        &latch,
        "rsp.example.com",
        Some("MATCH-1"),
        None,
        &catalog,
        false,
    )
    .expect("SE HAL download preview succeeds");
    match preview_out {
        rust_lib_nlpa2::lpa_ops::DownloadOutcome::PreviewOnly { preview, .. } => {
            assert_eq!(preview.iccid, "8901260000000000003");
        }
        other => panic!("expected PreviewOnly, got {other:?}"),
    }

    let install_out = rust_lib_nlpa2::lpa_ops::download_profile(
        host.clone(),
        &reader,
        &state,
        &latch,
        "rsp.example.com",
        Some("MATCH-1"),
        None,
        &catalog,
        true,
    )
    .expect("SE HAL download install succeeds");
    match install_out {
        rust_lib_nlpa2::lpa_ops::DownloadOutcome::Installed {
            installed_iccid, ..
        } => {
            assert_eq!(installed_iccid.as_deref(), Some("8901260000000000003"));
        }
        other => panic!("expected Installed, got {other:?}"),
    }

    let st = host.state.lock().expect("lock");
    assert_eq!(st.se_hal_begin_download_count, 2);
    assert_eq!(st.se_hal_end_download_count, 2);
    assert!(!st.se_hal_download_open);
    assert_eq!(
        st.se_hal_download_tags,
        vec!["BF2E", "BF38", "BF2E", "BF38", "BF21", "BF36"]
    );
    drop(st);

    rust_lib_nlpa2::lpa_ops::rename_profile(
        host.clone(),
        &reader,
        &state,
        &latch,
        PROFILE_A_ICCID,
        "Renamed",
        Some(TEST_EID),
    )
    .expect("SE HAL rename uses swapped-BCD ICCID");
    rust_lib_nlpa2::lpa_ops::delete_profile(
        host,
        &reader,
        &state,
        &latch,
        PROFILE_A_ICCID,
        Some(TEST_EID),
    )
    .expect("SE HAL delete uses swapped-BCD ICCID");

    let _ = std::fs::remove_dir_all(&files_dir);
}

#[test]
fn omapi_discovery_skips_empty_slots_with_pipe_and_flags_euicc_atr() {
    let mock_state = MockHostState {
        omapi_readers: vec![
            "SIM1".into(),
            "SIM2|No SIM card|Card not detected in slot".into(),
        ],
        ..MockHostState::default()
    };
    let host = MockHostTransport::new(mock_state);
    let files_dir = temp_dir_for_test("omapi_pipe");
    let ctrl = AppController::new(
        host.clone(),
        AppConfig {
            files_dir: files_dir.clone(),
            ..AppConfig::default()
        },
    );

    assert_eq!(ctrl.discovered_readers().len(), 1);
    assert_eq!(ctrl.discovered_readers()[0].raw_name, "SIM1");
    assert!(ctrl.build_ui_model().omapi_euicc_warning);

    let st = host.state.lock().expect("lock");
    let pipe_connects = st
        .calls
        .iter()
        .filter(|(op, args)| {
            op == "omapi.connect" && args["reader"].as_str().is_some_and(|r| r.contains('|'))
        })
        .count();
    assert_eq!(
        pipe_connects, 0,
        "empty slot strings with '|' must not be connected"
    );

    let _ = std::fs::remove_dir_all(&files_dir);
}
