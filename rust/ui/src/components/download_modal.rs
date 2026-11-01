use dioxus::prelude::*;
use crate::models::ActivationCodeInput;
use crate::theme::AMOLED_PALETTE;

#[component]
pub fn DownloadProfileModal(
    on_submit: EventHandler<ActivationCodeInput>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut lpa_string = use_signal(String::new);
    let mut confirmation_code = use_signal(String::new);
    let mut imei = use_signal(String::new);

    rsx! {
        div {
            style: "position: fixed; inset: 0; background-color: rgba(0,0,0,0.85); display: flex; align-items: center; justify-content: center; z-index: 50; padding: 16px;",
            div {
                style: "background-color: {AMOLED_PALETTE.surface}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 12px; width: 100%; max-width: 440px; padding: 20px; box-shadow: 0 12px 32px rgba(0,0,0,0.95);",
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; padding-bottom: 12px; border-bottom: 1px solid {AMOLED_PALETTE.surface_border};",
                    h3 {
                        style: "margin: 0; font-size: 15px; font-weight: 600; color: {AMOLED_PALETTE.text_primary}; letter-spacing: -0.2px;",
                        "Cài đặt hồ sơ eSIM mới"
                    }
                    button {
                        style: "background: transparent; border: none; color: {AMOLED_PALETTE.text_muted}; font-size: 16px; cursor: pointer; padding: 4px; line-height: 1;",
                        onclick: move |_| on_cancel.call(()),
                        "✕"
                    }
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 12px;",
                    div {
                        style: "display: flex; flex-direction: column; gap: 4px;",
                        label {
                            style: "color: {AMOLED_PALETTE.text_secondary}; font-size: 11px; font-weight: 500;",
                            "Mã kích hoạt GSMA (LPA:1$...)"
                        }
                        input {
                            style: "background-color: {AMOLED_PALETTE.surface_elevated}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 6px; padding: 8px 10px; color: {AMOLED_PALETTE.text_primary}; font-family: ui-monospace, 'Roboto Mono', monospace; font-size: 12px; outline: none;",
                            placeholder: "LPA:1$smdp.example.com$MATCHING-ID",
                            value: "{lpa_string}",
                            oninput: move |e| lpa_string.set(e.value()),
                        }
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 4px;",
                        label {
                            style: "color: {AMOLED_PALETTE.text_secondary}; font-size: 11px; font-weight: 500;",
                            "Mã xác nhận bảo mật (tùy chọn)"
                        }
                        input {
                            style: "background-color: {AMOLED_PALETTE.surface_elevated}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 6px; padding: 8px 10px; color: {AMOLED_PALETTE.text_primary}; font-size: 12px; outline: none;",
                            placeholder: "Nhập mã nếu nhà mạng yêu cầu",
                            value: "{confirmation_code}",
                            oninput: move |e| confirmation_code.set(e.value()),
                        }
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 4px;",
                        label {
                            style: "color: {AMOLED_PALETTE.text_secondary}; font-size: 11px; font-weight: 500;",
                            "Mã TAC / IMEI thiết bị (tùy chọn)"
                        }
                        input {
                            style: "background-color: {AMOLED_PALETTE.surface_elevated}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 6px; padding: 8px 10px; color: {AMOLED_PALETTE.text_primary}; font-family: ui-monospace, monospace; font-size: 12px; outline: none;",
                            placeholder: "IMEI 15 chữ số",
                            value: "{imei}",
                            oninput: move |e| imei.set(e.value()),
                        }
                    }
                }
                div {
                    style: "margin-top: 20px; display: flex; justify-content: flex-end; gap: 8px;",
                    button {
                        style: "background-color: transparent; color: {AMOLED_PALETTE.text_secondary}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 6px; padding: 6px 14px; font-size: 12px; cursor: pointer;",
                        onclick: move |_| on_cancel.call(()),
                        "Hủy"
                    }
                    button {
                        style: "background-color: {AMOLED_PALETTE.accent}; color: #FFFFFF; border: none; border-radius: 6px; padding: 6px 16px; font-size: 12px; font-weight: 500; cursor: pointer;",
                        onclick: move |_| {
                            let lpa_val = lpa_string.read().trim().to_string();
                            if !lpa_val.is_empty() {
                                let cc = confirmation_code.read().trim().to_string();
                                let imei_val = imei.read().trim().to_string();
                                on_submit.call(ActivationCodeInput {
                                    lpa: lpa_val,
                                    confirmation_code: if cc.is_empty() { None } else { Some(cc) },
                                    imei: if imei_val.is_empty() { None } else { Some(imei_val) },
                                });
                            }
                        },
                        "Tải hồ sơ"
                    }
                }
            }
        }
    }
}
