use dioxus::prelude::*;
use nlpa_ui::components::{ApduConsole, ProfileList, ReaderSelector};
use nlpa_ui::models::{ApduLogEntry, ProfileItem, ProfileState, ReaderInfo, ReaderType};
use nlpa_ui::theme::AMOLED_PALETTE;

fn main() {
    let mut dom = VirtualDom::new(App);
    dom.rebuild_in_place();
    println!("NekokoLPA2 Dioxus VirtualDom initialized successfully.");
}

#[component]
fn App() -> Element {
    let mut selected_reader = use_signal(|| Some("qrtr-0".to_string()));
    let mut profiles = use_signal(|| vec![
        ProfileItem {
            iccid: "8984040000000000001".to_string(),
            name: "Viettel 5G Test".to_string(),
            provider: "Viettel".to_string(),
            state: ProfileState::Enabled,
            icon_base64: None,
        },
        ProfileItem {
            iccid: "8984040000000000002".to_string(),
            name: "Vinaphone Data".to_string(),
            provider: "Vinaphone".to_string(),
            state: ProfileState::Disabled,
            icon_base64: None,
        },
    ]);

    let readers = vec![
        ReaderInfo {
            id: "qrtr-0".to_string(),
            name: "Qualcomm Modem (Slot 1)".to_string(),
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
            id: "ble-0".to_string(),
            name: "9eSIM B0 Plus".to_string(),
            kind: ReaderType::Ble,
            connected: false,
        },
    ];

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
            style: "background-color: {AMOLED_PALETTE.background}; min-height: 100vh; padding: 20px; color: {AMOLED_PALETTE.text_primary}; font-family: sans-serif;",
            h1 {
                style: "font-size: 22px; font-weight: 800; margin-bottom: 16px;",
                "NekokoLPA2 — Trình điều khiển eSIM (Rust/Dioxus)"
            }
            ReaderSelector {
                readers: readers,
                selected_id: selected_reader(),
                on_select: move |id| selected_reader.set(Some(id)),
            }
            div { style: "height: 20px;" }
            h2 {
                style: "font-size: 16px; font-weight: 700; margin-bottom: 8px;",
                "Danh sách cấu hình (Profiles)"
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
                    }
                    profiles.set(list);
                },
                on_delete: move |id| {
                    let list = profiles().into_iter().filter(|p| p.iccid != id).collect();
                    profiles.set(list);
                },
            }
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
        }
    }
}
