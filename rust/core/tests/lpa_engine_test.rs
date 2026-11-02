use nlpa_core::{
    EuiccProfile, LpaEngine, LpaError, NotificationMetadata, NotificationQueue, NotificationRecord,
    ProfileOperation, Ts43CarrierConfig,
};

#[test]
fn test_lpa_engine_lifecycle() {
    let mut engine = LpaEngine::new();

    let mut p1 = EuiccProfile::new("8984040000000000001".to_string(), false);
    p1.name = Some("Profile 1".to_string());
    p1.service_provider_name = Some("Viettel".to_string());

    let mut p2 = EuiccProfile::new("8984040000000000002".to_string(), false);
    p2.name = Some("Profile 2".to_string());
    p2.service_provider_name = Some("VNPT".to_string());

    engine.load_profiles(vec![p1, p2]);

    assert_eq!(engine.list_profiles().len(), 2);
    assert_eq!(engine.active_profile_iccid(), None);

    // Enable Profile 1
    engine
        .enable_profile("8984040000000000001")
        .expect("enable p1");
    assert_eq!(engine.active_profile_iccid(), Some("8984040000000000001"));

    // Re-enabling Profile 1 should error
    let err = engine
        .enable_profile("8984040000000000001")
        .expect_err("already enabled");
    assert!(matches!(err, LpaError::ProfileAlreadyEnabled { .. }));

    // Switch to Profile 2: Profile 1 should be automatically disabled
    engine
        .enable_profile("8984040000000000002")
        .expect("enable p2");
    assert_eq!(engine.active_profile_iccid(), Some("8984040000000000002"));
    let p1_check = engine.get_profile("8984040000000000001").unwrap();
    assert!(!p1_check.enabled);

    // Deleting active Profile 2 should fail
    let del_err = engine
        .delete_profile("8984040000000000002")
        .expect_err("cannot delete active");
    assert!(matches!(
        del_err,
        LpaError::CannotDeleteActiveProfile { .. }
    ));

    // Disable Profile 2
    engine
        .disable_profile("8984040000000000002")
        .expect("disable p2");
    assert_eq!(engine.active_profile_iccid(), None);

    // Now delete Profile 2
    engine
        .delete_profile("8984040000000000002")
        .expect("delete p2");
    assert_eq!(engine.list_profiles().len(), 1);

    // Set nickname
    engine
        .set_nickname("8984040000000000001", "Primary eSIM".to_string())
        .expect("set nickname");
    assert_eq!(
        engine
            .get_profile("8984040000000000001")
            .unwrap()
            .nickname
            .as_deref(),
        Some("Primary eSIM")
    );
}

#[test]
fn test_notification_queue_operations() {
    let mut queue = NotificationQueue::new();
    assert!(queue.is_empty());

    let meta1 = NotificationMetadata {
        seq_number: 101,
        profile_management_operation: 1, // Install
        notification_address: "smdp.carrier.com".to_string(),
        iccid: Some("8984040000000000001".to_string()),
    };
    let rec1 = NotificationRecord::from_metadata(meta1);
    assert_eq!(rec1.operation, ProfileOperation::Install);

    let ack_apdu = rec1.build_ack_apdu();
    assert_eq!(ack_apdu[0..2], [0x80, 0xE2]);

    queue.enqueue(rec1);
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.pending().len(), 1);

    queue.record_retry(101);
    let pending = queue.pending();
    assert_eq!(pending[0].retry_count, 1);

    assert!(queue.mark_delivered(101));
    assert_eq!(queue.pending().len(), 0);
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_ts43_carrier_headers() {
    let config = Ts43CarrierConfig::new(
        "https://entitlement.viettel.vn/ts43".to_string(),
        "v1.0".to_string(),
    );

    let headers = config.build_request_headers(Some("AKA_TOKEN_12345"));
    assert!(headers
        .iter()
        .any(|(k, v)| k == "Authorization" && v == "Bearer AKA_TOKEN_12345"));
    assert!(headers
        .iter()
        .any(|(k, v)| k == "Content-Type" && v == "application/vnd.gsma.eap-aka+json"));
}
