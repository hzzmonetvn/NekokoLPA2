//! Download, Batch Download, and SM-DS Discovery tab components.

use dioxus::prelude::*;
use nlpa_core::i18n::I18nCatalog;

use crate::components::tr;
use crate::models::UiModel;

/// Renders single download, batch download (up to 20 codes), and ES11 SM-DS discovery.
pub fn render(model: &UiModel, catalog: &I18nCatalog) -> Element {
    let locale = model.locale;
    let d = &model.download;
    let lbl_download = tr(catalog, locale, "downloadProfile", "Download Profile");
    let lbl_batch = tr(catalog, locale, "batchDownloadTitle", "Batch Download");
    let lbl_batch_hint = tr(
        catalog,
        locale,
        "batchDownloadHint",
        "Paste multiple LPA codes here (one per line, max 20)",
    );
    let lbl_paste = tr(
        catalog,
        locale,
        "pasteFromClipboardTooltip",
        "Paste from Clipboard",
    );
    let lbl_scan = tr(catalog, locale, "scanQrCodeTooltip", "Scan QR Code");
    let lbl_discover = tr(
        catalog,
        locale,
        "discoverProfiles",
        "Discover profiles (SM-DS)",
    );
    let lbl_cancel = tr(catalog, locale, "cancel", "Cancel");

    rsx! {
        div { class: "nlpa-col",
            // Mode switch row
            section { class: "nlpa-section",
                div { class: "nlpa-row",
                    button {
                        r#type: "button",
                        class: if !d.batch_mode { "nlpa-btn is-primary" } else { "nlpa-btn" },
                        "data-action": "toggle_batch_mode",
                        "data-b-enabled": "false",
                        "{lbl_download}"
                    }
                    button {
                        r#type: "button",
                        class: if d.batch_mode { "nlpa-btn is-primary" } else { "nlpa-btn" },
                        "data-action": "toggle_batch_mode",
                        "data-b-enabled": "true",
                        "{lbl_batch}"
                    }
                }
            }

            if !d.batch_mode {
                section { class: "nlpa-card",
                    div { class: "nlpa-row",
                        strong { "{lbl_download}" }
                        div { class: "nlpa-row",
                            button {
                                r#type: "button",
                                class: "nlpa-btn",
                                "data-action": "paste_lpa_clipboard",
                                "{lbl_paste}"
                            }
                            button {
                                r#type: "button",
                                class: "nlpa-btn",
                                "data-action": "scan_qr",
                                "{lbl_scan}"
                            }
                        }
                    }

                    label { class: "nlpa-muted", "LPA Activation Code (LPA:1$SM-DP+$MatchingID)" }
                    input {
                        id: "inp-lpa-combined",
                        r#type: "text",
                        class: "nlpa-input",
                        placeholder: "LPA:1$smdp.example.com$MATCHING-ID",
                        value: "{d.combined_lpa_input}",
                        "data-change-action": "set_download_field",
                        "data-extra-field": "combined_lpa",
                        "data-prop": "value",
                    }

                    label { class: "nlpa-muted", "SM-DP+ Address" }
                    input {
                        id: "inp-smdp",
                        r#type: "text",
                        class: "nlpa-input",
                        placeholder: "smdp.example.com",
                        value: "{d.smdp_input}",
                        "data-change-action": "set_download_field",
                        "data-extra-field": "smdp",
                        "data-prop": "value",
                    }

                    label { class: "nlpa-muted", "Matching ID" }
                    input {
                        id: "inp-matching",
                        r#type: "text",
                        class: "nlpa-input",
                        placeholder: "Optional or code token",
                        value: "{d.matching_id_input}",
                        "data-change-action": "set_download_field",
                        "data-extra-field": "matching_id",
                        "data-prop": "value",
                    }

                    label { class: "nlpa-muted", "SM-DP+ OID (optional)" }
                    input {
                        id: "inp-oid",
                        r#type: "text",
                        class: "nlpa-input",
                        placeholder: "2.999.1",
                        value: "{d.oid_input}",
                        "data-change-action": "set_download_field",
                        "data-extra-field": "oid",
                        "data-prop": "value",
                    }

                    label { class: "nlpa-muted",
                        if d.confirmation_required {
                            "Confirmation Code (Required)"
                        } else {
                            "Confirmation Code (Optional)"
                        }
                    }
                    input {
                        id: "inp-cc",
                        r#type: "text",
                        class: "nlpa-input",
                        placeholder: "Confirmation Code",
                        value: "{d.confirmation_code_input}",
                        "data-change-action": "set_download_field",
                        "data-extra-field": "confirmation_code",
                        "data-prop": "value",
                    }

                    div { class: "nlpa-row",
                        button {
                            r#type: "button",
                            class: "nlpa-btn is-primary",
                            disabled: d.downloading,
                            "data-action": "start_download",
                            "data-from-inputs": "combined_lpa:inp-lpa-combined,smdp:inp-smdp,matching_id:inp-matching,oid:inp-oid,confirmation_code:inp-cc",
                            if d.is_se_hal { "SE HAL Preview" } else { "{lbl_download}" }
                        }
                        if d.downloading {
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-danger",
                                "data-action": "cancel_download",
                                "{lbl_cancel}"
                            }
                        }
                    }

                    if let Some(pct) = d.progress_pct {
                        div { class: "nlpa-progress",
                            div { class: "nlpa-progress-fill", style: "width: {pct}%;" }
                        }
                    }
                    if let Some(step) = d.step_label.as_ref() {
                        div { class: "nlpa-muted", "{step}" }
                    }
                }

                if let Some(meta) = d.metadata.as_ref() {
                    section { class: "nlpa-card",
                        strong { "Profile Preview" }
                        div { "Name: {meta.profile_name}" }
                        div { class: "nlpa-muted", "Provider: {meta.spn}" }
                        if !meta.iccid.is_empty() {
                            div { class: "nlpa-mono", "ICCID: {meta.iccid}" }
                        }
                        div { class: "nlpa-row",
                            span { class: "nlpa-badge", "Estimated size: {meta.estimated_text}" }
                            span {
                                class: if meta.ci_trusted { "nlpa-badge is-success" } else { "nlpa-badge is-warn" },
                                "{meta.ci_summary}"
                            }
                        }
                        if meta.insufficient_storage {
                            div { class: "nlpa-banner is-danger",
                                "Warning: Predicted profile size exceeds available free NVRAM."
                            }
                        }
                        if d.is_se_hal && !d.se_hal_preview_confirmed {
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-primary",
                                "data-action": "confirm_download_install",
                                "SE HAL: Install Profile"
                            }
                        }
                    }
                }
            } else {
                section { class: "nlpa-card",
                    strong { "{lbl_batch}" }
                    textarea {
                        id: "inp-batch-lines",
                        class: "nlpa-textarea",
                        placeholder: "{lbl_batch_hint}",
                        "data-change-action": "set_download_field",
                        "data-extra-field": "batch_text",
                        "data-prop": "value",
                        "{d.batch_text}"
                    }
                    div { class: "nlpa-row",
                        button {
                            r#type: "button",
                            class: "nlpa-btn is-primary",
                            disabled: d.batch_running,
                            "data-action": "start_batch_download",
                            "data-from-inputs": "batch_text:inp-batch-lines",
                            "Start Batch (max 20)"
                        }
                        if !d.batch_items.is_empty() {
                            button {
                                r#type: "button",
                                class: "nlpa-btn",
                                "data-action": "export_batch_csv",
                                "Export CSV"
                            }
                        }
                    }
                    if let Some(summary) = d.batch_summary.as_ref() {
                        div { class: "nlpa-banner", "{summary}" }
                    }
                    for item in d.batch_items.iter() {
                        div { class: "nlpa-banner",
                            div { class: "nlpa-row",
                                strong { {format!("#{} · {}", item.index + 1, item.smdp)} }
                                span { class: "nlpa-badge", "{item.status}" }
                            }
                            if let Some(iccid) = item.installed_iccid.as_ref() {
                                div { class: "nlpa-mono nlpa-muted", "ICCID: {iccid}" }
                            }
                            if let Some(err) = item.error.as_ref() {
                                div { class: "nlpa-muted", "{err}" }
                            }
                        }
                    }
                }
            }

            if let Some(err) = d.error.as_ref() {
                div { class: "nlpa-banner is-danger", "{err}" }
            }
            if let Some(ok) = d.success_message.as_ref() {
                div { class: "nlpa-banner is-success", "{ok}" }
            }

            // SM-DS Discovery
            section { class: "nlpa-section",
                strong { "{lbl_discover}" }
                div { class: "nlpa-row",
                    div { class: "nlpa-col",
                        input {
                            id: "inp-smds-server",
                            r#type: "text",
                            class: "nlpa-input",
                            placeholder: "lpa.ds.gsma.com",
                            value: "{d.smds_server_input}",
                            "data-change-action": "set_download_field",
                            "data-extra-field": "smds_server",
                            "data-prop": "value",
                        }
                    }
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        disabled: d.discovering_smds,
                        "data-action": "discover_smds",
                        "data-from-inputs": "server:inp-smds-server",
                        if d.discovering_smds { "Discovering..." } else { "Discover" }
                    }
                }
                for ev in d.discovered_events.iter() {
                    div { class: "nlpa-banner",
                        div { class: "nlpa-row",
                            div { class: "nlpa-col",
                                strong { "{ev.address}" }
                                div { class: "nlpa-mono nlpa-muted", "{ev.event_id}" }
                            }
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-primary",
                                "data-action": "select_discovered_event",
                                "data-p-activation_code": "{ev.activation_code}",
                                "Use Code"
                            }
                        }
                    }
                }
            }
        }
    }
}
