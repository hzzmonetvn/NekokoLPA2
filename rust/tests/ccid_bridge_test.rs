use rust_lib_nlpa2::api::ccid::{
    atr_euicc_capability_enabled, atr_set_euicc_capability,
    nine_esim_enable_atr_programming_command, nine_esim_password_hex, nine_esim_set_atr_command,
    nine_esim_set_password_command,
};

#[test]
fn test_ccid_bridge_password_and_command() {
    assert_eq!(nine_esim_password_hex("123456".to_string()), "123456");

    let cmd = nine_esim_set_password_command("A1B2C3".to_string(), "123456".to_string())
        .expect("valid command");
    assert_eq!(
        nlpa_core::hex::bytes_to_hex(&cmd).to_ascii_uppercase(),
        "3027000009111111A1B2C3123456"
    );

    let prog_cmd = nine_esim_enable_atr_programming_command();
    assert_eq!(
        nlpa_core::hex::bytes_to_hex(&prog_cmd).to_ascii_uppercase(),
        "3097431800"
    );
}

#[test]
fn test_ccid_bridge_atr_operations() {
    let default_atr = "3B9F96801F038031E073FE21136702434F537EC101FF".to_string();
    assert!(atr_euicc_capability_enabled(default_atr.clone()).expect("query ok"));

    let disabled = atr_set_euicc_capability(default_atr.clone(), false).expect("toggle off ok");
    assert_eq!(disabled, "3B9F96801F018031E073FE21136702434F537EC101FD");
    assert!(!atr_euicc_capability_enabled(disabled.clone()).expect("query ok"));

    let re_enabled = atr_set_euicc_capability(disabled, true).expect("toggle on ok");
    assert_eq!(re_enabled, default_atr);

    let atr_bytes = nlpa_core::hex::hex_to_bytes("3B9F12801F038031E073FE21136702434F537EC1017B")
        .expect("valid hex");
    let set_atr = nine_esim_set_atr_command(atr_bytes).expect("set atr command ok");
    assert_eq!(
        nlpa_core::hex::bytes_to_hex(&set_atr).to_ascii_uppercase(),
        "3038000016159F12801F038031E073FE21136702434F537EC1017B"
    );
}
