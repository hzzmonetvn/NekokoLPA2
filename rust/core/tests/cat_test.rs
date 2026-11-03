use nlpa_core::{
    CatProactive, CatProactiveError, CatResponseDecision, ProactiveCommandDetails, RESULT_SUCCESS,
};

#[test]
fn test_status_and_fetch_apdu_generation() {
    let status_apdu = CatProactive::status();
    assert_eq!(status_apdu, vec![0x80, 0xF2, 0x00, 0x00, 0x00]);

    let fetch_apdu = CatProactive::fetch(0x1F);
    assert_eq!(fetch_apdu, vec![0x80, 0x12, 0x00, 0x00, 0x1F]);
}

#[test]
fn test_terminal_response_encoding() {
    let details = ProactiveCommandDetails::new(1, ProactiveCommandDetails::TYPE_REFRESH, 0x09);
    let resp =
        CatProactive::terminal_response(&details, RESULT_SUCCESS).expect("terminal response");

    // Header: [0x80, 0x14, 0x00, 0x00, 0x0C] + 12 body bytes
    assert_eq!(resp[0..5], [0x80, 0x14, 0x00, 0x00, 0x0C]);
    assert_eq!(&resp[5..10], &[0x81, 0x03, 1, 1, 0x09]); // command details (5 bytes)
    assert_eq!(&resp[10..14], &[0x82, 0x02, 0x82, 0x81]); // device identities (4 bytes)
    assert_eq!(&resp[14..17], &[0x83, 0x01, RESULT_SUCCESS]); // result (3 bytes)
}

#[test]
fn test_parse_fetch_response_refresh() {
    // Proactive command: D0 [len=9] (81 03 01 01 09) (82 02 81 82) + SW (90 00)
    let raw = vec![
        0xD0, 0x09, 0x81, 0x03, 0x01, 0x01, 0x09, 0x82, 0x02, 0x81, 0x82, 0x90, 0x00,
    ];
    let details = CatProactive::parse_fetch_response(&raw).expect("valid proactive refresh");
    assert_eq!(details.number, 1);
    assert_eq!(details.command_type, 1);
    assert_eq!(details.qualifier, 0x09);
    assert!(details.is_refresh());
    assert_eq!(details.decision(), CatResponseDecision::AcknowledgeSuccess);
}

#[test]
fn test_parse_fetch_response_naa_session_reset() {
    // Proactive command: D0 [len=5] (81 03 02 01 06) + SW (90 00)
    let raw = vec![0xD0, 0x05, 0x81, 0x03, 0x02, 0x01, 0x06, 0x90, 0x00];
    let details = CatProactive::parse_fetch_response(&raw).expect("valid proactive refresh NAA");
    assert_eq!(details.number, 2);
    assert_eq!(details.decision(), CatResponseDecision::RestartNaaSession);
}

#[test]
fn test_parse_fetch_response_non_refresh() {
    // Command type 0x21 (DISPLAY TEXT): D0 [len=5] (81 03 01 21 00) + SW (90 00)
    let raw = vec![0xD0, 0x05, 0x81, 0x03, 0x01, 0x21, 0x00, 0x90, 0x00];
    let details = CatProactive::parse_fetch_response(&raw).expect("display text command");
    assert!(!details.is_refresh());
    assert_eq!(
        details.decision(),
        CatResponseDecision::DeclineUnableToProcess
    );
}

#[test]
fn test_parse_fetch_response_errors() {
    assert_eq!(
        CatProactive::parse_fetch_response(&[0x90]).unwrap_err(),
        CatProactiveError::MissingStatusWord
    );
    assert_eq!(
        CatProactive::parse_fetch_response(&[0x00, 0x90, 0x00]).unwrap_err(),
        CatProactiveError::NotProactiveBerTlv
    );
    assert_eq!(
        CatProactive::parse_fetch_response(&[0xD0, 0x10, 0x81, 0x01, 0x90, 0x00]).unwrap_err(),
        CatProactiveError::TruncatedBerTlv
    );
}
