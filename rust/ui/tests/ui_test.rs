use dioxus::prelude::*;
use nlpa_ui::components::{ApduConsole, ProfileList, ReaderSelector};
use nlpa_ui::models::{ApduLogEntry, ProfileItem, ProfileState, ReaderInfo, ReaderType};

#[test]
fn test_virtual_dom_mount_and_render() {
    fn mock_app() -> Element {
        let profiles = vec![
            ProfileItem {
                iccid: "8984040000000000001".to_string(),
                name: "Viettel 5G Test".to_string(),
                provider: "Viettel".to_string(),
                state: ProfileState::Enabled,
                icon_base64: None,
            },
        ];

        let readers = vec![
            ReaderInfo {
                id: "qrtr-0".to_string(),
                name: "Modem Slot 1".to_string(),
                kind: ReaderType::QrtrModem,
                connected: true,
            },
        ];

        let logs = vec![
            ApduLogEntry {
                is_tx: true,
                data: "00A4040000".to_string(),
                sw: None,
            },
        ];

        rsx! {
            div {
                ReaderSelector {
                    readers: readers,
                    selected_id: Some("qrtr-0".to_string()),
                    on_select: move |_| {},
                }
                ProfileList {
                    profiles: profiles,
                    on_toggle: move |_| {},
                    on_delete: move |_| {},
                }
                ApduConsole {
                    logs: logs,
                    on_send: move |_| {},
                }
            }
        }
    }

    let mut dom = VirtualDom::new(mock_app);
    dom.rebuild_in_place();
}
