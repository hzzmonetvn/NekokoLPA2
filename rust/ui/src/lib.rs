//! Dioxus 0.6 SSR UI crate (`nlpa_ui`) for NekokoLPA2.

pub mod components;
pub mod models;
pub mod shell;
pub mod theme;

pub use models::{RenderMeta, TabKind, UiAction, UiModel};
pub use shell::{render_document, render_fragment, render_meta};
pub use theme::ThemeTokens;
