use nlpa_core::base64_encode;
use nlpa_core::lpa::ts43::{
    build_prepared_request, AppEligibility, Entitlement, EntitlementChannel, Identity,
    OdsaOperation, OdsaResponse, Operation, OperationResult, ServiceEntitlementRequest,
    SubscriptionResult,
};

#[test]
fn test_get_request_builds_eap_id_and_common_fields() {
    let identity = Identity {
        iccid: "8900000000000000001".to_string(),
        imsi: "234107813240779".to_string(),
        mcc: "234".to_string(),
        mnc: "10".to_string(),
        gid1: "5A".to_string(),
    };

    let entitlement = Entitlement {
        channel: EntitlementChannel::Google,
        ..Default::default()
    };
    let req = ServiceEntitlementRequest {
        app: "ap2009".to_string(),
        entitlement_version: "9.0".to_string(),
        terminal_id: "350000000000006".to_string(),
        terminal_vendor: "Goog".to_string(),
        terminal_model: "Pixel".to_string(),
        ..Default::default()
    };
    let op = OdsaOperation {
        operation: Operation::CheckEligibility,
        ..Default::default()
    };

    let prepared = build_prepared_request(
        &entitlement,
        &identity,
        &req,
        &op,
        "client-Primary-ODSA",
        "nai.epc",
    )
    .expect("build_prepared_request should succeed");

    assert!(!prepared.use_post);
    assert_eq!(
        prepared.get_param("EAP_ID"),
        Some("0234107813240779@nai.epc.mnc010.mcc234.3gppnetwork.org")
    );
    assert_eq!(prepared.get_param("operation"), Some("CheckEligibility"));
    assert_eq!(prepared.get_param("app"), Some("ap2009"));
    assert!(prepared.accept.contains("eap-relay"));

    let qs = prepared.query_string();
    assert!(qs.contains("EAP_ID="));
    assert!(qs.contains("operation=CheckEligibility"));
}

#[test]
fn test_post_request_emits_json_body() {
    let identity = Identity {
        iccid: "8900000000000000001".to_string(),
        imsi: "234107813240779".to_string(),
        mcc: "234".to_string(),
        mnc: "10".to_string(),
        gid1: "5A".to_string(),
    };

    let entitlement = Entitlement {
        channel: EntitlementChannel::Google,
        use_post: true,
        ..Default::default()
    };
    let req = ServiceEntitlementRequest {
        app: "ap2009".to_string(),
        authentication_token: "TOK".to_string(),
        ..Default::default()
    };
    let op = OdsaOperation {
        operation: Operation::ManageSubscription,
        ..Default::default()
    };

    let prepared = build_prepared_request(
        &entitlement,
        &identity,
        &req,
        &op,
        "client-Primary-ODSA",
        "nai.epc",
    )
    .expect("build_prepared_request should succeed");

    assert!(prepared.use_post);
    let body: serde_json::Value =
        serde_json::from_str(&prepared.json_body()).expect("valid JSON body");
    assert_eq!(body.get("token").and_then(|v| v.as_str()), Some("TOK"));
    assert_eq!(
        body.get("IMSI").and_then(|v| v.as_str()),
        Some("234107813240779")
    );
    assert_eq!(
        body.get("operation").and_then(|v| v.as_str()),
        Some("ManageSubscription")
    );
}

#[test]
fn test_decodes_json_odsa_response_with_download_info() {
    let raw_ac = base64_encode(b"1$smdp.example.com$MATCH-123");
    let json = serde_json::json!({
        "APPLICATION": {
            "OperationResult": 1,
            "SubscriptionResult": 2,
            "DownloadInfo": {
                "ProfileActivationCode": format!("encodedValue={raw_ac}"),
                "ProfileIccid": "8900000000000000002"
            }
        }
    })
    .to_string();

    let resp = OdsaResponse::decode(json.as_bytes());
    assert_eq!(resp.operation_result, OperationResult::SUCCESS);
    assert_eq!(
        resp.subscription_result,
        SubscriptionResult::DOWNLOAD_PROFILE
    );
    assert_eq!(
        resp.download.activation_code,
        "LPA:1$smdp.example.com$MATCH-123"
    );
    assert_eq!(resp.download.profile_iccid, "8900000000000000002");
}

#[test]
fn test_decodes_xml_odsa_response_token_and_eligibility() {
    let xml = r#"<?xml version="1.0"?>
<wap-provisioningdoc version="1.1">
  <characteristic type="TOKEN">
    <parm name="token" value="ABC123"/>
    <parm name="Validity" value="3600"/>
  </characteristic>
  <characteristic type="APPLICATION">
    <parm name="OperationResult" value="1"/>
    <parm name="PrimaryAppEligibility" value="1"/>
  </characteristic>
</wap-provisioningdoc>"#;

    let resp = OdsaResponse::decode(xml.as_bytes());
    assert_eq!(resp.authentication_token, "ABC123");
    assert_eq!(resp.auth_token_validity, 3600);
    assert_eq!(resp.operation_result, OperationResult::SUCCESS);
    assert_eq!(resp.primary_app_eligibility, AppEligibility::ENABLED);
}
