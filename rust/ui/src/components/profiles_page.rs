//! Profiles home tab: reader selector, EID card summary, search/sort, and profile cards.

use dioxus::prelude::*;
use nlpa_core::i18n::I18nCatalog;

use crate::components::tr;
use crate::models::{EuiccInfoView, ProfileCardView, ProfileSortKind, ReaderChipView, UiModel};

/// Renders the Profiles tab including reader chips, EID summary, and profile cards.
pub fn render(model: &UiModel, catalog: &I18nCatalog) -> Element {
    let locale = model.locale;
    let lbl_refresh = tr(catalog, locale, "refresh", "Refresh");
    let lbl_eid = tr(catalog, locale, "eid", "EID");
    let lbl_iccid = tr(catalog, locale, "iccid", "ICCID");
    let lbl_nvram = tr(catalog, locale, "nvram", "Free NVRAM");
    let lbl_enable = tr(catalog, locale, "enable", "Enable");
    let lbl_disable = tr(catalog, locale, "notifTypeDisable", "Disable");
    let lbl_rename = tr(catalog, locale, "rename", "Rename");
    let lbl_delete = tr(catalog, locale, "delete", "Delete");
    let lbl_copy_eid = tr(catalog, locale, "copyEid", "Copy EID");
    let lbl_copy_iccid = tr(catalog, locale, "copyIccid", "Copy ICCID");
    let lbl_search = tr(catalog, locale, "searchProfiles", "Search profiles...");
    let lbl_sort = tr(catalog, locale, "sortBy", "Sort by");
    let lbl_omapi_title = tr(
        catalog,
        locale,
        "omapiEuiccDetectedTitle",
        "Keep this eUICC available",
    );
    let lbl_omapi_body = tr(
        catalog,
        locale,
        "omapiEuiccSystemDisableWarning",
        "Disabling this SIM/eSIM slot in Android system settings may stop OMAPI from detecting it.",
    );

    let lbl_no_readers = tr(
        catalog,
        locale,
        "noReadersFound",
        "No compatible eUICC readers found",
    );
    let lbl_no_profiles = tr(
        catalog,
        locale,
        "noProfilesInstalled",
        "No profiles found on this eUICC",
    );
    let reader_count = model.readers.len();

    let filtered =
        filter_and_sort_profiles(&model.profiles, &model.profile_search, model.profile_sort);

    rsx! {
        div { class: "nlpa-col",
            // Reader Selector Section
            section { class: "nlpa-section",
                div { class: "nlpa-row",
                    div { class: "nlpa-muted", "Readers ({reader_count})" }
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "refresh_readers",
                        "{lbl_refresh}"
                    }
                }
                if model.readers.is_empty() {
                    div { class: "nlpa-muted", "{lbl_no_readers}" }
                } else {
                    div { class: "nlpa-chips",
                        for reader in model.readers.iter() {
                            ReaderChip { reader: reader.clone() }
                        }
                    }
                }
            }

            if let Some(err_msg) = model.omapi_corrupted_banner.as_ref() {
                div { class: "nlpa-banner is-danger",
                    strong { "OMAPI Safety Latch: " }
                    span { "{err_msg}" }
                }
            }

            if model.omapi_euicc_warning {
                div { class: "nlpa-banner is-warn",
                    strong { "{lbl_omapi_title}: " }
                    span { "{lbl_omapi_body}" }
                }
            }

            if let Some(info) = model.euicc_info.as_ref() {
                EuiccSummaryCard {
                    info: info.clone(),
                    lbl_eid: lbl_eid.clone(),
                    lbl_nvram: lbl_nvram.clone(),
                    lbl_copy_eid: lbl_copy_eid.clone(),
                }
            }

            // Search and sort controls
            section { class: "nlpa-section",
                div { class: "nlpa-row",
                    div { class: "nlpa-col",
                        input {
                            id: "inp-profile-search",
                            r#type: "search",
                            class: "nlpa-input",
                            placeholder: "{lbl_search}",
                            value: "{model.profile_search}",
                            "data-live-action": "set_profile_search",
                            "data-change-action": "set_profile_search",
                            "data-prop": "value",
                        }
                    }
                    select {
                        id: "sel-profile-sort",
                        class: "nlpa-select",
                        style: "width: auto;",
                        "aria-label": "{lbl_sort}",
                        "data-change-action": "set_profile_sort",
                        "data-prop": "sort",
                        option {
                            value: "enabled_first",
                            selected: model.profile_sort == ProfileSortKind::EnabledFirst,
                            "Enabled first"
                        }
                        option {
                            value: "name_asc",
                            selected: model.profile_sort == ProfileSortKind::NameAsc,
                            "Name (A-Z)"
                        }
                        option {
                            value: "name_desc",
                            selected: model.profile_sort == ProfileSortKind::NameDesc,
                            "Name (Z-A)"
                        }
                        option {
                            value: "iccid_asc",
                            selected: model.profile_sort == ProfileSortKind::IccidAsc,
                            "ICCID"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "refresh_profiles",
                        "{lbl_refresh}"
                    }
                }
            }

            if filtered.is_empty() {
                div { class: "nlpa-card",
                    div { class: "nlpa-muted", "{lbl_no_profiles}" }
                }
            } else {
                for item in filtered {
                    ProfileCard {
                        profile: item,
                        lbl_iccid: lbl_iccid.clone(),
                        lbl_copy_iccid: lbl_copy_iccid.clone(),
                        lbl_enable: lbl_enable.clone(),
                        lbl_disable: lbl_disable.clone(),
                        lbl_rename: lbl_rename.clone(),
                        lbl_delete: lbl_delete.clone(),
                    }
                }
            }
        }
    }
}

#[component]
fn ReaderChip(reader: ReaderChipView) -> Element {
    let cls = if reader.is_selected {
        "nlpa-chip is-selected"
    } else {
        "nlpa-chip"
    };
    rsx! {
        button {
            r#type: "button",
            class: "{cls}",
            "data-action": "select_reader",
            "data-p-id": "{reader.id}",
            span { class: "nlpa-badge", "{reader.transport_badge}" }
            span { "{reader.label}" }
            if !reader.slot_label.is_empty() {
                span { class: "nlpa-muted", "· {reader.slot_label}" }
            }
            if reader.needs_permission {
                span { class: "nlpa-badge is-warn", "Perm" }
            }
        }
    }
}

#[component]
fn EuiccSummaryCard(
    info: EuiccInfoView,
    lbl_eid: String,
    lbl_nvram: String,
    lbl_copy_eid: String,
) -> Element {
    rsx! {
        section { class: "nlpa-card",
            div { class: "nlpa-row",
                div { class: "nlpa-col",
                    div { class: "nlpa-muted", "{lbl_eid} · {info.reader_name}" }
                    div { class: "nlpa-mono", "{info.display_eid}" }
                }
                button {
                    r#type: "button",
                    class: "nlpa-btn",
                    "data-action": "copy_text",
                    "data-p-text": "{info.raw_eid}",
                    "{lbl_copy_eid}"
                }
            }
            div { class: "nlpa-row",
                span { class: "nlpa-badge", "{lbl_nvram}: {info.free_nvram_text}" }
                if !info.free_ram_text.is_empty() {
                    span { class: "nlpa-badge", "RAM: {info.free_ram_text}" }
                }
                if !info.svn.is_empty() {
                    span { class: "nlpa-badge", "SGP.22 v{info.svn}" }
                }
            }
            div { class: "nlpa-row",
                button {
                    r#type: "button",
                    class: "nlpa-btn",
                    "data-action": "open_euicc_details",
                    "Chip Details"
                }
                if info.is_estk {
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "open_estk_modal",
                        "eSTK"
                    }
                }
                if info.is_9esim {
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "open_nine_esim_modal",
                        "9eSIM"
                    }
                }
                if info.is_gm {
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "open_gm_psk_modal",
                        if info.gm_authenticated { "GM Auth (OK)" } else { "GM Auth" }
                    }
                }
                button {
                    r#type: "button",
                    class: "nlpa-btn is-danger",
                    "data-action": "open_reset_euicc",
                    "Reset eUICC"
                }
            }
        }
    }
}

#[component]
fn ProfileCard(
    profile: ProfileCardView,
    lbl_iccid: String,
    lbl_copy_iccid: String,
    lbl_enable: String,
    lbl_disable: String,
    lbl_rename: String,
    lbl_delete: String,
) -> Element {
    let card_cls = if profile.enabled {
        "nlpa-card is-enabled"
    } else {
        "nlpa-card"
    };
    let toggle_label = if profile.enabled {
        lbl_disable
    } else {
        lbl_enable
    };
    let toggle_enable = if profile.enabled { "false" } else { "true" };

    rsx! {
        article { class: "{card_cls}",
            div { class: "nlpa-row",
                div { class: "nlpa-col",
                    div { class: "nlpa-row",
                        strong {
                            if !profile.hide_flag && !profile.flag_emoji.is_empty() {
                                "{profile.flag_emoji} "
                            }
                            "{profile.display_name}"
                        }
                        if profile.enabled {
                            span { class: "nlpa-badge is-success", "Enabled" }
                        }
                        if profile.is_test_profile {
                            span { class: "nlpa-badge is-warn", "TEST" }
                        }
                        if profile.is_provisioning_profile {
                            span { class: "nlpa-badge is-warn", "PROV" }
                        }
                        if let Some(size) = profile.estimated_size_text.as_ref() {
                            span { class: "nlpa-badge", "~{size}" }
                        }
                    }
                    if !profile.operator_summary.is_empty() {
                        div { class: "nlpa-muted", "{profile.operator_summary}" }
                    }
                    div { class: "nlpa-mono nlpa-muted", "{lbl_iccid}: {profile.display_iccid}" }
                }
                button {
                    r#type: "button",
                    class: "nlpa-btn",
                    "data-action": "copy_text",
                    "data-p-text": "{profile.iccid}",
                    "{lbl_copy_iccid}"
                }
            }

            if !profile.tags.is_empty() {
                div { class: "nlpa-row",
                    for tag in profile.tags.iter() {
                        span { class: "nlpa-badge", "{tag}" }
                    }
                }
            }

            if let Some(badge) = profile.provider_badge.as_ref() {
                div { class: "nlpa-banner",
                    strong { "{badge.label}" }
                    if let Some(msisdn) = badge.msisdn.as_ref() {
                        span { " · {msisdn}" }
                    }
                    if let Some(quota) = badge.quota_summary.as_ref() {
                        div { class: "nlpa-muted", "{quota}" }
                    }
                    if let Some(url) = badge.portal_url.as_ref() {
                        button {
                            r#type: "button",
                            class: "nlpa-btn",
                            "data-action": "open_url",
                            "data-p-url": "{url}",
                            "Portal"
                        }
                    }
                }
            } else if profile.has_provider_plugin {
                div { class: "nlpa-row",
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "fetch_provider_badge",
                        "data-p-iccid": "{profile.iccid}",
                        "Load Provider Status"
                    }
                }
            }

            div { class: "nlpa-row",
                button {
                    r#type: "button",
                    class: if profile.enabled { "nlpa-btn" } else { "nlpa-btn is-primary" },
                    disabled: profile.switching,
                    "data-action": "request_switch_profile",
                    "data-p-iccid": "{profile.iccid}",
                    "data-b-enable": "{toggle_enable}",
                    "{toggle_label}"
                }
                button {
                    r#type: "button",
                    class: "nlpa-btn",
                    "data-action": "open_rename_profile",
                    "data-p-iccid": "{profile.iccid}",
                    "{lbl_rename}"
                }
                button {
                    r#type: "button",
                    class: "nlpa-btn is-danger",
                    disabled: profile.enabled,
                    "data-action": "open_delete_profile",
                    "data-p-iccid": "{profile.iccid}",
                    "{lbl_delete}"
                }
            }
        }
    }
}

fn filter_and_sort_profiles(
    profiles: &[ProfileCardView],
    query: &str,
    sort: ProfileSortKind,
) -> Vec<ProfileCardView> {
    let q = query.trim().to_ascii_lowercase();
    let mut out: Vec<ProfileCardView> = profiles
        .iter()
        .filter(|p| {
            if q.is_empty() {
                return true;
            }
            p.display_name.to_ascii_lowercase().contains(&q)
                || p.operator_summary.to_ascii_lowercase().contains(&q)
                || p.iccid.to_ascii_lowercase().contains(&q)
                || p.tags.iter().any(|t| t.to_ascii_lowercase().contains(&q))
        })
        .cloned()
        .collect();

    match sort {
        ProfileSortKind::EnabledFirst => out.sort_by(|a, b| {
            b.enabled
                .cmp(&a.enabled)
                .then_with(|| a.display_name.cmp(&b.display_name))
        }),
        ProfileSortKind::NameAsc => out.sort_by(|a, b| a.display_name.cmp(&b.display_name)),
        ProfileSortKind::NameDesc => out.sort_by(|a, b| b.display_name.cmp(&a.display_name)),
        ProfileSortKind::IccidAsc => out.sort_by(|a, b| a.iccid.cmp(&b.iccid)),
    }
    out
}
