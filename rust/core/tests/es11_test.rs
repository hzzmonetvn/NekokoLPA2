use nlpa_core::lpa::es11::{
    build_smds_auth_client_request, build_smds_initiate_auth_request, clean_smds_host,
    parse_smds_discovery_response, parse_smds_event_entries, DiscoveredEvent, ROOT_SMDS_ADDRESS,
};
use nlpa_core::Es9Error;

#[test]
fn test_clean_smds_host_normalization() {
    assert_eq!(clean_smds_host("  lpa.ds.gsma.com  "), "lpa.ds.gsma.com");
    assert_eq!(
        clean_smds_host("https://LPA.DS.GSMA.COM/gsma/rsp2/es11"),
        "lpa.ds.gsma.com"
    );
    assert_eq!(
        clean_smds_host("http://smds.example.com:8443/endpoint?query=1#frag"),
        "smds.example.com"
    );
    assert_eq!(clean_smds_host(ROOT_SMDS_ADDRESS), "lpa.ds.gsma.com");
}

#[test]
fn test_discovered_event_activation_code() {
    let ev = DiscoveredEvent::new("MATCH-12345", "https://smdp.example.com:443/path");
    assert_eq!(ev.event_id, "MATCH-12345");
    assert_eq!(ev.address, "smdp.example.com");
    assert_eq!(ev.activation_code(), "LPA:1$smdp.example.com$MATCH-12345");
}

#[test]
fn test_build_smds_requests() {
    let init_req = build_smds_initiate_auth_request("https://smds.gsma.com/", "CHALLENGE", "INFO1");
    assert!(init_req.contains("\"smdpAddress\":\"smds.gsma.com\""));
    assert!(init_req.contains("\"euiccChallenge\":\"CHALLENGE\""));
    assert!(init_req.contains("\"euiccInfo1\":\"INFO1\""));

    let auth_req = build_smds_auth_client_request("TX-999", "AUTH_SERVER_RESP");
    assert!(auth_req.contains("\"transactionId\":\"TX-999\""));
    assert!(auth_req.contains("\"authenticateServerResponse\":\"AUTH_SERVER_RESP\""));
}

#[test]
fn test_parse_smds_discovery_response_success() {
    let json = serde_json::json!({
        "header": {
            "functionExecutionStatus": {
                "status": "Executed-Success"
            }
        },
        "eventEntries": [
            {
                "eventId": "ORDER-001",
                "rspServerAddress": "smdp1.carrier.com"
            },
            {
                "eventId": "ORDER-002",
                "rspServerAddress": "https://smdp2.carrier.com:443/download"
            }
        ]
    })
    .to_string();

    let events = parse_smds_discovery_response(&json, Some(200)).expect("valid discovery response");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_id, "ORDER-001");
    assert_eq!(events[0].address, "smdp1.carrier.com");
    assert_eq!(
        events[0].activation_code(),
        "LPA:1$smdp1.carrier.com$ORDER-001"
    );

    assert_eq!(events[1].event_id, "ORDER-002");
    assert_eq!(events[1].address, "smdp2.carrier.com");
    assert_eq!(
        events[1].activation_code(),
        "LPA:1$smdp2.carrier.com$ORDER-002"
    );

    let parsed_val = nlpa_core::parse_json(&json).expect("valid json value");
    let from_entries =
        parse_smds_event_entries(&parsed_val).expect("parse_smds_event_entries succeeds");
    assert_eq!(from_entries, events);
}

#[test]
fn test_parse_smds_discovery_response_empty_events() {
    let json = serde_json::json!({
        "header": {
            "functionExecutionStatus": {
                "status": "Executed-Success"
            }
        },
        "eventEntries": []
    })
    .to_string();

    let events = parse_smds_discovery_response(&json, Some(200)).expect("valid empty response");
    assert!(events.is_empty());
}

#[test]
fn test_parse_smds_discovery_response_execution_failed() {
    let json = serde_json::json!({
        "header": {
            "functionExecutionStatus": {
                "status": "Failed",
                "statusCodeData": {
                    "subjectCode": "8.1.1",
                    "reasonCode": "3.8",
                    "message": "No events found"
                }
            }
        }
    })
    .to_string();

    let err = parse_smds_discovery_response(&json, Some(200)).unwrap_err();
    match err {
        Es9Error::ExecutionFailed {
            subject_code,
            reason_code,
            message,
            ..
        } => {
            assert_eq!(subject_code, "8.1.1");
            assert_eq!(reason_code, "3.8");
            assert_eq!(message.as_deref(), Some("No events found"));
        }
        other => panic!("expected ExecutionFailed, got: {other:?}"),
    }
}
