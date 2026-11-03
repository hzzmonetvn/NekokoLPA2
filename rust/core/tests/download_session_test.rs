use nlpa_core::{bytes_to_hex, hex_to_bytes, sha256, DownloadSessionError, ProfileDownloadSession};

#[test]
fn test_download_session_lifecycle() {
    let mut session =
        ProfileDownloadSession::new("smdp.example.com", Some("MATCH-123".into()), None);
    assert_eq!(session.smdp_address(), "smdp.example.com");
    assert_eq!(session.matching_id(), Some("MATCH-123"));
    assert_eq!(session.confirmation_code(), None);

    // Initial validation fails without EUICCInfo1 and challenge
    assert_eq!(
        session
            .validate_initiate_authentication_prerequisites()
            .unwrap_err(),
        DownloadSessionError::MissingEuiccInfo1
    );

    session.process_euicc_info1(&[0xBF, 0x20, 0x03, 0x82, 0x01, 0x01]);
    assert_eq!(
        session
            .validate_initiate_authentication_prerequisites()
            .unwrap_err(),
        DownloadSessionError::MissingEuiccChallenge
    );

    let challenge = [0xAAu8; 16];
    session.process_euicc_challenge(&challenge);
    assert!(session
        .validate_initiate_authentication_prerequisites()
        .is_ok());

    // Process InitiateAuthentication response
    let tx_id_hex = "0123456789ABCDEF0123456789ABCDEF"; // 16 octets, uppercase hex
    let server_cert = vec![0x30, 0x82, 0x01, 0x00];
    session
        .process_initiate_authentication_response(tx_id_hex, &server_cert)
        .expect("valid initiate auth");

    assert_eq!(session.get_transaction_id_hex(), Some(tx_id_hex));
    assert_eq!(
        session.get_transaction_id(),
        Some(hex_to_bytes(tx_id_hex).unwrap().as_slice())
    );
    assert_eq!(
        session.get_server_certificate(),
        Some(server_cert.as_slice())
    );

    // Confirmation code & HashCc calculation
    session.set_confirmation_code("1234");
    assert_eq!(session.confirmation_code(), Some("1234"));

    let hash_cc = session.compute_hash_cc().expect("compute hash cc");

    // SGP.22 verification: SHA256(SHA256(CC) || TransactionID)
    let cc_hash = sha256(b"1234");
    let mut expected_combined = Vec::new();
    expected_combined.extend_from_slice(&cc_hash);
    expected_combined.extend_from_slice(&hex_to_bytes(tx_id_hex).unwrap());
    let expected_hash_cc = sha256(&expected_combined);

    assert_eq!(hash_cc, expected_hash_cc);

    // Authenticate server response
    let auth_server_resp = vec![0xBF, 0x38, 0x10, 0x80, 0x08, 0x01, 0x02, 0x03, 0x04];
    session.process_authenticate_server_response(&auth_server_resp);
    assert_eq!(
        session.authenticate_server_response(),
        Some(auth_server_resp.as_slice())
    );

    // Authenticate client response: success
    session.process_authenticate_client_response(
        "Executed-Success",
        Some(&[0x01, 0x02]),
        Some(&[0x03, 0x04]),
        Some(&[0x05, 0x06]),
        None,
    );
    assert!(session.is_client_authenticated_successfully());
    assert_eq!(session.get_smdp_signed2(), Some([0x01, 0x02].as_slice()));
    assert_eq!(session.get_smdp_signature2(), Some([0x03, 0x04].as_slice()));
    assert_eq!(
        session.get_profile_metadata(),
        Some([0x05, 0x06].as_slice())
    );
}

#[test]
fn test_download_session_transaction_id_validation() {
    let mut session = ProfileDownloadSession::new("smdp.example.com", None, None);

    // Empty hex
    let err_empty = session
        .process_initiate_authentication_response("", &[])
        .unwrap_err();
    assert_eq!(
        err_empty,
        DownloadSessionError::InvalidTransactionIdLength(0)
    );

    // Non-hex characters
    let err_hex = session
        .process_initiate_authentication_response("ZZZZ", &[])
        .unwrap_err();
    assert!(matches!(
        err_hex,
        DownloadSessionError::InvalidTransactionIdHex(_)
    ));

    // Too long (>16 octets = 34 hex chars)
    let long_hex = "0102030405060708090A0B0C0D0E0F1011";
    let err_long = session
        .process_initiate_authentication_response(long_hex, &[])
        .unwrap_err();
    assert_eq!(
        err_long,
        DownloadSessionError::InvalidTransactionIdLength(17)
    );

    // Valid 8 octet transaction ID (regression test from SGP.22 where 16-char hex is 8 octets)
    let hex_8 = "0123456789abcdef";
    session
        .process_initiate_authentication_response(hex_8, &[])
        .expect("valid 8-octet id");
    assert_eq!(session.get_transaction_id().unwrap().len(), 8);
    assert_eq!(
        bytes_to_hex(session.get_transaction_id().unwrap()),
        "0123456789abcdef"
    );
}
