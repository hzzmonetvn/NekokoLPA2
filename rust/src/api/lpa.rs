//! GSMA SGP.22 LPA operations and notification management bridged to Flutter.

use flutter_rust_bridge::frb;
use nlpa_core::{
    compute_hash_cc as core_compute_hash_cc, NotificationPolicy as CoreNotificationPolicy,
    NotificationRecord as CoreNotificationRecord, NotificationStatus as CoreNotificationStatus,
    NotificationSyncPlan as CoreNotificationSyncPlan, ProfileOperation as CoreProfileOperation,
    StoredNotification as CoreStoredNotification,
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
