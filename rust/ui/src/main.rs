use dioxus::prelude::*;
use nlpa_ui::components::{
    ApduConsole, DownloadProfileModal, EuiccInfoDialog, ProfileList, ReaderSelector,
};
use nlpa_ui::models::{
    ActivationCodeInput, ApduLogEntry, EuiccDetails, ProfileItem, ProfileState, ReaderInfo,
    ReaderType,
};
use nlpa_ui::theme::AMOLED_PALETTE;

fn main() {
    let mut dom = VirtualDom::new(App);
    dom.rebuild_in_place();
    println!("NekokoLPA2 Dioxus VirtualDom initialized successfully.");
}

#[component]
fn App() -> Element {
    let mut selected_reader = use_signal(|| Some("qrtr-0".to_string()));
    let mut show_download_modal = use_signal(|| false);
    let mut show_info_dialog = use_signal(|| false);
    let mut status_banner = use_signal(|| None::<String>);

    let mut profiles = use_signal(|| vec![
        ProfileItem {
            iccid: "8984040000000000001".to_string(),
            name: "Viettel 5G Core".to_string(),
            provider: "Viettel Telecom".to_string(),
            state: ProfileState::Enabled,
            icon_base64: None,
        },
        ProfileItem {
            iccid: "8984040000000000002".to_string(),
            name: "Vinaphone Data".to_string(),
            provider: "VNPT Vinaphone".to_string(),
            state: ProfileState::Disabled,
            icon_base64: None,
        },
    ]);

    let readers = vec![
        ReaderInfo {
            id: "qrtr-0".to_string(),
            name: "Qualcomm Modem Slot 1".to_string(),
            kind: ReaderType::QrtrModem,
            connected: true,
        },
        ReaderInfo {
            id: "omapi-0".to_string(),
            name: "Android OMAPI UICC".to_string(),
            kind: ReaderType::Omapi,
            connected: false,
        },
        ReaderInfo {
            id: "usb-0".to_string(),
            name: "USB CCID Reader".to_string(),
            kind: ReaderType::UsbCcid,
            connected: false,
        },
    ];

    let euicc_details = EuiccDetails {
        eid: "89049032000001000000000012345678".to_string(),
        default_smdp: Some("smdp.prod.gsm.example.com".to_string()),
        free_nvram: Some("128.5 KB".to_string()),
        spec_version: Some("SGP.22 v2.2.2".to_string()),
    };

    let mut apdu_logs = use_signal(|| vec![
        ApduLogEntry {
            is_tx: true,
            data: "00A4040007A0000005591010".to_string(),
            sw: None,
        },
        ApduLogEntry {
            is_tx: false,
            data: "611A".to_string(),
            sw: Some("9000".to_string()),
        },
    ]);

    rsx! {
        div {
            style: "background-color: {AMOLED_PALETTE.background}; min-height: 100vh; padding: 16px 20px; color: {AMOLED_PALETTE.text_primary}; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; -webkit-font-smoothing: antialiased;",
            // Header bar
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; padding-bottom: 12px; border-bottom: 1px solid {AMOLED_PALETTE.surface_border};",
                div {
                    style: "display: flex; align-items: center; gap: 8px;",
                    div {
                        style: "width: 8px; height: 8px; border-radius: 50%; background-color: {AMOLED_PALETTE.signal_active};"
                    }
                    h1 {
                        style: "font-size: 16px; font-weight: 600; margin: 0; letter-spacing: -0.2px;",
                        "NekokoLPA2 — Trình quản lý eSIM (Rust)"
                    }
                }
                div {
                    style: "display: flex; gap: 6px;",
                    button {
                        style: "background-color: {AMOLED_PALETTE.surface}; color: {AMOLED_PALETTE.text_secondary}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 6px; padding: 6px 12px; font-size: 12px; font-weight: 500; cursor: pointer;",
                        onclick: move |_| show_info_dialog.set(true),
                        "eUICC"
                    }
                    button {
                        style: "background-color: {AMOLED_PALETTE.accent}; color: #FFFFFF; border: none; border-radius: 6px; padding: 6px 12px; font-size: 12px; font-weight: 500; cursor: pointer;",
                        onclick: move |_| show_download_modal.set(true),
                        "Cài hồ sơ"
                    }
                }
            }

            if let Some(msg) = status_banner() {
                div {
                    style: "background-color: {AMOLED_PALETTE.surface}; border-left: 3px solid {AMOLED_PALETTE.accent}; border-top: 1px solid {AMOLED_PALETTE.surface_border}; border-right: 1px solid {AMOLED_PALETTE.surface_border}; border-bottom: 1px solid {AMOLED_PALETTE.surface_border}; padding: 8px 12px; border-radius: 6px; margin-bottom: 12px; display: flex; justify-content: space-between; align-items: center; font-size: 12px;",
                    span { "{msg}" }
                    button {
                        style: "background: transparent; border: none; color: {AMOLED_PALETTE.text_muted}; cursor: pointer; padding: 2px 4px;",
                        onclick: move |_| status_banner.set(None),
                        "✕"
                    }
                }
            }

            // Reader selector
            ReaderSelector {
                readers: readers,
                selected_id: selected_reader(),
                on_select: move |id| selected_reader.set(Some(id)),
            }

            div { style: "height: 16px;" }

            // Profile section title with tabular count
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;",
                span {
                    style: "font-size: 13px; font-weight: 500; color: {AMOLED_PALETTE.text_secondary};",
                    "Danh sách cấu hình đã cài"
                }
                span {
                    style: "font-family: ui-monospace, monospace; font-size: 11px; color: {AMOLED_PALETTE.text_muted};",
                    "{profiles().len()} hồ sơ"
                }
            }

            ProfileList {
                profiles: profiles(),
                on_toggle: move |id| {
                    let mut list = profiles();
                    if let Some(item) = list.iter_mut().find(|p| p.iccid == id) {
                        item.state = match item.state {
                            ProfileState::Enabled => ProfileState::Disabled,
                            ProfileState::Disabled => ProfileState::Enabled,
                        };
                        status_banner.set(Some(format!("Đã cập nhật trạng thái hồ sơ: {}", item.name)));
                    }
                    profiles.set(list);
                },
                on_delete: move |id| {
                    let list: Vec<_> = profiles().into_iter().filter(|p| p.iccid != id).collect();
                    profiles.set(list);
                    status_banner.set(Some("Đã gỡ cấu hình khỏi eUICC".to_string()));
                },
            }

            // APDU console
            ApduConsole {
                logs: apdu_logs(),
                on_send: move |cmd| {
                    let mut current = apdu_logs();
                    current.push(ApduLogEntry {
                        is_tx: true,
                        data: cmd,
                        sw: None,
                    });
                    current.push(ApduLogEntry {
                        is_tx: false,
                        data: "6A82".to_string(),
                        sw: Some("9000".to_string()),
                    });
                    apdu_logs.set(current);
                },
            }

            if show_info_dialog() {
                EuiccInfoDialog {
                    details: euicc_details.clone(),
                    on_close: move |_| show_info_dialog.set(false),
                }
            }

            if show_download_modal() {
                DownloadProfileModal {
                    on_submit: move |input: ActivationCodeInput| {
                        let mut current_profiles = profiles();
                        current_profiles.push(ProfileItem {
                            iccid: "8984040000000000099".to_string(),
                            name: "Cấu hình mới".to_string(),
                            provider: input.lpa,
                            state: ProfileState::Disabled,
                            icon_base64: None,
                        });
                        profiles.set(current_profiles);
                        show_download_modal.set(false);
                        status_banner.set(Some("Đã cài đặt cấu hình eSIM thành công.".to_string()));
                    },
                    on_cancel: move |_| show_download_modal.set(false),
                }
            }
        }
    }
}
