use nlpa_core::{
    Asn1Reader, DeleteNotificationStatus, NotificationAction, NotificationJob, NotificationPolicy,
    NotificationRecord, NotificationSentResponse, NotificationStatus, NotificationSyncPlan,
    ProfileOperation, StoredNotification, NOTIFICATION_INSTALL, NOTIFICATION_LOCAL_DELETE,
    NOTIFICATION_LOCAL_DISABLE, NOTIFICATION_LOCAL_ENABLE,
};

#[test]
fn test_notification_operations_and_flags() {
    assert_eq!(
        ProfileOperation::from(NOTIFICATION_INSTALL),
        ProfileOperation::Install
    );
    assert_eq!(
        ProfileOperation::from(NOTIFICATION_LOCAL_ENABLE),
        ProfileOperation::Enable
    );
    assert_eq!(
        ProfileOperation::from(NOTIFICATION_LOCAL_DISABLE),
        ProfileOperation::Disable
    );
    assert_eq!(
        ProfileOperation::from(NOTIFICATION_LOCAL_DELETE),
        ProfileOperation::Delete
    );
    assert_eq!(ProfileOperation::from(0x80), ProfileOperation::Other(0x80));

    assert_eq!(ProfileOperation::Install.to_code(), NOTIFICATION_INSTALL);
    assert_eq!(
        ProfileOperation::Enable.to_code(),
        NOTIFICATION_LOCAL_ENABLE
    );
    assert_eq!(
        ProfileOperation::Disable.to_code(),
        NOTIFICATION_LOCAL_DISABLE
    );
    assert_eq!(
        ProfileOperation::Delete.to_code(),
        NOTIFICATION_LOCAL_DELETE
    );
}

#[test]
fn test_notification_policy_evaluation() {
    let mut policy = NotificationPolicy::default();
    assert!(!policy.any_enabled());

    policy.auto_send_install = true;
    policy.auto_remove_install = true;
    policy.delete_without_sending_enable = true;
    policy.auto_send_delete = true;
    policy.auto_remove_delete = false;

    assert!(policy.any_enabled());

    // Install evaluation
    let action_install = policy.evaluate_action(NOTIFICATION_INSTALL);
    assert_eq!(
        action_install,
        NotificationAction {
            should_send: true,
            should_remove: true,
            should_delete_without_sending: false,
        }
    );

    // Enable evaluation (delete without sending)
    let action_enable = policy.evaluate_action(NOTIFICATION_LOCAL_ENABLE);
    assert_eq!(
        action_enable,
        NotificationAction {
            should_send: false,
            should_remove: false,
            should_delete_without_sending: true,
        }
    );

    // Disable evaluation (not configured)
    let action_disable = policy.evaluate_action(NOTIFICATION_LOCAL_DISABLE);
    assert_eq!(
        action_disable,
        NotificationAction {
            should_send: false,
            should_remove: false,
            should_delete_without_sending: false,
        }
    );

    // Delete evaluation (send only, no auto remove)
    let action_delete = policy.evaluate_action(NOTIFICATION_LOCAL_DELETE);
    assert_eq!(
        action_delete,
        NotificationAction {
            should_send: true,
            should_remove: false,
            should_delete_without_sending: false,
        }
    );
}

#[test]
fn test_notification_sent_response_asn1_roundtrip() {
    for expected_status in [
        DeleteNotificationStatus::Ok,
        DeleteNotificationStatus::NothingToDelete,
        DeleteNotificationStatus::UndefinedError,
        DeleteNotificationStatus::Other(42),
    ] {
        let resp = NotificationSentResponse::new(expected_status);
        let encoded = resp.encode();
        assert_eq!(encoded[0], 0xBF);
        assert_eq!(encoded[1], 0x30);

        let mut reader = Asn1Reader::new(&encoded);
        let node = reader
            .read_next()
            .expect("valid ASN.1")
            .expect("node present");
        let decoded =
            NotificationSentResponse::decode(&node).expect("decode NotificationSentResponse");

        assert_eq!(decoded.delete_notification_status, expected_status);
    }
}

#[test]
fn test_notification_job_es9_request_formatting() {
    let job = NotificationJob {
        seq_number: 101,
        iccid: "89012345678901234567".to_string(),
        address: "smdp.example.com".to_string(),
        content_base64: "v7Ajgg==".to_string(),
        should_remove_after_send: true,
    };

    let json = job.build_es9_request_json();
    assert_eq!(json, r#"{"pendingNotification":"v7Ajgg=="}"#);
}

#[test]
fn test_notification_sync_plan_comprehensive() {
    let policy = NotificationPolicy {
        auto_send_install: true,
        auto_remove_install: true,
        auto_send_enable: true,
        auto_remove_enable: false,
        delete_without_sending_enable: false,
        auto_send_disable: false,
        auto_remove_disable: true,
        delete_without_sending_disable: true,
        auto_send_delete: true,
        auto_remove_delete: true,
    };

    // Notifications present on the card:
    // Seq 1: Install, Server A, raw bytes [0x01, 0x02]
    // Seq 2: Enable, Server A, already sent in DB
    // Seq 3: Disable, Server B, policy says delete_without_sending
    let card_notifications = vec![
        NotificationRecord {
            seq_number: 1,
            operation: ProfileOperation::Install,
            address: "smdp-a.com".to_string(),
            iccid: Some("890101".to_string()),
            delivered: false,
            retry_count: 0,
            raw_der: Some(vec![0x01, 0x02]),
        },
        NotificationRecord {
            seq_number: 2,
            operation: ProfileOperation::Enable,
            address: "smdp-a.com".to_string(),
            iccid: Some("890102".to_string()),
            delivered: false,
            retry_count: 0,
            raw_der: Some(vec![0x03, 0x04]),
        },
        NotificationRecord {
            seq_number: 3,
            operation: ProfileOperation::Disable,
            address: "smdp-b.com".to_string(),
            iccid: Some("890103".to_string()),
            delivered: false,
            retry_count: 0,
            raw_der: None,
        },
    ];

    // Local DB records:
    // Seq 2: Sent
    // Seq 99: Pending, but no longer on card -> must be pruned!
    let stored_notifications = vec![
        StoredNotification {
            seq_number: 2,
            iccid: "890102".to_string(),
            status: NotificationStatus::Sent,
            delete_pending: false,
        },
        StoredNotification {
            seq_number: 99,
            iccid: "890199".to_string(),
            status: NotificationStatus::Pending,
            delete_pending: false,
        },
    ];

    let plan = NotificationSyncPlan::compute(&card_notifications, &stored_notifications, &policy);

    // 1. Seq 99 should be pruned
    assert_eq!(plan.prune_db_seqs, vec![99]);

    // 2. Pending unsent count: Seq 1 and Seq 3 are not sent -> 2
    assert_eq!(plan.pending_unsent_count, 2);

    // 3. Outbound jobs: only Seq 1 for smdp-a.com (Seq 2 is already sent)
    assert_eq!(plan.jobs_by_server.len(), 1);
    let jobs_a = plan
        .jobs_by_server
        .get("smdp-a.com")
        .expect("jobs for smdp-a");
    assert_eq!(jobs_a.len(), 1);
    assert_eq!(jobs_a[0].seq_number, 1);
    assert!(jobs_a[0].should_remove_after_send);
    assert_eq!(
        jobs_a[0].build_es9_request_json(),
        r#"{"pendingNotification":"AQI="}"#
    );

    // 4. Immediate removals:
    // Seq 3 (delete_without_sending)
    assert!(plan.immediate_removals.contains(&3));

    // 5. Seq to ICCID lookup map
    assert_eq!(
        plan.seq_to_iccid.get(&1).map(String::as_str),
        Some("890101")
    );
    assert_eq!(
        plan.seq_to_iccid.get(&2).map(String::as_str),
        Some("890102")
    );
    assert_eq!(
        plan.seq_to_iccid.get(&3).map(String::as_str),
        Some("890103")
    );
}
