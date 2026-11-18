use nlpa_core::{hex_to_bytes, is_likely_euicc_atr, supports_euicc, DEFAULT_EUICC_ATR};

fn decode(hex: &str) -> Vec<u8> {
    hex_to_bytes(hex).expect("valid hex")
}

#[test]
fn reads_euicc_flag_in_tb3_following_td2_t15() {
    assert_eq!(supports_euicc(&decode("3B80802F82AD")), Some(true));
    assert_eq!(supports_euicc(&decode("3F80802F82AD")), Some(true));
    assert!(is_likely_euicc_atr(&decode("3B80802F82AD")));
    assert!(is_likely_euicc_atr(&decode("3F80802F82AD")));
}

#[test]
fn requires_both_b8_and_b2_in_etsi_global_tb_byte() {
    assert_eq!(supports_euicc(&decode("3B80802F80AF")), Some(false));
    assert_eq!(supports_euicc(&decode("3B80802F022D")), Some(false));
    assert!(!is_likely_euicc_atr(&decode("3B80802F80AF")));
    assert!(!is_likely_euicc_atr(&decode("3B80802F022D")));
}

#[test]
fn accepts_known_9esim_t15_global_ta_euicc_capability_byte() {
    // DEFAULT_EUICC_ATR sets bit 0x02 in TA3 (0x03) following TD2=0x1F (T=15)
    let default_bytes = decode(DEFAULT_EUICC_ATR);
    assert!(is_likely_euicc_atr(&default_bytes));

    // Toggling off bit 0x02 (0x03 -> 0x01, recalculated TCK=0xFD) is no longer an eUICC ATR
    let disabled_bytes = decode("3B9F96801F018031E073FE21136702434F537EC101FD");
    assert!(!is_likely_euicc_atr(&disabled_bytes));

    // Pure 0x02 TA3 following TD2=0x1F with valid TCK (80 ^ 80 ^ 1F ^ 02 = 1D)
    assert!(is_likely_euicc_atr(&decode("3B80801F021D")));
    // Only TD1=0x1F puts 0x02 in TA2 (specific mode byte), not global TA3
    assert!(!is_likely_euicc_atr(&decode("3B801F029D")));
}

#[test]
fn does_not_scan_historical_bytes_or_protocol_specific_tb() {
    assert_eq!(supports_euicc(&decode("3B028282")), Some(false));
    assert_eq!(supports_euicc(&decode("3B2082")), Some(false));
    assert_eq!(supports_euicc(&decode("3B80218223")), Some(false));
    assert!(!is_likely_euicc_atr(&decode("3B028282")));
    assert!(!is_likely_euicc_atr(&decode("3B2082")));
    assert!(!is_likely_euicc_atr(&decode("3B80218223")));
}

#[test]
fn optional_global_tb_can_be_absent_on_real_euicc_atr() {
    // Real ATR with TA3=0xC7 (standard clock stop + voltage classes A/B/C), no TB3
    let atr = decode("3B9F96801FC78031E073FE211B57AA866016010001BA");
    assert_eq!(supports_euicc(&atr), Some(false));
    assert!(!is_likely_euicc_atr(&atr));
}

#[test]
fn later_global_byte_cannot_override_first_global_tb() {
    assert_eq!(supports_euicc(&decode("3B8080AF802F8282")), Some(false));
}

#[test]
fn validates_layout_historical_length_tck_and_truncation() {
    for bad in ["3B80802F82AA", "3B80812F82AD", "3B0282", "3B0082"] {
        let bytes = decode(bad);
        assert_eq!(supports_euicc(&bytes), None, "expected None for {bad}");
        assert!(!is_likely_euicc_atr(&bytes), "expected false for {bad}");
    }

    let valid = decode("3B80802F82AD");
    for len in 0..valid.len() {
        assert_eq!(supports_euicc(&valid[..len]), None);
        assert!(!is_likely_euicc_atr(&valid[..len]));
    }

    assert_eq!(supports_euicc(&[0x00, 0x00]), None);
    assert_eq!(supports_euicc(&[0x3B; 34]), None);
    assert_eq!(supports_euicc(&[0x3B, 0x00]), Some(false));
}
