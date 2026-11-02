use crate::models::EuiccDetails;
use crate::theme::AMOLED_PALETTE;
use dioxus::prelude::*;

#[component]
pub fn EuiccInfoDialog(details: EuiccDetails, on_close: EventHandler<()>) -> Element {
    // Group EID into 4-character blocks for industrial legibility: 8904 9032 ...
    let formatted_eid = details
        .eid
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ");

    rsx! {
        div {
            style: "position: fixed; inset: 0; background-color: rgba(0,0,0,0.85); display: flex; align-items: center; justify-content: center; z-index: 50; padding: 16px;",
            div {
                style: "background-color: {AMOLED_PALETTE.surface}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 12px; width: 100%; max-width: 440px; padding: 20px; box-shadow: 0 12px 32px rgba(0,0,0,0.95);",
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; padding-bottom: 12px; border-bottom: 1px solid {AMOLED_PALETTE.surface_border};",
                    div {
                        style: "display: flex; align-items: center; gap: 8px;",
                        div {
                            style: "width: 8px; height: 8px; border-radius: 50%; background-color: {AMOLED_PALETTE.signal_active};"
                        }
                        h3 {
                            style: "margin: 0; font-size: 15px; font-weight: 600; color: {AMOLED_PALETTE.text_primary}; letter-spacing: -0.2px;",
                            "Thông số kỹ thuật eUICC"
                        }
                    }
                    button {
                        style: "background: transparent; border: none; color: {AMOLED_PALETTE.text_muted}; font-size: 16px; cursor: pointer; padding: 4px; line-height: 1;",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 12px; font-size: 12px;",
                    div {
                        style: "display: flex; flex-direction: column; gap: 4px;",
                        span {
                            style: "color: {AMOLED_PALETTE.text_secondary}; font-weight: 500; font-size: 11px;",
                            "Mã định danh EID"
                        }
                        span {
                            style: "color: {AMOLED_PALETTE.text_primary}; font-family: ui-monospace, 'Roboto Mono', monospace; font-size: 12px; word-break: break-all; background-color: {AMOLED_PALETTE.surface_elevated}; border: 1px solid {AMOLED_PALETTE.surface_border}; padding: 8px 10px; border-radius: 6px; letter-spacing: 0.5px;",
                            "{formatted_eid}"
                        }
                    }
                    if let Some(smdp) = &details.default_smdp {
                        div {
                            style: "display: flex; justify-content: space-between; align-items: center; padding: 8px 0; border-bottom: 1px solid {AMOLED_PALETTE.surface_border};",
                            span { style: "color: {AMOLED_PALETTE.text_secondary};", "Máy chủ SM-DP+ mặc định" }
                            span { style: "color: {AMOLED_PALETTE.accent}; font-family: ui-monospace, monospace; font-size: 12px;", "{smdp}" }
                        }
                    }
                    if let Some(storage) = &details.free_nvram {
                        div {
                            style: "display: flex; justify-content: space-between; align-items: center; padding: 8px 0; border-bottom: 1px solid {AMOLED_PALETTE.surface_border};",
                            span { style: "color: {AMOLED_PALETTE.text_secondary};", "Bộ nhớ khả dụng (NVRAM)" }
                            span { style: "color: {AMOLED_PALETTE.text_primary}; font-family: ui-monospace, monospace; font-size: 12px;", "{storage}" }
                        }
                    }
                    if let Some(version) = &details.spec_version {
                        div {
                            style: "display: flex; justify-content: space-between; align-items: center; padding: 8px 0;",
                            span { style: "color: {AMOLED_PALETTE.text_secondary};", "Tiêu chuẩn GSMA SGP.22" }
                            span { style: "color: {AMOLED_PALETTE.text_muted}; font-family: ui-monospace, monospace; font-size: 12px;", "{version}" }
                        }
                    }
                }
                div {
                    style: "margin-top: 20px; display: flex; justify-content: flex-end;",
                    button {
                        style: "background-color: {AMOLED_PALETTE.surface_elevated}; color: {AMOLED_PALETTE.text_primary}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 6px; padding: 6px 14px; font-size: 12px; font-weight: 500; cursor: pointer;",
                        onclick: move |_| on_close.call(()),
                        "Đóng"
                    }
                }
            }
        }
    }
}
