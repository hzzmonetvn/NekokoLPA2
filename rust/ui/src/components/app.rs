//! Root Dioxus component assembling the navigation shell, loading overlay, modals, and toast.

use std::sync::OnceLock;

use dioxus::prelude::*;
use nlpa_core::i18n::I18nCatalog;

static UI_CATALOG: OnceLock<I18nCatalog> = OnceLock::new();

fn shared_catalog() -> &'static I18nCatalog {
    UI_CATALOG.get_or_init(I18nCatalog::new)
}

use crate::components::{
    download_page, modals, notifications_page, profiles_page, settings_page, tr, transfer_page,
};
use crate::models::{TabKind, UiModel};
use crate::theme::ThemeTokens;

/// Props accepted by [`app_root`] for deterministic Dioxus SSR rendering.
#[derive(PartialEq, Props, Clone)]
pub struct AppRootProps {
    pub model: UiModel,
}

/// Top-level Dioxus root component rendered by [`crate::render_fragment`].
pub fn app_root(props: AppRootProps) -> Element {
    let model = props.model;
    let catalog = shared_catalog();
    let locale = model.locale;
    let tokens = ThemeTokens::resolve(
        model.theme_style,
        model.color_mode,
        model.system_dark,
        model.wallpaper_enabled,
        model.wallpaper_opacity,
    );

    let shell_class = tokens.shell_classes();
    let shell_style = tokens.shell_inline_style();

    let lbl_profiles = tr(catalog, locale, "profilesTitle", "Profiles");
    let lbl_download = tr(catalog, locale, "downloadProfile", "Download");
    let lbl_transfer = tr(catalog, locale, "transferTitle", "Transfer");
    let lbl_notifs = tr(catalog, locale, "notifications", "Notifications");
    let lbl_settings = tr(catalog, locale, "settingsTitle", "Settings");
    let lbl_refresh = tr(catalog, locale, "refresh", "Refresh");
    let lbl_cancel = tr(catalog, locale, "cancel", "Cancel");
    let lbl_retry = tr(catalog, locale, "retry", "Retry");
    let lbl_ok = tr(catalog, locale, "ok", "OK");
    let theme_wire = tokens.theme_style_wire();
    let overlay_cls = if tokens.wallpaper_active {
        "nlpa-loading-overlay is-wallpaper-glass"
    } else {
        "nlpa-loading-overlay"
    };

    let active_title = match model.active_tab {
        TabKind::Profiles => lbl_profiles.clone(),
        TabKind::Download => lbl_download.clone(),
        TabKind::Transfer => lbl_transfer.clone(),
        TabKind::Notifications => lbl_notifs.clone(),
        TabKind::Settings => lbl_settings.clone(),
    };

    rsx! {
        div {
            class: "{shell_class}",
            style: "{shell_style}",
            "data-theme-style": "{theme_wire}",
            "data-dark": if tokens.is_dark { "true" } else { "false" },
            "data-wallpaper": if tokens.wallpaper_active { "true" } else { "false" },
            header { class: "nlpa-topbar",
                h1 { class: "nlpa-title", "NekokoLPA · {active_title}" }
                button {
                    r#type: "button",
                    class: "nlpa-btn",
                    "data-action": "refresh_readers",
                    "{lbl_refresh}"
                }
            }

            main { class: "nlpa-main",
                match model.active_tab {
                    TabKind::Profiles => profiles_page::render(&model, catalog),
                    TabKind::Download => download_page::render(&model, catalog),
                    TabKind::Transfer => transfer_page::render(&model, catalog),
                    TabKind::Notifications => notifications_page::render(&model, catalog),
                    TabKind::Settings => settings_page::render(&model, catalog),
                }
            }

            if model.loading.active {
                div {
                    class: "{overlay_cls}",
                    div { class: "nlpa-loading-card",
                        div { class: "nlpa-row",
                            if model.loading.error.is_none() {
                                div { class: "nlpa-spinner" }
                            }
                            strong { "{model.loading.title}" }
                        }
                        if !model.loading.message.is_empty() {
                            div { class: "nlpa-muted", "{model.loading.message}" }
                        }
                        if let Some(err) = model.loading.error.as_ref() {
                            div { class: "nlpa-banner is-danger", "{err}" }
                        }
                        div { class: "nlpa-row",
                            if model.loading.can_cancel {
                                button {
                                    r#type: "button",
                                    class: "nlpa-btn",
                                    "data-action": "cancel_loading",
                                    "{lbl_cancel}"
                                }
                            }
                            if model.loading.can_retry {
                                button {
                                    r#type: "button",
                                    class: "nlpa-btn is-primary",
                                    "data-action": "retry_loading",
                                    "{lbl_retry}"
                                }
                            }
                        }
                    }
                }
            }

            {modals::render(&model, catalog)}

            if let Some((msg, is_err)) = model.toast.as_ref() {
                div { class: if *is_err { "nlpa-toast is-error" } else { "nlpa-toast" },
                    span { "{msg}" }
                    button {
                        r#type: "button",
                        class: "nlpa-btn",
                        "data-action": "dismiss_toast",
                        "{lbl_ok}"
                    }
                }
            }

            nav { class: "nlpa-bottom-nav",
                NavTabButton { tab: TabKind::Profiles, label: lbl_profiles, active: model.active_tab == TabKind::Profiles }
                NavTabButton { tab: TabKind::Download, label: lbl_download, active: model.active_tab == TabKind::Download }
                NavTabButton { tab: TabKind::Transfer, label: lbl_transfer, active: model.active_tab == TabKind::Transfer }
                NavTabButton { tab: TabKind::Notifications, label: lbl_notifs, active: model.active_tab == TabKind::Notifications }
                NavTabButton { tab: TabKind::Settings, label: lbl_settings, active: model.active_tab == TabKind::Settings }
            }
        }
    }
}

#[component]
fn NavTabButton(tab: TabKind, label: String, active: bool) -> Element {
    let tab_str = tab.as_str();
    rsx! {
        button {
            r#type: "button",
            class: if active { "nlpa-nav-btn is-active" } else { "nlpa-nav-btn" },
            "data-action": "switch_tab",
            "data-p-tab": "{tab_str}",
            span { "{label}" }
        }
    }
}
