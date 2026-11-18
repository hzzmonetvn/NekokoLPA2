use nlpa_core::{
    aes_cbc_encrypt, build_estk_mgt_read_apdu, build_gm_auth_apdu, build_gm_auth_apdu_with_rn,
    bytes_to_hex, calc_host_mac, esim_cc_badge, estk_extra_aids, is_gm_eid, match_provider_plugin,
    parse_esim_gg_response, parse_estk_mgt_string, parse_three_hk_response, EstkCardInfo,
    ProviderPluginKind, ESTK_AID_CHAN1, ESTK_AID_CHAN2, ESTK_ATR_MARKER, ESTK_MGT_AID,
};

#[test]
fn test_estk_constants_and_extra_aids() {
    assert_eq!(ESTK_ATR_MARKER, b"estkme");
    assert_eq!(
        bytes_to_hex(ESTK_MGT_AID).to_ascii_uppercase(),
        "A06573746B6D65FFFFFFFFFFFF6D6774"
    );

    let estk_atr = "3B 9F 95 80 1F C7 80 31 E0 73 FE 21 1B 65 73 74 6B 6D 65 00";
    assert_eq!(
        estk_extra_aids(estk_atr),
        vec![ESTK_AID_CHAN2.to_string(), ESTK_AID_CHAN1.to_string()]
    );
    assert!(estk_extra_aids("3B9F96801F038031E073FE21136702434F537EC101FF").is_empty());

    assert_eq!(build_estk_mgt_read_apdu(0), [0x00, 0x00, 0x00, 0x00, 0x00]);
    assert_eq!(parse_estk_mgt_string(b"ESTK-2025\0\0\n"), Some("ESTK-2025".to_string()));
    assert_eq!(parse_estk_mgt_string(&[0x00, 0x00]), None);

    let info = EstkCardInfo {
        firmware: Some("1.2.3".to_string()),
        bootloader: Some("1.0.0".to_string()),
        serial: Some("SN123".to_string()),
        sku: Some("ME".to_string()),
    };
    assert_eq!(info.serial.as_deref(), Some("SN123"));
}

#[test]
fn test_gm_eid_range_and_auth_apdu() {
    assert!(is_gm_eid("89086029202200002122000030900000"));
    assert!(is_gm_eid("89086029202200002122000035000001"));
    assert!(is_gm_eid("89086029202200002122000040000000"));
    assert!(!is_gm_eid("89086029202200002122000030899999"));
    assert!(!is_gm_eid("89086029202200002122000040000001"));
    assert!(!is_gm_eid("89086029202200002122000035"));

    let psk = [0x11u8; 16];
    let challenge = [0x22u8; 16];
    let rn_host = [0x33u8; 16];

    let apdu = build_gm_auth_apdu_with_rn(&psk, &challenge, &rn_host).expect("valid GM APDU");
    // 81 E2 91 00 1B BF 71 18 80 04 <mac:4> 81 10 <rn_host:16>
    assert_eq!(apdu.len(), 32);
    assert_eq!(&apdu[0..4], &[0x81, 0xE2, 0x91, 0x00]);
    assert_eq!(&apdu[4..8], &[0x1B, 0xBF, 0x71, 0x18]);
    assert_eq!(&apdu[8..10], &[0x80, 0x04]);
    assert_eq!(&apdu[14..16], &[0x81, 0x10]);
    assert_eq!(&apdu[16..32], &rn_host);

    let mut auth_data = [0u8; 16];
    auth_data[..8].copy_from_slice(&challenge[..8]);
    auth_data[8..16].copy_from_slice(&rn_host[..8]);
    let sk_raw = aes_cbc_encrypt(&psk, &[0u8; 16], &auth_data).expect("session key");
    let expected_mac = calc_host_mac(&sk_raw, &challenge).expect("host mac");
    assert_eq!(&apdu[10..14], &expected_mac);

    let random_apdu = build_gm_auth_apdu(&psk, &challenge).expect("random nonce APDU");
    assert_eq!(random_apdu.len(), 32);
    assert!(build_gm_auth_apdu(&[0x11; 15], &challenge).is_err());
    assert!(build_gm_auth_apdu(&psk, &[0x22; 8]).is_err());
}

#[test]
fn test_provider_plugin_matching_and_response_parsing() {
    assert_eq!(match_provider_plugin("8985223000000000001"), Some(ProviderPluginKind::EsimCc));
    assert_eq!(match_provider_plugin("8937204010000000001"), Some(ProviderPluginKind::EsimGg));
    assert_eq!(match_provider_plugin("8985231000000000001"), Some(ProviderPluginKind::ThreeHk));
    assert_eq!(match_provider_plugin("8985203000000000001"), Some(ProviderPluginKind::ThreeHk));
    assert_eq!(match_provider_plugin("8901410000000000001"), None);

    let cc = esim_cc_badge("8985223000000000001").expect("eSIM.cc badge");
    assert_eq!(cc.kind, ProviderPluginKind::EsimCc);
    assert_eq!(cc.portal_url.as_deref(), Some("https://www.esim.cc/card-bonus"));

    let gg_json = r#"{
        "number": "3725000000",
        "balance": 12.345,
        "package_name": "Test package",
        "total_data": 1000,
        "remaining_data": 250,
        "package_expiry": "2026-11-01"
    }"#;
    let gg = parse_esim_gg_response(gg_json, "8937204010000000001").expect("eSIM.gg badge");
    assert_eq!(gg.kind, ProviderPluginKind::EsimGg);
    assert_eq!(gg.msisdn.as_deref(), Some("+3725000000"));
    assert!(gg.label.contains("12.35 €"));
    let gg_summary = gg.quota_summary.as_deref().expect("summary");
    assert!(gg_summary.contains("Test package"));
    assert!(gg_summary.contains("250 B / 1,000 B"));
    assert!(gg_summary.contains("2026-11-01"));

    // Bulk results format stripping trailing F
    let gg_bulk = r#"{
        "results": [{
            "iccid": "8937204010000000001",
            "number": "+3725000000",
            "balance": 5.0
        }]
    }"#;
    let gg2 = parse_esim_gg_response(gg_bulk, "8937204010000000001F").expect("bulk match");
    assert_eq!(gg2.label, "eSIM.gg · 5.00 €");

    let thk_json = r#"{
        "msisdn": "60000000",
        "line_expiry": "2027-01-01",
        "packages": [
            { "name": "Test package", "total": 10, "used": 3, "expiry": "2026-12-01" }
        ]
    }"#;
    let thk = parse_three_hk_response(thk_json, "8985231000000000001").expect("3HK badge");
    assert_eq!(thk.kind, ProviderPluginKind::ThreeHk);
    assert_eq!(thk.msisdn.as_deref(), Some("+85260000000"));
    let thk_summary = thk.quota_summary.as_deref().expect("summary");
    assert!(thk_summary.contains("Test package"));
    assert!(thk_summary.contains("10.00 GiB"));
    assert!(thk_summary.contains("Exp 2027-01-01"));
}
