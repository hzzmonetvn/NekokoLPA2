//! TS.43 Carrier eSIM Transfer tab component.

use dioxus::prelude::*;
use nlpa_core::i18n::I18nCatalog;

use crate::components::tr;
use crate::models::UiModel;

/// Renders the GSMA TS.43 carrier-managed subscription transfer workflow.
pub fn render(model: &UiModel, catalog: &I18nCatalog) -> Element {
    let locale = model.locale;
    let tf = &model.transfer;
    let lbl_title = tr(catalog, locale, "transferTitle", "Transfer eSIM");
    let lbl_start = tr(catalog, locale, "transferStart", "Start transfer");
    let lbl_cancel = tr(catalog, locale, "cancel", "Cancel");

    rsx! {
        div { class: "nlpa-col",
            section { class: "nlpa-card",
                strong { "{lbl_title}" }
                if !tf.carrier_name.is_empty() {
                    div { "Source carrier: {tf.carrier_name}" }
                }
                if !tf.source_iccid.is_empty() {
                    div { class: "nlpa-mono nlpa-muted", "Source ICCID: {tf.source_iccid}" }
                }
                if !tf.source_mcc_mnc.is_empty() {
                    div { class: "nlpa-muted", "PLMN: {tf.source_mcc_mnc}" }
                }

                label { class: "nlpa-muted", "Entitlement Server (optional manual override)" }
                input {
                    id: "inp-transfer-server",
                    r#type: "text",
                    class: "nlpa-input",
                    placeholder: "https://entitlement.example.com",
                    value: "{tf.manual_entitlement_server}",
                    "data-change-action": "set_transfer_field",
                    "data-extra-field": "manual_server",
                    "data-prop": "value",
                }

                div { class: "nlpa-row",
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        disabled: tf.is_busy,
                        "data-action": "start_transfer_probe",
                        "data-from-inputs": "manual_server:inp-transfer-server",
                        "Check Eligibility"
                    }
                    button {
                        r#type: "button",
                        class: "nlpa-btn is-primary",
                        disabled: tf.is_busy || !tf.can_start,
                        "data-action": "start_transfer",
                        "data-from-inputs": "manual_server:inp-transfer-server",
                        "{lbl_start}"
                    }
                    if tf.is_busy {
                        button {
                            r#type: "button",
                            class: "nlpa-btn is-danger",
                            "data-action": "cancel_transfer",
                            "{lbl_cancel}"
                        }
                    }
                }

                if !tf.state_label.is_empty() || !tf.step_message.is_empty() {
                    div { class: "nlpa-banner",
                        if !tf.state_label.is_empty() {
                            span { class: "nlpa-badge", "{tf.state_label}" }
                            " "
                        }
                        span { "{tf.step_message}" }
                    }
                }
            }

            if let Some(title) = tf.prompt_title.as_ref() {
                section { class: "nlpa-card",
                    strong { "{title}" }
                    if let Some(body) = tf.prompt_text.as_ref() {
                        div { "{body}" }
                    }
                    if tf.prompt_free_text {
                        input {
                            id: "inp-transfer-prompt",
                            r#type: "text",
                            class: "nlpa-input",
                            value: "{tf.prompt_input}",
                        }
                    }
                    div { class: "nlpa-row",
                        if let Some(acc) = tf.prompt_accept_label.as_ref() {
                            button {
                                r#type: "button",
                                class: "nlpa-btn is-primary",
                                "data-action": "submit_transfer_input",
                                "data-b-accepted": "true",
                                "data-from-inputs": "text:inp-transfer-prompt",
                                "{acc}"
                            }
                        }
                        if let Some(rej) = tf.prompt_reject_label.as_ref() {
                            button {
                                r#type: "button",
                                class: "nlpa-btn",
                                "data-action": "submit_transfer_input",
                                "data-b-accepted": "false",
                                "data-from-inputs": "text:inp-transfer-prompt",
                                "{rej}"
                            }
                        }
                    }
                }
            }

            if let Some(ws_url) = tf.pending_websheet_url.as_ref() {
                section { class: "nlpa-card",
                    strong { "Carrier Websheet Authentication Pending" }
                    div { class: "nlpa-mono nlpa-muted", "{ws_url}" }
                    button {
                        r#type: "button",
                        class: "nlpa-btn is-primary",
                        "data-action": "open_url",
                        "data-p-url": "{ws_url}",
                        "Re-open Websheet URL"
                    }
                }
            }

            if let Some(del_iccid) = tf.pending_delete_source_iccid.as_ref() {
                section { class: "nlpa-card",
                    strong { "Delete Source Profile Required" }
                    div { "Carrier requires deleting source profile {del_iccid} before completion." }
                    button {
                        r#type: "button",
                        class: "nlpa-btn is-danger",
                        "data-action": "confirm_transfer_delete_source",
                        "Delete Source & Continue"
                    }
                }
            }

            if tf.pending_delayed_download {
                section { class: "nlpa-card",
                    strong { "Delayed Download Confirmation" }
                    input {
                        id: "inp-transfer-delayed-iccid",
                        r#type: "text",
                        class: "nlpa-input",
                        placeholder: "Installed ICCID",
                        value: "{tf.delayed_iccid_input}",
                    }
                    button {
                        r#type: "button",
                        class: "nlpa-btn is-primary",
                        "data-action": "complete_delayed_transfer",
                        "data-from-inputs": "iccid:inp-transfer-delayed-iccid",
                        "Complete Activation"
                    }
                }
            }

            if let Some(ac) = tf.activation_code.as_ref() {
                div { class: "nlpa-banner is-success",
                    strong { "Activation Code Ready: " }
                    span { class: "nlpa-mono", "{ac}" }
                }
            }

            if let Some(err) = tf.error.as_ref() {
                div { class: "nlpa-banner is-danger", "{err}" }
            }
        }
    }
}
