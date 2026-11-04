use rust_lib_nlpa2::api::telecom::{
    clean_iccid, create_euicc_profile, flag_emoji_for_country, flag_url_for_country, format_iccid,
    hex_decode, hex_encode, mcc_to_country, nibble_swap, parse_plmn_operator_id,
};

#[test]
fn test_telecom_bridge_mcc_and_flags() {
    let country = mcc_to_country("208".to_string(), None, None);
    assert_eq!(country, "FR");

    let uk_im = mcc_to_country("234".to_string(), Some("18".to_string()), None);
    assert_eq!(uk_im, "IM");

    let emoji = flag_emoji_for_country("FR".to_string());
    assert_eq!(emoji, "🇫🇷");

    let url = flag_url_for_country("US".to_string());
    assert!(url.contains("static.toss.im"));
    assert!(url.contains("u1F1FA_u1F1F8"));

    // PLMN parsing: 3-byte BCD (e.g. 208-01 = FR Orange)
    // 208-01 in 3 bytes: Byte 0 = 0x02, Byte 1 = 0xF8, Byte 2 = 0x10
    let plmn_bytes = vec![0x02, 0xF8, 0x10];
    let op = parse_plmn_operator_id(plmn_bytes).expect("valid plmn");
    assert_eq!(op.mcc, "208");
    assert_eq!(op.mnc, "01");
}

#[test]
fn test_telecom_bridge_codecs() {
    let hex = hex_encode(vec![0xDE, 0xAD, 0xBE, 0xEF]);
    assert_eq!(hex, "deadbeef");

    let bytes = hex_decode("DEADBEEF".to_string()).expect("valid hex");
    assert_eq!(bytes, vec![0xDE, 0xAD, 0xBE, 0xEF]);

    let swapped = nibble_swap("890104".to_string());
    assert_eq!(swapped, "981040");

    let formatted_iccid = format_iccid("89014103211118510720".to_string());
    assert_eq!(formatted_iccid, "8901 4103 2111 1851 0720");

    let cleaned = clean_iccid("8901410321111851072F".to_string());
    assert_eq!(cleaned, "8901410321111851072");

    let profile = create_euicc_profile("89014103211118510720".to_string(), true);
    assert_eq!(profile.iccid, "89014103211118510720");
    assert!(profile.enabled);
}
