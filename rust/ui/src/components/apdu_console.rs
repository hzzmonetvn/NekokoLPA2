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
            style: "background-color: {AMOLED_PALETTE.surface}; border: 1px solid {AMOLED_PALETTE.surface_variant}; border-radius: 12px; padding: 16px; margin-top: 16px;",
            div {
                style: "font-size: 15px; font-weight: bold; color: {AMOLED_PALETTE.text_primary}; margin-bottom: 8px;",
                "Giao diện lệnh APDU thô"
            }
            div {
                style: "background-color: {AMOLED_PALETTE.background}; border: 1px solid {AMOLED_PALETTE.surface_variant}; border-radius: 8px; padding: 10px; height: 160px; overflow-y: auto; font-family: monospace; font-size: 12px; display: flex; flex-direction: column; gap: 4px;",
                for (i, log) in logs.iter().enumerate() {
                    div {
                        key: "{i}",
                        style: if log.is_tx {
                            format!("color: {};", AMOLED_PALETTE.accent)
                        } else {
                            format!("color: {};", AMOLED_PALETTE.success)
                        },
                        if log.is_tx { ">> " } else { "<< " }
                        "{log.data}"
                        if let Some(sw) = &log.sw {
                            span { style: "color: {AMOLED_PALETTE.text_secondary}; margin-left: 6px;", "[SW: {sw}]" }
                        }
                    }
                }
            }
            div {
                style: "display: flex; gap: 8px; margin-top: 10px;",
                input {
                    style: "flex: 1; background-color: {AMOLED_PALETTE.background}; border: 1px solid {AMOLED_PALETTE.surface_variant}; border-radius: 8px; padding: 8px 12px; color: {AMOLED_PALETTE.text_primary}; font-family: monospace;",
                    placeholder: "Nhập lệnh hex APDU (vd: 00A40400...)",
                    value: "{input_text}",
                    oninput: move |e| input_text.set(e.value()),
                }
                button {
                    style: "background-color: {AMOLED_PALETTE.accent}; color: white; border: none; border-radius: 8px; padding: 8px 16px; font-weight: 600;",
                    onclick: move |_| {
                        let val = input_text.read().trim().to_string();
                        if !val.is_empty() {
                            on_send.call(val);
                            input_text.set(String::new());
                        }
                    },
                    "Gửi"
                }
            }
        }
    }
}
