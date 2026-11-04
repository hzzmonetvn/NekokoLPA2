use std::error::Error;
use std::sync::{Arc, Mutex};

use nlpa_core::base64_encode;
use nlpa_core::lpa::ts43::{
    ActivationResult, AkaRequest, AkaResponse, Device, Endpoint, Entitlement, EntitlementChannel,
    Identity, SourceSimProvider, TransferType, Ts43Client, Ts43Config, Ts43Event, Ts43HttpRequest,
    Ts43HttpResponse, Ts43State, Ts43Transport,
};

struct FakeTransport {
    bodies: Vec<String>,
    index: Mutex<usize>,
    sent: Mutex<Vec<Ts43HttpRequest>>,
}

impl FakeTransport {
    fn new(bodies: Vec<String>) -> Self {
        Self {
            bodies,
            index: Mutex::new(0),
            sent: Mutex::new(Vec::new()),
        }
    }

    fn sent_requests(&self) -> Vec<Ts43HttpRequest> {
        self.sent.lock().unwrap().clone()
    }
}

impl Ts43Transport for FakeTransport {
    fn do_raw(
        &self,
        request: &Ts43HttpRequest,
    ) -> Result<Ts43HttpResponse, Box<dyn Error + Send + Sync>> {
        self.sent.lock().unwrap().push(request.clone());
        let mut idx = self.index.lock().unwrap();
        let cur = *idx;
        if cur < self.bodies.len() - 1 {
            *idx += 1;
        }
        let body = self.bodies[cur].clone();
        Ok(Ts43HttpResponse::new(body.into_bytes(), Vec::new()))
    }
}

struct FakeSource;

impl SourceSimProvider for FakeSource {
    fn identity(&self) -> Result<Identity, Box<dyn Error + Send + Sync>> {
        Ok(Identity {
            iccid: "8900000000000000001".to_string(),
            imsi: "234107813240779".to_string(),
            mcc: "234".to_string(),
            mnc: "10".to_string(),
            gid1: "5A".to_string(),
        })
    }

    fn authenticate_aka(
        &self,
        _request: &AkaRequest,
    ) -> Result<AkaResponse, Box<dyn Error + Send + Sync>> {
        panic!("EAP-AKA relay not expected in this test");
    }
}

#[test]
fn test_source_credential_transfer_reaches_download_ready_then_done() {
    let ac = base64_encode(b"1$smdp.test$MID-1");
    let canned = vec![
        // 1. Authenticate -> token
        serde_json::json!({
            "TOKEN": { "token": "TOK", "Validity": "3600" }
        })
        .to_string(),
        // 2. CheckEligibility -> eligible
        serde_json::json!({
            "APPLICATION": { "OperationResult": "1", "PrimaryAppEligibility": "1" }
        })
        .to_string(),
        // 3. ManageSubscription -> download profile
        serde_json::json!({
            "APPLICATION": {
                "OperationResult": "1",
                "SubscriptionResult": "2",
                "DownloadInfo": {
                    "ProfileActivationCode": format!("encodedValue={ac}"),
                    "ProfileIccid": "8900000000000000002"
                }
            }
        })
        .to_string(),
    ];

    let transport = Arc::new(FakeTransport::new(canned));
    let source_sim = Arc::new(FakeSource);

    let mut client = Ts43Client::new(Ts43Config {
        transport: transport.clone(),
        source_sim: Some(source_sim),
        entitlement_records: None,
        entitlement: Entitlement {
            server: "https://entitlement.test/ts43".to_string(),
            channel: EntitlementChannel::Google,
            transfer_type: TransferType::REUSABLE_ACTIVATION_CODE,
            ..Default::default()
        },
        source: Endpoint {
            device: Device {
                imei: "350000000000006".to_string(),
                ..Default::default()
            },
        },
        target: Endpoint {
            device: Device {
                imei: "350000000000006".to_string(),
                eid: "89000000000000000000000000000001".to_string(),
                ..Default::default()
            },
        },
        companion: Device::default(),
        app_version: "1".to_string(),
        client_ts43: String::new(),
        realm: String::new(),
    });

    let mut result = client.transfer().expect("transfer should succeed");
    assert_eq!(result.state, Ts43State::PendingDownload);
    assert_eq!(
        result.download_config.activation_code,
        "LPA:1$smdp.test$MID-1"
    );
    assert!(matches!(result.event, Some(Ts43Event::DownloadReady(_))));

    // Source-credential path completes immediately on activation
    let done = client
        .complete_activation(
            &mut result,
            &ActivationResult {
                iccid: "8900000000000000002".to_string(),
            },
        )
        .expect("complete_activation should succeed");
    assert_eq!(done.state, Ts43State::Done);
    assert!(client.events().iter().any(|e| matches!(e, Ts43Event::Done)));

    // Verify 3 HTTP requests were dispatched
    assert_eq!(transport.sent_requests().len(), 3);
}
