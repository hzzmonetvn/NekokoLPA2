use rust_lib_nlpa2::api::lpa::{
    create_download_session, evaluate_profile_convergence, session_compute_hash_cc,
    session_process_auth_client_resp, session_process_auth_server_resp,
    session_process_euicc_challenge, session_process_euicc_info1,
    session_process_initiate_auth_resp, session_validate_initiate_auth, VerificationReadDto,
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
