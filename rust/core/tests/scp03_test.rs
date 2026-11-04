use nlpa_core::{
    build_delete_isdp_apdu, build_select_isdp_apdu, compute_card_cryptogram,
    compute_host_cryptogram, derive_session_key, InitializeUpdateResponse, IsdpAid, IsdpContainer,
    IsdpState, Scp03KeySet, Scp03Session, SecurityLevel, SessionState,
};

#[test]
fn test_isdp_aid_and_apdus() {
    let standard_aid_hex = "A0000005591010FFFFFFFF8900000100";
    let aid = IsdpAid::from_hex(standard_aid_hex).expect("valid aid");
    assert!(aid.has_standard_sgp22_prefix());
    assert_eq!(aid.to_hex(), standard_aid_hex);

    // SELECT APDU: 00 A4 04 00 10 A0000005591010FFFFFFFF8900000100
    let select_apdu = build_select_isdp_apdu(&aid);
    assert_eq!(select_apdu[0..4], [0x00, 0xA4, 0x04, 0x00]);
    assert_eq!(select_apdu[4], 16);
    assert_eq!(&select_apdu[5..], aid.as_bytes());

    // DELETE APDU: 80 E4 00 00 12 4F 10 <AID>
    let delete_apdu = build_delete_isdp_apdu(&aid);
    assert_eq!(delete_apdu[0..4], [0x80, 0xE4, 0x00, 0x00]);
    assert_eq!(delete_apdu[4], 18); // 2 + 16
    assert_eq!(delete_apdu[5], 0x4F);
    assert_eq!(delete_apdu[6], 16);
    assert_eq!(&delete_apdu[7..], aid.as_bytes());
}

#[test]
fn test_isdp_lifecycle_transitions() {
    let aid = IsdpAid::from_hex("A00000055910101122334455").unwrap();
    let mut container = IsdpContainer::new(aid);
    assert_eq!(container.state(), IsdpState::Created);

    // Created -> Personalized
    container
        .transition_to(IsdpState::Personalized)
        .expect("transition to personalized");
    assert_eq!(container.state(), IsdpState::Personalized);

    // Personalized -> Installed
    container
        .transition_to(IsdpState::Installed)
        .expect("transition to installed");
    assert_eq!(container.state(), IsdpState::Installed);

    // Installed -> Operating
    container
        .transition_to(IsdpState::Operating)
        .expect("transition to operating");
    assert_eq!(container.state(), IsdpState::Operating);

    // Operating -> Installed (disabled)
    container
        .transition_to(IsdpState::Installed)
        .expect("transition back to installed");

    // Installed -> Deleted
    container
        .transition_to(IsdpState::Deleted)
        .expect("transition to deleted");
    assert_eq!(container.state(), IsdpState::Deleted);

    // Deleted cannot transition anywhere
    assert!(container.transition_to(IsdpState::Created).is_err());
}

#[test]
fn test_scp03_session_key_derivation_and_cryptograms() {
    // Static keys (AES-128)
    let static_k_enc = [0x40u8; 16];
    let static_k_mac = [0x41u8; 16];
    let host_challenge = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let card_challenge = [0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18];

    // Derive S-ENC, S-MAC, S-RMAC
    let s_enc = derive_session_key(&static_k_enc, 0x04, &host_challenge, &card_challenge)
        .expect("derive s-enc");
    let s_mac = derive_session_key(&static_k_mac, 0x06, &host_challenge, &card_challenge)
        .expect("derive s-mac");
    let s_rmac = derive_session_key(&static_k_mac, 0x07, &host_challenge, &card_challenge)
        .expect("derive s-rmac");

    assert_eq!(s_enc.len(), 16);
    assert_eq!(s_mac.len(), 16);
    assert_eq!(s_rmac.len(), 16);
    assert_ne!(s_enc, s_mac);
    assert_ne!(s_mac, s_rmac);

    // Card and Host cryptograms
    let card_crypto =
        compute_card_cryptogram(&s_mac, &host_challenge, &card_challenge).expect("card cryptogram");
    let host_crypto =
        compute_host_cryptogram(&s_mac, &host_challenge, &card_challenge).expect("host cryptogram");

    assert_eq!(card_crypto.len(), 8);
    assert_eq!(host_crypto.len(), 8);
    assert_ne!(card_crypto, host_crypto);
}

#[test]
fn test_scp03_full_authentication_handshake_and_wrapping() {
    let k_enc = [0x01u8; 16];
    let k_mac = [0x02u8; 16];
    let key_set = Scp03KeySet::new(&k_enc, &k_mac, 0x01, 0x01).expect("valid key set");

    let host_challenge = [0xAA; 8];
    let mut session = Scp03Session::new(SecurityLevel::CDecCMac);

    // 1. Client builds INITIALIZE UPDATE command
    let init_update_apdu = session.build_initialize_update(host_challenge, 0x01, 0x01);
    assert_eq!(init_update_apdu[0..5], [0x80, 0x50, 0x01, 0x01, 0x08]);
    assert_eq!(&init_update_apdu[5..13], &host_challenge);
    assert_eq!(session.state(), SessionState::UpdateInitialized);

    // 2. Synthesize Card response with card challenge and valid card cryptogram
    let card_challenge = [0xBB; 8];
    let s_mac = derive_session_key(&key_set.k_mac, 0x06, &host_challenge, &card_challenge).unwrap();
    let valid_card_crypto =
        compute_card_cryptogram(&s_mac, &host_challenge, &card_challenge).unwrap();

    let mut card_resp_data = Vec::with_capacity(32);
    card_resp_data.extend_from_slice(&[0x10; 10]); // Diversification data (10 bytes)
    card_resp_data.extend_from_slice(&[0x01, 0x03, 0x70]); // Key info (3 bytes)
    card_resp_data.extend_from_slice(&card_challenge); // Card challenge (8 bytes)
    card_resp_data.extend_from_slice(&valid_card_crypto); // Card cryptogram (8 bytes)
    card_resp_data.extend_from_slice(&[0x00, 0x00, 0x01]); // Sequence counter (3 bytes)

    let parsed_resp = InitializeUpdateResponse::parse(&card_resp_data).expect("parse response");
    assert_eq!(parsed_resp.card_challenge, card_challenge);
    assert_eq!(parsed_resp.card_cryptogram, valid_card_crypto);

    // 3. Process INITIALIZE UPDATE response & verify Card Cryptogram
    let host_crypto = session
        .process_initialize_update_response(&card_resp_data, &key_set)
        .expect("verify card cryptogram & compute host cryptogram");

    // 4. Build EXTERNAL AUTHENTICATE APDU
    let ext_auth_apdu = session
        .build_external_authenticate(host_crypto)
        .expect("build external authenticate");
    assert_eq!(ext_auth_apdu[0..5], [0x84, 0x82, 0x03, 0x00, 0x10]);
    assert_eq!(&ext_auth_apdu[5..13], &host_crypto);
    assert_eq!(session.state(), SessionState::Authenticated);

    // 5. Wrap APDU with C-MAC and C-ENC
    let plain_payload = b"STORE DATA payload for ISD-P";
    let wrapped_apdu = session
        .wrap_apdu(0x80, 0xE2, 0x88, 0x00, plain_payload)
        .expect("wrap apdu");

    // Modified CLA bit 3 (0x80 | 0x04 = 0x84)
    assert_eq!(wrapped_apdu[0], 0x84);
    assert_eq!(wrapped_apdu[1], 0xE2);
    assert_eq!(wrapped_apdu[2], 0x88);
    assert_eq!(wrapped_apdu[3], 0x00);
    // Payload length + 8 bytes MAC
    let lc = wrapped_apdu[4] as usize;
    assert_eq!(wrapped_apdu.len(), 5 + lc);
}
