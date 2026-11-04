use rust_lib_nlpa2::api::lpa::{
    build_smds_auth_client, build_smds_init_auth, clean_smds_host_address, create_download_session,
    evaluate_profile_convergence, get_certificate_details, isdp_delete_apdu, isdp_select_apdu,
    parse_smds_events, predict_profile_size, scp03_authenticate_session,
    scp03_initialize_update_apdu, session_compute_hash_cc, session_process_auth_client_resp,
    session_process_auth_server_resp, session_process_euicc_challenge, session_process_euicc_info1,
    session_process_initiate_auth_resp, session_validate_initiate_auth, verify_smdp_certificate,
    VerificationReadDto,
};

#[test]
fn test_download_session_bridge_workflow() {
    let mut session = create_download_session(
        "smdp.example.com".to_string(),
        Some("MATCH-456".to_string()),
        Some("9999".to_string()),
    );
    assert_eq!(session.smdp_address, "smdp.example.com");
    assert_eq!(session.matching_id.as_deref(), Some("MATCH-456"));
    assert_eq!(session.confirmation_code.as_deref(), Some("9999"));

    // Validation fails before info1 and challenge
    assert!(session_validate_initiate_auth(session.clone()).is_err());

    session = session_process_euicc_info1(session, vec![0xBF, 0x20, 0x03, 0x82, 0x01, 0x01]);
    assert!(session_validate_initiate_auth(session.clone()).is_err());

    session = session_process_euicc_challenge(session, vec![0xCC; 16]);
    assert!(session_validate_initiate_auth(session.clone()).is_ok());

    // Ingest initiate authentication response
    let tx_id_hex = "A1B2C3D4E5F60102030405060708090A".to_string();
    let server_cert = vec![0x30, 0x82, 0x01, 0x05];
    let session_result =
        session_process_initiate_auth_resp(session, tx_id_hex.clone(), server_cert.clone());
    assert!(session_result.is_ok());
    session = session_result.unwrap();

    assert_eq!(
        session.transaction_id_hex.as_deref(),
        Some(tx_id_hex.as_str())
    );
    assert_eq!(session.server_certificate.as_ref(), Some(&server_cert));

    // Compute HashCc
    let hash_cc = session_compute_hash_cc(session.clone()).expect("valid hash cc");
    assert_eq!(hash_cc.len(), 32);

    // Ingest authenticate server response
    let auth_server_resp = vec![0xBF, 0x38, 0x04, 0x80, 0x02, 0x90, 0x00];
    session = session_process_auth_server_resp(session, auth_server_resp.clone());
    assert_eq!(
        session.authenticate_server_response.as_ref(),
        Some(&auth_server_resp)
    );

    // Ingest authenticate client response
    session = session_process_auth_client_resp(
        session,
        "Executed-Success".to_string(),
        Some(vec![0x11, 0x22]),
        Some(vec![0x33, 0x44]),
        Some(vec![0x55, 0x66]),
        Some(vec![0x77, 0x88]),
    );

    assert!(session.is_authenticated);
    assert_eq!(
        session.smdp_signed2.as_deref(),
        Some([0x11, 0x22].as_slice())
    );
    assert_eq!(
        session.smdp_signature2.as_deref(),
        Some([0x33, 0x44].as_slice())
    );
    assert_eq!(
        session.profile_metadata.as_deref(),
        Some([0x55, 0x66].as_slice())
    );
    assert_eq!(
        session.smdp_certificate.as_deref(),
        Some([0x77, 0x88].as_slice())
    );
}

#[test]
fn test_state_transition_bridge_convergence() {
    let reads = vec![
        VerificationReadDto {
            is_ok: false,
            actual_enabled: None,
            error: Some("Transient connection error".to_string()),
        },
        VerificationReadDto {
            is_ok: true,
            actual_enabled: Some(false),
            error: None,
        },
        VerificationReadDto {
            is_ok: true,
            actual_enabled: Some(true),
            error: None,
        },
    ];

    let result = evaluate_profile_convergence(true, 1, reads, Some(2), Some(6), Some(2));

    assert!(result.converged);
    assert_eq!(result.command_attempts, 1);
    assert_eq!(result.verification_attempts, 3);
    assert_eq!(result.actual_enabled, Some(true));
    assert!(result.error_message.is_none());
}

#[test]
fn test_state_transition_bridge_non_convergence() {
    let reads = vec![
        VerificationReadDto {
            is_ok: true,
            actual_enabled: Some(false),
            error: None,
        },
        VerificationReadDto {
            is_ok: true,
            actual_enabled: Some(false),
            error: None,
        },
    ];

    let result = evaluate_profile_convergence(true, 2, reads, Some(2), Some(2), Some(2));

    assert!(!result.converged);
    assert_eq!(result.command_attempts, 2);
    assert_eq!(result.actual_enabled, Some(false));
    assert!(result.error_message.is_some());
}

#[test]
fn test_x509_certificate_bridge_and_verification() {
    let cert_pem = r#"-----BEGIN CERTIFICATE-----
MIIBgjCCASmgAwIBAgIUWEquEgQb352AeoUtQMnjl1XLHB8wCgYIKoZIzj0EAwIw
FzEVMBMGA1UEAwwMR1NNQSBDSSBSb290MB4XDTI2MTEwNDAxNDk0NloXDTM2MTEw
MTAxNDk0NlowFzEVMBMGA1UEAwwMR1NNQSBDSSBSb290MFkwEwYHKoZIzj0CAQYI
KoZIzj0DAQcDQgAEk3qmJmGRMcnQfORD2VjY/sLWRScSzIGvC5ozrAFiZICrl97p
6sg0A9cDNmbOlMgxqns3CNy1U8x3QS7KfbAH/KNTMFEwHQYDVR0OBBYEFHk6byuv
JZU8AFRHVe5RkMhKuoauMB8GA1UdIwQYMBaAFHk6byuvJZU8AFRHVe5RkMhKuoau
MA8GA1UdEwEB/wQFMAMBAf8wCgYIKoZIzj0EAwIDRwAwRAIhAItJXitUqLUKvXn4
UZiPZhJ/SndoeYig43ohBL1hija8Ah8C7piEXm6JRFB4aoEpJ5aN8ATpCwJYmpsH
6Wanvctd
-----END CERTIFICATE-----"#;

    let cert_dto =
        get_certificate_details(cert_pem.to_string()).expect("get certificate details dto");
    assert_eq!(cert_dto.common_name.as_deref(), Some("GSMA CI Root"));
    assert!(cert_dto.is_ca);
    assert_eq!(cert_dto.ski_hex, "793A6F2BAF25953C00544755EE5190C84ABA86AE");

    // Verify SM-DP+ certificate constraint against matching and unmatching CI PKID lists
    let matching_pkid = "793A6F2BAF25953C00544755EE5190C84ABA86AE".to_string();
    let unmatching_pkid = "0102030405060708090A0B0C0D0E0F1011121314".to_string();

    let verify_ok = verify_smdp_certificate(
        cert_pem.to_string(),
        vec![unmatching_pkid.clone(), matching_pkid],
    )
    .expect("verification call");
    assert!(verify_ok);

    let verify_fail = verify_smdp_certificate(cert_pem.to_string(), vec![unmatching_pkid])
        .expect("verification call with no matching pkid");
    assert!(!verify_fail);
}

#[test]
fn test_isdp_apdu_bridge_generators() {
    let aid_hex = "A0000005591010FFFFFFFF8900000100";
    let select_apdu = isdp_select_apdu(aid_hex.to_string()).expect("valid isdp select apdu");
    assert_eq!(&select_apdu[0..4], &[0x00, 0xA4, 0x04, 0x00]);
    assert_eq!(select_apdu[4], 16);

    let delete_apdu = isdp_delete_apdu(aid_hex.to_string()).expect("valid isdp delete apdu");
    assert_eq!(&delete_apdu[0..4], &[0x80, 0xE4, 0x00, 0x00]);
    assert_eq!(delete_apdu[4], 18);
    assert_eq!(delete_apdu[5], 0x4F);

    // Invalid AID hex fails gracefully
    assert!(isdp_select_apdu("invalid_hex".to_string()).is_err());
    assert!(isdp_delete_apdu("1234".to_string()).is_err());
}

#[test]
fn test_scp03_bridge_initialization_and_authentication() {
    let host_challenge_hex = "1122334455667788".to_string();
    let init_apdu = scp03_initialize_update_apdu(host_challenge_hex.clone(), 0x01, 0x01)
        .expect("build init update");
    assert_eq!(&init_apdu[0..5], &[0x80, 0x50, 0x01, 0x01, 0x08]);
    assert_eq!(
        &init_apdu[5..13],
        &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]
    );

    // Test session authentication derivation
    let static_enc_hex = "01010101010101010101010101010101".to_string();
    let static_mac_hex = "02020202020202020202020202020202".to_string();
    let static_mac = vec![0x02u8; 16];
    let card_challenge = [0xBBu8; 8];

    // Compute valid card cryptogram using core derivation
    let s_mac = nlpa_core::derive_session_key(
        &static_mac,
        0x06,
        &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
        &card_challenge,
    )
    .expect("derive s-mac");
    let valid_card_crypto = nlpa_core::compute_card_cryptogram(
        &s_mac,
        &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
        &card_challenge,
    )
    .expect("compute card cryptogram");

    let mut card_resp_data = Vec::with_capacity(32);
    card_resp_data.extend_from_slice(&[0x10; 10]); // Diversification data (10 bytes)
    card_resp_data.extend_from_slice(&[0x01, 0x03, 0x70]); // Key info (3 bytes)
    card_resp_data.extend_from_slice(&card_challenge); // Card challenge (8 bytes)
    card_resp_data.extend_from_slice(&valid_card_crypto); // Card cryptogram (8 bytes)
    card_resp_data.extend_from_slice(&[0x00, 0x00, 0x01]); // Sequence counter (3 bytes)
    let card_resp_hex = nlpa_core::bytes_to_hex(&card_resp_data);

    let auth_result = scp03_authenticate_session(
        host_challenge_hex,
        card_resp_hex,
        static_enc_hex,
        static_mac_hex,
        0x03, // C-DEC and C-MAC
    )
    .expect("authenticate session bridge call");

    assert_eq!(auth_result.host_cryptogram_hex.len(), 16); // 8 bytes = 16 hex chars
    assert_eq!(auth_result.external_authenticate_apdu.len(), 21); // 5 header + 16 payload
    assert_eq!(
        &auth_result.external_authenticate_apdu[0..5],
        &[0x84, 0x82, 0x03, 0x00, 0x10]
    );
}

#[test]
fn test_smds_bridge_helpers_and_parsing() {
    let host = clean_smds_host_address("https://LPA.DS.GSMA.COM:443/endpoint".to_string());
    assert_eq!(host, "lpa.ds.gsma.com");

    let init_req = build_smds_init_auth(host, "CHALLENGE_HEX".to_string(), "INFO1_HEX".to_string());
    assert!(init_req.contains("\"smdpAddress\":\"lpa.ds.gsma.com\""));
    assert!(init_req.contains("\"euiccChallenge\":\"CHALLENGE_HEX\""));

    let auth_client_req = build_smds_auth_client("TX-123".to_string(), "SERVER_RESP".to_string());
    assert!(auth_client_req.contains("\"transactionId\":\"TX-123\""));

    let resp_json = r#"{
        "header": {
            "functionExecutionStatus": {
                "status": "Executed-Success"
            }
        },
        "eventEntries": [
            {
                "eventId": "EV-001",
                "rspServerAddress": "smdp.carrier.com"
            }
        ]
    }"#
    .to_string();

    let events = parse_smds_events(resp_json, Some(200)).expect("valid parse");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, "EV-001");
    assert_eq!(events[0].address, "smdp.carrier.com");
    assert_eq!(events[0].activation_code, "LPA:1$smdp.carrier.com$EV-001");
}

#[test]
fn test_profile_size_prediction_bridge() {
    let catalog_json = r#"{
        "reference_eum": "89043051",
        "results": [
            {
                "rsp": "smdp.test.com",
                "plmn": "310260",
                "serviceProviderName": "T-Mobile",
                "reference_size": 48000,
                "eum_sizes": {
                    "89043051": 49500
                }
            }
        ]
    }"#
    .to_string();

    // EUM exact match
    let size = predict_profile_size(
        catalog_json.clone(),
        Some("8904305112345678".to_string()),
        Some("smdp.test.com".to_string()),
        Some("310260".to_string()),
        Some("T-Mobile".to_string()),
    )
    .expect("valid catalog")
    .expect("size exists");
    assert_eq!(size, 49500);

    // Fallback reference size
    let fallback = predict_profile_size(
        catalog_json,
        Some("9999999912345678".to_string()),
        Some("smdp.test.com".to_string()),
        Some("310260".to_string()),
        Some("T-Mobile".to_string()),
    )
    .expect("valid catalog")
    .expect("fallback size exists");
    assert_eq!(fallback, 48000);
}
