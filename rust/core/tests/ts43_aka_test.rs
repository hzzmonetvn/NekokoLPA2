use nlpa_core::lpa::ts43::{imsi_eap_identity, respond_to_challenge, AkaChallenge, AkaResponse};
use nlpa_core::{base64_decode, bytes_to_hex};

#[test]
fn test_respond_to_challenge_matches_android_vector() {
    // Ported verbatim from ts43-go sim/aka_test.go & test/ts43/aka_test.dart
    const CHALLENGE_B64: &str =
        "AQIAfBcBAAABBQAAXOZSkCjxysgE43GWqHJvgQIFAABrikWGrekAALNU4TxmCDPoCwUAAJT0nqXeAYlqzT0UGXINENWBBQAA7z3fhImkg+vcCKWIZBdvuYIJAAAPRUFp7KWFo+Thr78Qj9hEkB2zA0i6KakODsufBC+BJQ==";
    const RESPONSE_B64: &str =
        "2wjHnwKln8mjjxDzMKJvLBzMHtm0X9SNBsUWEAbEiAdD7xeqqZ7nsXzukRkIhd6SDZ4bj7s=";
    const IDENTITY: &str = "0234107813240779@nai.epc.mnc010.mcc234.3gppnetwork.org";
    const WANT: &str =
        "020200281701000003030040C79F02A59FC9A38F0B0500001C141BDDDE8BEF2E502FB6793808DE7C";

    let challenge = AkaChallenge::from_base64(CHALLENGE_B64).expect("challenge");
    let resp_bytes = base64_decode(RESPONSE_B64).expect("resp base64");
    let response = AkaResponse::from_bytes(&resp_bytes).expect("aka response");

    let got = respond_to_challenge(&challenge, &response, IDENTITY).expect("respond");
    assert_eq!(bytes_to_hex(&got).to_uppercase(), WANT);
}

#[test]
fn test_aka_challenge_roundtrip() {
    const CHALLENGE_B64: &str =
        "AQIAfBcBAAABBQAAXOZSkCjxysgE43GWqHJvgQIFAABrikWGrekAALNU4TxmCDPoCwUAAJT0nqXeAYlqzT0UGXINENWBBQAA7z3fhImkg+vcCKWIZBdvuYIJAAAPRUFp7KWFo+Thr78Qj9hEkB2zA0i6KakODsufBC+BJQ==";
    let challenge = AkaChallenge::from_base64(CHALLENGE_B64).expect("valid challenge");
    assert_eq!(challenge.rand.len(), 16);
    assert_eq!(challenge.autn.len(), 16);
}

#[test]
fn test_aka_response_parses_success_security_context() {
    const RESPONSE_B64: &str =
        "2wjHnwKln8mjjxDzMKJvLBzMHtm0X9SNBsUWEAbEiAdD7xeqqZ7nsXzukRkIhd6SDZ4bj7s=";
    let resp_bytes = base64_decode(RESPONSE_B64).expect("base64");
    let response = AkaResponse::from_bytes(&resp_bytes).expect("parse security context");
    assert!(!response.is_sync_failure());
    assert!(response.res.is_some());
    assert!(response.ck.is_some());
    assert!(response.ik.is_some());
}

#[test]
fn test_imsi_eap_identity_builds_nai() {
    let id = imsi_eap_identity("234010", "234107813240779", "nai.epc").expect("identity");
    assert_eq!(id, "0234107813240779@nai.epc.mnc010.mcc234.3gppnetwork.org");
}
