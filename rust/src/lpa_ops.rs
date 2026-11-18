//! GSMA SGP.22 ES9+/ES10/ES11 operations, convergence switches, plugins, and notifications.

use core::fmt;
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use nlpa_core::apdu::validate_apdu_sw;
use nlpa_core::asn1::rsp::{
    DeleteProfileRequest, DisableProfileRequest, EnableProfileRequest, EuiccChallenge,
    EuiccConfiguredAddresses, EuiccInfo2, NotificationMetadata, RspProfileInfo, SetNicknameRequest,
    TAG_GET_CONFIGURED_ADDRESSES, TAG_GET_EUICC_CHALLENGE, TAG_GET_EUICC_INFO1,
    TAG_GET_EUICC_INFO2, TAG_GET_PROFILES_INFO, TAG_HANDLE_NOTIFICATION, TAG_LIST_NOTIFICATION,
    TAG_NOTIFICATION_METADATA, TAG_PROFILE_INFO, TAG_PROFILE_INFO_LIST, TAG_REMOVE_NOTIFICATION,
};
use nlpa_core::asn1::x509::{verify_ci_public_key_constraint, X509Certificate};
use nlpa_core::asn1::{decode_integer, decode_utf8_string, encode_integer, encode_tlv, Asn1Reader};
use nlpa_core::bpp::{
    build_cancel_session_request_der, decompose_bpp, parse_nxp_memory_tlv, NxpMemoryInfo,
    EUICC_MEMORY_RESET_APDU_DATA,
};
use nlpa_core::ccid::nine_esim::{
    enable_atr_programming_command, set_atr_command, set_euicc_capability, set_password_command,
};
use nlpa_core::channel::OmapiSafetyLatch;
use nlpa_core::crypto::{base64_decode, base64_encode};
use nlpa_core::es9::{
    build_authenticate_client_body, build_cancel_session_body, build_es9_headers,
    build_get_bound_profile_package_body, build_handle_notification_body,
    build_initiate_authentication_body, parse_authenticate_client_response,
    parse_get_bound_profile_package_response, parse_initiate_authentication_response,
    AuthenticateClientResponse, GetBoundProfilePackageResponse, InitiateAuthenticationResponse,
    ENDPOINT_AUTHENTICATE_CLIENT, ENDPOINT_CANCEL_SESSION, ENDPOINT_GET_BOUND_PROFILE_PACKAGE,
    ENDPOINT_HANDLE_NOTIFICATION, ENDPOINT_INITIATE_AUTHENTICATION,
};
use nlpa_core::hex::{bytes_to_hex, hex_to_bytes, swap_nibbles};
use nlpa_core::iccid::iccid_for_display;
use nlpa_core::imei::{calculate_imei_luhn_checksum, encode_stored_imei_bytes};
use nlpa_core::lpa::download_session::ProfileDownloadSession;
use nlpa_core::lpa::es11::{
    build_smds_auth_client_request, build_smds_initiate_auth_request, clean_smds_host,
    parse_smds_discovery_response, DiscoveredEvent,
};
use nlpa_core::lpa::state_transition::ProfileStateTransition;
use nlpa_core::mcc::parse_operator_id;
use nlpa_core::plugins::{
    build_estk_mgt_read_apdu, build_gm_auth_apdu, esim_cc_badge, is_gm_eid, match_provider_plugin,
    parse_esim_gg_response, parse_estk_mgt_string, parse_three_hk_response, EstkCardInfo,
    ProfileProviderBadge, ProviderPluginKind, ESTK_MGT_AID,
};
use nlpa_core::profile::EuiccProfile;
use nlpa_core::profile_size_predictor::ProfileSizeCatalog;
use nlpa_core::profile_tag::parse_nickname;
use nlpa_core::storage::PersistedState;

use crate::host::{HostClient, HostTransport, HttpRequestArgs};
use crate::readers::{
    candidate_isdr_aids, parse_eid_from_get_euicc_data, CardChannel, DiscoveredReader, ReaderError,
    TransportKind,
};

/// Unified LPA execution error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LpaOpError {
    Reader(ReaderError),
    Http { status: u16, message: String },
    Protocol(String),
    UncertainState(String),
    SafetyBlocked(String),
}

impl fmt::Display for LpaOpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reader(e) => write!(f, "{e}"),
            Self::Http { status, message } => write!(f, "HTTP {status}: {message}"),
            Self::Protocol(msg) => write!(f, "{msg}"),
            Self::UncertainState(msg) => write!(f, "{msg}"),
            Self::SafetyBlocked(msg) => write!(f, "{msg}"),
        }
    }
}

impl Error for LpaOpError {}

impl From<ReaderError> for LpaOpError {
    fn from(err: ReaderError) -> Self {
        Self::Reader(err)
    }
}

/// Complete chip & profile snapshot loaded for a selected reader.
#[derive(Debug, Clone, Default)]
pub struct CardSnapshot {
    pub eid: String,
    pub configured: EuiccConfiguredAddresses,
    pub info2: EuiccInfo2,
    pub nxp_memory: Option<NxpMemoryInfo>,
    pub estk_info: Option<EstkCardInfo>,
    pub gm_authenticated: bool,
    pub profiles: Vec<EuiccProfile>,
    pub notifications: Vec<RetrievedNotification>,
    pub apdu_log: Vec<(String, String)>,
}

/// One pending notification retrieved from `RetrieveNotificationsList` / `ListNotification`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievedNotification {
    pub metadata: NotificationMetadata,
    pub raw_notification_b64: String,
}

/// Preview metadata decoded after `AuthenticateClient`.
#[derive(Debug, Clone)]
pub struct DownloadPreview {
    pub smdp_address: String,
    pub matching_id: Option<String>,
    pub spn: String,
    pub profile_name: String,
    pub iccid: String,
    pub profile_class: u8,
    pub estimated_bytes: Option<i64>,
    pub free_nvram_bytes: Option<u64>,
    pub ci_trusted: bool,
    pub ci_summary: String,
    pub requires_confirmation_code: bool,
    pub transaction_id: String,
}

/// Outcome of `download_profile`.
#[derive(Debug, Clone)]
pub enum DownloadOutcome {
    PreviewOnly {
        preview: DownloadPreview,
        apdu_log: Vec<(String, String)>,
    },
    Installed {
        installed_iccid: Option<String>,
        apdu_log: Vec<(String, String)>,
    },
}

/// Normalized HTTP proxy configuration for RSP/ES9+ requests.
#[derive(Debug, Clone, Default)]
pub struct RspProxyConfig {
    pub proxy_url: Option<String>,
}

impl RspProxyConfig {
    /// Sanitizes user-configured proxy string (`http://`, `https://`, `socks5://`).
    #[must_use]
    pub fn from_setting(raw: Option<&str>) -> Self {
        let trimmed = raw.map(str::trim).unwrap_or("");
        if trimmed.is_empty() {
            Self { proxy_url: None }
        } else {
            Self {
                proxy_url: Some(trimmed.to_string()),
            }
        }
    }
}

/// ES9+ / ES11 HTTP client routed through Android `http.request`.
pub struct Es9Client<'a> {
    host: &'a dyn HostTransport,
    proxy: RspProxyConfig,
}

impl<'a> Es9Client<'a> {
    #[must_use]
    pub fn new(host: &'a dyn HostTransport, proxy_url: Option<&str>) -> Self {
        Self {
            host,
            proxy: RspProxyConfig::from_setting(proxy_url),
        }
    }

    fn post_es9(
        &self,
        host_addr: &str,
        endpoint: &str,
        body_json: &str,
    ) -> Result<(u16, String), LpaOpError> {
        let clean = clean_smds_host(host_addr);
        let url = format!("https://{clean}{endpoint}");
        let mut headers = BTreeMap::new();
        for (k, v) in build_es9_headers() {
            headers.insert(k.to_string(), v.to_string());
        }
        let client = HostClient::new(self.host);
        let resp = client
            .http_request(&HttpRequestArgs {
                method: "POST".into(),
                url,
                headers,
                body_base64: Some(base64_encode(body_json.as_bytes())),
                proxy_url: self.proxy.proxy_url.clone(),
                timeout_ms: 30_000,
                insecure_tls: true,
            })
            .map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;

        let raw_bytes = base64_decode(&resp.body_base64)
            .map_err(|e| LpaOpError::Protocol(format!("ES9 base64 decode failed: {e}")))?;
        let text = String::from_utf8_lossy(&raw_bytes).into_owned();
        if resp.status >= 400 && text.trim().is_empty() {
            return Err(LpaOpError::Http {
                status: resp.status,
                message: "Empty HTTP error body".into(),
            });
        }
        Ok((resp.status, text))
    }

    pub fn initiate_authentication(
        &self,
        smdp_address: &str,
        euicc_challenge_b64: &str,
        euicc_info1_b64: &str,
    ) -> Result<InitiateAuthenticationResponse, LpaOpError> {
        let body = build_initiate_authentication_body(
            &clean_smds_host(smdp_address),
            euicc_challenge_b64,
            euicc_info1_b64,
        );
        let (status, text) =
            self.post_es9(smdp_address, ENDPOINT_INITIATE_AUTHENTICATION, &body)?;
        parse_initiate_authentication_response(&text, Some(status))
            .map_err(|e| LpaOpError::Protocol(e.to_string()))
    }

    pub fn authenticate_client(
        &self,
        smdp_address: &str,
        transaction_id: &str,
        auth_server_resp_b64: &str,
    ) -> Result<AuthenticateClientResponse, LpaOpError> {
        let body = build_authenticate_client_body(transaction_id, auth_server_resp_b64);
        let (status, text) = self.post_es9(smdp_address, ENDPOINT_AUTHENTICATE_CLIENT, &body)?;
        parse_authenticate_client_response(&text, Some(status))
            .map_err(|e| LpaOpError::Protocol(e.to_string()))
    }

    pub fn get_bound_profile_package(
        &self,
        smdp_address: &str,
        transaction_id: &str,
        prepare_download_resp_b64: &str,
    ) -> Result<GetBoundProfilePackageResponse, LpaOpError> {
        let body = build_get_bound_profile_package_body(transaction_id, prepare_download_resp_b64);
        let (status, text) =
            self.post_es9(smdp_address, ENDPOINT_GET_BOUND_PROFILE_PACKAGE, &body)?;
        parse_get_bound_profile_package_response(&text, Some(status))
            .map_err(|e| LpaOpError::Protocol(e.to_string()))
    }

    pub fn cancel_session(
        &self,
        smdp_address: &str,
        transaction_id: &str,
        cancel_session_resp_b64: &str,
    ) -> Result<(), LpaOpError> {
        let body = build_cancel_session_body(transaction_id, cancel_session_resp_b64);
        let _ = self.post_es9(smdp_address, ENDPOINT_CANCEL_SESSION, &body)?;
        Ok(())
    }

    pub fn handle_notification(
        &self,
        server_address: &str,
        pending_notification_b64: &str,
    ) -> Result<(), LpaOpError> {
        let body = build_handle_notification_body(pending_notification_b64);
        let (status, text) = self.post_es9(server_address, ENDPOINT_HANDLE_NOTIFICATION, &body)?;
        if !(200..300).contains(&status) {
            return Err(LpaOpError::Http {
                status,
                message: text,
            });
        }
        Ok(())
    }
}

/// Validates and parses an 8-digit TAC or 15-digit IMEI string.
pub fn validate_device_imei_input(input: &str) -> Result<String, String> {
    let digits: String = input.chars().filter(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return Ok("35383741".to_string());
    }
    if digits.len() == 8 {
        return Ok(digits);
    }
    if digits.len() == 15 {
        let check = calculate_imei_luhn_checksum(&digits[..14]).map_err(|e| e.to_string())?;
        let actual = digits.as_bytes()[14] - b'0';
        if check != actual {
            return Err(format!("Invalid IMEI check digit (expected {check})"));
        }
        return Ok(digits);
    }
    Err("Enter an 8-digit TAC or a 15-digit Luhn-valid IMEI".into())
}

/// Encodes TAC (4 bytes) and optional full IMEI (8 bytes) for `AuthenticateServer` `DeviceInfo`.
fn build_device_info_tlv(device_imei: &str) -> Vec<u8> {
    let digits: String = device_imei.chars().filter(char::is_ascii_digit).collect();
    let tac_str = if digits.len() >= 8 {
        &digits[..8]
    } else {
        "35383741"
    };
    let mut tac_bytes = [0x35_u8, 0x38, 0x37, 0x41];
    if let Ok(parsed) = hex_to_bytes(tac_str) {
        if parsed.len() == 4 {
            tac_bytes.copy_from_slice(&parsed);
        }
    }

    let mut children = Vec::new();
    children.extend_from_slice(&encode_tlv(0x80, &tac_bytes));
    children.extend_from_slice(&encode_tlv(0xA1, &[])); // empty DeviceCapabilities
    if digits.len() == 15 {
        if let Ok(stored) = encode_stored_imei_bytes(&digits) {
            children.extend_from_slice(&encode_tlv(0x82, &stored));
        }
    }
    encode_tlv(0xA1, &children)
}

fn open_reader_channel(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
) -> Result<CardChannel, LpaOpError> {
    let aids = candidate_isdr_aids(&state.custom_aids, reader.atr_hex.as_deref());
    let mss = usize::from(state.mss.max(32));
    let borrowed = match reader.slot_index {
        Some(2) => state.se_hal_borrowed_slot2,
        _ => state.se_hal_borrowed_slot1,
    };
    Ok(CardChannel::open(
        host,
        reader,
        &aids,
        mss,
        omapi_latch,
        borrowed,
        state.proxy_url.clone(),
    )?)
}

/// Loads full eUICC snapshot (EID, addresses, EUICCInfo2, profiles, notifications, plugins).
pub fn load_card_snapshot(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
) -> Result<CardSnapshot, LpaOpError> {
    let mut ch = open_reader_channel(Arc::clone(&host), reader, state, omapi_latch)?;
    let mut snap = CardSnapshot::default();

    snap.eid = read_eid(&mut ch)?;
    if !ch.is_se_hal() {
        if let Ok(cfg) = read_configured_addresses(&mut ch) {
            snap.configured = cfg;
        }
    }
    if let Ok((info2, nxp)) = read_euicc_info2_and_nxp(&mut ch) {
        snap.info2 = info2;
        snap.nxp_memory = nxp;
    }
    snap.profiles = read_profiles(&mut ch)?;
    snap.notifications = read_notifications(&mut ch).unwrap_or_default();

    if !ch.is_se_hal() && reader.looks_like_estk() {
        snap.estk_info = read_estk_info(&mut ch);
    }
    if !ch.is_se_hal() && is_gm_eid(&snap.eid) {
        if let Some(psk_hex) = state.gm_psk_cache.get(&snap.eid) {
            if perform_gm_auth(&mut ch, psk_hex).is_ok() {
                snap.gm_authenticated = true;
            }
        }
    }

    snap.apdu_log = core::mem::take(&mut ch.apdu_log);
    let _ = ch.close();
    Ok(snap)
}

fn read_eid(ch: &mut CardChannel) -> Result<String, LpaOpError> {
    let get_eid_tlv = [0xBF, 0x3E, 0x03, 0x5C, 0x01, 0x5A];
    let resp = ch.transmit_es10_tlv(&get_eid_tlv)?;
    parse_eid_from_get_euicc_data(&resp)
        .ok_or_else(|| LpaOpError::Protocol("Card response lacked EID tag 0x5A".into()))
}

fn read_asn1_root(bytes: &[u8]) -> Result<nlpa_core::asn1::Asn1Node, LpaOpError> {
    Asn1Reader::new(bytes)
        .read_next()
        .map_err(|e| LpaOpError::Protocol(e.to_string()))?
        .ok_or_else(|| LpaOpError::Protocol("Empty ASN.1 payload".into()))
}

fn read_configured_addresses(ch: &mut CardChannel) -> Result<EuiccConfiguredAddresses, LpaOpError> {
    let tlv = encode_tlv(TAG_GET_CONFIGURED_ADDRESSES, &[]);
    let resp = ch.transmit_es10_tlv(&tlv)?;
    let node = read_asn1_root(&resp)?;
    EuiccConfiguredAddresses::decode(&node).map_err(|e| LpaOpError::Protocol(e.to_string()))
}

fn read_euicc_info2_and_nxp(
    ch: &mut CardChannel,
) -> Result<(EuiccInfo2, Option<NxpMemoryInfo>), LpaOpError> {
    let tlv = encode_tlv(TAG_GET_EUICC_INFO2, &[]);
    let resp = ch.transmit_es10_tlv(&tlv)?;
    let node = read_asn1_root(&resp)?;
    let info2 = EuiccInfo2::decode(&node).map_err(|e| LpaOpError::Protocol(e.to_string()))?;

    let nxp = if ch.is_se_hal() {
        parse_nxp_memory_tlv(&resp)
    } else {
        ch.transmit_raw(&[0x80, 0xCA, 0x00, 0xFE, 0x00])
            .ok()
            .and_then(|raw| parse_nxp_memory_tlv(&raw))
            .or_else(|| parse_nxp_memory_tlv(&resp))
    };
    Ok((info2, nxp))
}

fn read_profiles(ch: &mut CardChannel) -> Result<Vec<EuiccProfile>, LpaOpError> {
    let tlv = encode_tlv(TAG_GET_PROFILES_INFO, &[]);
    let resp = ch.transmit_es10_tlv(&tlv)?;
    let root = read_asn1_root(&resp)?;
    let mut profiles = Vec::new();
    let list_node = root.find_child(TAG_PROFILE_INFO_LIST).unwrap_or(root);
    if let Ok(children) = list_node.children() {
        for item in children {
            if item.is_tag(TAG_PROFILE_INFO) {
                if let Ok(rsp) = RspProfileInfo::decode(&item) {
                    let mut profile = EuiccProfile::new(rsp.iccid, rsp.profile_state == 1);
                    profile.nickname = rsp.nickname;
                    profile.service_provider_name = rsp.service_provider_name;
                    profile.name = rsp.profile_name;
                    profile.profile_class = rsp.profile_class;
                    if let Some(aid) = rsp.isdp_aid {
                        profile.isdp_aid = Some(bytes_to_hex(&aid));
                    }
                    apply_profile_extra_tlvs(&item, &mut profile);
                    profile.tags = parse_nickname(profile.nickname.as_deref())
                        .tags
                        .into_iter()
                        .map(|t| t.raw())
                        .collect();
                    profiles.push(profile);
                }
            }
        }
    }
    Ok(profiles)
}

fn apply_profile_extra_tlvs(item: &nlpa_core::asn1::Asn1Node, profile: &mut EuiccProfile) {
    if let Some(owner) = item.find_child(0xB7) {
        if let Some(mcc_mnc) = owner.find_child(0x80) {
            if let Some((mcc, mnc)) = parse_operator_id(&mcc_mnc.value) {
                profile.mcc = Some(mcc);
                profile.mnc = Some(mnc);
            }
        } else if let Some((mcc, mnc)) = parse_operator_id(&owner.value) {
            profile.mcc = Some(mcc);
            profile.mnc = Some(mnc);
        }
        if let Some(g1) = owner.find_child(0x81) {
            profile.gid1 = Some(bytes_to_hex(&g1.value).to_ascii_uppercase());
        }
        if let Some(g2) = owner.find_child(0x82) {
            profile.gid2 = Some(bytes_to_hex(&g2.value).to_ascii_uppercase());
        }
    }
    if let Some(notif_cfg) = item.find_child(0xB6) {
        if let Ok(entries) = notif_cfg.children() {
            for entry in entries {
                let addr_node = entry.find_child(0x81).or_else(|| {
                    if entry.is_tag(0x81) {
                        Some(entry.clone())
                    } else {
                        None
                    }
                });
                if let Some(node) = addr_node {
                    if let Ok(addr) = decode_utf8_string(&node.value) {
                        if !addr.trim().is_empty() {
                            profile.smdp_address = Some(addr);
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn read_notifications(ch: &mut CardChannel) -> Result<Vec<RetrievedNotification>, LpaOpError> {
    // Request full notifications list via RetrieveNotificationsList (0xBF2B).
    let req_tlv = encode_tlv(0xBF2B, &[]);
    if let Ok(resp) = ch.transmit_es10_tlv(&req_tlv) {
        if let Ok(parsed) = decode_retrieve_notifications_response(&resp) {
            if !parsed.is_empty() {
                return Ok(parsed);
            }
        }
    }
    // Fallback to ListNotification (0xBF28) metadata skeletons.
    let list_tlv = encode_tlv(TAG_LIST_NOTIFICATION, &[]);
    let resp = ch.transmit_es10_tlv(&list_tlv)?;
    let root = read_asn1_root(&resp)?;
    let mut out = Vec::new();
    if let Ok(nodes) = root.children() {
        for node in nodes {
            if let Ok(inner_nodes) = node.children() {
                for meta_node in inner_nodes {
                    if meta_node.is_tag(TAG_NOTIFICATION_METADATA) {
                        if let Ok(meta) = NotificationMetadata::decode(&meta_node) {
                            out.push(RetrievedNotification {
                                metadata: meta,
                                raw_notification_b64: String::new(),
                            });
                        }
                    }
                }
            } else if node.is_tag(TAG_NOTIFICATION_METADATA) {
                if let Ok(meta) = NotificationMetadata::decode(&node) {
                    out.push(RetrievedNotification {
                        metadata: meta,
                        raw_notification_b64: String::new(),
                    });
                }
            }
        }
    }
    Ok(out)
}

fn decode_retrieve_notifications_response(
    der: &[u8],
) -> Result<Vec<RetrievedNotification>, LpaOpError> {
    let root = read_asn1_root(der)?;
    let mut out = Vec::new();
    let container = root.find_child(0xA0).unwrap_or(root);
    if let Ok(entries) = container.children() {
        for entry in entries {
            let b64 = base64_encode(&entry.raw_bytes);
            if let Some(meta) = extract_metadata_from_pending_notification(&entry) {
                out.push(RetrievedNotification {
                    metadata: meta,
                    raw_notification_b64: b64,
                });
            }
        }
    }
    Ok(out)
}

fn extract_metadata_from_pending_notification(
    entry: &nlpa_core::asn1::Asn1Node,
) -> Option<NotificationMetadata> {
    if let Some(meta) = entry.find_child(TAG_NOTIFICATION_METADATA) {
        return NotificationMetadata::decode(&meta).ok();
    }
    // Check ProfileInstallationResult (0xBF37 -> 0xBF27 -> 0xBF2F) or OtherSignedNotification.
    if let Ok(children) = entry.children() {
        for child in children {
            if let Some(found) = extract_metadata_from_pending_notification(&child) {
                return Some(found);
            }
        }
    }
    None
}

/// Switches a profile (`enable=true` or `false`) enforcing non-retry and convergence checks.
pub fn switch_profile(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    iccid: &str,
    enable: bool,
    expected_eid: Option<&str>,
) -> Result<(Vec<(String, String)>, Vec<EuiccProfile>), LpaOpError> {
    let client = HostClient::new(host.as_ref());
    let is_omapi = reader.kind == TransportKind::Omapi;
    if is_omapi {
        let _ = client.omapi_set_profile_switch_in_progress(true);
    }

    let result = switch_profile_inner(
        host,
        reader,
        state,
        omapi_latch,
        iccid,
        enable,
        expected_eid,
    );

    if is_omapi {
        let _ = client.omapi_set_profile_switch_in_progress(false);
    }
    result
}

fn switch_profile_inner(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    iccid: &str,
    enable: bool,
    expected_eid: Option<&str>,
) -> Result<(Vec<(String, String)>, Vec<EuiccProfile>), LpaOpError> {
    let client = HostClient::new(host.as_ref());
    let policy = ProfileStateTransition::default();

    if reader.kind == TransportKind::SeHal {
        let slot = reader.slot_index.unwrap_or(1);
        let iccid_hex = se_hal_iccid_bcd_hex(iccid);
        let hal_res = client.se_hal_switch_profile(slot, &iccid_hex, enable, expected_eid);
        if let Err(err) = hal_res {
            let msg_lower = err.message.to_ascii_lowercase();
            let uncertain = err.code.contains("UNCERTAIN")
                || msg_lower.contains("uncertain")
                || msg_lower.contains("timeout");
            if !uncertain {
                return Err(LpaOpError::Reader(ReaderError::Host(err)));
            }
        }
        return verify_profile_convergence(
            host,
            reader,
            state,
            omapi_latch,
            iccid,
            enable,
            policy.max_verification_attempts,
        );
    }

    let mut ch = open_reader_channel(Arc::clone(&host), reader, state, omapi_latch)?;
    let req_tlv = if enable {
        EnableProfileRequest::new(iccid.to_string(), true).encode()
    } else {
        DisableProfileRequest::new(iccid.to_string(), true).encode()
    };

    // Single submission invariant: NEVER replay switch command after sending to the card.
    let send_res = ch.transmit_es10_tlv(&req_tlv).map_err(LpaOpError::Reader);
    let mut logs = core::mem::take(&mut ch.apdu_log);
    let _ = ch.close();

    match send_res {
        Ok(resp_der) => {
            if let Some(code) = extract_management_result_code(&resp_der) {
                if code != 0 {
                    return Err(LpaOpError::Protocol(format!(
                        "Profile switch failed with SGP.22 result code {code}"
                    )));
                }
            }
            let (verify_logs, profiles) =
                verify_profile_convergence(host, reader, state, omapi_latch, iccid, enable, 3)?;
            logs.extend(verify_logs);
            Ok((logs, profiles))
        }
        Err(LpaOpError::Reader(ref r)) if r.is_sw6881() => {
            // Card returned 6881 after profile switch refreshed logical channels;
            // NEVER replay enable/disable — perform read-back verification only!
            let (verify_logs, profiles) = verify_profile_convergence(
                host,
                reader,
                state,
                omapi_latch,
                iccid,
                enable,
                policy.max_verification_attempts,
            )?;
            logs.extend(verify_logs);
            Ok((logs, profiles))
        }
        Err(e) => Err(e),
    }
}

fn verify_profile_convergence(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    iccid: &str,
    target_enabled: bool,
    attempts: usize,
) -> Result<(Vec<(String, String)>, Vec<EuiccProfile>), LpaOpError> {
    let clean_target = nlpa_core::iccid::iccid_for_display(iccid);
    let mut all_logs = Vec::new();
    let mut last_err = None;

    for _ in 0..attempts.max(1) {
        match open_reader_channel(Arc::clone(&host), reader, state, omapi_latch) {
            Ok(mut ch) => {
                let res = read_profiles(&mut ch);
                all_logs.extend(core::mem::take(&mut ch.apdu_log));
                let _ = ch.close();
                if let Ok(profiles) = res {
                    let matched = profiles
                        .iter()
                        .find(|p| nlpa_core::iccid::iccid_for_display(&p.iccid) == clean_target);
                    if matched.is_some_and(|p| p.enabled == target_enabled) {
                        return Ok((all_logs, profiles));
                    }
                    last_err = Some(format!(
                        "Profile state read-back has not converged to enabled={target_enabled}"
                    ));
                }
            }
            Err(e) => {
                last_err = Some(e.to_string());
            }
        }
    }

    Err(LpaOpError::UncertainState(last_err.unwrap_or_else(|| {
        "Unable to verify profile switch convergence; refresh manually before retrying.".into()
    })))
}

/// Renames a profile on the physical eUICC (or SE HAL).
pub fn rename_profile(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    iccid: &str,
    nickname: &str,
    eid_hex: Option<&str>,
) -> Result<Vec<(String, String)>, LpaOpError> {
    if reader.kind == TransportKind::SeHal {
        let client = HostClient::new(host.as_ref());
        let slot = reader.slot_index.unwrap_or(1);
        let iccid_hex = se_hal_iccid_bcd_hex(iccid);
        let rx = client
            .se_hal_rename_profile(slot, &iccid_hex, nickname, eid_hex)
            .map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;
        return Ok(vec![("SE_HAL_RENAME".into(), rx)]);
    }
    let mut ch = open_reader_channel(host, reader, state, omapi_latch)?;
    let req_tlv = SetNicknameRequest::new(iccid.to_string(), nickname.to_string()).encode();
    let resp = ch.transmit_es10_tlv(&req_tlv)?;
    if let Some(code) = extract_management_result_code(&resp) {
        if code != 0 {
            return Err(LpaOpError::Protocol(format!(
                "SetNickname failed with code {code}"
            )));
        }
    }
    Ok(core::mem::take(&mut ch.apdu_log))
}

/// Permanently deletes a disabled profile from the eUICC.
pub fn delete_profile(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    iccid: &str,
    eid_hex: Option<&str>,
) -> Result<Vec<(String, String)>, LpaOpError> {
    if reader.kind == TransportKind::SeHal {
        let client = HostClient::new(host.as_ref());
        let slot = reader.slot_index.unwrap_or(1);
        let iccid_hex = se_hal_iccid_bcd_hex(iccid);
        let rx = client
            .se_hal_delete_profile(slot, &iccid_hex, eid_hex)
            .map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;
        return Ok(vec![("SE_HAL_DELETE".into(), rx)]);
    }
    let mut ch = open_reader_channel(host, reader, state, omapi_latch)?;
    let req_tlv = DeleteProfileRequest::new(iccid.to_string()).encode();
    let resp = ch.transmit_es10_tlv(&req_tlv)?;
    if let Some(code) = extract_management_result_code(&resp) {
        if code != 0 {
            return Err(LpaOpError::Protocol(format!(
                "DeleteProfile failed with code {code}"
            )));
        }
    }
    Ok(core::mem::take(&mut ch.apdu_log))
}

/// Resets eUICC operational profile memory using ES10c `EuiccMemoryReset`.
pub fn reset_euicc_memory(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
) -> Result<Vec<(String, String)>, LpaOpError> {
    let mut ch = open_reader_channel(host, reader, state, omapi_latch)?;
    let resp = ch.transmit_es10_tlv(EUICC_MEMORY_RESET_APDU_DATA)?;
    if let Some(code) = extract_management_result_code(&resp) {
        if code != 0 && code != 1 {
            return Err(LpaOpError::Protocol(format!(
                "EuiccMemoryReset failed with code {code}"
            )));
        }
    }
    Ok(core::mem::take(&mut ch.apdu_log))
}

/// Removes a notification from the eUICC ONLY after verifying local backup preservation.
pub fn remove_notification_safe(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    seq_number: u32,
    is_backed_up: bool,
    eid_hex: Option<&str>,
) -> Result<Vec<(String, String)>, LpaOpError> {
    if !is_backed_up {
        return Err(LpaOpError::SafetyBlocked(format!(
            "Refusing to remove notification #{seq_number} before local history backup is persisted"
        )));
    }
    if reader.kind == TransportKind::SeHal {
        let client = HostClient::new(host.as_ref());
        let slot = reader.slot_index.unwrap_or(1);
        let rx = client
            .se_hal_remove_notification(slot, seq_number, eid_hex)
            .map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;
        return Ok(vec![("SE_HAL_REMOVE_NOTIF".into(), rx)]);
    }
    let mut ch = open_reader_channel(host, reader, state, omapi_latch)?;
    let int_tlv = encode_integer(i64::from(seq_number));
    let seq_octets = int_tlv.get(2..).unwrap_or(&[0x00]);
    let req_tlv = encode_tlv(TAG_REMOVE_NOTIFICATION, &encode_tlv(0x80, seq_octets));
    let _ = ch.transmit_es10_tlv(&req_tlv)?;
    Ok(core::mem::take(&mut ch.apdu_log))
}

/// Sends a notification payload over ES9+ `handleNotification`.
pub fn dispatch_notification_es9(
    host: &dyn HostTransport,
    proxy_url: Option<&str>,
    server: &str,
    pending_notification_b64: &str,
) -> Result<(), LpaOpError> {
    if server.trim().is_empty() {
        return Err(LpaOpError::Protocol(
            "Missing notification server address".into(),
        ));
    }
    if pending_notification_b64.trim().is_empty() {
        return Err(LpaOpError::Protocol(
            "Signed notification payload is empty".into(),
        ));
    }
    Es9Client::new(host, proxy_url).handle_notification(server, pending_notification_b64)
}

/// Runs SGP.22 profile download (or preview-only if `confirm_install` is `false`).
#[allow(clippy::too_many_arguments)]
pub fn download_profile(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    smdp_address: &str,
    matching_id: Option<&str>,
    confirmation_code: Option<&str>,
    size_catalog: &ProfileSizeCatalog,
    confirm_install: bool,
) -> Result<DownloadOutcome, LpaOpError> {
    let clean_smdp = clean_smds_host(smdp_address);
    if clean_smdp.is_empty() {
        return Err(LpaOpError::Protocol("SM-DP+ address is required".into()));
    }

    let mut ch = open_reader_channel(Arc::clone(&host), reader, state, omapi_latch)?;
    let eid = read_eid(&mut ch).unwrap_or_default();
    let free_nvram = read_euicc_info2_and_nxp(&mut ch).ok().map(|(info2, nxp)| {
        nxp.and_then(|n| n.free_non_volatile_memory)
            .unwrap_or(u64::from(info2.free_nvram))
    });

    let info1_tlv = encode_tlv(TAG_GET_EUICC_INFO1, &[]);
    let info1_bytes = ch.transmit_es10_tlv(&info1_tlv)?;

    let hal_guard = if ch.is_se_hal() {
        let slot = reader.slot_index.unwrap_or(1);
        Some(SeHalDownloadGuard::begin(host.as_ref(), slot)?)
    } else {
        None
    };

    let challenge_tlv = encode_tlv(TAG_GET_EUICC_CHALLENGE, &[]);
    let challenge_bytes = transmit_download_tlv(&mut ch, hal_guard.as_ref(), &challenge_tlv)?;
    let chal_node = read_asn1_root(&challenge_bytes)?;
    let challenge =
        EuiccChallenge::decode(&chal_node).map_err(|e| LpaOpError::Protocol(e.to_string()))?;

    let mut session = ProfileDownloadSession::new(
        clean_smdp.clone(),
        matching_id.map(ToString::to_string),
        confirmation_code.map(ToString::to_string),
    );
    session.process_euicc_info1(&info1_bytes);
    session.process_euicc_challenge(&challenge.challenge);

    let es9 = Es9Client::new(host.as_ref(), state.proxy_url.as_deref());
    let init_resp = es9.initiate_authentication(
        &clean_smdp,
        &base64_encode(&challenge.challenge),
        &base64_encode(&info1_bytes),
    )?;
    session
        .process_initiate_authentication_response(
            &init_resp.transaction_id,
            &init_resp.server_certificate,
        )
        .map_err(|e| LpaOpError::Protocol(e.to_string()))?;

    let ci_trusted = verify_server_ci(&init_resp);
    let auth_req = build_authenticate_server_request(
        &init_resp,
        matching_id.unwrap_or(""),
        &state.device_imei,
    );
    let auth_resp = transmit_download_tlv(&mut ch, hal_guard.as_ref(), &auth_req)?;
    session.process_authenticate_server_response(&auth_resp);

    let auth_client_res = es9.authenticate_client(
        &clean_smdp,
        &init_resp.transaction_id,
        &base64_encode(&auth_resp),
    );
    let auth_client = match auth_client_res {
        Ok(ok) => ok,
        Err(err) => {
            send_cancel_session_best_effort(&mut ch, &es9, &clean_smdp, &session, 1);
            return Err(err);
        }
    };

    session.process_authenticate_client_response(
        "Executed-Success",
        auth_client.smdp_signed2.as_deref(),
        auth_client.smdp_signature2.as_deref(),
        auth_client.profile_metadata.as_deref(),
        auth_client.smdp_certificate.as_deref(),
    );

    let preview = build_preview(
        &clean_smdp,
        matching_id,
        &init_resp.transaction_id,
        auth_client.profile_metadata.as_deref(),
        &eid,
        free_nvram,
        ci_trusted,
        size_catalog,
        confirmation_code.is_none(),
    );

    if !confirm_install {
        send_cancel_session_best_effort(&mut ch, &es9, &clean_smdp, &session, 0);
        if let Some(guard) = hal_guard {
            guard.finish();
        }
        let logs = core::mem::take(&mut ch.apdu_log);
        let _ = ch.close();
        return Ok(DownloadOutcome::PreviewOnly {
            preview,
            apdu_log: logs,
        });
    }

    let signed2 = auth_client
        .smdp_signed2
        .as_deref()
        .ok_or_else(|| LpaOpError::Protocol("Missing smdpSigned2 in AuthenticateClient".into()))?;
    let sig2 = auth_client.smdp_signature2.as_deref().ok_or_else(|| {
        LpaOpError::Protocol("Missing smdpSignature2 in AuthenticateClient".into())
    })?;
    let cert2 = auth_client
        .smdp_certificate
        .as_deref()
        .unwrap_or(&init_resp.server_certificate);
    let hash_cc = if confirmation_code.is_some_and(|c| !c.trim().is_empty()) {
        session.compute_hash_cc().ok()
    } else {
        None
    };

    let prep_req = build_prepare_download_request(signed2, sig2, hash_cc.as_deref(), cert2);
    let prep_resp = match transmit_download_tlv(&mut ch, hal_guard.as_ref(), &prep_req) {
        Ok(r) => r,
        Err(e) => {
            send_cancel_session_best_effort(&mut ch, &es9, &clean_smdp, &session, 2);
            return Err(e);
        }
    };

    let bpp_resp = match es9.get_bound_profile_package(
        &clean_smdp,
        &init_resp.transaction_id,
        &base64_encode(&prep_resp),
    ) {
        Ok(r) => r,
        Err(e) => {
            send_cancel_session_best_effort(&mut ch, &es9, &clean_smdp, &session, 2);
            return Err(e);
        }
    };

    let segments = decompose_bpp(&bpp_resp.bound_profile_package)
        .map_err(|e| LpaOpError::Protocol(format!("BPP decomposition failed: {e}")))?;

    if let Some(guard) = hal_guard {
        let segments_hex: Vec<String> = segments.iter().map(|s| bytes_to_hex(s)).collect();
        let dl_res = guard.client.se_hal_download(guard.slot, &segments_hex);
        guard.finish();
        let rx_hex = dl_res.map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;
        ch.apdu_log
            .push(("SE_HAL_DOWNLOAD_BPP".into(), rx_hex.clone()));
        let _ = strip_hal_download_sw(&rx_hex)?;
    } else {
        for seg in &segments {
            if let Err(e) = ch.transmit_es10_tlv(seg) {
                send_cancel_session_best_effort(&mut ch, &es9, &clean_smdp, &session, 3);
                return Err(LpaOpError::Reader(e));
            }
        }
    }

    let logs = core::mem::take(&mut ch.apdu_log);
    let _ = ch.close();
    Ok(DownloadOutcome::Installed {
        installed_iccid: (!preview.iccid.is_empty()).then_some(preview.iccid),
        apdu_log: logs,
    })
}

/// Discovers pending profile download events from an ES11 SM-DS server.
pub fn discover_smds_events(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    smds_address: &str,
) -> Result<(Vec<DiscoveredEvent>, Vec<(String, String)>), LpaOpError> {
    let clean_smds = clean_smds_host(smds_address);
    let mut ch = open_reader_channel(Arc::clone(&host), reader, state, omapi_latch)?;
    let info1_tlv = encode_tlv(TAG_GET_EUICC_INFO1, &[]);
    let info1 = ch.transmit_es10_tlv(&info1_tlv)?;

    let hal_guard = if ch.is_se_hal() {
        let slot = reader.slot_index.unwrap_or(1);
        Some(SeHalDownloadGuard::begin(host.as_ref(), slot)?)
    } else {
        None
    };

    let chal_tlv = encode_tlv(TAG_GET_EUICC_CHALLENGE, &[]);
    let chal_raw = transmit_download_tlv(&mut ch, hal_guard.as_ref(), &chal_tlv)?;
    let chal_node = read_asn1_root(&chal_raw)?;
    let challenge =
        EuiccChallenge::decode(&chal_node).map_err(|e| LpaOpError::Protocol(e.to_string()))?;

    let es9 = Es9Client::new(host.as_ref(), state.proxy_url.as_deref());
    let init_body = build_smds_initiate_auth_request(
        &clean_smds,
        &base64_encode(&challenge.challenge),
        &base64_encode(&info1),
    );
    let (st1, resp1) = es9.post_es9(&clean_smds, ENDPOINT_INITIATE_AUTHENTICATION, &init_body)?;
    let init_resp = parse_initiate_authentication_response(&resp1, Some(st1))
        .map_err(|e| LpaOpError::Protocol(e.to_string()))?;

    let auth_req = build_authenticate_server_request(&init_resp, "", &state.device_imei);
    let auth_resp = transmit_download_tlv(&mut ch, hal_guard.as_ref(), &auth_req)?;
    if let Some(guard) = hal_guard {
        guard.finish();
    }
    let auth_body =
        build_smds_auth_client_request(&init_resp.transaction_id, &base64_encode(&auth_resp));
    let (st2, resp2) = es9.post_es9(&clean_smds, ENDPOINT_AUTHENTICATE_CLIENT, &auth_body)?;
    let events = parse_smds_discovery_response(&resp2, Some(st2))
        .map_err(|e| LpaOpError::Protocol(e.to_string()))?;

    let logs = core::mem::take(&mut ch.apdu_log);
    let _ = ch.close();
    Ok((events, logs))
}

/// Fetches optional carrier provider plugin details (eSIM.gg / 3HK / eSIM.cc) for an ICCID.
pub fn fetch_provider_badge(
    host: &dyn HostTransport,
    proxy_url: Option<&str>,
    iccid: &str,
) -> Option<ProfileProviderBadge> {
    let kind = match_provider_plugin(iccid)?;
    match kind {
        ProviderPluginKind::EsimCc => esim_cc_badge(iccid),
        ProviderPluginKind::EsimGg => {
            let url = format!("https://esim.gg/api/profile/{iccid}");
            let body = http_get_text(host, proxy_url, &url)?;
            parse_esim_gg_response(&body, iccid)
        }
        ProviderPluginKind::ThreeHk => {
            let url = format!("https://www.three.com.hk/api/esim/{iccid}");
            let body = http_get_text(host, proxy_url, &url)?;
            parse_three_hk_response(&body, iccid)
        }
    }
}

/// Runs eSTK management commands (`info`, `switch_slot`) over `ESTK_MGT_AID`.
pub fn execute_estk_command(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    command: &str,
    arg: &str,
) -> Result<Option<EstkCardInfo>, LpaOpError> {
    let aids = vec![bytes_to_hex(ESTK_MGT_AID)];
    let mss = usize::from(state.mss.max(32));
    let mut ch = CardChannel::open(
        host,
        reader,
        &aids,
        mss,
        omapi_latch,
        false,
        state.proxy_url.clone(),
    )?;

    let res = match command {
        "switch_slot" => {
            let p1 = if arg.trim() == "1" { 0x31 } else { 0x30 };
            let _ = ch.transmit_raw(&[0x00, 0xCA, p1, 0x00, 0x00])?;
            None
        }
        _ => read_estk_info(&mut ch),
    };

    let _ = ch.close();
    Ok(res)
}

/// Sends a raw hex APDU command on the reader's active ISD-R logical channel.
pub fn send_raw_apdu(
    host: Arc<dyn HostTransport>,
    reader: &DiscoveredReader,
    state: &PersistedState,
    omapi_latch: &Arc<Mutex<OmapiSafetyLatch>>,
    apdu_hex: &str,
) -> Result<String, LpaOpError> {
    let apdu = hex_to_bytes(apdu_hex.trim())
        .map_err(|e| LpaOpError::Protocol(format!("Invalid APDU hex: {e}")))?;
    let mut ch = open_reader_channel(host, reader, state, omapi_latch)?;
    let resp = ch.transmit_raw(&apdu)?;
    let _ = ch.close();
    Ok(bytes_to_hex(&resp))
}

/// Sets a 9eSIM USB writer password.
pub fn nine_esim_set_password(
    host: &dyn HostTransport,
    reader_name: &str,
    device_name: &str,
    password: &str,
) -> Result<(), LpaOpError> {
    let client = HostClient::new(host);
    let _ = client
        .ccid_connect(reader_name)
        .map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;
    let cmd = set_password_command(device_name, password)
        .map_err(|e| LpaOpError::Protocol(e.to_string()))?;
    let rx = client
        .ccid_transceive(reader_name, &bytes_to_hex(&cmd))
        .map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;
    let _ = client.ccid_disconnect(reader_name);
    ensure_9esim_ok(&rx)
}

/// Programs a 9eSIM USB writer ATR with updated T=15 eUICC capability flag.
pub fn nine_esim_set_atr(
    host: &dyn HostTransport,
    reader_name: &str,
    atr_hex: &str,
    euicc_mode: bool,
) -> Result<String, LpaOpError> {
    let updated = set_euicc_capability(atr_hex, euicc_mode)
        .map_err(|e| LpaOpError::Protocol(e.to_string()))?;
    let atr_bytes = hex_to_bytes(&updated).map_err(|e| LpaOpError::Protocol(e.to_string()))?;
    let cmd = set_atr_command(&atr_bytes).map_err(|e| LpaOpError::Protocol(e.to_string()))?;
    let client = HostClient::new(host);
    let _ = client
        .ccid_connect(reader_name)
        .map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;
    let rx1 = client
        .ccid_transceive(
            reader_name,
            &bytes_to_hex(&enable_atr_programming_command()),
        )
        .map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;
    ensure_9esim_ok(&rx1)?;
    let rx2 = client
        .ccid_transceive(reader_name, &bytes_to_hex(&cmd))
        .map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;
    let _ = client.ccid_disconnect(reader_name);
    ensure_9esim_ok(&rx2)?;
    Ok(updated)
}

fn ensure_9esim_ok(rx_hex: &str) -> Result<(), LpaOpError> {
    let upper = rx_hex.trim().to_ascii_uppercase();
    if upper.ends_with("9000") || upper.ends_with("6100") {
        Ok(())
    } else {
        Err(LpaOpError::Protocol(format!(
            "9eSIM writer returned unexpected status: {upper}"
        )))
    }
}

fn perform_gm_auth(ch: &mut CardChannel, psk_hex: &str) -> Result<(), LpaOpError> {
    let psk = hex_to_bytes(psk_hex.trim())
        .map_err(|e| LpaOpError::Protocol(format!("Invalid GM PSK hex: {e}")))?;
    let chal_tlv = encode_tlv(TAG_GET_EUICC_CHALLENGE, &[]);
    let chall_raw = ch.transmit_es10_tlv(&chal_tlv)?;
    let chal_node = read_asn1_root(&chall_raw)?;
    let challenge =
        EuiccChallenge::decode(&chal_node).map_err(|e| LpaOpError::Protocol(e.to_string()))?;
    let apdu = build_gm_auth_apdu(&psk, &challenge.challenge).map_err(LpaOpError::Protocol)?;
    let _ = ch.transmit_raw(&apdu)?;
    Ok(())
}

fn read_estk_info(ch: &mut CardChannel) -> Option<EstkCardInfo> {
    let read_str = |ch: &mut CardChannel, cmd_data: u8| -> Option<String> {
        let apdu = build_estk_mgt_read_apdu(cmd_data);
        let resp = ch.transmit_raw(&apdu).ok()?;
        if resp.len() < 2 {
            return None;
        }
        parse_estk_mgt_string(&resp[..resp.len() - 2])
    };
    let serial = read_str(ch, 0);
    let bootloader = read_str(ch, 1);
    let firmware = read_str(ch, 2);
    let sku = read_str(ch, 3);
    if serial.is_none() && bootloader.is_none() && firmware.is_none() && sku.is_none() {
        return None;
    }
    Some(EstkCardInfo {
        firmware,
        bootloader,
        serial,
        sku,
    })
}

fn se_hal_iccid_bcd_hex(iccid: &str) -> String {
    let mut clean: String = iccid
        .trim()
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_ascii_uppercase();
    if clean.len() % 2 != 0 {
        clean.push('F');
    }
    swap_nibbles(&clean).to_ascii_uppercase()
}

struct SeHalDownloadGuard<'a> {
    client: HostClient<'a>,
    slot: i32,
    active: bool,
}

impl<'a> SeHalDownloadGuard<'a> {
    fn begin(host: &'a dyn HostTransport, slot: i32) -> Result<Self, LpaOpError> {
        let client = HostClient::new(host);
        client
            .se_hal_begin_download(slot)
            .map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;
        Ok(Self {
            client,
            slot,
            active: true,
        })
    }

    fn finish(mut self) {
        if self.active {
            self.active = false;
            let _ = self.client.se_hal_end_download(self.slot);
        }
    }
}

impl Drop for SeHalDownloadGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            self.active = false;
            let _ = self.client.se_hal_end_download(self.slot);
        }
    }
}

fn transmit_download_tlv(
    ch: &mut CardChannel,
    hal_guard: Option<&SeHalDownloadGuard<'_>>,
    tlv: &[u8],
) -> Result<Vec<u8>, LpaOpError> {
    if let Some(guard) = hal_guard {
        let req_hex = bytes_to_hex(tlv);
        let resp_hex = guard
            .client
            .se_hal_download(guard.slot, std::slice::from_ref(&req_hex))
            .map_err(|e| LpaOpError::Reader(ReaderError::Host(e)))?;
        ch.apdu_log.push((req_hex, resp_hex.clone()));
        strip_hal_download_sw(&resp_hex)
    } else {
        ch.transmit_es10_tlv(tlv).map_err(LpaOpError::Reader)
    }
}

fn strip_hal_download_sw(resp_hex: &str) -> Result<Vec<u8>, LpaOpError> {
    let raw = hex_to_bytes(resp_hex).map_err(|e| LpaOpError::Protocol(e.to_string()))?;
    if raw.len() < 2 {
        return Err(LpaOpError::Reader(ReaderError::ApduFailed {
            sw1: 0x6F,
            sw2: 0x00,
        }));
    }
    let sw1 = raw[raw.len() - 2];
    let sw2 = raw[raw.len() - 1];
    if validate_apdu_sw(sw1, sw2).is_err() {
        return Err(LpaOpError::Reader(ReaderError::ApduFailed { sw1, sw2 }));
    }
    Ok(raw[..raw.len() - 2].to_vec())
}

fn send_cancel_session_best_effort(
    ch: &mut CardChannel,
    es9: &Es9Client<'_>,
    smdp: &str,
    session: &ProfileDownloadSession,
    reason: u8,
) {
    if ch.is_se_hal() {
        return;
    }
    let (Some(tx_octets), Some(tx_hex)) = (
        session.get_transaction_id(),
        session.get_transaction_id_hex(),
    ) else {
        return;
    };
    let cancel_req = build_cancel_session_request_der(tx_octets, reason);
    if let Ok(card_resp) = ch.transmit_es10_tlv(&cancel_req) {
        let _ = es9.cancel_session(smdp, tx_hex, &base64_encode(&card_resp));
    }
}

fn verify_server_ci(init_resp: &InitiateAuthenticationResponse) -> bool {
    let Some(expected_pkid) = init_resp.euicc_ci_pk_id_to_be_used.as_deref() else {
        return false;
    };
    let Ok(cert) = X509Certificate::from_der(&init_resp.server_certificate) else {
        return false;
    };
    verify_ci_public_key_constraint(&cert, &[expected_pkid.to_vec()])
}

fn build_authenticate_server_request(
    init: &InitiateAuthenticationResponse,
    matching_id: &str,
    device_imei: &str,
) -> Vec<u8> {
    let mut children = Vec::new();
    // serverSigned1 (already wrapped with 0x30)
    children.extend_from_slice(&init.server_signed1);
    // serverSignature1 (strip tag 0x5F37 if present, re-wrap consistently)
    let sig_val = Asn1Reader::new(&init.server_signature1)
        .read_next()
        .ok()
        .flatten()
        .map(|n| n.value)
        .unwrap_or_else(|| init.server_signature1.clone());
    children.extend_from_slice(&encode_tlv(0x5F37, &sig_val));
    if let Some(pkid) = init.euicc_ci_pk_id_to_be_used.as_deref() {
        let pkid_val = Asn1Reader::new(pkid)
            .read_next()
            .ok()
            .flatten()
            .map(|n| n.value)
            .unwrap_or_else(|| pkid.to_vec());
        children.extend_from_slice(&encode_tlv(0x04, &pkid_val));
    }
    children.extend_from_slice(&init.server_certificate);

    // ctxParams1 -> ctxParamsForCommonAuthentication (0xA0)
    let mut ctx_common = Vec::new();
    ctx_common.extend_from_slice(&encode_tlv(0x80, matching_id.as_bytes()));
    ctx_common.extend_from_slice(&build_device_info_tlv(device_imei));
    children.extend_from_slice(&encode_tlv(0xA0, &ctx_common));

    encode_tlv(0xBF38, &children)
}

fn build_prepare_download_request(
    smdp_signed2: &[u8],
    smdp_signature2: &[u8],
    hash_cc: Option<&[u8]>,
    smdp_certificate: &[u8],
) -> Vec<u8> {
    let mut children = Vec::new();
    children.extend_from_slice(smdp_signed2);
    let sig_val = Asn1Reader::new(smdp_signature2)
        .read_next()
        .ok()
        .flatten()
        .map(|n| n.value)
        .unwrap_or_else(|| smdp_signature2.to_vec());
    children.extend_from_slice(&encode_tlv(0x5F37, &sig_val));
    if let Some(cc) = hash_cc {
        children.extend_from_slice(&encode_tlv(0x04, cc));
    }
    children.extend_from_slice(smdp_certificate);
    encode_tlv(0xBF21, &children)
}

#[allow(clippy::too_many_arguments)]
fn build_preview(
    smdp: &str,
    matching_id: Option<&str>,
    tx_id: &str,
    metadata_der: Option<&[u8]>,
    eid: &str,
    free_nvram_bytes: Option<u64>,
    ci_trusted: bool,
    size_catalog: &ProfileSizeCatalog,
    requires_confirmation_code: bool,
) -> DownloadPreview {
    let mut spn = String::new();
    let mut profile_name = String::new();
    let mut iccid = String::new();
    let mut profile_class = 2_u8;
    let mut plmn: Option<String> = None;

    if let Some(der) = metadata_der {
        if let Ok(Some(root)) = Asn1Reader::new(der).read_next() {
            if let Ok(children) = root.children() {
                for c in children {
                    match c.tag {
                        0x5A => {
                            let swapped = swap_nibbles(&bytes_to_hex(&c.value));
                            iccid = iccid_for_display(&swapped).to_string();
                        }
                        0x91 => {
                            if let Ok(s) = decode_utf8_string(&c.value) {
                                spn = s;
                            }
                        }
                        0x92 => {
                            if let Ok(s) = decode_utf8_string(&c.value) {
                                profile_name = s;
                            }
                        }
                        0x95 => {
                            if let Ok(v) = decode_integer(&c.value) {
                                profile_class = v as u8;
                            }
                        }
                        0xB7 => {
                            let raw_mcc = c
                                .find_child(0x80)
                                .map(|n| n.value)
                                .unwrap_or_else(|| c.value.clone());
                            if let Some((mcc, mnc)) = parse_operator_id(&raw_mcc) {
                                plmn = Some(format!("{mcc}{mnc}"));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    let estimated_bytes = size_catalog.predict_size(
        (!eid.is_empty()).then_some(eid),
        Some(smdp),
        plmn.as_deref(),
        (!spn.is_empty()).then_some(spn.as_str()),
    );

    DownloadPreview {
        smdp_address: smdp.to_string(),
        matching_id: matching_id.map(ToString::to_string),
        spn,
        profile_name,
        iccid,
        profile_class,
        estimated_bytes,
        free_nvram_bytes,
        ci_trusted,
        ci_summary: if ci_trusted {
            "CI PKID verified".into()
        } else {
            "Unverified / Unknown CI".into()
        },
        requires_confirmation_code,
        transaction_id: tx_id.to_string(),
    }
}

fn extract_management_result_code(der: &[u8]) -> Option<i64> {
    let root = Asn1Reader::new(der).read_next().ok().flatten()?;
    if let Some(child) = root.find_child(0x80) {
        return decode_integer(&child.value).ok();
    }
    if let Some(child) = root.find_child(0x02) {
        return decode_integer(&child.value).ok();
    }
    None
}

fn http_get_text(host: &dyn HostTransport, proxy_url: Option<&str>, url: &str) -> Option<String> {
    let client = HostClient::new(host);
    let mut headers = BTreeMap::new();
    headers.insert("Accept".into(), "application/json".into());
    let resp = client
        .http_request(&HttpRequestArgs {
            method: "GET".into(),
            url: url.to_string(),
            headers,
            body_base64: None,
            proxy_url: proxy_url.map(ToString::to_string),
            timeout_ms: 10_000,
            insecure_tls: false,
        })
        .ok()?;
    if !(200..300).contains(&resp.status) {
        return None;
    }
    let bytes = base64_decode(&resp.body_base64).ok()?;
    String::from_utf8(bytes).ok()
}

#[must_use]
pub fn is_install_notification_tag(der: &[u8]) -> bool {
    Asn1Reader::new(der)
        .read_next()
        .ok()
        .flatten()
        .is_some_and(|n| n.is_tag(TAG_HANDLE_NOTIFICATION) || n.is_tag(0xBF37))
}
