//! Notifications queue and local notification history tab component.

use dioxus::prelude::*;
use nlpa_core::i18n::I18nCatalog;

use crate::components::tr;
use crate::models::UiModel;

/// Renders pending eUICC notifications and searchable local notification history.
pub fn render(model: &UiModel, catalog: &I18nCatalog) -> Element {
    let locale = model.locale;
    let nv = &model.notifications;
    let lbl_notifs = tr(catalog, locale, "notifications", "Notifications");
    let lbl_history = tr(catalog, locale, "notificationHistory", "Notification History");
    let lbl_remove = tr(catalog, locale, "remove", "Remove");
    let lbl_all_caught_up = tr(
        catalog,
        locale,
        "allCaughtUp",
        "All caught up! No pending notifications on card.",
    );
    let pending_count = nv.pending.len();
    let history_count = nv.history.len();

    let q = nv.history_search.trim().to_ascii_lowercase();
    let filtered_history: Vec<(usize, crate::models::HistoryNotifView)> = nv
        .history
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            if q.is_empty() {
                return true;
            }
            item.iccid.to_ascii_lowercase().contains(&q)
                || item.notification_address.to_ascii_lowercase().contains(&q)
                || item.operation_label.to_ascii_lowercase().contains(&q)
                || item.seq_number.to_string().contains(&q)
        })
        .map(|(i, item)| (i, item.clone()))
        .collect();

    rsx! {
        div { class: "nlpa-col",
            section { class: "nlpa-section",
                div { class: "nlpa-row",
                    button {
                        r#type: "button",
                        class: if !nv.show_history { "nlpa-btn is-primary" } else { "nlpa-btn" },
                        "data-action": "toggle_notif_history",
                        "data-b-show": "false",
                        "{lbl_notifs} ({pending_count})"
                    }
                    button {
                        r#type: "button",
                        class: if nv.show_history { "nlpa-btn is-primary" } else { "nlpa-btn" },
                        "data-action": "toggle_notif_history",
                        "data-b-show": "true",
                        "{lbl_history} ({history_count})"
                    }
                }
                div { class: "nlpa-row",
                    label { class: "nlpa-toggle",
                        span { "Auto Send" }
                        input {
                            r#type: "checkbox",
                            checked: nv.auto_send,
                            "data-change-action": "set_auto_notif",
                            "data-prop": "auto_send",
                        }
                    }
                    label { class: "nlpa-toggle",
                        span { "Auto Remove" }
                        input {
                            r#type: "checkbox",
                            checked: nv.auto_remove,
                            "data-change-action": "set_auto_notif",
                            "data-prop": "auto_remove",
                        }
                    }
                }
                if !nv.show_history {
                    div { class: "nlpa-row",
                        button {
                            r#type: "button",
                            class: "nlpa-btn",
                            disabled: nv.loading,
                            "data-action": "sync_notifications",
                            "Sync Card Queue"
                        }
                        button {
                            r#type: "button",
                            class: "nlpa-btn is-primary",
                            disabled: nv.loading || nv.pending.is_empty(),
                            "data-action": "process_all_notifications",
                            "Process All"
                        }
                    }
                }
            }

            if let Some(err) = nv.error.as_ref() {
                div { class: "nlpa-banner is-danger", "{err}" }
            }
            if let Some(msg) = nv.notice.as_ref() {
                div { class: "nlpa-banner is-success", "{msg}" }
            }

            if !nv.show_history {
                if nv.pending.is_empty() {
                    div { class: "nlpa-card",
                        div { class: "nlpa-muted", "{lbl_all_caught_up}" }
                    }
                } else {
                    for item in nv.pending.iter() {
                        section { class: "nlpa-card",
                            div { class: "nlpa-row",
                                strong { "#{item.seq_number} · {item.operation_label}" }
                                span { class: "nlpa-badge", {format!("0x{:02X}", item.operation_code)} }
                            }
                            if !item.iccid.is_empty() {
                                div { class: "nlpa-mono nlpa-muted", "ICCID: {item.iccid}" }
                            }
                            div { class: "nlpa-mono nlpa-muted", "{item.notification_address}" }
                            div { class: "nlpa-row",
                                button {
                                    r#type: "button",
                                    class: "nlpa-btn is-primary",
                                    "data-action": "send_notification",
                                    "data-n-seq": "{item.seq_number}",
                                    "Send"
                                }
                                button {
                                    r#type: "button",
                                    class: "nlpa-btn is-danger",
                                    "data-action": "remove_notification",
                                    "data-n-seq": "{item.seq_number}",
                                    "{lbl_remove}"
                                }
                            }
                        }
                    }
                }
            } else {
                section { class: "nlpa-section",
                    input {
                        id: "inp-history-search",
                        r#type: "search",
                        class: "nlpa-input",
                        placeholder: "Search notification history...",
                        value: "{nv.history_search}",
                        "data-live-action": "set_notif_history_search",
                        "data-change-action": "set_notif_history_search",
                        "data-prop": "value",
                    }
                }
                if filtered_history.is_empty() {
                    div { class: "nlpa-card",
                        div { class: "nlpa-muted", "No notification history entries." }
                    }
                } else {
                    for (idx, item) in filtered_history {
                        section { class: "nlpa-card",
                            div { class: "nlpa-row",
                                strong { "#{item.seq_number} · {item.operation_label}" }
                                span { class: "nlpa-badge", "{item.status_label}" }
                            }
                            if !item.iccid.is_empty() {
                                div { class: "nlpa-mono nlpa-muted", "ICCID: {item.iccid}" }
                            }
                            div { class: "nlpa-mono nlpa-muted", "{item.notification_address}" }
                            if !item.timestamp.is_empty() {
                                div { class: "nlpa-muted", "{item.timestamp}" }
                            }
                            div { class: "nlpa-row",
                                if !item.raw_b64.is_empty() && !item.notification_address.is_empty() {
                                    button {
                                        r#type: "button",
                                        class: "nlpa-btn",
                                        "data-action": "resend_history_notification",
                                        "data-n-seq": "{item.seq_number}",
                                        "data-p-raw_b64": "{item.raw_b64}",
                                        "data-p-server": "{item.notification_address}",
                                        "Resend"
                                    }
                                }
                                button {
                                    r#type: "button",
                                    class: "nlpa-btn is-danger",
                                    "data-action": "delete_history_entry",
                                    "data-n-index": "{idx}",
                                    "Delete History Entry"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
