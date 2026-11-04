use nlpa_core::ccid::nine_esim::{
    calculate_crc24, enable_atr_programming_command, euicc_capability_enabled, password_hex,
    set_atr_command, set_euicc_capability, set_password_command, DEFAULT_9ESIM_ATR,
};
use nlpa_core::hex::bytes_to_hex;

#[test]
fn test_nine_esim_enable_atr_programming() {
    let cmd = enable_atr_programming_command();
    assert_eq!(bytes_to_hex(&cmd).to_ascii_uppercase(), "3097431800");
}

#[test]
fn test_nine_esim_set_atr_command() {
    let atr_bytes = nlpa_core::hex::hex_to_bytes("3B9F12801F038031E073FE21136702434F537EC1017B")
        .expect("valid hex");

    let cmd = set_atr_command(&atr_bytes).expect("build set_atr command");
    assert_eq!(
        bytes_to_hex(&cmd).to_ascii_uppercase(),
        "3038000016159F12801F038031E073FE21136702434F537EC1017B"
    );
}

#[test]
fn test_nine_esim_euicc_capability_helpers() {
    assert!(euicc_capability_enabled(DEFAULT_9ESIM_ATR).expect("query ok"));

    let disabled = set_euicc_capability(DEFAULT_9ESIM_ATR, false).expect("toggle off ok");
    assert_eq!(disabled, "3B9F96801F018031E073FE21136702434F537EC101FD");
    assert!(!euicc_capability_enabled(&disabled).expect("query ok"));

    let re_enabled = set_euicc_capability(&disabled, true).expect("toggle on ok");
    assert_eq!(re_enabled, DEFAULT_9ESIM_ATR);
}

#[test]
fn test_nine_esim_password_and_command() {
    // 6-digit hex password is preserved uppercase
    assert_eq!(password_hex("123456"), "123456");
    assert_eq!(password_hex("abcdef"), "ABCDEF");

    // Arbitrary text is hashed using CRC-24
    let crc = calculate_crc24(b"test-password");
    assert_eq!(password_hex("test-password"), format!("{crc:06X}"));

    // Set password command
    let cmd = set_password_command("A1B2C3", "123456").expect("valid command");
    assert_eq!(
        bytes_to_hex(&cmd).to_ascii_uppercase(),
        "3027000009111111A1B2C3123456"
    );

    // Invalid device name format
    assert!(set_password_command("A1B2", "123456").is_err());
    assert!(set_password_command("NOTHEX", "123456").is_err());
}
