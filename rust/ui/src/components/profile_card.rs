use dioxus::prelude::*;
use crate::models::{ProfileItem, ProfileState};
use crate::theme::AMOLED_PALETTE;

#[component]
pub fn ProfileCard(
    profile: ProfileItem,
    on_toggle: EventHandler<String>,
    on_delete: EventHandler<String>,
) -> Element {
    let is_enabled = profile.state == ProfileState::Enabled;
    let iccid = profile.iccid.clone();
    let iccid_del = profile.iccid.clone();

    rsx! {
        div {
            style: "background-color: {AMOLED_PALETTE.surface}; border: 1px solid {AMOLED_PALETTE.surface_variant}; border-radius: 12px; padding: 16px; margin-bottom: 12px; display: flex; align-items: center; justify-content: space-between;",
            div {
                style: "display: flex; flex-direction: column; gap: 4px;",
                span {
                    style: "color: {AMOLED_PALETTE.text_primary}; font-weight: bold; font-size: 16px;",
                    "{profile.name}"
                }
                span {
                    style: "color: {AMOLED_PALETTE.text_secondary}; font-size: 13px;",
                    "ICCID: {profile.iccid} • {profile.provider}"
                }
            }
            div {
                style: "display: flex; gap: 8px; align-items: center;",
                button {
                    style: if is_enabled {
                        format!("background-color: {}; color: white; border: none; border-radius: 8px; padding: 6px 12px; font-weight: 600;", AMOLED_PALETTE.success)
                    } else {
                        format!("background-color: {}; color: {}; border: 1px solid {}; border-radius: 8px; padding: 6px 12px;", AMOLED_PALETTE.surface_variant, AMOLED_PALETTE.text_primary, AMOLED_PALETTE.surface_variant)
                    },
                    onclick: move |_| on_toggle.call(iccid.clone()),
                    if is_enabled { "Đang bật" } else { "Bật" }
                }
                button {
                    style: "background-color: transparent; color: {AMOLED_PALETTE.error}; border: 1px solid {AMOLED_PALETTE.error}; border-radius: 8px; padding: 6px 10px;",
                    onclick: move |_| on_delete.call(iccid_del.clone()),
                    "Xóa"
                }
            }
        }
    }
}
