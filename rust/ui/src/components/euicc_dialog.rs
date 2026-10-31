use dioxus::prelude::*;
use crate::models::EuiccDetails;
use crate::theme::AMOLED_PALETTE;

#[component]
pub fn EuiccInfoDialog(
    details: EuiccDetails,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            style: "position: fixed; inset: 0; background-color: rgba(0,0,0,0.85); display: flex; align-items: center; justify-content: center; z-index: 50; padding: 16px;",
            div {
                style: "background-color: {AMOLED_PALETTE.surface}; border: 1px solid {AMOLED_PALETTE.surface_variant}; border-radius: 16px; width: 100%; max-width: 480px; padding: 20px; box-shadow: 0 10px 30px rgba(0,0,0,0.9);",
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                    h3 {
                        style: "margin: 0; font-size: 18px; font-weight: bold; color: {AMOLED_PALETTE.text_primary};",
                        "Thông tin chi tiết eUICC"
                    }
                    button {
                        style: "background: transparent; border: none; color: {AMOLED_PALETTE.text_secondary}; font-size: 18px; cursor: pointer;",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 12px; font-size: 13px;",
                    div {
                        style: "display: flex; flex-direction: column; gap: 2px;",
                        span { style: "color: {AMOLED_PALETTE.text_secondary}; font-weight: 500;", "EID:" }
                        span { style: "color: {AMOLED_PALETTE.text_primary}; font-family: monospace; word-break: break-all; background-color: {AMOLED_PALETTE.surface_variant}; padding: 6px 8px; border-radius: 6px;", "{details.eid}" }
                    }
                    if let Some(smdp) = &details.default_smdp {
                        div {
                            style: "display: flex; justify-content: space-between;",
                            span { style: "color: {AMOLED_PALETTE.text_secondary};", "SM-DP+ Mặc định:" }
                            span { style: "color: {AMOLED_PALETTE.accent}; font-weight: 500;", "{smdp}" }
                        }
                    }
                    if let Some(storage) = &details.free_nvram {
                        div {
                            style: "display: flex; justify-content: space-between;",
                            span { style: "color: {AMOLED_PALETTE.text_secondary};", "Bộ nhớ trống (NVRAM):" }
                            span { style: "color: {AMOLED_PALETTE.success}; font-weight: 500;", "{storage}" }
                        }
                    }
                    if let Some(version) = &details.spec_version {
                        div {
                            style: "display: flex; justify-content: space-between;",
                            span { style: "color: {AMOLED_PALETTE.text_secondary};", "Phiên bản GSMA SGP.22:" }
                            span { style: "color: {AMOLED_PALETTE.text_primary}; font-weight: 500;", "{version}" }
                        }
                    }
                }
                div {
                    style: "display: flex; justify-content: flex-end; margin-top: 20px;",
                    button {
                        style: "background-color: {AMOLED_PALETTE.surface_variant}; color: {AMOLED_PALETTE.text_primary}; border: 1px solid {AMOLED_PALETTE.surface_variant}; border-radius: 8px; padding: 8px 16px; font-weight: 600; cursor: pointer;",
                        onclick: move |_| on_close.call(()),
                        "Đóng"
                    }
                }
            }
        }
    }
}
