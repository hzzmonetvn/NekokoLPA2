use nlpa_core::{resolve_locale, I18nCatalog, LocaleChoice, ALL_LOCALE_CHOICES};

#[test]
fn test_all_11_arb_locales_load_with_vietnamese_included() {
    assert_eq!(ALL_LOCALE_CHOICES.len(), 12);
    assert!(ALL_LOCALE_CHOICES.contains(&LocaleChoice::Vi));
    assert_eq!(LocaleChoice::Vi.code(), "vi");
    assert_eq!(LocaleChoice::Vi.label(), "Tiếng Việt");

    let catalog = I18nCatalog::new();
    for &choice in ALL_LOCALE_CHOICES {
        if choice == LocaleChoice::System {
            continue;
        }
        assert!(
            catalog.len(choice) > 900,
            "expected > 900 loaded keys for {choice:?}, got {}",
            catalog.len(choice)
        );
    }

    // Vietnamese strings resolve cleanly
    assert_eq!(catalog.get(LocaleChoice::Vi, "amoledTheme"), "Đen AMOLED");
    assert_eq!(catalog.get(LocaleChoice::Vi, "language"), "Ngôn ngữ");
    assert_eq!(catalog.get(LocaleChoice::En, "amoledTheme"), "AMOLED Black");

    // Non-En locales missing newer keys fall back to En cleanly
    assert_eq!(catalog.get(LocaleChoice::De, "amoledTheme"), "AMOLED Black");
    assert_eq!(catalog.get(LocaleChoice::En, "nonexistent_key"), "");
}

#[test]
fn test_locale_resolution_and_placeholder_formatting() {
    assert_eq!(resolve_locale(LocaleChoice::Vi, "en-US"), LocaleChoice::Vi);
    assert_eq!(resolve_locale(LocaleChoice::System, "vi-VN"), LocaleChoice::Vi);
    assert_eq!(resolve_locale(LocaleChoice::System, "vi_VN"), LocaleChoice::Vi);
    assert_eq!(resolve_locale(LocaleChoice::System, "zh-TW"), LocaleChoice::ZhTw);
    assert_eq!(resolve_locale(LocaleChoice::System, "zh_HK"), LocaleChoice::ZhTw);
    assert_eq!(resolve_locale(LocaleChoice::System, "zh_MO"), LocaleChoice::ZhTw);
    assert_eq!(resolve_locale(LocaleChoice::System, "zh-Hant-HK"), LocaleChoice::ZhTw);
    assert_eq!(resolve_locale(LocaleChoice::System, "zh-CN"), LocaleChoice::Zh);
    assert_eq!(resolve_locale(LocaleChoice::System, "zh_Hans"), LocaleChoice::Zh);
    assert_eq!(resolve_locale(LocaleChoice::System, "ja-JP"), LocaleChoice::Ja);
    assert_eq!(resolve_locale(LocaleChoice::System, "ko-KR"), LocaleChoice::Ko);
    assert_eq!(resolve_locale(LocaleChoice::System, "de-DE"), LocaleChoice::De);
    assert_eq!(resolve_locale(LocaleChoice::System, "es-ES"), LocaleChoice::Es);
    assert_eq!(resolve_locale(LocaleChoice::System, "fr-FR"), LocaleChoice::Fr);
    assert_eq!(resolve_locale(LocaleChoice::System, "it-IT"), LocaleChoice::It);
    assert_eq!(resolve_locale(LocaleChoice::System, "ru-RU"), LocaleChoice::Ru);
    assert_eq!(resolve_locale(LocaleChoice::System, ""), LocaleChoice::En);
    assert_eq!(resolve_locale(LocaleChoice::System, "pt-BR"), LocaleChoice::En);

    let catalog = I18nCatalog::new();
    let msg = catalog.format(
        LocaleChoice::En,
        "remoteConnectionFailedMessage",
        &[("error", "timeout")],
    );
    assert!(msg.contains("timeout"));

    assert_eq!(catalog.format(LocaleChoice::En, "profileCount", &[("count", "1")]), "1 profile");
    assert_eq!(catalog.format(LocaleChoice::En, "profileCount", &[("count", "5")]), "5 profiles");
    assert_eq!(catalog.format(LocaleChoice::Vi, "missingKeyFallback", &[]), "missingKeyFallback");
}
