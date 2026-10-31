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
                style: "background-color: {AMOLED_PALETTE.surface}; border: 1px solid {AMOLED_PALETTE.surface_variant}; border-radius: 16px; width: 100%; max-width: 480px; padding: 20px; box-shadow: 0 10px 30px rgba(0,0,0,0.9);",
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                    h3 {
                        style: "margin: 0; font-size: 18px; font-weight: bold; color: {AMOLED_PALETTE.text_primary};",
                        "Tải xuống cấu hình eSIM"
                    }
                    button {
                        style: "background: transparent; border: none; color: {AMOLED_PALETTE.text_secondary}; font-size: 18px; cursor: pointer;",
                        onclick: move |_| on_cancel.call(()),
                        "✕"
                    }
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 14px;",
                    div {
                        style: "display: flex; flex-direction: column; gap: 4px;",
                        label {
                            style: "color: {AMOLED_PALETTE.text_secondary}; font-size: 12px; font-weight: 500;",
                            "Chuỗi mã kích hoạt LPA (LPA:1$...)"
                        }
                        input {
                            style: "background-color: {AMOLED_PALETTE.background}; border: 1px solid {AMOLED_PALETTE.surface_variant}; border-radius: 8px; padding: 10px 12px; color: {AMOLED_PALETTE.text_primary}; font-family: monospace; font-size: 13px;",
                            placeholder: "LPA:1$smdp.example.com$MATCHING-ID",
                            value: "{lpa_string}",
                            oninput: move |e| lpa_string.set(e.value()),
                        }
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 4px;",
                        label {
                            style: "color: {AMOLED_PALETTE.text_secondary}; font-size: 12px; font-weight: 500;",
                            "Mã xác nhận (Nếu nhà mạng yêu cầu)"
                        }
                        input {
                            style: "background-color: {AMOLED_PALETTE.background}; border: 1px solid {AMOLED_PALETTE.surface_variant}; border-radius: 8px; padding: 10px 12px; color: {AMOLED_PALETTE.text_primary}; font-size: 13px;",
                            placeholder: "Mã bí mật từ nhà mạng",
                            value: "{confirmation_code}",
                            oninput: move |e| confirmation_code.set(e.value()),
                        }
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 4px;",
                        label {
                            style: "color: {AMOLED_PALETTE.text_secondary}; font-size: 12px; font-weight: 500;",
                            "IMEI / TAC thiết bị (Tùy chọn)"
                        }
                        input {
                            style: "background-color: {AMOLED_PALETTE.background}; border: 1px solid {AMOLED_PALETTE.surface_variant}; border-radius: 8px; padding: 10px 12px; color: {AMOLED_PALETTE.text_primary}; font-size: 13px;",
                            placeholder: "35383741...",
                            value: "{imei}",
                            oninput: move |e| imei.set(e.value()),
                        }
                    }
                }
                div {
                    style: "display: flex; justify-content: flex-end; gap: 10px; margin-top: 24px;",
                    button {
                        style: "background-color: transparent; color: {AMOLED_PALETTE.text_secondary}; border: 1px solid {AMOLED_PALETTE.surface_variant}; border-radius: 8px; padding: 8px 16px; font-weight: 500; cursor: pointer;",
                        onclick: move |_| on_cancel.call(()),
                        "Hủy"
                    }
                    button {
                        style: "background-color: {AMOLED_PALETTE.accent}; color: white; border: none; border-radius: 8px; padding: 8px 18px; font-weight: 600; cursor: pointer;",
                        onclick: move |_| {
                            let lpa = lpa_string.read().trim().to_string();
                            if !lpa.is_empty() {
                                let conf = confirmation_code.read().trim().to_string();
                                let imei_val = imei.read().trim().to_string();
                                on_submit.call(ActivationCodeInput {
                                    lpa,
                                    confirmation_code: if conf.is_empty() { None } else { Some(conf) },
                                    imei: if imei_val.is_empty() { None } else { Some(imei_val) },
                                });
                            }
                        },
                        "Bắt đầu tải xuống"
                    }
                }
            }
        }
    }
}
