use nlpa_core::{
    build_close_channel_request, build_open_channel_request, build_remocard_headers,
    build_send_apdu_request, build_send_raw_apdu_request, bytes_to_hex, clean_remocard_url,
    decrypt_remocard_payload, encrypt_remocard_payload_with_iv, parse_list_slots_response,
    parse_open_channel_response, parse_send_apdu_response, resolve_remocard_key, RemoCardError,
    RemoCardServer, RemoCardSlot,
};

#[test]
fn test_remocard_url_cleaning_headers_and_request_builders() {
    let (clean, pwd) = clean_remocard_url("http://user:s3cr3t@192.168.1.50:33777/");
    assert_eq!(clean, "http://192.168.1.50:33777/");
    assert_eq!(pwd.as_deref(), Some("s3cr3t"));

    let (clean2, pwd2) = clean_remocard_url("https://onlypass@remo.example:8443");
    assert_eq!(clean2, "https://remo.example:8443");
    assert_eq!(pwd2.as_deref(), Some("onlypass"));

    let server = RemoCardServer {
        id: "srv1".to_string(),
        name: "Desk Reader".to_string(),
        base_url: "http://192.168.1.50:33777".to_string(),
        token: Some("tok123".to_string()),
        aes_key_hex: None,
        enabled: true,
    };

    let headers = build_remocard_headers(&server);
    assert!(headers.contains(&("X-Remocard-Version".to_string(), "2".to_string())));
    assert!(headers.contains(&("Authorization".to_string(), "Bearer tok123".to_string())));

    assert_eq!(
        build_open_channel_request("Slot 1", &["A000000559".to_string()]),
        r#"{"aids":["A000000559"],"reader":"Slot 1"}"#
    );
    assert_eq!(
        build_send_apdu_request("Slot 1", "A000000559", "80E2910000"),
        r#"{"aid":"A000000559","apdu":"80E2910000","reader":"Slot 1"}"#
    );
    assert_eq!(
        build_send_raw_apdu_request("Slot 1", "00A4040000"),
        r#"{"apdu":"00A4040000","reader":"Slot 1"}"#
    );
    assert_eq!(
        build_close_channel_request("Slot 1", &["A000000559".to_string()]),
        r#"{"aids":["A000000559"],"reader":"Slot 1"}"#
    );
}

#[test]
fn test_remocard_aes_cbc_codec_and_response_parsers() {
    let key = [0x42u8; 32];
    let iv = [0x09u8; 16];
    let server = RemoCardServer {
        id: "srv1".to_string(),
        name: "Encrypted Reader".to_string(),
        base_url: "http://127.0.0.1:33777".to_string(),
        token: None,
        aes_key_hex: Some(bytes_to_hex(&key)),
        enabled: true,
    };
    assert_eq!(resolve_remocard_key(&server), Some(key));

    let slots_plain = r#"{"slots":[{"name":"Slot 1","eid":"89012345678901234567890123456789"}]}"#;
    let slots_enc = encrypt_remocard_payload_with_iv(slots_plain, &key, &iv);
    assert!(slots_enc.contains(r#""data":"#));
    assert_eq!(
        decrypt_remocard_payload(&slots_enc, &key).expect("decrypt"),
        slots_plain
    );

    let slots = parse_list_slots_response(&slots_enc, Some(&server)).expect("parse slots");
    assert_eq!(
        slots,
        vec![RemoCardSlot {
            name: "Slot 1".to_string(),
            eid: Some("89012345678901234567890123456789".to_string()),
        }]
    );

    let open_enc = encrypt_remocard_payload_with_iv(
        r#"{"success":true,"aid":"A0000005591010FFFFFFFF8900000100"}"#,
        &key,
        &iv,
    );
    assert_eq!(
        parse_open_channel_response(&open_enc, Some(&server)).expect("open aid"),
        "A0000005591010FFFFFFFF8900000100"
    );

    let apdu_plain = r#"{"response":"9F109000"}"#;
    assert_eq!(
        parse_send_apdu_response(apdu_plain, None).expect("plaintext apdu"),
        vec![0x9F, 0x10, 0x90, 0x00]
    );

    assert!(matches!(
        parse_open_channel_response(r#"{"success":false}"#, None),
        Err(RemoCardError::ChannelOpenFailed)
    ));
    assert!(matches!(
        parse_list_slots_response(r#"{"error":"Invalid password"}"#, None),
        Err(RemoCardError::ServerError(_))
    ));
}
