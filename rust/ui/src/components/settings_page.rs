//! Settings tab component: display/theme/wallpaper/language, privacy, readers, and tools.

use dioxus::prelude::*;
use nlpa_core::i18n::I18nCatalog;
use nlpa_core::storage::{ColorMode, ThemeStyle};

use crate::components::tr;
use crate::models::{all_locale_options, locale_code, UiModel};

/// Renders the complete Settings tab inside transparent `.nlpa-settings` glass containers.
pub fn render(model: &UiModel, catalog: &I18nCatalog) -> Element {
    let locale = model.locale;
    let s = &model.settings;
    let current_locale_code = locale_code(s.locale_choice.unwrap_or(locale));
    let opacity_pct = (model.wallpaper_opacity.clamp(0.05, 0.95) * 100.0).round() as i32;

    let lbl_appearance = tr(catalog, locale, "appearance", "Appearance");
    let lbl_theme_style = tr(catalog, locale, "themeStyle", "Theme Style");
    let lbl_amoled = tr(catalog, locale, "amoledTheme", "AMOLED Black");
    let lbl_custom = tr(catalog, locale, "customDesign", "Nekoko Style");
    let lbl_dark = tr(catalog, locale, "darkMode", "Dark Mode");
    let lbl_light = tr(catalog, locale, "light", "Light");
    let lbl_system = tr(catalog, locale, "system", "System");
    let lbl_wallpaper = tr(catalog, locale, "customBackground", "Custom Background");
    let lbl_choose_bg = tr(
        catalog,
        locale,
        "chooseBackground",
        "Choose background image",
    );
    let lbl_remove_bg = tr(
        catalog,
        locale,
        "removeBackground",
        "Remove background image",
    );
    let lbl_opacity = tr(
        catalog,
        locale,
        "backgroundImageOpacity",
        "Background opacity",
    );
    let lbl_language = tr(catalog, locale, "language", "Language");
    let lbl_reader_types = tr(catalog, locale, "readerTypes", "Reader Types");
    let lbl_adb_title = tr(catalog, locale, "adbBridgeTitle", "Connect using computer");
    let lbl_adb_start = tr(catalog, locale, "adbBridgeStart", "Start connection");
    let lbl_adb_disconnect = tr(catalog, locale, "adbBridgeDisconnect", "Disconnect");
    let lbl_adb_copy = tr(catalog, locale, "adbBridgeCopy", "Copy ADB command");
    let lbl_remote_readers = tr(catalog, locale, "remoteReaders", "Remote Readers");
    let lbl_mss = tr(
        catalog,
        locale,
        "apduMaxSegmentSize",
        "APDU Max Segment Size",
    );
    let lbl_save = tr(catalog, locale, "save", "Save");
    let lbl_imei = tr(catalog, locale, "deviceImei", "Device TAC");
    let lbl_aids = tr(
        catalog,
        locale,
        "configureDefaultAids",
        "Configure default Application IDs",
    );
    let lbl_export_db = tr(catalog, locale, "exportDatabase", "Export Database");
    let lbl_import_db = tr(catalog, locale, "importDatabase", "Import Database");
    let lbl_aram_title = tr(catalog, locale, "aramInfoTitle", "ARA-M Information");
    let lbl_check_updates = tr(catalog, locale, "checkUpdates", "Check for Updates");
    let lbl_app_logs = tr(catalog, locale, "applicationLogs", "Application Logs");
    let lbl_omapi = tr(catalog, locale, "enableOmapi", "OMAPI Connector");
    let lbl_tmapi = tr(catalog, locale, "enableTmapi", "Telephony API Connector");
    let lbl_qrtr = tr(catalog, locale, "qrtrReaderTitle", "Qualcomm modem (QRTR)");
    let lbl_se_hal = tr(
        catalog,
        locale,
        "seHalReaderTitle",
        "SE HAL (direct access)",
    );
    let lbl_ccid = tr(catalog, locale, "enableCcid", "USB CCID Connector");
    let opacity_value = format!("{:.2}", model.wallpaper_opacity.clamp(0.05, 0.95));

    let locales = all_locale_options();

    rsx! {
        div { class: "nlpa-settings",
            // Display: Theme, Color Mode, Wallpaper, Opacity, Language
            section { class: "nlpa-section",
                strong { "{lbl_appearance}" }

                div { class: "nlpa-muted", "{lbl_theme_style}" }
                div { class: "nlpa-row",
                    button {
                        r#type: "button",
                        class: if model.theme_style == ThemeStyle::Amoled { "nlpa-btn is-primary" } else { "nlpa-btn" },
                        "data-action": "set_theme_style",
                        "data-p-style": "amoled",
                        "{lbl_amoled}"
                    }
                    button {
                        r#type: "button",
                        class: if model.theme_style == ThemeStyle::Custom { "nlpa-btn is-primary" } else { "nlpa-btn" },
                        "data-action": "set_theme_style",
                        "data-p-style": "custom",
                        "{lbl_custom}"
                    }
                    button {
                        r#type: "button",
                        class: if model.theme_style == ThemeStyle::Stock { "nlpa-btn is-primary" } else { "nlpa-btn" },
                        "data-action": "set_theme_style",
                        "data-p-style": "stock",
                        "Stock"
                    }
                }

                div { class: "nlpa-muted", "Color Mode" }
                div { class: "nlpa-row",
                    button {
                        r#type: "button",
                        class: if model.color_mode == ColorMode::Dark { "nlpa-btn is-primary" } else { "nlpa-btn" },
                        "data-action": "set_color_mode",
                        "data-p-mode": "dark",
                        "{lbl_dark}"
                    }
                    button {
                        r#type: "button",
                        class: if model.color_mode == ColorMode::Light { "nlpa-btn is-primary" } else { "nlpa-btn" },
                        "data-action": "set_color_mode",
                        "data-p-mode": "light",
                        "{lbl_light}"
                    }
                    button {
                        r#type: "button",
                        class: if model.color_mode == ColorMode::System { "nlpa-btn is-primary" } else { "nlpa-btn" },
                        "data-action": "set_color_mode",
                        "data-p-mode": "system",
                        "{lbl_system}"
                    }
                }

                div { class: "nlpa-muted", "{lbl_wallpaper}" }
                div { class: "nlpa-row",
                    button {
                        r#type: "button",
                        class: "nlpa-btn is-primary",
                        "data-action": "pick_wallpaper",
                        "{lbl_choose_bg}"
                    }
                    if model.wallpaper_enabled {
                        button {
                            r#type: "button",
                            class: "nlpa-btn is-danger",
                            "data-action": "clear_wallpaper",
                            "{lbl_remove_bg}"
                        }
                    }
                }

                div { class: "nlpa-row",
                    span { class: "nlpa-muted", "{lbl_opacity}" }
                    span { id: "nlpa-opacity-label", class: "nlpa-badge", "{opacity_pct}%" }
                }
                input {
                    id: "inp-wallpaper-opacity",
                    r#type: "range",
                    class: "nlpa-range",
                    min: "0.05",
                    max: "0.95",
                    step: "0.05",
                    value: "{opacity_value}",
                    "data-live-action": "set_wallpaper_opacity",
                    "data-change-action": "set_wallpaper_opacity",
                    "data-prop": "opacity",
                    "data-kind": "float",
                    "data-immediate": "css-alpha",
                }

                label { class: "nlpa-muted", r#for: "sel-locale", "{lbl_language}" }
                select {
                    id: "sel-locale",
                    class: "nlpa-select",
                    "data-change-action": "set_locale",
                    "data-prop": "locale",
                    for opt in locales {
                        option {
                            value: "{opt.code}",
                            selected: current_locale_code == opt.code,
                            "{opt.native_name}"
                        }
                    }
                }
            }

            // Privacy & Redaction
            section { class: "nlpa-section",
                strong { "Privacy & Redaction" }
                RedactionToggleRow { label: "Reveal Sensitive Profile Data", flag: "reveal_sensitive", is_checked: s.reveal_sensitive }
                RedactionToggleRow { label: "Mask EID First Digits", flag: "redact_eid_first", is_checked: s.redact_eid_first }
                RedactionToggleRow { label: "Mask EID Middle Digits", flag: "redact_eid_middle", is_checked: s.redact_eid_middle }
                RedactionToggleRow { label: "Mask EID Last Digits", flag: "redact_eid_last", is_checked: s.redact_eid_last }
                RedactionToggleRow { label: "Mask ICCID First Digits", flag: "redact_iccid_first", is_checked: s.redact_iccid_first }
                RedactionToggleRow { label: "Mask ICCID Middle Digits", flag: "redact_iccid_middle", is_checked: s.redact_iccid_middle }
                RedactionToggleRow { label: "Mask ICCID Last Digits", flag: "redact_iccid_last", is_checked: s.redact_iccid_last }

                RedactionSelectRow {
                    id: "sel-nick-redaction",
                    label: "Nickname Masking",
                    field: "nickname",
                    current: s.nickname_mode.clone(),
                    options: vec![
                        ("none", "Show full nickname"),
                        ("redact_phone_numbers", "Mask phone numbers"),
                        ("hide_nickname", "Hide nickname"),
                    ],
                }
                RedactionSelectRow {
                    id: "sel-op-redaction",
                    label: "Operator Display",
                    field: "operator",
                    current: s.operator_mode.clone(),
                    options: vec![
                        ("none", "Full operator name"),
                        ("country_name", "Country name only"),
                    ],
                }
                RedactionSelectRow {
                    id: "sel-tags-redaction",
                    label: "Tags Visibility",
                    field: "tags",
                    current: s.tags_mode.clone(),
                    options: vec![("none", "Show tags"), ("hide_tags", "Hide tags")],
                }
                RedactionSelectRow {
                    id: "sel-flag-redaction",
                    label: "Flag Display",
                    field: "flag",
                    current: s.flag_mode.clone(),
                    options: vec![
                        ("none", "Show flag"),
                        ("hide_flag", "Hide flag"),
                        ("shuffled_flags", "Shuffled flag"),
                    ],
                }
                RedactionSelectRow {
                    id: "sel-icon-redaction",
                    label: "Icon Display",
                    field: "icon",
                    current: s.icon_mode.clone(),
                    options: vec![
                        ("none", "Show icon"),
                        ("hide_icon", "Hide icon"),
                        ("random_icon", "Random icon"),
                    ],
                }
            }

            // Readers Configuration
            section { class: "nlpa-section",
                strong { "{lbl_reader_types}" }
                ReaderToggleRow {
                    label: lbl_omapi,
                    reader_type: "omapi",
                    is_checked: s.enable_omapi,
                }
                ReaderToggleRow {
                    label: lbl_tmapi,
                    reader_type: "tmapi",
                    is_checked: s.enable_tmapi,
                }
                ReaderToggleRow {
                    label: "NBridge (Root / Shizuku)".to_string(),
                    reader_type: "nbridge",
                    is_checked: s.enable_nbridge,
                }
                ReaderToggleRow {
                    label: lbl_adb_title.clone(),
                    reader_type: "adb_bridge",
                    is_checked: s.enable_adb_bridge,
                }
                ReaderToggleRow {
                    label: lbl_qrtr,
                    reader_type: "qrtr",
                    is_checked: s.enable_qrtr,
                }
                ReaderToggleRow {
                    label: lbl_se_hal,
                    reader_type: "se_hal",
                    is_checked: s.enable_se_hal,
                }
                if s.enable_se_hal {
                    ReaderToggleRow {
                        label: "SIM1: Borrow Channel 1".to_string(),
                        reader_type: "se_hal_borrowed_slot1",
                        is_checked: s.se_hal_borrowed_slot1,
                    }
                    ReaderToggleRow {
                        label: "SIM2: Borrow Channel 1".to_string(),
                        reader_type: "se_hal_borrowed_slot2",
                        is_checked: s.se_hal_borrowed_slot2,
                    }
                }
                ReaderToggleRow {
                    label: lbl_ccid,
                    reader_type: "ccid",
                    is_checked: s.enable_ccid,
                }
                ReaderToggleRow {
                    label: lbl_remote_readers.clone(),
                    reader_type: "remote",
                    is_checked: s.enable_remote,
                }
            }

            // ADB Bridge Connection Panel
            if s.enable_adb_bridge {
                section { class: "nlpa-section",
                    div { class: "nlpa-row",
                        strong { "{lbl_adb_title}" }
                        span {
                            class: if s.adb_connected { "nlpa-badge is-success" } else { "nlpa-badge" },
                            "{s.adb_status_text}"
                        }
                    }
                    if !s.adb_command.is_empty() {
                        div { class: "nlpa-banner nlpa-mono", "{s.adb_command}" }
                    }
                    div { class: "nlpa-row",
                        if !s.adb_connected && !s.adb_waiting {
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-primary",
                                "data-action": "adb_start_connect",
                                "{lbl_adb_start}"
                            }
                        }
                        if s.adb_waiting {
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-danger",
                                "data-action": "adb_cancel_connect",
                                "Cancel"
                            }
                        }
                        if s.adb_connected {
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-danger",
                                "data-action": "adb_disconnect",
                                "{lbl_adb_disconnect}"
                            }
                        }
                        button {
                            r#type: "button",
                            class: "nlpa-btn",
                            "data-action": "adb_copy_command",
                            "{lbl_adb_copy}"
                        }
                    }
                }
            }

            // RemoCard Servers CRUD
            if s.enable_remote {
                section { class: "nlpa-section",
                    strong { "{lbl_remote_readers}" }
                    for srv in s.remocard_servers.iter() {
                        div { class: "nlpa-banner",
                            div { class: "nlpa-row",
                                div { class: "nlpa-col",
                                    strong { "{srv.name}" }
                                    div { class: "nlpa-mono nlpa-muted", "{srv.base_url}" }
                                }
                                button {
                                    r#type: "button",
                                    class: if srv.enabled { "nlpa-btn is-primary" } else { "nlpa-btn" },
                                    "data-action": "toggle_remo_card_server",
                                    "data-p-id": "{srv.id}",
                                    "data-b-enabled": if srv.enabled { "false" } else { "true" },
                                    if srv.enabled { "Enabled" } else { "Disabled" }
                                }
                                button {
                                    r#type: "button",
                                    class: "nlpa-btn is-danger",
                                    "data-action": "delete_remo_card_server",
                                    "data-p-id": "{srv.id}",
                                    "Delete"
                                }
                            }
                        }
                    }
                    input { id: "inp-remote-name", r#type: "text", class: "nlpa-input", placeholder: "Server name" }
                    input { id: "inp-remote-url", r#type: "text", class: "nlpa-input", placeholder: "https://remote.example.com" }
                    input { id: "inp-remote-token", r#type: "text", class: "nlpa-input", placeholder: "Bearer token (optional)" }
                    input { id: "inp-remote-key", r#type: "text", class: "nlpa-input", placeholder: "AES-256 key hex (optional)" }
                    button {
                        r#type: "button",
                        class: "nlpa-btn is-primary",
                        "data-action": "add_remo_card_server",
                        "data-from-inputs": "name:inp-remote-name,base_url:inp-remote-url,token:inp-remote-token,aes_key_hex:inp-remote-key",
                        "Add RemoCard Server"
                    }
                }
            }

            // Transport, MSS, Proxy, IMEI, Custom AIDs
            section { class: "nlpa-section",
                strong { "Transport & Card" }

                label { class: "nlpa-muted", "{lbl_mss}" }
                div { class: "nlpa-row",
                    div { class: "nlpa-col",
                        input {
                            id: "inp-mss",
                            r#type: "number",
                            class: "nlpa-input",
                            min: "32",
                            max: "255",
                            value: "{s.mss}",
                        }
                    }
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "set_mss",
                        "data-int-inputs": "mss:inp-mss",
                        "{lbl_save}"
                    }
                }

                label { class: "nlpa-muted", "HTTP/SOCKS Proxy URL" }
                div { class: "nlpa-row",
                    div { class: "nlpa-col",
                        input {
                            id: "inp-proxy",
                            r#type: "text",
                            class: "nlpa-input",
                            placeholder: "http://127.0.0.1:8080",
                            value: "{s.proxy_url}",
                        }
                    }
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "set_proxy_url",
                        "data-from-inputs": "url:inp-proxy",
                        "{lbl_save}"
                    }
                }

                label { class: "nlpa-muted", "{lbl_imei}" }
                div { class: "nlpa-row",
                    div { class: "nlpa-col",
                        input {
                            id: "inp-imei",
                            r#type: "text",
                            class: "nlpa-input",
                            placeholder: "35383741",
                            value: "{s.device_imei}",
                        }
                    }
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "set_device_imei",
                        "data-from-inputs": "imei:inp-imei",
                        "{lbl_save}"
                    }
                }
                if let Some(err) = s.imei_error.as_ref() {
                    div { class: "nlpa-banner is-danger", "{err}" }
                }

                label { class: "nlpa-muted", "{lbl_aids}" }
                for aid in s.custom_aids.iter() {
                    div { class: "nlpa-row",
                        span { class: "nlpa-mono", "{aid}" }
                        button {
                            r#type: "button",
                            class: "nlpa-btn is-danger",
                            "data-action": "remove_custom_aid",
                            "data-p-aid": "{aid}",
                            "Remove"
                        }
                    }
                }
                div { class: "nlpa-row",
                    div { class: "nlpa-col",
                        input {
                            id: "inp-aid-hex",
                            r#type: "text",
                            class: "nlpa-input",
                            placeholder: "A0000005591010FFFFFFFF8900000100",
                        }
                    }
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "add_custom_aid",
                        "data-from-inputs": "aid:inp-aid-hex",
                        "Add AID"
                    }
                }
            }

            // State Backup, ARA-M, Updates, Diagnostics
            section { class: "nlpa-section",
                strong { "Tools & About" }
                div { class: "nlpa-muted",
                    "Version {s.app_version} ({s.version_code}) · "
                    if s.is_privileged { "Privileged" } else { "Community" }
                }
                div { class: "nlpa-row",
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "export_database",
                        "{lbl_export_db}"
                    }
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "import_database",
                        "{lbl_import_db}"
                    }
                }
                div { class: "nlpa-row",
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "open_aram_modal",
                        "{lbl_aram_title}"
                    }
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        disabled: s.update_checking,
                        "data-action": "check_updates",
                        "{lbl_check_updates}"
                    }
                }
                div { class: "nlpa-row",
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "open_logs_modal",
                        "{lbl_app_logs}"
                    }
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "open_apdu_console",
                        "APDU Console"
                    }
                }

                if let Some(note) = s.update_notice.as_ref() {
                    div { class: "nlpa-banner", "{note}" }
                }
                if let Some(upd) = s.update_candidate.as_ref() {
                    div { class: "nlpa-banner is-success",
                        strong { "Update available: {upd.version} (build {upd.build})" }
                        if !upd.changelog.is_empty() {
                            div { class: "nlpa-muted", "{upd.changelog}" }
                        }
                        if !upd.url.is_empty() {
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-primary",
                                "data-action": "open_url",
                                "data-p-url": "{upd.url}",
                                "Open Release"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RedactionToggleRow(label: String, flag: &'static str, is_checked: bool) -> Element {
    rsx! {
        label { class: "nlpa-toggle",
            span { "{label}" }
            input {
                r#type: "checkbox",
                checked: is_checked,
                "data-change-action": "toggle_redaction_flag",
                "data-extra-flag": "{flag}",
                "data-prop": "value",
            }
        }
    }
}

#[component]
fn ReaderToggleRow(label: String, reader_type: &'static str, is_checked: bool) -> Element {
    rsx! {
        label { class: "nlpa-toggle",
            span { "{label}" }
            input {
                r#type: "checkbox",
                checked: is_checked,
                "data-change-action": "toggle_reader_type",
                "data-extra-reader_type": "{reader_type}",
                "data-prop": "enabled",
            }
        }
    }
}

#[component]
fn RedactionSelectRow(
    id: &'static str,
    label: &'static str,
    field: &'static str,
    current: String,
    options: Vec<(&'static str, &'static str)>,
) -> Element {
    rsx! {
        div { class: "nlpa-row",
            label { class: "nlpa-muted", r#for: "{id}", "{label}" }
            select {
                id: "{id}",
                class: "nlpa-select",
                style: "width: auto;",
                "data-change-action": "set_redaction_mode",
                "data-extra-field": "{field}",
                "data-prop": "value",
                for (val, text) in options {
                    option {
                        value: "{val}",
                        selected: current == val,
                        "{text}"
                    }
                }
            }
        }
    }
}
