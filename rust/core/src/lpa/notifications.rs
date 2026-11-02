//! GSMA SGP.22 eUICC Notification Dispatcher and Queue Manager.

use std::collections::BTreeMap;

use crate::asn1::encode_tlv;
use crate::asn1::rsp::{NotificationMetadata, TAG_HANDLE_NOTIFICATION};
use crate::lpa::es10::wrap_store_data_apdu;

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
        match op {
            1 => Self::Install,
            2 => Self::Enable,
            3 => Self::Disable,
            4 => Self::Delete,
            other => Self::Other(other),
        }
    }
}

impl ProfileOperation {
    #[must_use]
    pub fn to_code(self) -> u8 {
        match self {
            Self::Install => 1,
            Self::Enable => 2,
            Self::Disable => 3,
            Self::Delete => 4,
            Self::Other(o) => o,
        }
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
