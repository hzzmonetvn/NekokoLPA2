use nlpa_core::{
    build_manage_channel_close_apdu, build_manage_channel_open_apdu, chain_apdu_request,
    chain_apdu_request_with_default_mss, effective_store_data_mss, frame_apdu_command,
    map_logical_channel_cla, ChannelError, OmapiChannelTracker, OmapiSafetyLatch,
    ReassemblyAction, ResponseReassembler, DEFAULT_STORE_DATA_MSS,
};

#[test]
fn test_logical_channel_cla_mapping() {
    // Basic channels (0..=3)
    assert_eq!(map_logical_channel_cla(0x00, 0), Ok(0x00));
    assert_eq!(map_logical_channel_cla(0x80, 1), Ok(0x81));
    assert_eq!(map_logical_channel_cla(0x00, 2), Ok(0x02));
    assert_eq!(map_logical_channel_cla(0x80, 3), Ok(0x83));

    // Extended logical channels (4..=19)
    assert_eq!(map_logical_channel_cla(0x00, 4), Ok(0x40));
    assert_eq!(map_logical_channel_cla(0x80, 4), Ok(0xC0));
    assert_eq!(map_logical_channel_cla(0x00, 5), Ok(0x41));
    assert_eq!(map_logical_channel_cla(0x80, 19), Ok(0xCF));

    // Unsupported channel >= 20
    assert_eq!(
        map_logical_channel_cla(0x00, 20),
        Err(ChannelError::UnsupportedLogicalChannel(20))
    );
}

#[test]
fn test_manage_channel_apdus() {
    assert_eq!(
        build_manage_channel_open_apdu(),
        [0x00, 0x70, 0x00, 0x00, 0x01]
    );

    assert_eq!(
        build_manage_channel_close_apdu(1),
        Ok([0x00, 0x70, 0x80, 0x01, 0x00])
    );
    assert_eq!(
        build_manage_channel_close_apdu(0),
        Err(ChannelError::CannotCloseBasicChannel)
    );
}

#[test]
fn test_frame_apdu_command() {
    // 1. Header only (default Le=0x00 for non-0xE2)
    let framed = frame_apdu_command(0x00, 0xA4, 0x04, 0x00, None, None);
    assert_eq!(framed, vec![0x00, 0xA4, 0x04, 0x00, 0x00]);

    // 2. Short length data with Le
    let data = vec![0xA0, 0x00, 0x00, 0x05, 0x59];
    let framed2 = frame_apdu_command(0x00, 0xA4, 0x04, 0x00, Some(&data), Some(256));
    assert_eq!(
        framed2,
        vec![0x00, 0xA4, 0x04, 0x00, 0x05, 0xA0, 0x00, 0x00, 0x05, 0x59, 0x01, 0x00]
    );

    // 3. STORE DATA (0xE2) does not append default Le
    let framed3 = frame_apdu_command(0x80, 0xE2, 0x80, 0x00, Some(&data), None);
    assert_eq!(
        framed3,
        vec![0x80, 0xE2, 0x80, 0x00, 0x05, 0xA0, 0x00, 0x00, 0x05, 0x59]
    );
}

#[test]
fn test_chain_apdu_request_store_data() {
    let payload = vec![0xAA; 500]; // 500 bytes with MSS 255 -> 2 packets (255 + 245)
    let chained = chain_apdu_request(0x80, 0xE2, 0x00, 0x00, &payload, None, 255);
    assert_eq!(chained.len(), 2);

    // Block 0: not last -> P1 bit 7 clear (0x00), P2 = 0
    assert_eq!(chained[0][0], 0x80); // CLA
    assert_eq!(chained[0][1], 0xE2); // INS
    assert_eq!(chained[0][2], 0x00); // P1
    assert_eq!(chained[0][3], 0x00); // P2 (block 0)
    assert_eq!(chained[0][4], 255); // Lc
    assert_eq!(chained[0].len(), 5 + 255);

    // Block 1: last -> P1 bit 7 set (0x80), P2 = 1
    assert_eq!(chained[1][0], 0x80); // CLA
    assert_eq!(chained[1][1], 0xE2); // INS
    assert_eq!(chained[1][2], 0x80); // P1 (0x00 | 0x80)
    assert_eq!(chained[1][3], 0x01); // P2 (block 1)
    assert_eq!(chained[1][4], 245); // Lc
    assert_eq!(chained[1].len(), 5 + 245);
}

#[test]
fn test_chain_apdu_request_standard_cla_chaining() {
    let payload = vec![0xBB; 300]; // MSS 200 -> 2 packets (200 + 100)
    let chained = chain_apdu_request(0x00, 0x2A, 0x01, 0x02, &payload, Some(0), 200);
    assert_eq!(chained.len(), 2);

    // Packet 0: chaining flag set (CLA | 0x10)
    assert_eq!(chained[0][0], 0x10);
    assert_eq!(chained[0][4], 200);

    // Packet 1: last block, original CLA, Le appended
    assert_eq!(chained[1][0], 0x00);
    assert_eq!(chained[1][4], 100);
    assert_eq!(*chained[1].last().unwrap(), 0x00); // Le
}

#[test]
fn test_response_reassembler_flow() {
    let mut reassembler = ResponseReassembler::new(10);
    let current_apdu = [0x80, 0xE2, 0x80, 0x00];

    // 1. SW1=0x61, 0x10: 16 bytes available
    let chunk1 = vec![0x01, 0x02, 0x03, 0x61, 0x10];
    let action1 = reassembler
        .process_chunk(&current_apdu, &chunk1)
        .expect("step 1");
    assert_eq!(
        action1,
        ReassemblyAction::SendGetResponse([0x80, 0xC0, 0x00, 0x00, 0x10])
    );

    // 2. Final response 0x9000
    let chunk2 = vec![0x04, 0x05, 0x90, 0x00];
    let action2 = reassembler
        .process_chunk(&current_apdu, &chunk2)
        .expect("step 2");
    assert_eq!(
        action2,
        ReassemblyAction::Complete(vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x90, 0x00])
    );
}

#[test]
fn test_omapi_channel_tracker_and_safety_latch() {
    let mut tracker = OmapiChannelTracker::new();

    let token1 = tracker.claim(1).expect("should claim ch 1");
    assert!(tracker.ensure_owned(1, token1).is_ok());

    // Duplicate claim rejected
    assert_eq!(
        tracker.claim(1),
        Err(ChannelError::ChannelAlreadyOwned { channel: 1 })
    );

    // Wrong token rejected
    assert_eq!(
        tracker.ensure_owned(1, token1 + 10),
        Err(ChannelError::StaleChannelToken {
            channel: 1,
            token: token1 + 10
        })
    );

    assert!(tracker.release_if_owned(1, token1));
    assert!(!tracker.release_if_owned(1, token1));

    // Safety Latch
    let mut latch = OmapiSafetyLatch::new();
    assert!(!latch.is_poisoned());
    assert!(latch.check_safety().is_ok());

    latch.poison("Modem bus reset during profile installation");
    assert!(latch.is_poisoned());
    assert!(latch.check_safety().is_err());

    latch.recover();
    assert!(!latch.is_poisoned());
    assert!(latch.check_safety().is_ok());
}

#[test]
fn test_default_store_data_mss_and_clamping() {
    assert_eq!(DEFAULT_STORE_DATA_MSS, 120);
    assert_eq!(effective_store_data_mss(None), 120);
    assert_eq!(effective_store_data_mss(Some(0)), 255);
    assert_eq!(effective_store_data_mss(Some(300)), 255);
    assert_eq!(effective_store_data_mss(Some(120)), 120);

    // 250 bytes at default MSS=120 -> 3 blocks (120 + 120 + 10)
    let payload = vec![0xCC; 250];
    let packets = chain_apdu_request_with_default_mss(0x80, 0xE2, 0x11, 0x00, &payload, None, None);
    assert_eq!(packets.len(), 3);
    assert_eq!(packets[0][2], 0x11); // not last
    assert_eq!(packets[0][3], 0x00); // block 0
    assert_eq!(packets[0][4], 120);
    assert_eq!(packets[1][2], 0x11); // not last
    assert_eq!(packets[1][3], 0x01); // block 1
    assert_eq!(packets[1][4], 120);
    assert_eq!(packets[2][2], 0x91); // last (bit 7 set)
    assert_eq!(packets[2][3], 0x02); // block 2
    assert_eq!(packets[2][4], 10);

    // Clamping 0 -> 255 in default wrapper
    let payload_300 = [0x11u8; 300];
    let clamped =
        chain_apdu_request_with_default_mss(0x80, 0xE2, 0x11, 0x00, &payload_300, None, Some(0));
    assert_eq!(clamped.len(), 2);
    assert_eq!(clamped[0][4], 255);
    assert_eq!(clamped[1][4], 45);
}
