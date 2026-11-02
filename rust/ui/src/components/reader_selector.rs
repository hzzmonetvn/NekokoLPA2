use crate::models::{ReaderInfo, ReaderType};
use crate::theme::AMOLED_PALETTE;
use dioxus::prelude::*;

#[component]
pub fn ReaderSelector(
    readers: Vec<ReaderInfo>,
    selected_id: Option<String>,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            style: "display: flex; gap: 8px; overflow-x: auto; padding: 4px 0; -webkit-overflow-scrolling: touch;",
            for reader in readers {
                {
                    let is_selected = selected_id.as_deref() == Some(&reader.id);
                    let rid = reader.id.clone();
                    let (kind_label, icon_tag) = match reader.kind {
                        ReaderType::UsbCcid => ("USB CCID", "USB"),
                        ReaderType::Omapi => ("OMAPI", "SYS"),
                        ReaderType::Remote => ("Cloud", "NET"),
                        ReaderType::QrtrModem => ("QRTR Modem", "MDM"),
                    };

                    rsx! {
                        button {
                            key: "{reader.id}",
                            style: if is_selected {
                                format!(
                                    "background-color: {}; border: 1px solid {}; border-radius: 8px; padding: 8px 12px; cursor: pointer; min-width: 136px; text-align: left; outline: none; transition: background-color 120ms ease;",
                                    AMOLED_PALETTE.surface_elevated,
                                    AMOLED_PALETTE.accent
                                )
                            } else {
                                format!(
                                    "background-color: {}; border: 1px solid {}; border-radius: 8px; padding: 8px 12px; cursor: pointer; min-width: 136px; text-align: left; outline: none; transition: background-color 120ms ease;",
                                    AMOLED_PALETTE.surface,
                                    AMOLED_PALETTE.surface_border
                                )
                            },
                            onclick: move |_| on_select.call(rid.clone()),
                            div {
                                style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;",
                                span {
                                    style: "font-family: ui-monospace, 'Roboto Mono', monospace; font-size: 10px; color: {AMOLED_PALETTE.accent}; letter-spacing: 0.5px; font-weight: 500;",
                                    "{kind_label}"
                                }
                                span {
                                    style: "font-family: ui-monospace, monospace; font-size: 9px; color: {AMOLED_PALETTE.text_muted}; background-color: {AMOLED_PALETTE.surface_border}; padding: 1px 4px; border-radius: 3px;",
                                    "{icon_tag}"
                                }
                            }
                            div {
                                style: "font-size: 13px; color: {AMOLED_PALETTE.text_primary}; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                "{reader.name}"
                            }
                        }
                    }
                }
            }
        }
    }
}
