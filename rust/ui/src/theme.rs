//! Theme resolution and mobile-first responsive CSS generation for the Dioxus SSR UI.

use nlpa_core::storage::{ColorMode, ThemeStyle};
use serde::{Deserialize, Serialize};

/// Resolved visual theme parameters applied to the root shell and Android bars.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThemeTokens {
    pub theme_style: ThemeStyle,
    pub is_amoled: bool,
    pub is_dark: bool,
    pub is_stock: bool,
    pub wallpaper_active: bool,
    pub wallpaper_opacity: f32,
    pub status_bar_black: bool,
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self {
            theme_style: ThemeStyle::Amoled,
            is_amoled: true,
            is_dark: true,
            is_stock: false,
            wallpaper_active: false,
            wallpaper_opacity: 0.35,
            status_bar_black: true,
        }
    }
}

impl ThemeTokens {
    /// Resolves effective UI tokens from persisted style, color mode, system dark hint, and wallpaper state.
    #[must_use]
    pub fn resolve(
        theme_style: ThemeStyle,
        color_mode: ColorMode,
        system_dark: bool,
        wallpaper_enabled: bool,
        wallpaper_opacity: f32,
    ) -> Self {
        let is_dark = match color_mode {
            ColorMode::Dark => true,
            ColorMode::Light => false,
            ColorMode::System => system_dark,
        };
        let is_amoled = theme_style == ThemeStyle::Amoled && is_dark;
        let is_stock = theme_style == ThemeStyle::Stock;
        let clamped_opacity = if wallpaper_opacity.is_finite() {
            wallpaper_opacity.clamp(0.05, 0.95)
        } else {
            0.35
        };

        Self {
            theme_style,
            is_amoled,
            is_dark,
            is_stock,
            wallpaper_active: wallpaper_enabled,
            wallpaper_opacity: clamped_opacity,
            status_bar_black: is_amoled && !wallpaper_enabled,
        }
    }

    /// Returns the wire-format string identifier of `theme_style`.
    #[must_use]
    pub const fn theme_style_wire(&self) -> &'static str {
        match self.theme_style {
            ThemeStyle::Amoled => "amoled",
            ThemeStyle::Custom => "custom",
            ThemeStyle::Stock => "stock",
        }
    }

    /// Returns the CSS class list for `.nlpa-shell`.
    #[must_use]
    pub fn shell_classes(&self) -> String {
        let mut classes = Vec::with_capacity(5);
        classes.push("nlpa-shell");
        if self.is_dark {
            classes.push("is-dark");
        } else {
            classes.push("is-light");
        }
        if self.is_amoled {
            classes.push("is-amoled");
        }
        if self.is_stock {
            classes.push("is-stock");
        }
        if self.wallpaper_active {
            classes.push("is-wallpaper");
        }
        classes.join(" ")
    }

    /// Returns the inline CSS custom property assignment for `--nlpa-glass-alpha`.
    #[must_use]
    pub fn shell_inline_style(&self) -> String {
        format!("--nlpa-glass-alpha: {:.2};", self.wallpaper_opacity)
    }
}

/// Complete stylesheet embedded inside the initial HTML5 document.
pub const APP_CSS: &str = r#"
:root {
  color-scheme: dark light;
}
* {
  box-sizing: border-box;
  max-width: 100%;
}
html, body {
  margin: 0;
  padding: 0;
  width: 100%;
  min-height: 100%;
  background: transparent;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  font-size: 14px;
  line-height: 1.45;
  -webkit-tap-highlight-color: transparent;
  overflow-x: hidden;
}
.nlpa-shell {
  --bg: #111318;
  --surface: #181c24;
  --surface-2: #212733;
  --card: #1a202b;
  --border: #2c3445;
  --text: #eef2f8;
  --muted: #98a2b3;
  --accent: #3b82f6;
  --accent-fg: #ffffff;
  --success: #22c55e;
  --warn: #f59e0b;
  --danger: #ef4444;
  --overlay-bg: rgba(8, 10, 14, 0.72);
  --nlpa-glass-alpha: 0.35;
  min-height: 100vh;
  width: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg);
  color: var(--text);
  overflow-x: hidden;
}
.nlpa-shell.is-light {
  --bg: #f4f6f9;
  --surface: #ffffff;
  --surface-2: #edf1f6;
  --card: #ffffff;
  --border: #d7dee8;
  --text: #111827;
  --muted: #526071;
  --accent: #2563eb;
  --accent-fg: #ffffff;
  --success: #16a34a;
  --warn: #d97706;
  --danger: #dc2626;
  --overlay-bg: rgba(244, 246, 249, 0.76);
}
.nlpa-shell.is-amoled {
  --bg: #000000;
  --surface: #070707;
  --surface-2: #121212;
  --card: #0b0b0b;
  --border: #222222;
  --text: #f5f7fa;
  --muted: #9aa3b2;
  --accent: #60a5fa;
  --accent-fg: #000000;
  --overlay-bg: rgba(0, 0, 0, 0.84);
}
.nlpa-shell.is-stock {
  --accent: #0d9488;
}
.nlpa-shell.is-wallpaper,
.nlpa-shell.is-wallpaper.is-amoled {
  background: transparent !important;
  --bg: transparent;
}
.nlpa-shell.is-wallpaper.is-dark {
  --surface: rgba(12, 14, 18, var(--nlpa-glass-alpha, 0.35));
  --surface-2: rgba(24, 28, 36, var(--nlpa-glass-alpha, 0.35));
  --card: rgba(10, 12, 16, var(--nlpa-glass-alpha, 0.35));
  --border: rgba(255, 255, 255, 0.14);
  --overlay-bg: rgba(0, 0, 0, calc(var(--nlpa-glass-alpha, 0.35) * 0.65));
}
.nlpa-shell.is-wallpaper.is-light {
  --surface: rgba(255, 255, 255, var(--nlpa-glass-alpha, 0.35));
  --surface-2: rgba(243, 246, 250, var(--nlpa-glass-alpha, 0.35));
  --card: rgba(255, 255, 255, var(--nlpa-glass-alpha, 0.35));
  --border: rgba(17, 24, 39, 0.12);
  --overlay-bg: rgba(255, 255, 255, calc(var(--nlpa-glass-alpha, 0.35) * 0.55));
}
.nlpa-shell.is-wallpaper .nlpa-card,
.nlpa-shell.is-wallpaper .nlpa-section,
.nlpa-shell.is-wallpaper .nlpa-settings,
.nlpa-shell.is-wallpaper .nlpa-topbar,
.nlpa-shell.is-wallpaper .nlpa-bottom-nav,
.nlpa-shell.is-wallpaper .nlpa-modal,
.nlpa-shell.is-wallpaper .nlpa-loading-overlay,
.nlpa-shell.is-wallpaper .nlpa-loading-card {
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
}
.nlpa-topbar {
  position: sticky;
  top: 0;
  z-index: 15;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: calc(env(safe-area-inset-top, 0px) + 10px) 14px 10px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
}
.nlpa-title {
  margin: 0;
  font-size: 17px;
  font-weight: 700;
  letter-spacing: -0.01em;
}
.nlpa-main {
  flex: 1;
  width: 100%;
  max-width: 720px;
  margin: 0 auto;
  padding: 12px 12px 84px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: transparent;
}
.nlpa-settings {
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: transparent;
}
.nlpa-card,
.nlpa-section {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 13px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.nlpa-card.is-enabled {
  border-color: var(--success);
}
.nlpa-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  flex-wrap: wrap;
}
.nlpa-col {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
  flex: 1;
}
.nlpa-chips {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  padding-bottom: 2px;
  scrollbar-width: none;
}
.nlpa-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text);
  border-radius: 999px;
  padding: 7px 12px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
}
.nlpa-chip.is-selected {
  background: var(--accent);
  color: var(--accent-fg);
  border-color: var(--accent);
}
.nlpa-badge {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  padding: 2px 8px;
  font-size: 11px;
  font-weight: 600;
  background: var(--surface-2);
  color: var(--muted);
  border: 1px solid var(--border);
}
.nlpa-badge.is-success {
  color: var(--success);
  border-color: var(--success);
}
.nlpa-badge.is-warn {
  color: var(--warn);
  border-color: var(--warn);
}
.nlpa-badge.is-danger {
  color: var(--danger);
  border-color: var(--danger);
}
.nlpa-mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
  overflow-wrap: anywhere;
  word-break: break-word;
}
.nlpa-muted {
  color: var(--muted);
  font-size: 12.5px;
}
.nlpa-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text);
  border-radius: 10px;
  padding: 8px 12px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}
.nlpa-btn.is-primary {
  background: var(--accent);
  border-color: var(--accent);
  color: var(--accent-fg);
}
.nlpa-btn.is-danger {
  border-color: var(--danger);
  color: var(--danger);
}
.nlpa-btn:disabled {
  opacity: 0.48;
  cursor: not-allowed;
}
.nlpa-input,
.nlpa-select,
.nlpa-textarea {
  width: 100%;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text);
  border-radius: 10px;
  padding: 9px 11px;
  font-size: 13.5px;
  font-family: inherit;
}
.nlpa-textarea {
  min-height: 92px;
  resize: vertical;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12.5px;
}
.nlpa-range {
  width: 100%;
  accent-color: var(--accent);
}
.nlpa-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 4px 0;
}
.nlpa-toggle input[type="checkbox"] {
  width: 20px;
  height: 20px;
  accent-color: var(--accent);
  flex-shrink: 0;
}
.nlpa-banner {
  border-radius: 12px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  background: var(--surface);
  font-size: 13px;
}
.nlpa-banner.is-warn {
  border-color: var(--warn);
}
.nlpa-banner.is-danger {
  border-color: var(--danger);
}
.nlpa-banner.is-success {
  border-color: var(--success);
}
.nlpa-progress {
  width: 100%;
  height: 8px;
  border-radius: 999px;
  background: var(--surface-2);
  overflow: hidden;
}
.nlpa-progress-fill {
  height: 100%;
  background: var(--accent);
  transition: width 0.2s ease;
}
.nlpa-loading-overlay {
  position: fixed;
  inset: 0;
  z-index: 40;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: var(--overlay-bg);
}
.nlpa-loading-card,
.nlpa-modal {
  width: 100%;
  max-width: 440px;
  max-height: 85vh;
  overflow-y: auto;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.nlpa-modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 45;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
  background: var(--overlay-bg);
}
.nlpa-toast {
  position: fixed;
  left: 12px;
  right: 12px;
  bottom: 72px;
  max-width: 520px;
  margin: 0 auto;
  z-index: 50;
  padding: 10px 14px;
  border-radius: 12px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.nlpa-toast.is-error {
  border-color: var(--danger);
}
.nlpa-bottom-nav {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 25;
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  background: var(--surface);
  border-top: 1px solid var(--border);
  padding: 6px 4px calc(env(safe-area-inset-bottom, 0px) + 6px);
}
.nlpa-nav-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  background: transparent;
  border: 0;
  color: var(--muted);
  font-size: 11px;
  font-weight: 600;
  padding: 4px 2px;
  cursor: pointer;
}
.nlpa-nav-btn.is-active {
  color: var(--accent);
}
.nlpa-spinner {
  width: 26px;
  height: 26px;
  border: 3px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: nlpa-spin 0.8s linear infinite;
}
@keyframes nlpa-spin {
  to { transform: rotate(360deg); }
}
"#;
