use dioxus::prelude::*;
use crate::models::ApduLogEntry;
use crate::theme::AMOLED_PALETTE;

#[component]
pub fn ApduConsole(
    logs: Vec<ApduLogEntry>,
    on_send: EventHandler<String>,
) -> Element {
    let mut input_text = use_signal(String::new);

    rsx! {
        div {
            style: "background-color: {AMOLED_PALETTE.surface}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 8px; padding: 12px 16px; margin-top: 16px;",
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;",
                span {
                    style: "font-size: 13px; font-weight: 500; color: {AMOLED_PALETTE.text_primary};",
                    "Giao diện dòng lệnh APDU"
                }
                span {
                    style: "font-family: ui-monospace, monospace; font-size: 11px; color: {AMOLED_PALETTE.text_muted};",
                    "ISO/IEC 7816-4"
                }
            }
            div {
                style: "background-color: {AMOLED_PALETTE.background}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 6px; padding: 8px 10px; height: 140px; overflow-y: auto; font-family: ui-monospace, 'Roboto Mono', monospace; font-size: 11px; display: flex; flex-direction: column; gap: 4px;",
                for (i, log) in logs.iter().enumerate() {
                    div {
                        key: "{i}",
                        style: "display: flex; gap: 6px; line-height: 1.4;",
                        span {
                            style: if log.is_tx {
                                format!("color: {};", AMOLED_PALETTE.accent)
                            } else {
                                format!("color: {};", AMOLED_PALETTE.signal_active)
                            },
                            if log.is_tx { "TX ▸" } else { "RX ◂" }
                        }
                        span {
                            style: "color: {AMOLED_PALETTE.text_primary}; word-break: break-all;",
                            "{log.data}"
                        }
                        if let Some(sw) = &log.sw {
                            span {
                                style: "color: {AMOLED_PALETTE.text_muted}; margin-left: auto; flex-shrink: 0;",
                                "[SW {sw}]"
                            }
                        }
                    }
                }
            }
            div {
                style: "display: flex; gap: 6px; margin-top: 8px;",
                input {
                    style: "flex: 1; background-color: {AMOLED_PALETTE.surface_elevated}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 6px; padding: 6px 10px; color: {AMOLED_PALETTE.text_primary}; font-family: ui-monospace, 'Roboto Mono', monospace; font-size: 11px; outline: none;",
                    placeholder: "Nhập mã lệnh APDU hex (vd: 00A40400...)",
                    value: "{input_text}",
                    oninput: move |e| input_text.set(e.value()),
                }
                button {
                    style: "background-color: {AMOLED_PALETTE.surface_elevated}; color: {AMOLED_PALETTE.text_primary}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 6px; padding: 6px 14px; font-size: 12px; font-weight: 500; cursor: pointer;",
                    onclick: move |_| {
                        let val = input_text.read().trim().to_string();
                        if !val.is_empty() {
                            on_send.call(val);
                            input_text.set(String::new());
                        }
                    },
                    "Gửi APDU"
                }
            }
        }
    }
}
