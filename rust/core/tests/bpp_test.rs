use nlpa_core::{
    build_cancel_session_request_der, bytes_to_hex, decompose_bpp, encode_tlv, hex_to_bytes,
    parse_nxp_memory_tlv, parse_nxp_memory_tlv_with_firmware, Asn1Error, NxpMemoryInfo,
    EUICC_MEMORY_RESET_APDU_DATA,
};

#[test]
fn test_decompose_bpp_segments_match_sgp22_order_and_reconstruct_exact_der() {
    let init_sc = encode_tlv(0xBF23, &[0x80, 0x02, 0x11, 0x22]);

    let e87_1 = encode_tlv(0x87, &[0x01, 0x02]);
    let e87_2 = encode_tlv(0x87, &[0x03, 0x04, 0x05]);
    let seq_a0 = encode_tlv(0xA0, &[e87_1.clone(), e87_2.clone()].concat());

    let e88_1 = encode_tlv(0x88, &[0x10, 0x20]);
    let e88_2 = encode_tlv(0x88, &[0x30]);
    let seq_a1 = encode_tlv(0xA1, &[e88_1.clone(), e88_2.clone()].concat());

    let e87_b1 = encode_tlv(0x87, &[0xAA]);
    let e87_b2 = encode_tlv(0x87, &[0xBB]);
    let seq_a2 = encode_tlv(0xA2, &[e87_b1.clone(), e87_b2.clone()].concat());

    let e86_1 = encode_tlv(0x86, &[0x55, 0x66]);
    let e86_2 = encode_tlv(0x86, &[0x77, 0x88, 0x99]);
    let seq_a3 = encode_tlv(0xA3, &[e86_1.clone(), e86_2.clone()].concat());

    let bpp_inner = [
        init_sc.clone(),
        seq_a0.clone(),
        seq_a1.clone(),
        seq_a2.clone(),
        seq_a3.clone(),
    ]
    .concat();
    let bpp_der = encode_tlv(0xBF36, &bpp_inner);

    let segments = decompose_bpp(&bpp_der).expect("valid BPP");
    // Expected 11 segments:
    // 0: BF36 TL + BF23
    // 1: A0 TL + first 87
    // 2: second 87
    // 3: A1 TL
    // 4: first 88
    // 5: second 88
    // 6: A2 TL + first 87
    // 7: second 87
    // 8: A3 TL
    // 9: first 86
    // 10: second 86
    assert_eq!(segments.len(), 11);

    // Concatenating all segments must reproduce the exact BoundProfilePackage DER bytes
    let reassembled: Vec<u8> = segments.iter().flat_map(|s| s.iter().copied()).collect();
    assert_eq!(reassembled, bpp_der);

    // Verify A1 and A3 headers are standalone segments
    assert_eq!(segments[3], vec![0xA1, (e88_1.len() + e88_2.len()) as u8]);
    assert_eq!(segments[4], e88_1);
    assert_eq!(segments[5], e88_2);
    assert_eq!(segments[8], vec![0xA3, (e86_1.len() + e86_2.len()) as u8]);
    assert_eq!(segments[9], e86_1);
    assert_eq!(segments[10], e86_2);
}

#[test]
fn test_decompose_bpp_rejects_invalid_root_or_empty() {
    assert!(matches!(decompose_bpp(&[]), Err(Asn1Error::UnexpectedEof { .. })));

    let wrong = encode_tlv(0xBF20, &[0x82, 0x03, 0x02, 0x03, 0x00]);
    assert!(matches!(
        decompose_bpp(&wrong),
        Err(Asn1Error::UnexpectedTag {
            expected: 0xBF36,
            actual: 0xBF20
        })
    ));
}

#[test]
fn test_cancel_session_request_der_matches_sgp22_vector() {
    let der = build_cancel_session_request_der(&[0x01, 0x02, 0x03, 0x04], 0x00);
    assert_eq!(bytes_to_hex(&der).to_ascii_uppercase(), "BF4109800401020304810100");
}

#[test]
fn test_euicc_memory_reset_apdu_data_matches_lpac() {
    assert_eq!(bytes_to_hex(EUICC_MEMORY_RESET_APDU_DATA).to_ascii_uppercase(), "BF3404820205E0");
}

#[test]
fn test_parse_nxp_memory_tlv() {
    // Two NVM chunks (0x00010000 + 0x00002000 = 73728), volatile (0x00004000 = 16384),
    // b3 = 10 (160B overhead on non-98.0.22/98.16.25), b14 = 64
    let mut df25_val = Vec::new();
    df25_val.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x00, 0x00]); // 65536
    df25_val.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x20, 0x00]); // +8192 = 73728
    df25_val.extend_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x40, 0x00]); // volatile 16384
    df25_val.extend_from_slice(&[0x03, 0x00, 0x00, 0x00, 0x00, 0x0A]); // b3 = 10
    df25_val.extend_from_slice(&[0x0E, 0x00, 0x00, 0x00, 0x00, 0x40]); // b14 = 64

    let df25 = encode_tlv(0xDF25, &df25_val);
    let fe = encode_tlv(0xFE, &df25);

    assert_eq!(
        parse_nxp_memory_tlv(&fe),
        Some(NxpMemoryInfo {
            free_non_volatile_memory: Some(73_728),
            free_volatile_memory: Some(16_384),
        })
    );
    assert_eq!(
        parse_nxp_memory_tlv_with_firmware(&fe, "98.0.22"),
        Some(NxpMemoryInfo {
            free_non_volatile_memory: Some(73_728 - 64),
            free_volatile_memory: Some(16_384),
        })
    );
    assert_eq!(
        parse_nxp_memory_tlv_with_firmware(&fe, "98.86.1"),
        Some(NxpMemoryInfo {
            free_non_volatile_memory: Some(73_728 - 160),
            free_volatile_memory: Some(16_384),
        })
    );

    // DF31 OS info TLV alone is not memory info
    let df31 = encode_tlv(0xFE, &encode_tlv(0xDF31, &[0x81, 0x03, b'1', b'.', b'0']));
    assert_eq!(parse_nxp_memory_tlv(&df31), None);
    assert_eq!(parse_nxp_memory_tlv(&hex_to_bytes("FE00").unwrap()), None);
}
