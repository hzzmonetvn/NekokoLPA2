use dioxus::prelude::*;
use crate::components::profile_card::ProfileCard;
use crate::models::ProfileItem;
use crate::theme::AMOLED_PALETTE;

#[component]
pub fn ProfileList(
    profiles: Vec<ProfileItem>,
    on_toggle: EventHandler<String>,
    on_delete: EventHandler<String>,
) -> Element {
    if profiles.is_empty() {
        return rsx! {
            div {
                style: "padding: 32px; text-align: center; color: {AMOLED_PALETTE.text_secondary};",
                "Chưa có cấu hình eSIM nào được cài đặt."
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
