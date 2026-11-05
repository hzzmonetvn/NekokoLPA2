//! GSMA SGP.22 LPA operations and notification management bridged to Flutter.

use flutter_rust_bridge::frb;
use nlpa_core::{
    base64_decode, build_authenticate_client_body as core_build_authenticate_client_body,
    build_cancel_session_body as core_build_cancel_session_body,
    build_delete_isdp_apdu as core_build_delete_isdp_apdu,
    build_get_bound_profile_package_body as core_build_get_bound_profile_package_body,
    build_initiate_authentication_body as core_build_initiate_authentication_body,
    build_manage_channel_close_apdu as core_build_manage_channel_close_apdu,
    build_manage_channel_open_apdu as core_build_manage_channel_open_apdu,
    build_select_isdp_apdu as core_build_select_isdp_apdu, bytes_to_hex,
    chain_apdu_request as core_chain_apdu_request, compute_hash_cc as core_compute_hash_cc,
    format_ci_pkid, frame_apdu_command as core_frame_apdu_command, hex_to_bytes,
    is_valid_notification_address, map_logical_channel_cla as core_map_logical_channel_cla,
    parse_authenticate_client_response as core_parse_authenticate_client_response,
    parse_get_bound_profile_package_response as core_parse_get_bound_profile_package_response,
    parse_initiate_authentication_response as core_parse_initiate_authentication_response,
    verify_ci_public_key_constraint, IsdpAid, NotificationPolicy as CoreNotificationPolicy,
    NotificationRecord as CoreNotificationRecord, NotificationStatus as CoreNotificationStatus,
    NotificationSyncPlan as CoreNotificationSyncPlan,
    ProfileDownloadSession as CoreProfileDownloadSession, ProfileOperation as CoreProfileOperation,
    ProfileStateTransition as CoreProfileStateTransition, Scp03KeySet, Scp03Session,
    SecurityLevel as CoreSecurityLevel, StoredNotification as CoreStoredNotification,
    X509Certificate,
};

#[frb]
pub struct NotificationPolicyDto {
    pub auto_send_install: bool,
    pub auto_remove_install: bool,
    pub auto_send_enable: bool,
    pub auto_remove_enable: bool,
    pub delete_without_sending_enable: bool,
    pub auto_send_disable: bool,
    pub auto_remove_disable: bool,
    pub delete_without_sending_disable: bool,
    pub auto_send_delete: bool,
    pub auto_remove_delete: bool,
}

#[frb]
pub struct NotificationActionDto {
    pub should_send: bool,
    pub should_remove: bool,
    pub should_delete_without_sending: bool,
}

#[frb]
pub struct NotificationRecordDto {
    pub seq_number: u32,
    pub operation_byte: u8,
    pub address: String,
    pub iccid: Option<String>,
    pub raw_der: Option<Vec<u8>>,
}

#[frb]
pub struct StoredNotificationDto {
    pub seq_number: u32,
    pub iccid: String,
    pub status: u8,
    pub delete_pending: bool,
}

#[frb]
pub struct NotificationJobDto {
    pub seq_number: u32,
    pub iccid: String,
    pub address: String,
    pub content_base64: String,
    pub should_remove_after_send: bool,
}

#[frb]
pub struct NotificationSyncPlanDto {
    pub prune_db_seqs: Vec<u32>,
    pub pending_unsent_count: usize,
    pub jobs: Vec<NotificationJobDto>,
    pub immediate_removals: Vec<u32>,
}

#[frb]
pub fn evaluate_notification_action(
    policy: NotificationPolicyDto,
    operation_byte: u8,
) -> NotificationActionDto {
    let core_policy = CoreNotificationPolicy {
        auto_send_install: policy.auto_send_install,
        auto_remove_install: policy.auto_remove_install,
        auto_send_enable: policy.auto_send_enable,
        auto_remove_enable: policy.auto_remove_enable,
        delete_without_sending_enable: policy.delete_without_sending_enable,
        auto_send_disable: policy.auto_send_disable,
        auto_remove_disable: policy.auto_remove_disable,
        delete_without_sending_disable: policy.delete_without_sending_disable,
        auto_send_delete: policy.auto_send_delete,
        auto_remove_delete: policy.auto_remove_delete,
    };
    let action = core_policy.evaluate_action(operation_byte);
    NotificationActionDto {
        should_send: action.should_send,
        should_remove: action.should_remove,
        should_delete_without_sending: action.should_delete_without_sending,
    }
}

#[frb]
pub fn plan_notification_sync(
    card_notifications: Vec<NotificationRecordDto>,
    stored_notifications: Vec<StoredNotificationDto>,
    policy: NotificationPolicyDto,
) -> NotificationSyncPlanDto {
    let core_policy = CoreNotificationPolicy {
        auto_send_install: policy.auto_send_install,
        auto_remove_install: policy.auto_remove_install,
        auto_send_enable: policy.auto_send_enable,
        auto_remove_enable: policy.auto_remove_enable,
        delete_without_sending_enable: policy.delete_without_sending_enable,
        auto_send_disable: policy.auto_send_disable,
        auto_remove_disable: policy.auto_remove_disable,
        delete_without_sending_disable: policy.delete_without_sending_disable,
        auto_send_delete: policy.auto_send_delete,
        auto_remove_delete: policy.auto_remove_delete,
    };

    let core_cards: Vec<CoreNotificationRecord> = card_notifications
        .into_iter()
        .map(|c| CoreNotificationRecord {
            seq_number: c.seq_number,
            operation: CoreProfileOperation::from(c.operation_byte),
            address: c.address,
            iccid: c.iccid,
            delivered: false,
            retry_count: 0,
            raw_der: c.raw_der,
        })
        .collect();

    let core_stored: Vec<CoreStoredNotification> = stored_notifications
        .into_iter()
        .map(|s| CoreStoredNotification {
            seq_number: s.seq_number,
            iccid: s.iccid,
            status: CoreNotificationStatus::from(s.status),
            delete_pending: s.delete_pending,
        })
        .collect();

    let plan = CoreNotificationSyncPlan::compute(&core_cards, &core_stored, &core_policy);

    let mut jobs = Vec::new();
    for list in plan.jobs_by_server.values() {
        for j in list {
            jobs.push(NotificationJobDto {
                seq_number: j.seq_number,
                iccid: j.iccid.clone(),
                address: j.address.clone(),
                content_base64: j.content_base64.clone(),
                should_remove_after_send: j.should_remove_after_send,
            });
        }
    }

    NotificationSyncPlanDto {
        prune_db_seqs: plan.prune_db_seqs,
        pending_unsent_count: plan.pending_unsent_count,
        jobs,
        immediate_removals: plan.immediate_removals,
    }
}

#[frb]
pub fn is_valid_notification_address_bridge(address: Option<String>) -> bool {
    address.is_some_and(|address| is_valid_notification_address(&address))
}

#[cfg(test)]
mod notification_address_bridge_tests {
    use super::is_valid_notification_address_bridge;

    #[test]
    fn validates_addresses_and_preserves_nullable_input_behavior() {
        assert!(is_valid_notification_address_bridge(Some(
            "smdp.example.com".to_string()
        )));
        assert!(is_valid_notification_address_bridge(Some(
            "rsp.example.test.".to_string()
        )));
        assert!(!is_valid_notification_address_bridge(Some(
            " malformed.example.com".to_string()
        )));
        assert!(!is_valid_notification_address_bridge(None));
    }
}

#[frb]
pub fn build_handle_notification_json(content_base64: String) -> String {
    format!(r#"{{"pendingNotification":"{content_base64}"}}"#)
}

#[frb]
pub fn compute_sgp22_hash_cc(confirmation_code: String, transaction_id_octets: Vec<u8>) -> Vec<u8> {
    core_compute_hash_cc(&confirmation_code, &transaction_id_octets).to_vec()
}

// ---------------------------------------------------------------------------
// ISO 7816-4 APDU Transport & Logical Channel Bridge
// ---------------------------------------------------------------------------

#[frb]
pub fn map_channel_cla(cla: u8, channel: u8) -> Result<u8, String> {
    core_map_logical_channel_cla(cla, channel).map_err(|e| e.to_string())
}

#[frb]
pub fn build_open_logical_channel_apdu() -> Vec<u8> {
    core_build_manage_channel_open_apdu().to_vec()
}

#[frb]
pub fn build_close_logical_channel_apdu(channel: u8) -> Result<Vec<u8>, String> {
    core_build_manage_channel_close_apdu(channel)
        .map(|arr| arr.to_vec())
        .map_err(|e| e.to_string())
}

#[frb]
pub fn frame_apdu(
    cla: u8,
    ins: u8,
    p1: u8,
    p2: u8,
    data: Option<Vec<u8>>,
    le: Option<usize>,
) -> Vec<u8> {
    core_frame_apdu_command(cla, ins, p1, p2, data.as_deref(), le)
}

#[frb]
pub fn chain_apdu_packets(
    cla: u8,
    ins: u8,
    p1: u8,
    p2: u8,
    data: Vec<u8>,
    le: Option<usize>,
    mss: Option<usize>,
) -> Vec<Vec<u8>> {
    core_chain_apdu_request(cla, ins, p1, p2, &data, le, mss.unwrap_or(255))
}

// ---------------------------------------------------------------------------
// GSMA SGP.22 ES9+ Protocol Bridge
// ---------------------------------------------------------------------------

#[frb]
pub struct InitiateAuthResponseDto {
    pub transaction_id: String,
    pub server_signed1: Vec<u8>,
    pub server_signature1: Vec<u8>,
    pub euicc_ci_pk_id_to_be_used: Option<Vec<u8>>,
    pub server_certificate: Vec<u8>,
}

#[frb]
pub struct AuthenticateClientResponseDto {
    pub transaction_id: String,
    pub profile_metadata: Option<Vec<u8>>,
    pub smdp_signed2: Option<Vec<u8>>,
    pub smdp_signature2: Option<Vec<u8>>,
    pub smdp_certificate: Option<Vec<u8>>,
}

#[frb]
pub struct GetBppResponseDto {
    pub transaction_id: String,
    pub bound_profile_package: Vec<u8>,
}

#[frb]
pub fn build_initiate_auth_req(
    smdp_address: String,
    euicc_challenge_b64: String,
    euicc_info1_b64: String,
) -> String {
    core_build_initiate_authentication_body(&smdp_address, &euicc_challenge_b64, &euicc_info1_b64)
}

#[frb]
pub fn parse_initiate_auth_resp(
    json_str: String,
    http_status: Option<u16>,
) -> Result<InitiateAuthResponseDto, String> {
    let resp = core_parse_initiate_authentication_response(&json_str, http_status)
        .map_err(|e| e.to_string())?;
    Ok(InitiateAuthResponseDto {
        transaction_id: resp.transaction_id,
        server_signed1: resp.server_signed1,
        server_signature1: resp.server_signature1,
        euicc_ci_pk_id_to_be_used: resp.euicc_ci_pk_id_to_be_used,
        server_certificate: resp.server_certificate,
    })
}

#[frb]
pub fn build_authenticate_client_req(
    transaction_id: String,
    auth_server_resp_b64: String,
) -> String {
    core_build_authenticate_client_body(&transaction_id, &auth_server_resp_b64)
}

#[frb]
pub fn parse_authenticate_client_resp(
    json_str: String,
    http_status: Option<u16>,
) -> Result<AuthenticateClientResponseDto, String> {
    let resp = core_parse_authenticate_client_response(&json_str, http_status)
        .map_err(|e| e.to_string())?;
    Ok(AuthenticateClientResponseDto {
        transaction_id: resp.transaction_id,
        profile_metadata: resp.profile_metadata,
        smdp_signed2: resp.smdp_signed2,
        smdp_signature2: resp.smdp_signature2,
        smdp_certificate: resp.smdp_certificate,
    })
}

#[frb]
pub fn build_get_bpp_req(transaction_id: String, prepare_download_resp_b64: String) -> String {
    core_build_get_bound_profile_package_body(&transaction_id, &prepare_download_resp_b64)
}

#[frb]
pub fn parse_get_bpp_resp(
    json_str: String,
    http_status: Option<u16>,
) -> Result<GetBppResponseDto, String> {
    let resp = core_parse_get_bound_profile_package_response(&json_str, http_status)
        .map_err(|e| e.to_string())?;
    Ok(GetBppResponseDto {
        transaction_id: resp.transaction_id,
        bound_profile_package: resp.bound_profile_package,
    })
}

#[frb]
pub fn build_cancel_session_req(transaction_id: String, cancel_resp_b64: String) -> String {
    core_build_cancel_session_body(&transaction_id, &cancel_resp_b64)
}

// ---------------------------------------------------------------------------
// GSMA SGP.22 Download Session & State Transition Bridge
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
#[frb]
pub struct DownloadSessionStateDto {
    pub smdp_address: String,
    pub matching_id: Option<String>,
    pub confirmation_code: Option<String>,
    pub euicc_info1_bytes: Option<Vec<u8>>,
    pub euicc_challenge: Option<Vec<u8>>,
    pub transaction_id_hex: Option<String>,
    pub transaction_id_octets: Option<Vec<u8>>,
    pub server_certificate: Option<Vec<u8>>,
    pub authenticate_server_response: Option<Vec<u8>>,
    pub authenticate_client_status: Option<String>,
    pub smdp_signed2: Option<Vec<u8>>,
    pub smdp_signature2: Option<Vec<u8>>,
    pub profile_metadata: Option<Vec<u8>>,
    pub smdp_certificate: Option<Vec<u8>>,
    pub is_authenticated: bool,
}

impl DownloadSessionStateDto {
    fn from_core(session: &CoreProfileDownloadSession) -> Self {
        Self {
            smdp_address: session.smdp_address().to_string(),
            matching_id: session.matching_id().map(ToString::to_string),
            confirmation_code: session.confirmation_code().map(ToString::to_string),
            euicc_info1_bytes: session.euicc_info1().map(|b| b.to_vec()),
            euicc_challenge: session.euicc_challenge().map(|b| b.to_vec()),
            transaction_id_hex: session.get_transaction_id_hex().map(ToString::to_string),
            transaction_id_octets: session.get_transaction_id().map(|b| b.to_vec()),
            server_certificate: session.server_certificate().map(|b| b.to_vec()),
            authenticate_server_response: session
                .authenticate_server_response()
                .map(|b| b.to_vec()),
            authenticate_client_status: session
                .authenticate_client_status()
                .map(ToString::to_string),
            smdp_signed2: session.get_smdp_signed2().map(|b| b.to_vec()),
            smdp_signature2: session.get_smdp_signature2().map(|b| b.to_vec()),
            profile_metadata: session.get_profile_metadata().map(|b| b.to_vec()),
            smdp_certificate: session.smdp_certificate().map(|b| b.to_vec()),
            is_authenticated: session.is_client_authenticated_successfully(),
        }
    }

    fn to_core(&self) -> CoreProfileDownloadSession {
        let mut session = CoreProfileDownloadSession::new(
            &self.smdp_address,
            self.matching_id.clone(),
            self.confirmation_code.clone(),
        );
        if let Some(ref info1) = self.euicc_info1_bytes {
            session.process_euicc_info1(info1);
        }
        if let Some(ref challenge) = self.euicc_challenge {
            session.process_euicc_challenge(challenge);
        }
        if let (Some(ref tx_hex), Some(ref cert)) =
            (&self.transaction_id_hex, &self.server_certificate)
        {
            let _ = session.process_initiate_authentication_response(tx_hex, cert);
        }
        if let Some(ref auth_resp) = self.authenticate_server_response {
            session.process_authenticate_server_response(auth_resp);
        }
        if let Some(ref status) = self.authenticate_client_status {
            session.process_authenticate_client_response(
                status,
                self.smdp_signed2.as_deref(),
                self.smdp_signature2.as_deref(),
                self.profile_metadata.as_deref(),
                self.smdp_certificate.as_deref(),
            );
        }
        session
    }
}

#[frb]
pub fn create_download_session(
    smdp_address: String,
    matching_id: Option<String>,
    confirmation_code: Option<String>,
) -> DownloadSessionStateDto {
    let session = CoreProfileDownloadSession::new(smdp_address, matching_id, confirmation_code);
    DownloadSessionStateDto::from_core(&session)
}

#[frb]
pub fn session_process_euicc_info1(
    session: DownloadSessionStateDto,
    info1: Vec<u8>,
) -> DownloadSessionStateDto {
    let mut core = session.to_core();
    core.process_euicc_info1(&info1);
    DownloadSessionStateDto::from_core(&core)
}

#[frb]
pub fn session_process_euicc_challenge(
    session: DownloadSessionStateDto,
    challenge: Vec<u8>,
) -> DownloadSessionStateDto {
    let mut core = session.to_core();
    core.process_euicc_challenge(&challenge);
    DownloadSessionStateDto::from_core(&core)
}

#[frb]
pub fn session_validate_initiate_auth(session: DownloadSessionStateDto) -> Result<(), String> {
    let core = session.to_core();
    core.validate_initiate_authentication_prerequisites()
        .map_err(|e| e.to_string())
}

#[frb]
pub fn session_process_initiate_auth_resp(
    session: DownloadSessionStateDto,
    transaction_id_hex: String,
    server_certificate: Vec<u8>,
) -> Result<DownloadSessionStateDto, String> {
    let mut core = session.to_core();
    core.process_initiate_authentication_response(&transaction_id_hex, &server_certificate)
        .map_err(|e| e.to_string())?;
    Ok(DownloadSessionStateDto::from_core(&core))
}

#[frb]
pub fn session_compute_hash_cc(session: DownloadSessionStateDto) -> Result<Vec<u8>, String> {
    let core = session.to_core();
    core.compute_hash_cc()
        .map(|hash| hash.to_vec())
        .map_err(|e| e.to_string())
}

#[frb]
pub fn session_process_auth_server_resp(
    session: DownloadSessionStateDto,
    resp_bytes: Vec<u8>,
) -> DownloadSessionStateDto {
    let mut core = session.to_core();
    core.process_authenticate_server_response(&resp_bytes);
    DownloadSessionStateDto::from_core(&core)
}

#[frb]
pub fn session_process_auth_client_resp(
    session: DownloadSessionStateDto,
    status: String,
    smdp_signed2: Option<Vec<u8>>,
    smdp_signature2: Option<Vec<u8>>,
    profile_metadata: Option<Vec<u8>>,
    smdp_certificate: Option<Vec<u8>>,
) -> DownloadSessionStateDto {
    let mut core = session.to_core();
    core.process_authenticate_client_response(
        status,
        smdp_signed2.as_deref(),
        smdp_signature2.as_deref(),
        profile_metadata.as_deref(),
        smdp_certificate.as_deref(),
    );
    DownloadSessionStateDto::from_core(&core)
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[frb]
pub struct StateTransitionResultDto {
    pub converged: bool,
    pub command_attempts: usize,
    pub verification_attempts: usize,
    pub actual_enabled: Option<bool>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[frb]
pub struct VerificationReadDto {
    pub is_ok: bool,
    pub actual_enabled: Option<bool>,
    pub error: Option<String>,
}

#[frb]
pub fn evaluate_profile_convergence(
    target_enabled: bool,
    command_attempt: usize,
    verification_reads: Vec<VerificationReadDto>,
    max_command_attempts: Option<usize>,
    max_verification_attempts: Option<usize>,
    max_transient_failures_without_read: Option<usize>,
) -> StateTransitionResultDto {
    let transition = CoreProfileStateTransition::new(
        max_command_attempts.unwrap_or(2),
        max_verification_attempts.unwrap_or(6),
        max_transient_failures_without_read.unwrap_or(2),
    );
    let reads: Vec<Result<Option<bool>, String>> = verification_reads
        .into_iter()
        .map(|r| {
            if r.is_ok {
                Ok(r.actual_enabled)
            } else {
                Err(r
                    .error
                    .unwrap_or_else(|| "Transient read error".to_string()))
            }
        })
        .collect();

    match transition.evaluate_convergence(target_enabled, command_attempt, &reads) {
        Ok(res) => StateTransitionResultDto {
            converged: true,
            command_attempts: res.command_attempts,
            verification_attempts: res.verification_attempts,
            actual_enabled: Some(res.actual_enabled),
            error_message: None,
        },
        Err(err) => StateTransitionResultDto {
            converged: false,
            command_attempts: command_attempt,
            verification_attempts: reads.len(),
            actual_enabled: match &err {
                nlpa_core::StateTransitionError::NonConvergence { actual_enabled, .. } => {
                    *actual_enabled
                }
                _ => None,
            },
            error_message: Some(err.to_string()),
        },
    }
}

// ---------------------------------------------------------------------------
// GSMA SGP.22 X.509 PKIX & CI Public Key Constraint Bridge
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
#[frb]
pub struct CertificateDetailsDto {
    pub common_name: Option<String>,
    pub organization: Option<String>,
    pub country: Option<String>,
    pub serial_number_hex: String,
    pub ski_hex: String,
    pub aki_hex: Option<String>,
    pub is_ca: bool,
}

fn parse_cert_bytes(cert_str: &str) -> Result<X509Certificate, String> {
    let trimmed = cert_str.trim();
    if trimmed.starts_with("-----BEGIN CERTIFICATE-----") {
        X509Certificate::from_pem(trimmed).map_err(|e| e.to_string())
    } else if let Ok(bytes) = hex_to_bytes(trimmed) {
        X509Certificate::from_der(&bytes).map_err(|e| e.to_string())
    } else if let Ok(bytes) = base64_decode(trimmed) {
        X509Certificate::from_der(&bytes).map_err(|e| e.to_string())
    } else {
        Err("Invalid certificate format (expected PEM, DER hex, or Base64)".to_string())
    }
}

#[frb]
pub fn get_certificate_details(cert_str: String) -> Result<CertificateDetailsDto, String> {
    let cert = parse_cert_bytes(&cert_str)?;
    let ski = cert.ski();
    let aki_hex = cert
        .authority_key_identifier
        .as_ref()
        .map(|aki| bytes_to_hex(aki).to_ascii_uppercase());
    let serial_number_hex = cert.serial_number_hex();

    Ok(CertificateDetailsDto {
        common_name: cert.subject.common_name,
        organization: cert.subject.organization,
        country: cert.subject.country,
        serial_number_hex,
        ski_hex: format_ci_pkid(&ski),
        aki_hex,
        is_ca: cert.is_ca,
    })
}

#[frb]
pub fn verify_smdp_certificate(
    cert_str: String,
    allowed_ci_pkids_hex: Vec<String>,
) -> Result<bool, String> {
    let cert = parse_cert_bytes(&cert_str)?;
    let mut allowed_bytes = Vec::with_capacity(allowed_ci_pkids_hex.len());
    for s in allowed_ci_pkids_hex {
        let b =
            hex_to_bytes(&s).map_err(|e| format!("Invalid hex in allowed CI PKID list: {e}"))?;
        allowed_bytes.push(b);
    }
    Ok(verify_ci_public_key_constraint(&cert, &allowed_bytes))
}

// ---------------------------------------------------------------------------
// GSMA SGP.22 ISD-P & GlobalPlatform SCP03 Bridge
// ---------------------------------------------------------------------------

#[frb]
pub fn isdp_select_apdu(aid_hex: String) -> Result<Vec<u8>, String> {
    let aid = IsdpAid::from_hex(&aid_hex).map_err(|e| e.to_string())?;
    Ok(core_build_select_isdp_apdu(&aid))
}

#[frb]
pub fn isdp_delete_apdu(aid_hex: String) -> Result<Vec<u8>, String> {
    let aid = IsdpAid::from_hex(&aid_hex).map_err(|e| e.to_string())?;
    Ok(core_build_delete_isdp_apdu(&aid))
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[frb]
pub struct Scp03AuthResultDto {
    pub host_cryptogram_hex: String,
    pub external_authenticate_apdu: Vec<u8>,
}

#[frb]
pub fn scp03_initialize_update_apdu(
    host_challenge_hex: String,
    key_version: u8,
    key_id: u8,
) -> Result<Vec<u8>, String> {
    let challenge_bytes = hex_to_bytes(&host_challenge_hex).map_err(|e| e.to_string())?;
    if challenge_bytes.len() != 8 {
        return Err("Host challenge must be exactly 8 bytes (16 hex chars)".to_string());
    }
    let mut challenge = [0u8; 8];
    challenge.copy_from_slice(&challenge_bytes);

    let mut session = Scp03Session::new(CoreSecurityLevel::CMac);
    Ok(session.build_initialize_update(challenge, key_version, key_id))
}

#[frb]
pub fn scp03_authenticate_session(
    host_challenge_hex: String,
    card_response_hex: String,
    static_enc_key_hex: String,
    static_mac_key_hex: String,
    security_level: u8,
) -> Result<Scp03AuthResultDto, String> {
    let challenge_bytes = hex_to_bytes(&host_challenge_hex).map_err(|e| e.to_string())?;
    if challenge_bytes.len() != 8 {
        return Err("Host challenge must be exactly 8 bytes".to_string());
    }
    let mut host_challenge = [0u8; 8];
    host_challenge.copy_from_slice(&challenge_bytes);

    let card_response = hex_to_bytes(&card_response_hex).map_err(|e| e.to_string())?;
    let k_enc = hex_to_bytes(&static_enc_key_hex).map_err(|e| e.to_string())?;
    let k_mac = hex_to_bytes(&static_mac_key_hex).map_err(|e| e.to_string())?;

    let key_set = Scp03KeySet::new(&k_enc, &k_mac, 0x01, 0x01).map_err(|e| e.to_string())?;
    let sec_level = CoreSecurityLevel::from_u8(security_level).map_err(|e| e.to_string())?;

    let mut session = Scp03Session::new(sec_level);
    // Arm session with initialize update
    session.build_initialize_update(host_challenge, key_set.key_version, key_set.key_id);

    // Process card response, verify card cryptogram, compute host cryptogram
    let host_cryptogram = session
        .process_initialize_update_response(&card_response, &key_set)
        .map_err(|e| e.to_string())?;

    // Build external authenticate APDU
    let external_authenticate_apdu = session
        .build_external_authenticate(host_cryptogram)
        .map_err(|e| e.to_string())?;

    Ok(Scp03AuthResultDto {
        host_cryptogram_hex: bytes_to_hex(&host_cryptogram).to_ascii_uppercase(),
        external_authenticate_apdu,
    })
}

// ---------------------------------------------------------------------------
// GSMA SGP.22 ES11 SM-DS Discovery & Profile Size Prediction Bridge
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
#[frb]
pub struct DiscoveredEventDto {
    pub event_id: String,
    pub address: String,
    pub activation_code: String,
}

#[frb]
pub fn clean_smds_host_address(raw_address: String) -> String {
    nlpa_core::clean_smds_host(&raw_address)
}

#[frb]
pub fn build_smds_init_auth(smds_address: String, challenge: String, info1: String) -> String {
    nlpa_core::build_smds_initiate_auth_request(&smds_address, &challenge, &info1)
}

#[frb]
pub fn build_smds_auth_client(transaction_id: String, auth_server_resp: String) -> String {
    nlpa_core::build_smds_auth_client_request(&transaction_id, &auth_server_resp)
}

#[frb]
pub fn parse_smds_events(
    json_response: String,
    http_status: Option<u16>,
) -> Result<Vec<DiscoveredEventDto>, String> {
    let events = nlpa_core::parse_smds_discovery_response(&json_response, http_status)
        .map_err(|e| e.to_string())?;
    Ok(events
        .into_iter()
        .map(|e| DiscoveredEventDto {
            activation_code: e.activation_code(),
            event_id: e.event_id,
            address: e.address,
        })
        .collect())
}

#[frb]
pub fn predict_profile_size(
    catalog_json: String,
    eid: Option<String>,
    smdp_address: Option<String>,
    plmn: Option<String>,
    service_provider_name: Option<String>,
) -> Result<Option<i64>, String> {
    let catalog =
        nlpa_core::ProfileSizeCatalog::from_json(&catalog_json).map_err(|e| e.to_string())?;
    Ok(catalog.predict_size(
        eid.as_deref(),
        smdp_address.as_deref(),
        plmn.as_deref(),
        service_provider_name.as_deref(),
    ))
}
