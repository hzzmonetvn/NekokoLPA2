//! GSMA SGP.22 eUICC Notification Dispatcher, Policy Engine, and Queue Manager.
//!
//! Provides native Rust models and evaluation routines for ES9+ `handleNotification`
//! and ES10b `RemoveNotificationFromList` workflows:
//! - Profile management operation bitmasks (Install, Enable, Disable, Delete).
//! - Policy evaluation determining auto-send, auto-remove, and delete-without-sending actions.
//! - ES9+ JSON payload generation (`{"pendingNotification": "<base64>"}`).
//! - Notification sync planning between card-reported notifications and local store records.

use std::collections::{BTreeMap, BTreeSet};

use crate::asn1::encode_tlv;
use crate::asn1::rsp::{NotificationMetadata, TAG_HANDLE_NOTIFICATION};
use crate::crypto::base64_encode;
use crate::lpa::es10::wrap_store_data_apdu;

// =========================================================================
// SGP.22 Notification Event Bit Flags (ProfileManagementOperation)
// =========================================================================

pub const NOTIFICATION_INSTALL: u8 = 1 << 0; // 0x01
pub const NOTIFICATION_LOCAL_ENABLE: u8 = 1 << 1; // 0x02
pub const NOTIFICATION_LOCAL_DISABLE: u8 = 1 << 2; // 0x04
pub const NOTIFICATION_LOCAL_DELETE: u8 = 1 << 3; // 0x08
pub const NOTIFICATION_RPM_ENABLE: u8 = 1 << 4; // 0x10
pub const NOTIFICATION_RPM_DISABLE: u8 = 1 << 5; // 0x20
pub const NOTIFICATION_RPM_DELETE: u8 = 1 << 6; // 0x40
pub const NOTIFICATION_LOAD_RPM_PACKAGE: u8 = 1 << 7; // 0x80

/// Profile management operation types defined in GSMA SGP.22.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileOperation {
    Install,
    Enable,
    Disable,
    Delete,
    Other(u8),
}

impl From<u8> for ProfileOperation {
    fn from(op: u8) -> Self {
        if (op & NOTIFICATION_INSTALL) != 0 || op == 1 {
            Self::Install
        } else if (op & NOTIFICATION_LOCAL_ENABLE) != 0 || op == 2 {
            Self::Enable
        } else if (op & NOTIFICATION_LOCAL_DISABLE) != 0 || op == 3 {
            Self::Disable
        } else if (op & NOTIFICATION_LOCAL_DELETE) != 0 || op == 4 {
            Self::Delete
        } else {
            Self::Other(op)
        }
    }
}

impl ProfileOperation {
    #[must_use]
    pub const fn to_code(self) -> u8 {
        match self {
            Self::Install => NOTIFICATION_INSTALL,
            Self::Enable => NOTIFICATION_LOCAL_ENABLE,
            Self::Disable => NOTIFICATION_LOCAL_DISABLE,
            Self::Delete => NOTIFICATION_LOCAL_DELETE,
            Self::Other(o) => o,
        }
    }
}

/// Notification lifecycle status in local database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationStatus {
    Pending,
    Sent,
    Failed,
    DeletedWithoutSending,
    Other(u8),
}

impl From<u8> for NotificationStatus {
    fn from(s: u8) -> Self {
        match s {
            0 => Self::Pending,
            1 => Self::Sent,
            2 => Self::Failed,
            3 => Self::DeletedWithoutSending,
            other => Self::Other(other),
        }
    }
}

impl NotificationStatus {
    #[must_use]
    pub const fn to_u8(self) -> u8 {
        match self {
            Self::Pending => 0,
            Self::Sent => 1,
            Self::Failed => 2,
            Self::DeletedWithoutSending => 3,
            Self::Other(o) => o,
        }
    }

    #[must_use]
    pub const fn is_sent(self) -> bool {
        matches!(self, Self::Sent)
    }
}

/// Action decision evaluated for a specific notification based on user policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NotificationAction {
    pub should_send: bool,
    pub should_remove: bool,
    pub should_delete_without_sending: bool,
}

/// User automation policy controlling notification dispatch and cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NotificationPolicy {
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

impl NotificationPolicy {
    /// Returns `true` if any automated send or delete action is enabled.
    #[must_use]
    pub const fn any_enabled(&self) -> bool {
        self.auto_send_install
            || self.auto_send_enable
            || self.delete_without_sending_enable
            || self.auto_send_disable
            || self.delete_without_sending_disable
            || self.auto_send_delete
    }

    /// Evaluates the automated action for a given operation byte (or bitmask).
    #[must_use]
    pub fn evaluate_action(&self, operation_byte: u8) -> NotificationAction {
        let mut action = NotificationAction::default();

        if (operation_byte & NOTIFICATION_INSTALL) != 0 || operation_byte == 1 {
            action.should_send = self.auto_send_install;
            action.should_remove = self.auto_remove_install;
        } else if (operation_byte & NOTIFICATION_LOCAL_ENABLE) != 0 || operation_byte == 2 {
            action.should_send = self.auto_send_enable;
            action.should_remove = self.auto_remove_enable;
            action.should_delete_without_sending = self.delete_without_sending_enable;
        } else if (operation_byte & NOTIFICATION_LOCAL_DISABLE) != 0 || operation_byte == 3 {
            action.should_send = self.auto_send_disable;
            action.should_remove = self.auto_remove_disable;
            action.should_delete_without_sending = self.delete_without_sending_disable;
        } else if (operation_byte & NOTIFICATION_LOCAL_DELETE) != 0 || operation_byte == 4 {
            action.should_send = self.auto_send_delete;
            action.should_remove = self.auto_remove_delete;
        }

        action
    }
}

/// A stored eUICC notification record pending delivery to SM-DP+ or SM-DS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationRecord {
    pub seq_number: u32,
    pub operation: ProfileOperation,
    pub address: String,
    pub iccid: Option<String>,
    pub delivered: bool,
    pub retry_count: u32,
    pub raw_der: Option<Vec<u8>>,
}

impl NotificationRecord {
    #[must_use]
    pub fn from_metadata(meta: NotificationMetadata) -> Self {
        Self {
            seq_number: meta.seq_number,
            operation: ProfileOperation::from(meta.profile_management_operation),
            address: meta.notification_address,
            iccid: meta.iccid,
            delivered: false,
            retry_count: 0,
            raw_der: None,
        }
    }

    /// Builds the ES10b.HandleNotification APDU to report acknowledgment status back to eUICC.
    #[must_use]
    pub fn build_ack_apdu(&self) -> Vec<u8> {
        let payload = crate::asn1::encode_integer(i64::from(self.seq_number));
        let tlv = encode_tlv(TAG_HANDLE_NOTIFICATION, &payload);
        wrap_store_data_apdu(&tlv, true, true)
    }
}

/// An outbound transmission job targeted at an SM-DP+ server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationJob {
    pub seq_number: u32,
    pub iccid: String,
    pub address: String,
    pub content_base64: String,
    pub should_remove_after_send: bool,
}

impl NotificationJob {
    /// Formats the GSMA SGP.22 ES9+ `handleNotification` JSON request body.
    #[must_use]
    pub fn build_es9_request_json(&self) -> String {
        format!(r#"{{"pendingNotification":"{}"}}"#, self.content_base64)
    }
}

/// Light database record for synchronization comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredNotification {
    pub seq_number: u32,
    pub iccid: String,
    pub status: NotificationStatus,
    pub delete_pending: bool,
}

/// Comprehensive plan generated by comparing card notifications with local store.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NotificationSyncPlan {
    /// Sequence numbers of stale unsent/failed DB entries to prune because they no longer exist on card.
    pub prune_db_seqs: Vec<u32>,
    /// Number of notifications on current card that are not yet successfully sent.
    pub pending_unsent_count: usize,
    /// Outbound jobs grouped by notification address (SM-DP+ FQDN).
    pub jobs_by_server: BTreeMap<String, Vec<NotificationJob>>,
    /// Notifications to remove immediately from the card.
    pub immediate_removals: Vec<u32>,
    /// Mapping of sequence numbers to ICCID for reliable DB updates.
    pub seq_to_iccid: BTreeMap<u32, String>,
}

impl NotificationSyncPlan {
    /// Computes the synchronization and dispatch plan.
    #[must_use]
    pub fn compute(
        card_notifications: &[NotificationRecord],
        stored_notifications: &[StoredNotification],
        policy: &NotificationPolicy,
    ) -> Self {
        let mut plan = Self::default();

        let card_seq_set: BTreeSet<u32> = card_notifications.iter().map(|n| n.seq_number).collect();
        let stored_by_seq: BTreeMap<u32, &StoredNotification> = stored_notifications
            .iter()
            .map(|s| (s.seq_number, s))
            .collect();

        // 1. Pruning stale records from DB
        for s in stored_notifications {
            if (s.status == NotificationStatus::Pending || s.status == NotificationStatus::Failed)
                && !card_seq_set.contains(&s.seq_number)
            {
                plan.prune_db_seqs.push(s.seq_number);
            }
        }

        // 2. Process each card notification
        for n in card_notifications {
            let seq = n.seq_number;
            let iccid = n.iccid.clone().unwrap_or_else(|| "Unknown".to_string());
            plan.seq_to_iccid.insert(seq, iccid.clone());

            let is_already_sent = stored_by_seq.get(&seq).is_some_and(|s| s.status.is_sent());

            if !is_already_sent {
                plan.pending_unsent_count += 1;
            }

            let op_code = n.operation.to_code();
            let action = policy.evaluate_action(op_code);

            if action.should_delete_without_sending {
                plan.immediate_removals.push(seq);
                continue;
            }

            if action.should_send && !n.address.is_empty() {
                if is_already_sent {
                    if action.should_remove {
                        plan.immediate_removals.push(seq);
                    }
                } else {
                    let content_b64 = n.raw_der.as_deref().map(base64_encode).unwrap_or_default();

                    let job = NotificationJob {
                        seq_number: seq,
                        iccid,
                        address: n.address.clone(),
                        content_base64: content_b64,
                        should_remove_after_send: action.should_remove,
                    };

                    plan.jobs_by_server
                        .entry(n.address.clone())
                        .or_default()
                        .push(job);
                }
            } else if action.should_remove {
                plan.immediate_removals.push(seq);
            }
        }

        // Deduplicate removals while keeping order
        let mut seen = BTreeSet::new();
        plan.immediate_removals.retain(|seq| seen.insert(*seq));

        plan
    }
}

/// Priority-ordered notification queue.
#[derive(Debug, Clone, Default)]
pub struct NotificationQueue {
    notifications: BTreeMap<u32, NotificationRecord>,
}

impl NotificationQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or updates a notification record.
    pub fn enqueue(&mut self, record: NotificationRecord) {
        self.notifications.insert(record.seq_number, record);
    }

    /// Returns a list of notifications that are not yet marked as delivered.
    #[must_use]
    pub fn pending(&self) -> Vec<&NotificationRecord> {
        self.notifications
            .values()
            .filter(|n| !n.delivered)
            .collect()
    }

    /// Marks a notification sequence number as delivered.
    pub fn mark_delivered(&mut self, seq_number: u32) -> bool {
        if let Some(record) = self.notifications.get_mut(&seq_number) {
            record.delivered = true;
            true
        } else {
            false
        }
    }

    /// Records a failed transmission attempt, incrementing retry count.
    pub fn record_retry(&mut self, seq_number: u32) {
        if let Some(record) = self.notifications.get_mut(&seq_number) {
            record.retry_count += 1;
        }
    }

    /// Number of total tracked notifications.
    #[must_use]
    pub fn len(&self) -> usize {
        self.notifications.len()
    }

    /// Returns `true` if the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }
}
