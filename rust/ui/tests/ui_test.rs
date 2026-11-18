//! Integration tests for Dioxus 0.6 SSR rendering, themes, wallpaper glass, i18n, and XSS safety.

use nlpa_core::i18n::{LocaleChoice, ALL_LOCALE_CHOICES};
use nlpa_core::storage::{ColorMode, ThemeStyle};
use nlpa_ui::models::{
    all_locale_options, BatchItemView, DiscoveredSmdsEventView, DownloadMetadataView,
    EstkInfoView, EuiccInfoView, HistoryNotifView, ModalState, PendingNotifView, ProfileCardView,
    ProfileLoadingState, ProviderBadgeView, ReaderChipView, TabKind, UiAction, UiModel,
    UpdateCandidateView,
};
use nlpa_ui::{render_document, render_fragment, render_meta};

fn sample_model() -> UiModel {
    UiModel {
        readers: vec![
            ReaderChipView {
                id: "omapi:SIM1".into(),
                label: "SIM1".into(),
                transport_badge: "OMAPI".into(),
                slot_label: "Slot 1".into(),
                eid_preview: Some("8904...1234".into()),
                is_selected: true,
                is_estk: true,
                ..ReaderChipView::default()
            },
            ReaderChipView {
                id: "ccid:ReaderA".into(),
                label: "9eSIM USB".into(),
                transport_badge: "CCID".into(),
                is_9esim: true,
                ..ReaderChipView::default()
            },
        ],
        selected_reader_id: Some("omapi:SIM1".into()),
        euicc_info: Some(EuiccInfoView {
            reader_name: "SIM1".into(),
            raw_eid: "89049032001001234560000123456789".into(),
            display_eid: "8904************6789".into(),
            default_smdp: "rsp.example.com".into(),
            root_smds: "lpa.ds.gsma.com".into(),
            free_nvram_bytes: Some(262_144),
            free_nvram_text: "256 KiB".into(),
            free_ram_bytes: Some(16_384),
            free_ram_text: "16 KiB".into(),
            svn: "2.2.0".into(),
            firmware_ver: "4.1".into(),
            profile_version: "2.1.0".into(),
            is_estk: true,
            estk_info: Some(EstkInfoView {
                firmware: Some("v2.3.1".into()),
                bootloader: Some("v1.0".into()),
                serial: Some("ESTK0001".into()),
                sku: Some("MEG".into()),
            }),
            is_9esim: true,
            is_gm: true,
            gm_authenticated: false,
            gm_status_note: None,
        }),
        profiles: vec![ProfileCardView {
            iccid: "8901260000000000001F".into(),
            display_iccid: "8901**********0001".into(),
            display_name: "Carrier Alpha".into(),
            operator_summary: "Carrier Alpha (US)".into(),
            enabled: true,
            is_test_profile: true,
            flag_emoji: "🇺🇸".into(),
            tags: vec!["work".into()],
            estimated_size_text: Some("42 KB".into()),
            provider_badge: Some(ProviderBadgeView {
                label: "eSIM.gg".into(),
                msisdn: Some("+447700900123".into()),
                quota_summary: Some("5 GB / 10 GB".into()),
                portal_url: Some("https://portal.example.com".into()),
            }),
            ..ProfileCardView::default()
        }],
        ..UiModel::default()
    }
}

#[test]
fn initial_document_contains_amoled_css_and_bridge_scripts() {
    let model = sample_model();
    let doc = render_document(&model);
    assert!(doc.starts_with("<!DOCTYPE html>"));
    assert!(doc.contains("<div id=\"app\">"));
    assert!(doc.contains("#000000"));
    assert!(doc.contains("window.__nlpaDispatch"));
    assert!(doc.contains("window.__nlpaPatch"));
    assert!(doc.contains("is-amoled"));
    assert!(doc.contains("Carrier Alpha"));
}

#[test]
fn wallpaper_glass_surfaces_active_during_loading_overlay_and_settings() {
    let mut model = sample_model();
    model.wallpaper_enabled = true;
    model.wallpaper_opacity = 0.42;
    model.loading = ProfileLoadingState {
        active: true,
        title: "Refreshing profiles".into(),
        message: "Reading eUICC...".into(),
        error: None,
        can_cancel: true,
        can_retry: false,
        is_card_refreshing: true,
    };

    let meta_loading = render_meta(&model);
    assert!(meta_loading.wallpaper_active);
    assert!((meta_loading.wallpaper_opacity - 0.42).abs() < 0.01);
    assert!(!meta_loading.status_bar_black);
    assert!(meta_loading.html.contains("is-wallpaper"));
    assert!(meta_loading.html.contains("--nlpa-glass-alpha: 0.42;"));
    assert!(meta_loading.html.contains("is-wallpaper-glass"));
    assert!(meta_loading.html.contains("Refreshing profiles"));

    model.loading.active = false;
    model.active_tab = TabKind::Settings;
    let settings_html = render_fragment(&model);
    assert!(settings_html.contains("nlpa-settings"));
    assert!(settings_html.contains("is-wallpaper"));
    assert!(settings_html.contains("inp-wallpaper-opacity"));
    assert!(settings_html.contains("min=\"0.05\""));
    assert!(settings_html.contains("max=\"0.95\""));
    assert!(settings_html.contains("pick_wallpaper"));
    assert!(settings_html.contains("clear_wallpaper"));
}

#[test]
fn settings_language_selector_exposes_all_12_choices_and_vietnamese_renders() {
    let mut model = sample_model();
    model.active_tab = TabKind::Settings;
    model.locale = LocaleChoice::Vi;

    let options = all_locale_options();
    assert_eq!(options.len(), ALL_LOCALE_CHOICES.len());
    assert_eq!(options.len(), 12);
    assert!(options.iter().any(|o| o.choice == LocaleChoice::Vi && o.code == "vi"));

    let html = render_fragment(&model);
    for opt in &options {
        let val_attr = format!("value=\"{}\"", opt.code);
        assert!(html.contains(&val_attr), "missing locale option {}", opt.code);
        assert!(html.contains(opt.native_name));
    }
    assert!(html.contains("Tiếng Việt (vi)"));
    assert!(html.contains("Cài đặt"));
    assert!(html.contains("Ngôn ngữ"));
    assert!(html.contains("Đen AMOLED"));
    assert!(html.contains("Hình nền tùy chỉnh"));
    assert!(html.contains("Độ mờ hình nền"));

    model.settings.locale_choice = Some(LocaleChoice::System);
    let sys_html = render_fragment(&model);
    let Some(sys_start) = sys_html.find("value=\"system\"") else {
        panic!("missing system option");
    };
    let Some(sys_end) = sys_html[sys_start..].find('>') else {
        panic!("unterminated system option tag");
    };
    assert!(sys_html[sys_start..sys_start + sys_end].contains("selected"));
    let Some(vi_start) = sys_html.find("value=\"vi\"") else {
        panic!("missing vi option");
    };
    let Some(vi_end) = sys_html[vi_start..].find('>') else {
        panic!("unterminated vi option tag");
    };
    assert!(!sys_html[vi_start..vi_start + vi_end].contains("selected"));
    assert!(sys_html.contains("Ngôn ngữ"));
}

#[test]
fn all_tabs_and_modals_render_cleanly() {
    let mut model = sample_model();

    model.omapi_euicc_warning = true;
    model.omapi_corrupted_banner = Some("OMAPI_SESSION_CORRUPTED".into());
    model.active_tab = TabKind::Profiles;
    let profiles_html = render_fragment(&model);
    assert!(profiles_html.contains("OMAPI Safety Latch"));
    assert!(profiles_html.contains("Keep this eUICC available"));

    model.active_tab = TabKind::Download;
    model.download.metadata = Some(DownloadMetadataView {
        spn: "Test Carrier".into(),
        profile_name: "Travel Plan".into(),
        iccid: "8901260000000000002F".into(),
        profile_class: 2,
        estimated_bytes: Some(500_000),
        estimated_text: "500 KB".into(),
        ci_trusted: false,
        ci_summary: "Unknown CI".into(),
        requires_confirmation_code: true,
        insufficient_storage: true,
    });
    model.download.discovered_events = vec![DiscoveredSmdsEventView {
        event_id: "EVT-1".into(),
        address: "smdp.example.com".into(),
        activation_code: "LPA:1$smdp.example.com$EVT-1".into(),
    }];
    let dl_html = render_fragment(&model);
    assert!(dl_html.contains("Travel Plan"));
    assert!(dl_html.contains("insufficient") || dl_html.contains("exceeds available free NVRAM"));
    assert!(dl_html.contains("EVT-1"));

    model.download.batch_mode = true;
    model.download.batch_text = "LPA:1$a$b\nLPA:1$c$d".into();
    model.download.batch_items = vec![BatchItemView {
        index: 0,
        raw_code: "LPA:1$a$b".into(),
        smdp: "a".into(),
        matching_id: "b".into(),
        status: "Installed".into(),
        error: None,
        installed_iccid: Some("8901".into()),
        size_text: Some("40 KB".into()),
    }];
    let batch_html = render_fragment(&model);
    assert!(batch_html.contains("inp-batch-lines"));
    assert!(batch_html.contains("LPA:1$a$b"));
    assert!(batch_html.contains("#1 · a"));

    model.active_tab = TabKind::Transfer;
    model.transfer.carrier_name = "ExampleTel".into();
    model.transfer.can_start = true;
    model.transfer.prompt_title = Some("Confirm".into());
    model.transfer.prompt_accept_label = Some("Accept".into());
    model.transfer.pending_delete_source_iccid = Some("89010000".into());
    let tf_html = render_fragment(&model);
    assert!(tf_html.contains("ExampleTel"));
    assert!(tf_html.contains("Delete Source"));

    model.active_tab = TabKind::Notifications;
    model.notifications.pending = vec![PendingNotifView {
        seq_number: 7,
        operation_label: "Install".into(),
        operation_code: 0x01,
        notification_address: "rsp.example.com".into(),
        iccid: "89010000".into(),
        is_install: true,
    }];
    let notif_html = render_fragment(&model);
    assert!(notif_html.contains("#7"));
    assert!(notif_html.contains("0x01"));

    model.notifications.show_history = true;
    model.notifications.history = vec![HistoryNotifView {
        seq_number: 7,
        operation_label: "Install".into(),
        notification_address: "rsp.example.com".into(),
        iccid: "89010000".into(),
        status_label: "Sent".into(),
        timestamp: "2026-11-18T10:00:00Z".into(),
        raw_b64: "AQID".into(),
    }];
    let hist_html = render_fragment(&model);
    assert!(hist_html.contains("Sent"));

    model.active_tab = TabKind::Settings;
    model.settings.update_candidate = Some(UpdateCandidateView {
        version: "2.6.0".into(),
        build: 700,
        url: "https://example.com/release".into(),
        download_url: "https://example.com/app.apk".into(),
        changelog: "Fixes".into(),
    });
    let set_html = render_fragment(&model);
    assert!(set_html.contains("2.6.0"));

    let modals = [
        ModalState::EuiccDetails,
        ModalState::RenameProfile {
            iccid: "8901".into(),
            current_name: "My eSIM".into(),
            tags_csv: "work".into(),
        },
        ModalState::ConfirmDeleteProfile {
            iccid: "8901".into(),
            display_name: "My eSIM".into(),
        },
        ModalState::ConfirmSwitchProfile {
            iccid: "8901".into(),
            display_name: "My eSIM".into(),
            enable: true,
            is_se_hal: true,
        },
        ModalState::ConfirmEuiccReset,
        ModalState::EstkManagement {
            atr_hex: "3B9F".into(),
        },
        ModalState::NineEsimManagement {
            device_name: "AABBCC".into(),
            atr: "3B9F".into(),
            euicc_mode: true,
        },
        ModalState::GmPskPrompt {
            eid: "8904".into(),
        },
        ModalState::BatchExport {
            csv_text: "a,b\n1,2".into(),
        },
        ModalState::AramInfo,
        ModalState::Logs,
        ModalState::ApduConsole,
    ];
    for modal in modals {
        model.modal = modal;
        let m_html = render_fragment(&model);
        assert!(m_html.contains("nlpa-modal-backdrop"));
    }
}

#[test]
fn xss_payloads_are_html_escaped_with_no_inline_script_execution() {
    let mut model = sample_model();
    model.profiles[0].display_name = "<script>alert(1)</script>'\"".into();
    model.toast = Some(("<img src=x onerror=alert(2)>".into(), true));
    let html = render_fragment(&model);
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(!html.contains("<img src=x"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("&lt;img"));
}

#[test]
fn theme_light_and_stock_modes_resolve_cleanly() {
    let mut model = sample_model();
    model.theme_style = ThemeStyle::Stock;
    model.color_mode = ColorMode::Light;
    let meta = render_meta(&model);
    assert_eq!(meta.theme_style, "stock");
    assert!(!meta.dark);
    assert!(!meta.status_bar_black);
    assert!(meta.html.contains("is-light"));
    assert!(meta.html.contains("is-stock"));
}

#[test]
fn ui_actions_deserialize_from_json_payloads() {
    let cases = [
        r#"{"action":"switch_tab","tab":"settings"}"#,
        r#"{"action":"set_locale","locale":"vi"}"#,
        r#"{"action":"set_wallpaper_opacity","opacity":0.55}"#,
        r#"{"action":"request_switch_profile","iccid":"8901","enable":true}"#,
        r#"{"action":"set_mss","mss":120}"#,
        r#"{"action":"start_download","smdp":"rsp.example.com"}"#,
    ];
    for raw in cases {
        let parsed: Result<UiAction, _> = serde_json::from_str(raw);
        assert!(parsed.is_ok(), "failed to parse action: {raw}");
    }
}
