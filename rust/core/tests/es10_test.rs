use nlpa_core::{
    build_delete_profile_apdu, build_disable_profile_apdu, build_enable_profile_apdu,
    build_euicc_memory_reset_apdu, build_get_euicc_challenge_apdu,
    build_get_euicc_configured_addresses_apdu, build_get_euicc_info1_apdu,
    build_get_euicc_info2_apdu, build_get_profiles_info_apdu, build_list_notifications_apdu,
    build_remove_notification_apdu, build_select_isdr_apdu, build_set_nickname_apdu,
    parse_es10_response, wrap_store_data_apdu, ApduError, ISDR_AID,
};

#[test]
fn test_build_select_isdr_apdu() {
    let apdu = build_select_isdr_apdu();
    assert_eq!(apdu[0..4], [0x00, 0xA4, 0x04, 0x00]);
    assert_eq!(apdu[4], ISDR_AID.len() as u8);
    assert_eq!(&apdu[5..5 + ISDR_AID.len()], &ISDR_AID);
    assert_eq!(apdu[apdu.len() - 1], 0x00);
}

#[test]
fn test_wrap_store_data_apdu() {
    let payload = vec![0xBF, 0x20, 0x00];
    let apdu = wrap_store_data_apdu(&payload, true, true);
    assert_eq!(apdu[0..4], [0x80, 0xE2, 0x88, 0x00]);
    assert_eq!(apdu[4], 3);
    assert_eq!(&apdu[5..8], &payload);
    assert_eq!(apdu[8], 0x00);
}

#[test]
fn test_es10_command_builders_header() {
    let apdus = vec![
        build_get_euicc_configured_addresses_apdu(),
        build_get_euicc_info1_apdu(),
        build_get_euicc_info2_apdu(),
        build_get_euicc_challenge_apdu(),
        build_get_profiles_info_apdu(),
        build_enable_profile_apdu("898404", true),
        build_disable_profile_apdu("898404", false),
        build_delete_profile_apdu("898404"),
        build_set_nickname_apdu("898404", "Test"),
        build_list_notifications_apdu(Some(1)),
        build_remove_notification_apdu(1),
        build_euicc_memory_reset_apdu(1),
    ];

    for apdu in apdus {
        assert_eq!(apdu[0], 0x80);
        assert_eq!(apdu[1], 0xE2);
        assert_eq!(apdu[2], 0x88); // STORE DATA single block
        assert_eq!(apdu[3], 0x00);
        let lc = apdu[4] as usize;
        assert_eq!(apdu.len(), 5 + lc + 1);
        assert_eq!(apdu[apdu.len() - 1], 0x00);
    }
}

#[test]
fn test_parse_es10_response_success_and_error() {
    let success_resp = vec![0xBF, 0x20, 0x02, 0xAA, 0xBB, 0x90, 0x00];
    let payload = parse_es10_response(&success_resp).expect("0x9000 is success");
    assert_eq!(payload, vec![0xBF, 0x20, 0x02, 0xAA, 0xBB]);

    let busy_resp = vec![0x68, 0x81];
    let err = parse_es10_response(&busy_resp).expect_err("0x6881 should be CardBusy");
    assert_eq!(err, ApduError::CardBusy);

    let short_resp = vec![0x90];
    let err_short = parse_es10_response(&short_resp).expect_err("too short response");
    assert!(matches!(err_short, ApduError::Failure { .. }));
}
