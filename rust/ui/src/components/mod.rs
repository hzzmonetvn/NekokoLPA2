//! Dioxus 0.6 SSR UI components and localization lookup helpers.

pub mod app;
pub mod download_page;
pub mod modals;
pub mod notifications_page;
pub mod profiles_page;
pub mod settings_page;
pub mod transfer_page;

use nlpa_core::i18n::{resolve_locale, I18nCatalog, LocaleChoice};

/// Translates an ARB key using [`I18nCatalog`], falling back to `fallback` if missing.
#[must_use]
pub fn tr(catalog: &I18nCatalog, locale: LocaleChoice, key: &str, fallback: &str) -> String {
    let resolved = resolve_locale(locale, "");
    let value = catalog.get(resolved, key);
    if value.is_empty() || value == key {
        fallback.to_string()
    } else {
        value.to_string()
    }
}
