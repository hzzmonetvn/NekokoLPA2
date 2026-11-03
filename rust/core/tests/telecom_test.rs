use nlpa_core::{
    bytes_to_hex, calculate_imei_luhn_checksum, encode_stored_imei_bytes, format_iccid_grouped,
    hex_to_bytes, iccid_for_display, imei_digits_from_stored_bytes, swap_nibbles, validate_apdu_sw,
    ActivationCode, ActivationCodeError, ApduError, Asn1Reader, EuiccProfile, HexError, ImeiError,
};

#[test]
fn test_hex_encode_and_decode_roundtrip() {
    let original = vec![0x00, 0xA4, 0x04, 0x00, 0x07, 0xA0, 0x00, 0x00, 0x05, 0x59];
    let hex = bytes_to_hex(&original);
    assert_eq!(hex, "00a4040007a000000559");

    let decoded = hex_to_bytes(&hex).expect("valid hex should decode");
    assert_eq!(decoded, original);
}

#[test]
fn test_hex_decode_with_spaces_and_case() {
    let input = "00 A4 04 00 07 A0 00 00 05 59";
    let decoded = hex_to_bytes(input).expect("spaced hex should decode");
    assert_eq!(
        decoded,
        vec![0x00, 0xA4, 0x04, 0x00, 0x07, 0xA0, 0x00, 0x00, 0x05, 0x59]
    );

    let err = hex_to_bytes("00 ZZ").expect_err("invalid hex should fail");
    assert_eq!(
        err,
        HexError::InvalidCharacter {
            char: 'Z',
            index: 3,
        }
    );
}

#[test]
fn test_swap_nibbles() {
    let input = "898404";
    let swapped = swap_nibbles(input);
    assert_eq!(swapped, "984840");

    let odd = "123";
    let swapped_odd = swap_nibbles(odd);
    assert_eq!(swapped_odd, "1F32");
}

#[test]
fn test_activation_code_parser_and_display() {
    let raw = "LPA:1$smdp.example.com$MATCH-12345$1.3.6.1.4.1$CONFIRM";
    let parsed = ActivationCode::parse(raw).expect("should parse valid LPA code");
    assert_eq!(parsed.format, "LPA:1");
    assert_eq!(parsed.smdp_address, "smdp.example.com");
    assert_eq!(parsed.matching_id, "MATCH-12345");
    assert_eq!(parsed.smdp_oid.as_deref(), Some("1.3.6.1.4.1"));
    assert_eq!(parsed.confirmation_code.as_deref(), Some("CONFIRM"));

    let formatted = parsed.to_string();
    assert_eq!(formatted, raw);
}

#[test]
fn test_activation_code_parser_minimal() {
    let raw = "LPA:1$smdp.prod.net$";
    let parsed = ActivationCode::parse(raw).expect("should parse minimal code");
    assert_eq!(parsed.smdp_address, "smdp.prod.net");
    assert_eq!(parsed.matching_id, "");
    assert_eq!(parsed.smdp_oid, None);
    assert_eq!(parsed.confirmation_code, None);

    let err = ActivationCode::parse("INVALID$CODE").expect_err("should reject invalid header");
    assert_eq!(err, ActivationCodeError::InvalidHeader);
}

#[test]
fn test_activation_code_extraction_and_validation() {
    // 1. Extract from URI query string
    let url =
        "https://lpa.example.com/download?code=LPA:1$smdp.example.com$MATCH-99&carrier=telecom";
    let extracted = ActivationCode::extract_from_text(url).expect("extract from url");
    assert_eq!(extracted.smdp_address, "smdp.example.com");
    assert_eq!(extracted.matching_id, "MATCH-99");
    assert!(extracted.is_valid_smdp_address());

    // 2. Extract from informal text with lowercase prefix
    let text =
        "Scan this profile: lpa:1$rsp.truphone.com$ABCDEF123456$1.2.3$1$4321 on your device.";
    let extracted2 = ActivationCode::extract_from_text(text).expect("extract from text");
    assert_eq!(extracted2.smdp_address, "rsp.truphone.com");
    assert_eq!(extracted2.matching_id, "ABCDEF123456");
    assert_eq!(extracted2.smdp_oid.as_deref(), Some("1.2.3"));
    assert!(extracted2.confirmation_code_required);
    assert_eq!(extracted2.confirmation_code.as_deref(), Some("4321"));
    assert!(extracted2.is_valid_smdp_address());

    // 3. Invalid SMDP address detection
    let bad_ac = ActivationCode::new("bad address with spaces", "MATCH", None, None);
    assert!(!bad_ac.is_valid_smdp_address());
}

#[test]
fn test_iccid_formatting() {
    let raw = "8984040000000000001F";
    let display = iccid_for_display(raw);
    assert_eq!(display, "8984040000000000001");

    let grouped = format_iccid_grouped(raw);
    assert_eq!(grouped, "8984 0400 0000 0000 001");
}

#[test]
fn test_apdu_status_validation() {
    assert_eq!(validate_apdu_sw(0x90, 0x00), Ok(()));
    assert_eq!(
        validate_apdu_sw(0x69, 0x85),
        Err(ApduError::ConditionOfUseNotSatisfied)
    );
    assert_eq!(validate_apdu_sw(0x68, 0x81), Err(ApduError::CardBusy));
    assert_eq!(
        validate_apdu_sw(0x6A, 0x80),
        Err(ApduError::InvalidDataEncoding)
    );
    assert_eq!(
        validate_apdu_sw(0x6A, 0x88),
        Err(ApduError::ReferenceDataNotFound)
    );
    assert_eq!(
        validate_apdu_sw(0x91, 0x1A),
        Err(ApduError::ProactiveRefresh {
            sw1: 0x91,
            sw2: 0x1A
        })
    );
    assert_eq!(
        validate_apdu_sw(0x6F, 0x00),
        Err(ApduError::Failure {
            sw1: 0x6F,
            sw2: 0x00
        })
    );
}

#[test]
fn test_imei_luhn_and_bcd_roundtrip() {
    let body = "86012345678901";
    let check = calculate_imei_luhn_checksum(body).expect("should calculate luhn check digit");
    assert_eq!(check, 4);

    let full_imei = format!("{body}{check}");
    assert_eq!(full_imei.len(), 15);

    let bcd = encode_stored_imei_bytes(&full_imei).expect("should encode 15 digits");
    let recovered = imei_digits_from_stored_bytes(&bcd).expect("should decode bcd bytes");
    assert_eq!(recovered, full_imei);

    let bad_len = calculate_imei_luhn_checksum("123").expect_err("should reject invalid length");
    assert_eq!(
        bad_len,
        ImeiError::InvalidLength {
            expected: "14 digits",
            actual: 3,
        }
    );
}

#[test]
fn test_asn1_reader_and_hierarchy() {
    // ASN.1 Sequence (0x30, len 0x06): Tag 0x02 (INTEGER, len 1, val 0x01), Tag 0x04 (OCTET STRING, len 1, val 0xAA)
    let der_data = vec![0x30, 0x06, 0x02, 0x01, 0x01, 0x04, 0x01, 0xAA];
    let mut reader = Asn1Reader::new(&der_data);
    let nodes = reader.read_all().expect("should parse valid DER");
    assert_eq!(nodes.len(), 1);

    let seq = &nodes[0];
    assert!(seq.is_tag(0x30));
    assert_eq!(seq.raw_bytes, der_data);

    let children = seq.children().expect("should decode children");
    assert_eq!(children.len(), 2);
    assert!(children[0].is_tag(0x02));
    assert_eq!(children[0].value, vec![0x01]);
    assert!(children[1].is_tag(0x04));
    assert_eq!(children[1].value, vec![0xAA]);

    let octet = seq.find_child(0x04).expect("should find child tag 0x04");
    assert_eq!(octet.value, vec![0xAA]);
}

#[test]
fn test_euicc_profile_display_name() {
    let mut p = EuiccProfile::new("8984040000000000001F", true);
    assert_eq!(p.iccid, "8984040000000000001");
    assert_eq!(p.raw_iccid, "8984040000000000001F");
    assert_eq!(p.display_name(), "Unnamed Profile");

    p.service_provider_name = Some("Viettel Telecom".to_string());
    assert_eq!(p.display_name(), "Viettel Telecom");

    p.name = Some("5G Core Trial".to_string());
    assert_eq!(p.display_name(), "5G Core Trial");

    p.nickname = Some("My Daily eUICC".to_string());
    assert_eq!(p.display_name(), "My Daily eUICC");
}
