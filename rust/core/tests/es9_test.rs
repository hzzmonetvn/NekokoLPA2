use nlpa_core::{
    build_authenticate_client_body, build_cancel_session_body, build_es9_headers,
    build_get_bound_profile_package_body, build_handle_notification_body,
    build_initiate_authentication_body, parse_authenticate_client_response,
    parse_get_bound_profile_package_response, parse_initiate_authentication_response, parse_json,
    Es9Error, ENDPOINT_AUTHENTICATE_CLIENT, ENDPOINT_GET_BOUND_PROFILE_PACKAGE,
    ENDPOINT_INITIATE_AUTHENTICATION,
};

#[test]
fn test_es9_endpoints_and_headers() {
    assert_eq!(
        ENDPOINT_INITIATE_AUTHENTICATION,
        "/gsma/rsp2/es9plus/initiateAuthentication"
    );
    assert_eq!(
        ENDPOINT_AUTHENTICATE_CLIENT,
        "/gsma/rsp2/es9plus/authenticateClient"
    );
    assert_eq!(
        ENDPOINT_GET_BOUND_PROFILE_PACKAGE,
        "/gsma/rsp2/es9plus/getBoundProfilePackage"
    );

    let headers = build_es9_headers();
    assert_eq!(headers.len(), 3);
    assert_eq!(headers[0], ("User-Agent", "gsma-rsp-lpad"));
    assert_eq!(headers[1], ("X-Admin-Protocol", "gsma/rsp/v2.2.0"));
    assert_eq!(headers[2], ("Content-Type", "application/json"));
}

#[test]
fn test_es9_request_body_builders() {
    let init_body = build_initiate_authentication_body("smdp.io", "Q0hBTExFTkdF", "SU5GTzE=");
    assert!(init_body.contains(r#""smdpAddress":"smdp.io""#));
    assert!(init_body.contains(r#""euiccChallenge":"Q0hBTExFTkdF""#));

    let auth_body = build_authenticate_client_body("TX-1234", "U0VSVkVSX1JFU1A=");
    assert_eq!(
        auth_body,
        r#"{"transactionId":"TX-1234","authenticateServerResponse":"U0VSVkVSX1JFU1A="}"#
    );

    let bpp_body = build_get_bound_profile_package_body("TX-1234", "UFJFUEFSRV9SRVNQ");
    assert_eq!(
        bpp_body,
        r#"{"transactionId":"TX-1234","prepareDownloadResponse":"UFJFUEFSRV9SRVNQ"}"#
    );

    let cancel_body = build_cancel_session_body("TX-1234", "Q0FOQ0VMX1JFU1A=");
    assert_eq!(
        cancel_body,
        r#"{"transactionId":"TX-1234","cancelSessionResponse":"Q0FOQ0VMX1JFU1A="}"#
    );

    let notif_body = build_handle_notification_body("UEVORElOR19OT1RJRklDQVRJT04=");
    assert_eq!(
        notif_body,
        r#"{"pendingNotification":"UEVORElOR19OT1RJRklDQVRJT04="}"#
    );
}

#[test]
fn test_parse_json_structures() {
    let raw = r#"{
        "name": "NekokoLPA2",
        "version": 2.5,
        "active": true,
        "carrier_vi": "Viettel 5G Việt Nam - Thẻ eSIM",
        "greeting": "Xin chào thế giới! 🇻🇳",
        "features": ["esim", "omapi", "qrtr"],
        "metadata": {
            "vendor": "GSMA"
        }
    }"#;
    let json = parse_json(raw).expect("should parse json");
    assert_eq!(json.get_str("name"), Some("NekokoLPA2"));
    assert_eq!(json.get_str("greeting"), Some("Xin chào thế giới! 🇻🇳"));
    assert_eq!(
        json.get("metadata").and_then(|m| m.get_str("vendor")),
        Some("GSMA")
    );
}

#[test]
fn test_parse_initiate_authentication_response_success() {
    let valid_json = r#"{
        "header": {
            "functionExecutionStatus": {
                "status": "ExecutedSuccess"
            }
        },
        "transactionId": "TX-9999",
        "serverSigned1": "AQIDBAUGBwg=",
        "serverSignature1": "CQoLDA0ODxA=",
        "serverCertificate": "EQESExQVFhc="
    }"#;

    let parsed = parse_initiate_authentication_response(valid_json, Some(200))
        .expect("should parse valid response");
    assert_eq!(parsed.transaction_id, "TX-9999");
    assert_eq!(parsed.server_signed1, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(
        parsed.server_signature1,
        vec![9, 10, 11, 12, 13, 14, 15, 16]
    );
}

#[test]
fn test_parse_authenticate_client_response_success() {
    let valid_json = r#"{
        "header": {
            "functionExecutionStatus": {
                "status": "ExecutedSuccess"
            }
        },
        "transactionId": "TX-7777",
        "profileMetadata": "AQID",
        "smdpSigned2": "BAUG",
        "smdpSignature2": "BwgJ",
        "smdpCertificate": "CgsM"
    }"#;

    let parsed = parse_authenticate_client_response(valid_json, Some(200))
        .expect("should parse authenticate client response");
    assert_eq!(parsed.transaction_id, "TX-7777");
    assert_eq!(parsed.profile_metadata.as_deref(), Some(&[1, 2, 3][..]));
    assert_eq!(parsed.smdp_signed2.as_deref(), Some(&[4, 5, 6][..]));
}

#[test]
fn test_parse_es9_error_status() {
    let err_json = r#"{
        "header": {
            "functionExecutionStatus": {
                "status": "Failed",
                "statusCodeData": {
                    "subjectCode": "8.1.1",
                    "reasonCode": "3.8",
                    "subjectIdentifier": "1.3.6.1.4.1",
                    "message": "Transaction expired or unknown"
                }
            }
        }
    }"#;

    let err = parse_initiate_authentication_response(err_json, Some(400))
        .expect_err("should return execution failed");
    match err {
        Es9Error::ExecutionFailed {
            subject_code,
            reason_code,
            subject_identifier,
            message,
            status_code,
        } => {
            assert_eq!(subject_code, "8.1.1");
            assert_eq!(reason_code, "3.8");
            assert_eq!(subject_identifier.as_deref(), Some("1.3.6.1.4.1"));
            assert_eq!(message.as_deref(), Some("Transaction expired or unknown"));
            assert_eq!(status_code, Some(400));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
fn test_parse_get_bound_profile_package_response() {
    let json_resp = r#"{
        "header": {
            "functionExecutionStatus": {
                "status": "ExecutedSuccess"
            }
        },
        "transactionId": "TX-8888",
        "boundProfilePackage": "AQID"
    }"#;

    let parsed = parse_get_bound_profile_package_response(json_resp, Some(200))
        .expect("should parse bpp response");
    assert_eq!(parsed.transaction_id, "TX-8888");
    assert_eq!(parsed.bound_profile_package, vec![1, 2, 3]);
}
