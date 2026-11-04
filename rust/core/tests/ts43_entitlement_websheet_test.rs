use nlpa_core::lpa::ts43::{
    discover_entitlement_for_channel, parse_websheet_result, EntitlementChannel, EntitlementRecord,
    Identity, Operation, SimType, WebsheetCallbackEvent,
};

const RECORDS_JSON: &str = r#"[
  {"mccmnc":"20404","gid1":"5A","carrierName":"SaskTel","channel":"google",
   "entitlementServer":"https://sasktel.example","transferTypePsim":2,
   "transferTypeEsim":2,"supportPsimToEsimTransfer":true,
   "supportEsimToEsimTransfer":true,"useHttpPost":false},
  {"mccmnc":"20404","gid1":null,"carrierName":"Fallback","channel":"google",
   "entitlementServer":"https://fallback.example","transferTypePsim":6,
   "transferTypeEsim":6,"supportPsimToEsimTransfer":true,
   "supportEsimToEsimTransfer":true,"useHttpPost":true}
]"#;

fn make_id(gid1: &str) -> Identity {
    Identity {
        iccid: "8900".to_string(),
        imsi: "204040000000000".to_string(),
        mcc: "204".to_string(),
        mnc: "04".to_string(),
        gid1: gid1.to_string(),
    }
}

#[test]
fn test_gid1_match_wins_over_fallback() {
    let records: Vec<EntitlementRecord> =
        serde_json::from_str(RECORDS_JSON).expect("valid JSON records");
    let e = discover_entitlement_for_channel(
        &records,
        &make_id("5A"),
        SimType::Esim,
        EntitlementChannel::Google,
    )
    .expect("should match SaskTel");

    assert_eq!(e.server, "https://sasktel.example");
    assert_eq!(e.transfer_type, 2);
    assert!(!e.use_post);
}

#[test]
fn test_falls_back_to_null_gid1_record() {
    let records: Vec<EntitlementRecord> =
        serde_json::from_str(RECORDS_JSON).expect("valid JSON records");
    let e = discover_entitlement_for_channel(
        &records,
        &make_id("ZZ"),
        SimType::Esim,
        EntitlementChannel::Google,
    )
    .expect("should fallback to null-gid1 record");

    assert_eq!(e.server, "https://fallback.example");
    assert!(e.use_post);
    assert_eq!(e.transfer_type, 6);
}

#[test]
fn test_unknown_mcc_mnc_throws_not_found() {
    let records: Vec<EntitlementRecord> =
        serde_json::from_str(RECORDS_JSON).expect("valid JSON records");
    let unknown_id = Identity {
        iccid: String::new(),
        imsi: "999990000".to_string(),
        mcc: "999".to_string(),
        mnc: "99".to_string(),
        gid1: String::new(),
    };

    let result = discover_entitlement_for_channel(
        &records,
        &unknown_id,
        SimType::Esim,
        EntitlementChannel::Google,
    );
    assert!(result.is_err());
}

#[test]
fn test_websheet_json_activation_code_callback() {
    let text = r#"{"event":"profileReadyWithActivationCode","activationCode":"LPA:1$smdp$M","iccid":"8900"}"#;
    let r = parse_websheet_result(text).expect("valid websheet json");
    assert_eq!(
        r.event,
        WebsheetCallbackEvent::ProfileReadyWithActivationCode
    );
    assert_eq!(r.activation_code, "LPA:1$smdp$M");
    assert_eq!(r.iccid, "8900");
}

#[test]
fn test_websheet_js_odsa_service_flow_callback() {
    let text = "TS43_ODSA_CALLBACK ODSAServiceFlow.dismissFlow()";
    let r = parse_websheet_result(text).expect("valid JS callback");
    assert_eq!(r.event, WebsheetCallbackEvent::DismissFlow);
}

#[test]
fn test_websheet_url_callback() {
    let text =
        "https://carrier.example/_callback?event=finishFlow&next_action=acquireConfiguration";
    let r = parse_websheet_result(text).expect("valid URL callback");
    assert_eq!(r.event, WebsheetCallbackEvent::FinishFlow);
    assert_eq!(r.next_action, Operation::AcquireConfiguration);
}
