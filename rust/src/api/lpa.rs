//! GSMA SGP.22 LPA operations and notification management bridged to Flutter.

use flutter_rust_bridge::frb;
use nlpa_core::{
    build_authenticate_client_body as core_build_authenticate_client_body,
    build_cancel_session_body as core_build_cancel_session_body,
    build_get_bound_profile_package_body as core_build_get_bound_profile_package_body,
    build_initiate_authentication_body as core_build_initiate_authentication_body,
    build_manage_channel_close_apdu as core_build_manage_channel_close_apdu,
    build_manage_channel_open_apdu as core_build_manage_channel_open_apdu,
    chain_apdu_request as core_chain_apdu_request, compute_hash_cc as core_compute_hash_cc,
    frame_apdu_command as core_frame_apdu_command,
    map_logical_channel_cla as core_map_logical_channel_cla,
    parse_authenticate_client_response as core_parse_authenticate_client_response,
    parse_get_bound_profile_package_response as core_parse_get_bound_profile_package_response,
    parse_initiate_authentication_response as core_parse_initiate_authentication_response,
    NotificationPolicy as CoreNotificationPolicy, NotificationRecord as CoreNotificationRecord,
    NotificationStatus as CoreNotificationStatus, NotificationSyncPlan as CoreNotificationSyncPlan,
    ProfileOperation as CoreProfileOperation, StoredNotification as CoreStoredNotification,
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
