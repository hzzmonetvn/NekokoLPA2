use nlpa_core::{
    country_name, display_name, filter_tags, flag_emoji, flag_url, format_size_bytes, hide_flag,
    hide_icon, operator_summary, redact_eid, redact_eid_preview, redact_iccid_preview,
    stable_index, FlagRedactionMode, IconRedactionMode, NicknameRedactionMode,
    OperatorRedactionMode, ProfileView, RedactionSettings, SizeUnit, TagsRedactionMode,
};

#[test]
fn stable_index_is_deterministic_and_zero_modulo_is_zero() {
    assert_eq!(stable_index("abc", 17), stable_index("abc", 17));
    assert_eq!(stable_index("abc", 0), 0);
}

#[test]
fn eid_redaction_preview_collapses_mask_runs() {
    let settings = RedactionSettings {
        redact_eid_first: false,
        redact_eid_middle: false,
        redact_eid_last: false,
        ..Default::default()
    };
    assert_eq!(
        redact_eid_preview("12345678901234567890123456789012", &settings),
        "********"
    );
}

#[test]
fn eid_reveal_full_returns_unchanged() {
    let settings = RedactionSettings {
        reveal_sensitive_profile_data: true,
        ..Default::default()
    };
    assert_eq!(
        redact_eid("12345678901234567890123456789012", false, &settings),
        "12345678901234567890123456789012"
    );
}

#[test]
fn iccid_preview_keeps_enabled_ranges() {
    let settings = RedactionSettings {
        redact_iccid_first: true,
        redact_iccid_middle: false,
        redact_iccid_last: true,
        ..Default::default()
    };
    assert_eq!(
        redact_iccid_preview("12345678901234567890", &settings),
        "12345678******567890"
    );
}

#[test]
fn display_name_hides_nickname_with_fallback() {
    let profile = ProfileView {
        nickname: Some("Secret".into()),
        name: Some("Plan".into()),
        ..Default::default()
    };
    let settings = RedactionSettings {
        nickname_redaction_mode: NicknameRedactionMode::HideNickname,
        ..Default::default()
    };
    assert_eq!(display_name(&profile, &settings), "Plan");
}

#[test]
fn operator_summary_uses_country_name_mode() {
    let profile = ProfileView {
        flag: "VN".into(),
        ..Default::default()
    };
    let settings = RedactionSettings {
        operator_redaction_mode: OperatorRedactionMode::CountryName,
        ..Default::default()
    };
    assert_eq!(
        operator_summary(&profile, &settings, &[]).as_deref(),
        Some("Vietnam Carrier")
    );
}

#[test]
fn hide_tags_returns_empty() {
    let settings = RedactionSettings {
        tags_redaction_mode: TagsRedactionMode::HideTags,
        ..Default::default()
    };
    assert!(filter_tags(&["a".into()], &settings).is_empty());
}

#[test]
fn shuffled_flag_url_has_expected_shape() {
    let profile = ProfileView {
        iccid: "123456789".into(),
        ..Default::default()
    };
    let settings = RedactionSettings {
        flag_redaction_mode: FlagRedactionMode::ShuffledFlags,
        ..Default::default()
    };
    assert!(
        flag_url(&profile, &settings, &[]).starts_with("https://static.toss.im/2d-emojis/png/4x/u")
    );
}

#[test]
fn flag_emoji_uses_regional_indicators() {
    assert_eq!(flag_emoji("US"), "🇺🇸");
}

#[test]
fn country_name_known_and_unknown_codes() {
    assert_eq!(country_name("VN"), "Vietnam");
    assert_eq!(country_name("US"), "United States");
    assert_eq!(country_name("ZZ"), "ZZ");
}

#[test]
fn redaction_visibility_modes_and_size_formatter() {
    let settings = RedactionSettings {
        flag_redaction_mode: FlagRedactionMode::HideFlag,
        icon_redaction_mode: IconRedactionMode::HideIcon,
        ..Default::default()
    };
    assert!(hide_flag(&settings));
    assert!(hide_icon(&settings));
    assert_eq!(format_size_bytes(999, SizeUnit::B), "999 B");
    assert!(format_size_bytes(1000, SizeUnit::Kb).ends_with(" kB"));
    assert!(format_size_bytes(1024, SizeUnit::Kib).ends_with(" kiB"));
    assert!(format_size_bytes(1024, SizeUnit::AdaptiveSi).ends_with(" KB"));
    assert!(format_size_bytes(1024, SizeUnit::AdaptiveBi).ends_with(" kiB"));
}
