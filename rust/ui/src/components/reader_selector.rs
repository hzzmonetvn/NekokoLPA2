use dioxus::prelude::*;
use crate::models::{ReaderInfo, ReaderType};
use crate::theme::AMOLED_PALETTE;

#[component]
pub fn ReaderSelector(
    readers: Vec<ReaderInfo>,
    selected_id: Option<String>,
    on_select: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            style: "display: flex; gap: 8px; overflow-x: auto; padding: 8px 0;",
            for reader in readers {
                {
                    let is_selected = selected_id.as_deref() == Some(&reader.id);
                    let rid = reader.id.clone();
                    let kind_badge = match reader.kind {
                        ReaderType::UsbCcid => "USB CCID",
                        ReaderType::Omapi => "OMAPI",
                        ReaderType::Remote => "Cloud",
                        ReaderType::QrtrModem => "QRTR Modem",
                    };

                    rsx! {
                        div {
                            key: "{reader.id}",
                            style: if is_selected {
                                format!("background-color: {}; border: 1px solid {}; border-radius: 10px; padding: 10px 14px; cursor: pointer; min-width: 140px;", AMOLED_PALETTE.surface_variant, AMOLED_PALETTE.accent)
                            } else {
                                format!("background-color: {}; border: 1px solid {}; border-radius: 10px; padding: 10px 14px; cursor: pointer; min-width: 140px;", AMOLED_PALETTE.surface, AMOLED_PALETTE.surface_variant)
                            },
                            onclick: move |_| on_select.call(rid.clone()),
                            div {
                                style: "font-size: 11px; color: {AMOLED_PALETTE.accent}; text-transform: uppercase; font-weight: bold; margin-bottom: 2px;",
                                "{kind_badge}"
                            }
                            div {
                                style: "font-size: 14px; color: {AMOLED_PALETTE.text_primary}; font-weight: 500;",
                                "{reader.name}"
                            }
                        }
                    }
                }
            }
        }
    }
}
