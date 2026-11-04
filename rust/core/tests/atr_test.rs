use nlpa_core::atr::{Atr, AtrError, DEFAULT_EUICC_ATR};

#[test]
fn test_default_euicc_atr_parsing() {
    let atr = Atr::from_hex(DEFAULT_EUICC_ATR).expect("valid ATR");
    assert_eq!(atr.ts(), 0x3B);
    assert_eq!(atr.t0(), 0x9F);
    assert_eq!(atr.historical_bytes_count(), 15);
    assert_eq!(atr.global_interface_byte_index().expect("has T=15"), 5);
    assert!(atr.euicc_capability_enabled().expect("valid query"));
}

#[test]
fn test_euicc_capability_toggling_and_tck() {
    let mut atr = Atr::from_hex(DEFAULT_EUICC_ATR).expect("valid ATR");
    assert!(atr.euicc_capability_enabled().unwrap());

    // Toggle off
    atr.set_euicc_capability(false).expect("disable succeeds");
    assert!(!atr.euicc_capability_enabled().unwrap());
    assert_eq!(atr.to_hex(), "3B9F96801F018031E073FE21136702434F537EC101FD");

    // Toggle on again
    atr.set_euicc_capability(true).expect("enable succeeds");
    assert!(atr.euicc_capability_enabled().unwrap());
    assert_eq!(atr.to_hex(), DEFAULT_EUICC_ATR);
}

#[test]
fn test_atr_error_cases() {
    // Inverse convention 0x3F is rejected (only direct 0x3B supported)
    assert!(matches!(
        Atr::from_bytes(&[0x3F, 0x00]),
        Err(AtrError::UnsupportedConvention(0x3F))
    ));

    // Too short
    assert!(matches!(
        Atr::from_bytes(&[0x3B]),
        Err(AtrError::InvalidLength(1))
    ));

    // Odd hex string
    assert!(matches!(
        Atr::from_hex("3B9F1"),
        Err(AtrError::InvalidHex(_))
    ));

    // ATR without T=15
    let atr_t0_only = Atr::from_hex("3B800100").expect("valid ATR format");
    assert!(matches!(
        atr_t0_only.global_interface_byte_index(),
        Err(AtrError::MissingT15Protocol)
    ));
}
