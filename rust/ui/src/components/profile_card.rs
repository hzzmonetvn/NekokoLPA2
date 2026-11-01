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

    // Format ICCID with readable 4-digit grouping: 8984 0400 ...
    let formatted_iccid = if profile.iccid.len() >= 19 {
        format!(
            "{} {} {} {} {}",
            &profile.iccid[0..4],
            &profile.iccid[4..8],
            &profile.iccid[8..12],
            &profile.iccid[12..16],
            &profile.iccid[16..]
        )
    } else {
        profile.iccid.clone()
    };

    rsx! {
        div {
            style: "background-color: {AMOLED_PALETTE.surface}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 8px; padding: 12px 16px; margin-bottom: 8px; display: flex; align-items: center; justify-content: space-between; gap: 12px;",
            div {
                style: "display: flex; align-items: center; gap: 12px; min-width: 0;",
                // Telecommunications hardware status dot (Signature element)
                div {
                    style: if is_enabled {
                        format!("width: 8px; height: 8px; border-radius: 50%; background-color: {}; flex-shrink: 0;", AMOLED_PALETTE.signal_active)
                    } else {
                        format!("width: 8px; height: 8px; border-radius: 50%; background-color: {}; flex-shrink: 0;", AMOLED_PALETTE.signal_inactive)
                    }
                }
                div {
                    style: "display: flex; flex-direction: column; gap: 2px; min-width: 0;",
                    div {
                        style: "display: flex; align-items: center; gap: 8px;",
                        span {
                            style: "color: {AMOLED_PALETTE.text_primary}; font-weight: 500; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                            "{profile.name}"
                        }
                        span {
                            style: "font-size: 11px; color: {AMOLED_PALETTE.text_muted}; background-color: {AMOLED_PALETTE.surface_elevated}; padding: 1px 6px; border-radius: 4px; font-weight: 400;",
                            "{profile.provider}"
                        }
                    }
                    span {
                        style: "font-family: ui-monospace, 'Roboto Mono', monospace; color: {AMOLED_PALETTE.text_secondary}; font-size: 11px; letter-spacing: 0.3px;",
                        "{formatted_iccid}"
                    }
                }
            }
            div {
                style: "display: flex; gap: 6px; align-items: center; flex-shrink: 0;",
                button {
                    style: if is_enabled {
                        format!(
                            "background-color: {}; color: {}; border: 1px solid {}; border-radius: 6px; padding: 6px 12px; font-size: 12px; font-weight: 500; cursor: pointer; outline: none;",
                            AMOLED_PALETTE.accent_muted,
                            AMOLED_PALETTE.accent,
                            AMOLED_PALETTE.accent
                        )
                    } else {
                        format!(
                            "background-color: {}; color: {}; border: 1px solid {}; border-radius: 6px; padding: 6px 12px; font-size: 12px; font-weight: 500; cursor: pointer; outline: none;",
                            AMOLED_PALETTE.surface_elevated,
                            AMOLED_PALETTE.text_secondary,
                            AMOLED_PALETTE.surface_border
                        )
                    },
                    onclick: move |_| on_toggle.call(iccid.clone()),
                    if is_enabled { "Đang hoạt động" } else { "Kích hoạt" }
                }
                button {
                    style: "background-color: transparent; color: {AMOLED_PALETTE.text_muted}; border: 1px solid {AMOLED_PALETTE.surface_border}; border-radius: 6px; padding: 6px 10px; font-size: 12px; cursor: pointer; outline: none;",
                    onclick: move |_| on_delete.call(iccid_del.clone()),
                    "Gỡ"
                }
            }
        }
    }
}
