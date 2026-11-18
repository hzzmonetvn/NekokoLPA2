//! Modal dialog views rendered over the active tab when `ModalState` is open.

use dioxus::prelude::*;
use nlpa_core::i18n::I18nCatalog;

use crate::components::tr;
use crate::models::{ModalState, UiModel};

/// Renders the currently open modal dialog, if any.
pub fn render(model: &UiModel, catalog: &I18nCatalog) -> Element {
    let locale = model.locale;
    let lbl_close = tr(catalog, locale, "close", "Close");
    let lbl_cancel = tr(catalog, locale, "cancel", "Cancel");
    let lbl_save = tr(catalog, locale, "save", "Save");
    let lbl_delete = tr(catalog, locale, "delete", "Delete");
    let lbl_rename_profile = tr(catalog, locale, "renameProfile", "Rename Profile");
    let lbl_delete_profile = tr(catalog, locale, "deleteProfile", "Delete Profile");
    let lbl_aram_title = tr(catalog, locale, "aramInfoTitle", "ARA-M Information");
    let lbl_app_logs = tr(catalog, locale, "applicationLogs", "Application Logs");

    match &model.modal {
        ModalState::None => rsx! {},
        ModalState::EuiccDetails => {
            let Some(info) = model.euicc_info.as_ref() else {
                return rsx! {};
            };
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        div { class: "nlpa-row",
                            strong { "eUICC Chip Details" }
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_close}" }
                        }
                        div { class: "nlpa-muted", "Reader: {info.reader_name}" }
                        div { class: "nlpa-mono", "EID: {info.raw_eid}" }
                        if !info.default_smdp.is_empty() {
                            div { class: "nlpa-mono", "Default SM-DP+: {info.default_smdp}" }
                        }
                        if !info.root_smds.is_empty() {
                            div { class: "nlpa-mono", "Root SM-DS: {info.root_smds}" }
                        }
                        div { "Free NVRAM: {info.free_nvram_text}" }
                        if !info.free_ram_text.is_empty() {
                            div { "Free RAM: {info.free_ram_text}" }
                        }
                        if !info.svn.is_empty() {
                            div { "SVN: {info.svn}" }
                        }
                        if !info.firmware_ver.is_empty() {
                            div { "Firmware: {info.firmware_ver}" }
                        }
                        if !info.profile_version.is_empty() {
                            div { "Profile Version: {info.profile_version}" }
                        }
                        div { class: "nlpa-row",
                            button {
                                r#type: "button",
                                class: "nlpa-btn",
                                "data-action": "copy_text",
                                "data-p-text": "{info.raw_eid}",
                                "Copy Full EID"
                            }
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-danger",
                                "data-action": "open_reset_euicc",
                                "Memory Reset"
                            }
                        }
                    }
                }
            }
        }
        ModalState::RenameProfile {
            iccid,
            current_name,
            tags_csv,
        } => {
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        strong { "{lbl_rename_profile}" }
                        div { class: "nlpa-mono nlpa-muted", "{iccid}" }
                        label { class: "nlpa-muted", "Nickname" }
                        input {
                            id: "inp-rename-name",
                            r#type: "text",
                            class: "nlpa-input",
                            value: "{current_name}",
                        }
                        label { class: "nlpa-muted", "Tags (comma-separated, optional)" }
                        input {
                            id: "inp-rename-tags",
                            r#type: "text",
                            class: "nlpa-input",
                            placeholder: "e.g. work, 2026-12-31",
                            value: "{tags_csv}",
                        }
                        div { class: "nlpa-row",
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_cancel}" }
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-primary",
                                "data-action": "submit_rename_profile",
                                "data-p-iccid": "{iccid}",
                                "data-from-inputs": "name:inp-rename-name,tags_csv:inp-rename-tags",
                                "{lbl_save}"
                            }
                        }
                    }
                }
            }
        }
        ModalState::ConfirmDeleteProfile {
            iccid,
            display_name,
        } => {
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        strong { "{lbl_delete_profile}" }
                        div { "Delete {display_name} ({iccid}) permanently from the eUICC?" }
                        div { class: "nlpa-row",
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_cancel}" }
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-danger",
                                "data-action": "confirm_delete_profile",
                                "data-p-iccid": "{iccid}",
                                "{lbl_delete}"
                            }
                        }
                    }
                }
            }
        }
        ModalState::ConfirmSwitchProfile {
            iccid,
            display_name,
            enable,
            is_se_hal,
        } => {
            let heading = if *enable {
                "Enable Profile"
            } else {
                "Disable Profile"
            };
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        strong { "{heading}" }
                        div { "Profile: {display_name}" }
                        div { class: "nlpa-mono nlpa-muted", "{iccid}" }
                        if *is_se_hal {
                            div { class: "nlpa-banner is-warn",
                                "SE HAL switch note: mobile connection may drop briefly. Never retry automatically if reply is uncertain — verify card state first."
                            }
                        }
                        div { class: "nlpa-row",
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_cancel}" }
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-primary",
                                "data-action": "confirm_switch_profile",
                                "data-p-iccid": "{iccid}",
                                "data-b-enable": if *enable { "true" } else { "false" },
                                "Confirm"
                            }
                        }
                    }
                }
            }
        }
        ModalState::ConfirmEuiccReset => {
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        strong { "Reset eUICC Memory" }
                        div { class: "nlpa-banner is-danger",
                            "This erases ALL operational profiles from the eUICC. Only proceed if you understand the risk."
                        }
                        div { class: "nlpa-row",
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_cancel}" }
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-danger",
                                "data-action": "confirm_reset_euicc",
                                "Erase All Profiles"
                            }
                        }
                    }
                }
            }
        }
        ModalState::EstkManagement { atr_hex } => {
            let estk = model.euicc_info.as_ref().and_then(|e| e.estk_info.as_ref());
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        div { class: "nlpa-row",
                            strong { "eSTK Management" }
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_close}" }
                        }
                        div { class: "nlpa-mono nlpa-muted", "ATR: {atr_hex}" }
                        if let Some(info) = estk {
                            if let Some(fw) = info.firmware.as_ref() {
                                div { "Firmware: {fw}" }
                            }
                            if let Some(bl) = info.bootloader.as_ref() {
                                div { "Bootloader: {bl}" }
                            }
                            if let Some(sn) = info.serial.as_ref() {
                                div { class: "nlpa-mono", "Serial: {sn}" }
                            }
                            if let Some(sku) = info.sku.as_ref() {
                                div { "SKU: {sku}" }
                            }
                        }
                        div { class: "nlpa-row",
                            button {
                                r#type: "button",
                                class: "nlpa-btn",
                                "data-action": "estk_command",
                                "data-p-command": "info",
                                "Refresh Info"
                            }
                            button {
                                r#type: "button",
                                class: "nlpa-btn",
                                "data-action": "estk_command",
                                "data-p-command": "switch_slot",
                                "data-p-arg": "0",
                                "Slot 0"
                            }
                            button {
                                r#type: "button",
                                class: "nlpa-btn",
                                "data-action": "estk_command",
                                "data-p-command": "switch_slot",
                                "data-p-arg": "1",
                                "Slot 1"
                            }
                        }
                    }
                }
            }
        }
        ModalState::NineEsimManagement {
            device_name,
            atr,
            euicc_mode,
        } => {
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        div { class: "nlpa-row",
                            strong { "9eSIM USB Writer" }
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_close}" }
                        }
                        label { class: "nlpa-muted", "Device ID (6 hex chars)" }
                        input { id: "inp-9esim-dev", r#type: "text", class: "nlpa-input", value: "{device_name}" }
                        label { class: "nlpa-muted", "New Password" }
                        input { id: "inp-9esim-pass", r#type: "password", class: "nlpa-input" }
                        button {
                            r#type: "button",
                            class: "nlpa-btn",
                            "data-action": "nine_esim_set_password",
                            "data-from-inputs": "device_name:inp-9esim-dev,password:inp-9esim-pass",
                            "Set Password"
                        }
                        label { class: "nlpa-muted", "ATR" }
                        input { id: "inp-9esim-atr", r#type: "text", class: "nlpa-input", value: "{atr}" }
                        label { class: "nlpa-toggle",
                            span { "eUICC Capability Mode" }
                            input { id: "chk-9esim-mode", r#type: "checkbox", checked: *euicc_mode }
                        }
                        button {
                            r#type: "button",
                            class: "nlpa-btn is-primary",
                            "data-action": "nine_esim_set_atr",
                            "data-from-inputs": "atr:inp-9esim-atr",
                            "data-from-checks": "euicc_mode:chk-9esim-mode",
                            "Program ATR"
                        }
                    }
                }
            }
        }
        ModalState::GmPskPrompt { eid } => {
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        strong { "GM Authentication PSK" }
                        div { class: "nlpa-mono nlpa-muted", "EID: {eid}" }
                        input {
                            id: "inp-gm-psk",
                            r#type: "password",
                            class: "nlpa-input",
                            placeholder: "16-byte or 32-char hex PSK",
                        }
                        div { class: "nlpa-row",
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_cancel}" }
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-primary",
                                "data-action": "submit_gm_psk",
                                "data-from-inputs": "psk:inp-gm-psk",
                                "Authenticate"
                            }
                        }
                    }
                }
            }
        }
        ModalState::BatchExport { csv_text } => {
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        div { class: "nlpa-row",
                            strong { "Batch Results CSV" }
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_close}" }
                        }
                        textarea { class: "nlpa-textarea", readonly: true, "{csv_text}" }
                        button {
                            r#type: "button",
                            class: "nlpa-btn is-primary",
                            "data-action": "copy_text",
                            "data-p-text": "{csv_text}",
                            "Copy CSV"
                        }
                    }
                }
            }
        }
        ModalState::AramInfo => {
            let abis_text = model
                .settings
                .aram_info
                .as_ref()
                .map(|i| i.abis.join(", "))
                .unwrap_or_default();
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        div { class: "nlpa-row",
                            strong { "{lbl_aram_title}" }
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_close}" }
                        }
                        if let Some(info) = model.settings.aram_info.as_ref() {
                            div { class: "nlpa-muted", "Signing SHA-256:" }
                            for h in info.sha256_hashes.iter() {
                                div { class: "nlpa-mono", "{h}" }
                            }
                            div { class: "nlpa-muted", "Signing SHA-1:" }
                            for h in info.sha1_hashes.iter() {
                                div { class: "nlpa-mono", "{h}" }
                            }
                            div { class: "nlpa-muted", "Device ABIs: {abis_text}" }
                        } else {
                            div { class: "nlpa-muted", "No certificate hashes loaded." }
                        }
                    }
                }
            }
        }
        ModalState::Logs => {
            let logs_text = model.settings.logs.join("\n");
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        div { class: "nlpa-row",
                            strong { "{lbl_app_logs}" }
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_close}" }
                        }
                        textarea {
                            class: "nlpa-textarea",
                            readonly: true,
                            "{logs_text}"
                        }
                        button {
                            r#type: "button",
                            class: "nlpa-btn",
                            "data-action": "copy_logs",
                            "Copy Logs"
                        }
                    }
                }
            }
        }
        ModalState::ApduConsole => {
            rsx! {
                div { class: "nlpa-modal-backdrop",
                    div { class: "nlpa-modal",
                        div { class: "nlpa-row",
                            strong { "APDU Console" }
                            button { r#type: "button", class: "nlpa-btn", "data-action": "close_modal", "{lbl_close}" }
                        }
                        div { class: "nlpa-row",
                            div { class: "nlpa-col",
                                input {
                                    id: "inp-raw-apdu",
                                    r#type: "text",
                                    class: "nlpa-input nlpa-mono",
                                    placeholder: "80E2880003BF3E0000",
                                }
                            }
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-primary",
                                "data-action": "send_raw_apdu",
                                "data-from-inputs": "hex:inp-raw-apdu",
                                "Send"
                            }
                        }
                        for (tx, rx) in model.settings.apdu_history.iter() {
                            div { class: "nlpa-banner nlpa-mono",
                                div { "> {tx}" }
                                div { class: "nlpa-muted", "< {rx}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
