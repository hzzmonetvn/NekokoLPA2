use nlpa_core::{
    bytes_to_hex, find_matching_ci_pkid, format_ci_pkid, hex_to_bytes, oid_bytes_to_string,
    parse_ci_pkid_hex_list, sha1, verify_ci_public_key_constraint, X509Certificate,
};

#[test]
fn test_oid_bytes_to_string() {
    assert_eq!(oid_bytes_to_string(&[0x55, 0x04, 0x03]), "2.5.4.3");
    assert_eq!(oid_bytes_to_string(&[0x55, 0x1D, 0x0E]), "2.5.29.14");
    assert_eq!(oid_bytes_to_string(&[0x55, 0x1D, 0x23]), "2.5.29.35");
}

#[test]
fn test_parse_ci_pkid_hex_list_and_formatting() {
    let raw_hex = "84983E441C3BD26EBAAE4AA1F95129E5E54670F1";
    let list = parse_ci_pkid_hex_list(&[raw_hex]).expect("valid pkid hex");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].len(), 20);
    assert_eq!(format_ci_pkid(&list[0]), raw_hex);

    // Invalid length
    assert!(parse_ci_pkid_hex_list(&["1234"]).is_err());
}

#[test]
fn test_x509_pem_and_der_parsing() {
    // Standard self-signed test certificate for GSMA CI / SM-DP+
    let cert_pem = r#"-----BEGIN CERTIFICATE-----
MIIBgjCCASmgAwIBAgIUWEquEgQb352AeoUtQMnjl1XLHB8wCgYIKoZIzj0EAwIw
FzEVMBMGA1UEAwwMR1NNQSBDSSBSb290MB4XDTI2MTEwNDAxNDk0NloXDTM2MTEw
MTAxNDk0NlowFzEVMBMGA1UEAwwMR1NNQSBDSSBSb290MFkwEwYHKoZIzj0CAQYI
KoZIzj0DAQcDQgAEk3qmJmGRMcnQfORD2VjY/sLWRScSzIGvC5ozrAFiZICrl97p
6sg0A9cDNmbOlMgxqns3CNy1U8x3QS7KfbAH/KNTMFEwHQYDVR0OBBYEFHk6byuv
JZU8AFRHVe5RkMhKuoauMB8GA1UdIwQYMBaAFHk6byuvJZU8AFRHVe5RkMhKuoau
MA8GA1UdEwEB/wQFMAMBAf8wCgYIKoZIzj0EAwIDRwAwRAIhAItJXitUqLUKvXn4
UZiPZhJ/SndoeYig43ohBL1hija8Ah8C7piEXm6JRFB4aoEpJ5aN8ATpCwJYmpsH
6Wanvctd
-----END CERTIFICATE-----"#;

    let cert = X509Certificate::from_pem(cert_pem).expect("parse cert pem");
    assert_eq!(cert.version, 3);
    assert_eq!(cert.subject.common_name.as_deref(), Some("GSMA CI Root"));
    assert_eq!(cert.issuer.common_name.as_deref(), Some("GSMA CI Root"));
    assert!(cert.is_ca);

    // Verify SKI (20 bytes)
    let ski = cert.ski();
    assert_eq!(ski.len(), 20);
    let ski_hex = bytes_to_hex(&ski).to_ascii_uppercase();
    assert_eq!(ski_hex, "793A6F2BAF25953C00544755EE5190C84ABA86AE");

    // Verify AKI matches SKI for self-signed root
    assert_eq!(
        cert.authority_key_identifier.as_deref(),
        Some(ski.as_slice())
    );
}

#[test]
fn test_sgp22_ci_public_key_constraint_verification() {
    let cert_pem = r#"-----BEGIN CERTIFICATE-----
MIIBgjCCASmgAwIBAgIUWEquEgQb352AeoUtQMnjl1XLHB8wCgYIKoZIzj0EAwIw
FzEVMBMGA1UEAwwMR1NNQSBDSSBSb290MB4XDTI2MTEwNDAxNDk0NloXDTM2MTEw
MTAxNDk0NlowFzEVMBMGA1UEAwwMR1NNQSBDSSBSb290MFkwEwYHKoZIzj0CAQYI
KoZIzj0DAQcDQgAEk3qmJmGRMcnQfORD2VjY/sLWRScSzIGvC5ozrAFiZICrl97p
6sg0A9cDNmbOlMgxqns3CNy1U8x3QS7KfbAH/KNTMFEwHQYDVR0OBBYEFHk6byuv
JZU8AFRHVe5RkMhKuoauMB8GA1UdIwQYMBaAFHk6byuvJZU8AFRHVe5RkMhKuoau
MA8GA1UdEwEB/wQFMAMBAf8wCgYIKoZIzj0EAwIDRwAwRAIhAItJXitUqLUKvXn4
UZiPZhJ/SndoeYig43ohBL1hija8Ah8C7piEXm6JRFB4aoEpJ5aN8ATpCwJYmpsH
6Wanvctd
-----END CERTIFICATE-----"#;

    let cert = X509Certificate::from_pem(cert_pem).expect("parse cert pem");
    let target_ci_pkid = hex_to_bytes("793A6F2BAF25953C00544755EE5190C84ABA86AE").unwrap();
    let other_ci_pkid = hex_to_bytes("0102030405060708090A0B0C0D0E0F1011121314").unwrap();

    // 1. Verification succeeds when target CI is present in eUICC's allowed list
    let allowed_list = vec![other_ci_pkid.clone(), target_ci_pkid.clone()];
    assert!(verify_ci_public_key_constraint(&cert, &allowed_list));
    assert_eq!(
        find_matching_ci_pkid(&cert, &allowed_list),
        Some(target_ci_pkid.as_slice())
    );

    // 2. Verification fails when allowed list contains only unmatching CIs
    let untrusted_list = vec![other_ci_pkid];
    assert!(!verify_ci_public_key_constraint(&cert, &untrusted_list));
    assert_eq!(find_matching_ci_pkid(&cert, &untrusted_list), None);

    // 3. Verification fails on empty allowed list
    assert!(!verify_ci_public_key_constraint(&cert, &[]));
}

#[test]
fn test_rfc5280_method1_fallback_ski_computation() {
    let mut cert = X509Certificate {
        version: 3,
        serial_number: vec![0x01],
        signature_algorithm: "1.2.840.10045.2.1".to_string(),
        issuer: Default::default(),
        validity: nlpa_core::Validity {
            not_before: "2026-01-01".to_string(),
            not_after: "2036-01-01".to_string(),
        },
        subject: Default::default(),
        subject_public_key_info: nlpa_core::SubjectPublicKeyInfo {
            algorithm_oid: "1.2.840.10045.2.1".to_string(),
            public_key_bytes: b"test_public_key_bytes_for_rfc5280_method1_calculation".to_vec(),
            raw_bytes: Vec::new(),
        },
        subject_key_identifier: None, // No extension present
        authority_key_identifier: None,
        is_ca: false,
        key_usage: None,
        raw_tbs: Vec::new(),
        signature_value: Vec::new(),
    };

    let computed_ski = cert.ski();
    let expected_sha1 = sha1(b"test_public_key_bytes_for_rfc5280_method1_calculation");
    assert_eq!(computed_ski, expected_sha1);

    // When extension is set to 20 bytes, it takes precedence
    let explicit_ski = [0x55u8; 20];
    cert.subject_key_identifier = Some(explicit_ski.to_vec());
    assert_eq!(cert.ski(), explicit_ski);
}
