use nlpa_core::{
    calculate_imei_luhn_checksum, clamp_wallpaper_opacity, ColorMode, LocaleChoice,
    NicknameRedactionMode, PersistedNotification, PersistedState, RedactionSettings,
    RemoCardServer, ThemeStyle, DEFAULT_STORE_DATA_MSS,
};

#[test]
fn test_persisted_state_defaults_and_imei() {
    let state = PersistedState::default();
    assert_eq!(state.theme_style, ThemeStyle::Amoled);
    assert_eq!(state.color_mode, ColorMode::Dark);
    assert_eq!(state.locale, LocaleChoice::System);
    assert!(!state.wallpaper_enabled);
    assert!((state.wallpaper_opacity - 0.35).abs() < 1e-6);
    assert_eq!(state.mss, DEFAULT_STORE_DATA_MSS as u8);
    assert!(state.enable_omapi);
    assert!(state.enable_tmapi);
    assert!(state.enable_nbridge);
    assert!(state.enable_adb_bridge);
    assert!(!state.enable_qrtr);
    assert!(!state.enable_se_hal);
    assert!(!state.se_hal_borrowed_slot1);
    assert!(!state.se_hal_borrowed_slot2);
    assert!(state.enable_ccid);
    assert!(!state.enable_remote);
    assert_eq!(state.custom_aids.len(), 6);

    assert_eq!(state.device_imei.len(), 15);
    let body = &state.device_imei[..14];
    let expected_check = calculate_imei_luhn_checksum(body).expect("valid 14-digit body");
    assert_eq!(
        state.device_imei.as_bytes()[14] - b'0',
        expected_check,
        "default IMEI must satisfy Luhn check"
    );
}

#[test]
fn test_persisted_state_json_roundtrip_and_clamping() {
    let mut state = PersistedState {
        theme_style: ThemeStyle::Custom,
        color_mode: ColorMode::System,
        locale: LocaleChoice::Vi,
        wallpaper_enabled: true,
        wallpaper_opacity: 0.55,
        mss: 160,
        proxy_url: Some("http://127.0.0.1:8080".to_string()),
        redaction: RedactionSettings {
            nickname_redaction_mode: NicknameRedactionMode::RedactPhoneNumbers,
            ..Default::default()
        },
        ..Default::default()
    };
    state.remocard_servers.push(RemoCardServer {
        id: "home".to_string(),
        name: "Home".to_string(),
        base_url: "http://192.168.1.10:33777".to_string(),
        token: Some("pw".to_string()),
        aes_key_hex: None,
        enabled: true,
    });
    state
        .profile_nicknames_cache
        .insert("890123".to_string(), "Main SIM".to_string());
    state
        .profile_tags
        .insert("890123".to_string(), "t:Work".to_string());
    state
        .gm_psk_cache
        .insert("890860".to_string(), "AABB".to_string());
    state.notifications_history.push(PersistedNotification {
        eid: "8900".to_string(),
        iccid: "890123".to_string(),
        seq_number: 42,
        operation: "install".to_string(),
        address: "smdp.example.com".to_string(),
        content_base64: "AQID".to_string(),
        status: "sent".to_string(),
        timestamp_ms: 1_700_000_000_000,
        response_code: Some(200),
        response_content: None,
    });

    let json = state.to_json_string().expect("serialize");
    let decoded = PersistedState::from_json_str(&json).expect("deserialize");
    assert_eq!(decoded, state);

    // Partial empty JSON falls back cleanly via #[serde(default)]
    let empty_decoded = PersistedState::from_json_str("{}").expect("empty JSON defaults");
    assert_eq!(empty_decoded, PersistedState::default());

    // Out-of-bounds values are clamped on load
    let clamped = PersistedState::from_json_str(r#"{"wallpaper_opacity": 2.0, "mss": 10}"#)
        .expect("clamp values");
    assert!((clamped.wallpaper_opacity - 0.95).abs() < 1e-6);
    assert_eq!(clamped.mss, 60);

    assert!((clamp_wallpaper_opacity(-1.0) - 0.05).abs() < 1e-6);
    assert!((clamp_wallpaper_opacity(f32::NAN) - 0.35).abs() < 1e-6);
}
