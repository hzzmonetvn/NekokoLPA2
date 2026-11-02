use nlpa_core::{
    parse_profile_package, validate_profile_elements, Asn1Reader, DeleteProfileRequest,
    DisableProfileRequest, EnableProfileRequest, EuiccChallenge, EuiccConfiguredAddresses,
    EuiccInfo1, EuiccInfo2, EuiccMemoryResetRequest, NotificationMetadata, PeType, RspProfileInfo,
    SetNicknameRequest,
};

#[test]
fn test_configured_addresses_roundtrip() {
    let addrs = EuiccConfiguredAddresses {
        default_smdp_address: Some("smdp.prod.net".to_string()),
        root_smds_address: Some("smds.gsma.com".to_string()),
    };
    let encoded = addrs.encode();
    let mut reader = Asn1Reader::new(&encoded);
    let node = reader.read_next().unwrap().unwrap();
    let decoded = EuiccConfiguredAddresses::decode(&node).unwrap();
    assert_eq!(decoded, addrs);
}

#[test]
fn test_euicc_info1_roundtrip() {
    let info = EuiccInfo1 {
        svn: vec![0x02, 0x02, 0x02],
        euicc_firmware_ver: vec![0x01, 0x00, 0x03],
        ext_card_res: vec![0xAA, 0xBB],
    };
    let encoded = info.encode();
    let mut reader = Asn1Reader::new(&encoded);
    let node = reader.read_next().unwrap().unwrap();
    let decoded = EuiccInfo1::decode(&node).unwrap();
    assert_eq!(decoded, info);
}

#[test]
fn test_euicc_info2_roundtrip() {
    let info = EuiccInfo2 {
        profile_version: vec![0x02, 0x02, 0x00],
        svn: vec![0x02, 0x02, 0x02],
        euicc_firmware_ver: vec![0x01, 0x02, 0x03],
        ext_card_res: Vec::new(),
        free_nvram: 131_072,
        free_ram: 16_384,
    };
    let encoded = info.encode();
    let mut reader = Asn1Reader::new(&encoded);
    let node = reader.read_next().unwrap().unwrap();
    let decoded = EuiccInfo2::decode(&node).unwrap();
    assert_eq!(decoded.free_nvram, 131_072);
    assert_eq!(decoded.free_ram, 16_384);
}

#[test]
fn test_euicc_challenge_roundtrip() {
    let raw = [0x55; 16];
    let ch = EuiccChallenge::new(raw);
    let encoded = ch.encode();
    let mut reader = Asn1Reader::new(&encoded);
    let node = reader.read_next().unwrap().unwrap();
    let decoded = EuiccChallenge::decode(&node).unwrap();
    assert_eq!(decoded.challenge, raw);
}

#[test]
fn test_rsp_profile_info_roundtrip() {
    let info = RspProfileInfo {
        iccid: "8984040000000000001".to_string(),
        isdp_aid: Some(vec![0xA0, 0x00, 0x00, 0x05, 0x59]),
        profile_state: 1,
        nickname: Some("Viettel Work eSIM".to_string()),
        service_provider_name: Some("Viettel".to_string()),
        profile_name: Some("Unlimited Data 5G".to_string()),
        profile_class: 2,
    };
    let encoded = info.encode();
    let mut reader = Asn1Reader::new(&encoded);
    let node = reader.read_next().unwrap().unwrap();
    let decoded = RspProfileInfo::decode(&node).unwrap();
    assert_eq!(decoded.iccid, "8984040000000000001");
    assert_eq!(decoded.nickname.as_deref(), Some("Viettel Work eSIM"));
    assert_eq!(decoded.profile_state, 1);
}

#[test]
fn test_set_nickname_request_roundtrip() {
    let req = SetNicknameRequest::new(
        "8984040000000000001".to_string(),
        "Roaming Japan".to_string(),
    );
    let encoded = req.encode();
    let mut reader = Asn1Reader::new(&encoded);
    let node = reader.read_next().unwrap().unwrap();
    let decoded = SetNicknameRequest::decode(&node).unwrap();
    assert_eq!(decoded, req);
}

#[test]
fn test_enable_disable_delete_profile_requests() {
    let en = EnableProfileRequest::new("8984040000000000001".to_string(), true);
    let en_enc = en.encode();
    let en_dec =
        EnableProfileRequest::decode(&Asn1Reader::new(&en_enc).read_next().unwrap().unwrap())
            .unwrap();
    assert_eq!(en_dec, en);

    let dis = DisableProfileRequest::new("8984040000000000001".to_string(), false);
    let dis_enc = dis.encode();
    let dis_dec =
        DisableProfileRequest::decode(&Asn1Reader::new(&dis_enc).read_next().unwrap().unwrap())
            .unwrap();
    assert_eq!(dis_dec, dis);

    let del = DeleteProfileRequest::new("8984040000000000001".to_string());
    let del_enc = del.encode();
    let del_dec =
        DeleteProfileRequest::decode(&Asn1Reader::new(&del_enc).read_next().unwrap().unwrap())
            .unwrap();
    assert_eq!(del_dec, del);
}

#[test]
fn test_notification_metadata_and_memory_reset() {
    let meta = NotificationMetadata {
        seq_number: 42,
        profile_management_operation: 2,
        notification_address: "smdp.carrier.com".to_string(),
        iccid: Some("8984040000000000001".to_string()),
    };
    let meta_enc = meta.encode();
    let meta_dec =
        NotificationMetadata::decode(&Asn1Reader::new(&meta_enc).read_next().unwrap().unwrap())
            .unwrap();
    assert_eq!(meta_dec.seq_number, 42);
    assert_eq!(meta_dec.profile_management_operation, 2);
    assert_eq!(meta_dec.notification_address, "smdp.carrier.com");

    let rst = EuiccMemoryResetRequest::new(0x03);
    let rst_enc = rst.encode();
    let rst_dec =
        EuiccMemoryResetRequest::decode(&Asn1Reader::new(&rst_enc).read_next().unwrap().unwrap())
            .unwrap();
    assert_eq!(rst_dec.reset_options, 0x03);
}

#[test]
fn test_profile_package_elements() {
    // Header (0xBF20), PinCodes (0xBF22), End (0xBF2E)
    let bpp_stream = vec![
        0xBF, 0x20, 0x03, 0x01, 0x02, 0x03, // PE-Header
        0xBF, 0x22, 0x02, 0xAA, 0xBB, // PE-PinCodes
        0xBF, 0x2E, 0x00, // PE-End
    ];
    let pes = parse_profile_package(&bpp_stream).expect("valid BPP stream");
    assert_eq!(pes.len(), 3);
    assert_eq!(pes[0].pe_type, PeType::Header);
    assert_eq!(pes[1].pe_type, PeType::PinCodes);
    assert_eq!(pes[2].pe_type, PeType::End);
    assert!(validate_profile_elements(&pes));
}
