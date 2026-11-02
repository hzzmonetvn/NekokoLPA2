use crate::components::profile_card::ProfileCard;
use crate::models::ProfileItem;
use crate::theme::AMOLED_PALETTE;
use dioxus::prelude::*;

#[component]
pub fn ProfileList(
    profiles: Vec<ProfileItem>,
    on_toggle: EventHandler<String>,
    on_delete: EventHandler<String>,
) -> Element {
    if profiles.is_empty() {
        return rsx! {
            div {
                style: "padding: 32px 16px; text-align: center; color: {AMOLED_PALETTE.text_muted}; background-color: {AMOLED_PALETTE.surface}; border: 1px dashed {AMOLED_PALETTE.surface_border}; border-radius: 8px; font-size: 13px;",
                "Không có cấu hình eSIM trên eUICC hiện tại"
            }
        };
    }

    rsx! {
        div {
            style: "display: flex; flex-direction: column; width: 100%;",
            for profile in profiles {
                ProfileCard {
                    key: "{profile.iccid}",
                    profile: profile,
                    on_toggle: move |id| on_toggle.call(id),
                    on_delete: move |id| on_delete.call(id),
                }
            }
        }
    }
}
