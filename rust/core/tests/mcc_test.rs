use nlpa_core::{get_flag_emoji, mcc_to_country_code, parse_operator_id};

#[test]
fn test_mcc_to_country_code_mappings() {
    // Asia
    assert_eq!(mcc_to_country_code("452", None, None), "VN"); // Vietnam
    assert_eq!(mcc_to_country_code("440", None, None), "JP"); // Japan
    assert_eq!(mcc_to_country_code("460", None, None), "CN"); // China
    assert_eq!(mcc_to_country_code("450", None, None), "KR"); // South Korea

    // Americas
    assert_eq!(mcc_to_country_code("310", None, None), "US"); // United States
    assert_eq!(mcc_to_country_code("302", None, None), "CA"); // Canada
    assert_eq!(mcc_to_country_code("724", None, None), "BR"); // Brazil

    // Europe
    assert_eq!(mcc_to_country_code("208", None, None), "FR"); // France
    assert_eq!(mcc_to_country_code("262", None, None), "DE"); // Germany
    assert_eq!(mcc_to_country_code("234", None, None), "GB"); // United Kingdom
    assert_eq!(mcc_to_country_code("234", Some("50"), None), "JE"); // Jersey

    // Unknown
    assert_eq!(mcc_to_country_code("999", None, None), "UN");
}

#[test]
fn test_get_flag_emoji() {
    assert_eq!(get_flag_emoji("VN"), "🇻🇳");
    assert_eq!(get_flag_emoji("vn"), "🇻🇳");
    assert_eq!(get_flag_emoji("US"), "🇺🇸");
    assert_eq!(get_flag_emoji("JP"), "🇯🇵");
    assert_eq!(get_flag_emoji("DE"), "🇩🇪");

    // Invalid codes return globe
    assert_eq!(get_flag_emoji(""), "🌐");
    assert_eq!(get_flag_emoji("V"), "🌐");
    assert_eq!(get_flag_emoji("VNN"), "🌐");
    assert_eq!(get_flag_emoji("12"), "🌐");
}

#[test]
fn test_parse_operator_id_plmn() {
    // 3GPP PLMN 3-byte format:
    // e.g., MCC 452, MNC 04 (Viettel Vietnam):
    // Byte 0: (5 << 4) | 4 = 0x54
    // Byte 1: (F << 4) | 2 = 0xF2 (MNC digit 3 is F => 2-digit MNC)
    // Byte 2: (4 << 4) | 0 = 0x40
    let plmn_2digit = [0x54, 0xF2, 0x40];
    let (mcc, mnc) = parse_operator_id(&plmn_2digit).expect("valid plmn");
    assert_eq!(mcc, "452");
    assert_eq!(mnc, "04");

    // 3-digit MNC: MCC 310, MNC 410 (AT&T US):
    // Byte 0: (1 << 4) | 3 = 0x13
    // Byte 1: (0 << 4) | 0 = 0x00 (n3 = 0, m3 = 0)
    // Byte 2: (1 << 4) | 4 = 0x14 (n2 = 1, n1 = 4)
    let plmn_3digit = [0x13, 0x00, 0x14];
    let (mcc3, mnc3) = parse_operator_id(&plmn_3digit).expect("valid 3-digit plmn");
    assert_eq!(mcc3, "310");
    assert_eq!(mnc3, "410");

    assert!(parse_operator_id(&[0x12]).is_none());
}
