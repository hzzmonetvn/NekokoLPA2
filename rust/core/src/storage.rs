//! JSON-backed persistent application settings, profile tags/nicknames cache,
//! notification history, and RemoCard configuration.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::channel::DEFAULT_STORE_DATA_MSS;
use crate::i18n::LocaleChoice;
use crate::redaction::RedactionSettings;
use crate::remocard::RemoCardServer;

const MIN_WALLPAPER_OPACITY: f32 = 0.05;
const MAX_WALLPAPER_OPACITY: f32 = 0.95;
const DEFAULT_WALLPAPER_OPACITY: f32 = 0.35;
const DEFAULT_DEVICE_IMEI: &str = "353837410000002";

/// Visual theme family for the Android UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeStyle {
    #[default]
    Amoled,
    Custom,
    Stock,
}

/// Dark/light color mode preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorMode {
    #[default]
    Dark,
    Light,
    System,
}

/// Persisted eUICC notification delivery log entry.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistedNotification {
    pub eid: String,
    pub iccid: String,
    pub seq_number: u32,
    pub operation: String,
    pub address: String,
    pub content_base64: String,
    pub status: String,
    pub timestamp_ms: u64,
    pub response_code: Option<u16>,
    pub response_content: Option<String>,
}

/// Complete serializable application state persisted to `state.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistedState {
    pub theme_style: ThemeStyle,
    pub color_mode: ColorMode,
    pub locale: LocaleChoice,
    pub wallpaper_enabled: bool,
    pub wallpaper_opacity: f32,
    pub mss: u8,
    pub enable_omapi: bool,
    pub enable_tmapi: bool,
    pub enable_nbridge: bool,
    pub enable_adb_bridge: bool,
    pub enable_qrtr: bool,
    pub enable_se_hal: bool,
    pub se_hal_borrowed_slot1: bool,
    pub se_hal_borrowed_slot2: bool,
    pub enable_ccid: bool,
    pub enable_remote: bool,
    pub custom_aids: Vec<String>,
    pub remocard_servers: Vec<RemoCardServer>,
    pub proxy_url: Option<String>,
    pub device_imei: String,
    pub redaction: RedactionSettings,
    pub profile_nicknames_cache: BTreeMap<String, String>,
    pub profile_tags: BTreeMap<String, String>,
    pub notifications_history: Vec<PersistedNotification>,
    pub gm_psk_cache: BTreeMap<String, String>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            theme_style: ThemeStyle::Amoled,
            color_mode: ColorMode::Dark,
            locale: LocaleChoice::System,
            wallpaper_enabled: false,
            wallpaper_opacity: DEFAULT_WALLPAPER_OPACITY,
            mss: DEFAULT_STORE_DATA_MSS as u8,
            enable_omapi: true,
            enable_tmapi: true,
            enable_nbridge: true,
            enable_adb_bridge: true,
            enable_qrtr: false,
            enable_se_hal: false,
            se_hal_borrowed_slot1: false,
            se_hal_borrowed_slot2: false,
            enable_ccid: true,
            enable_remote: false,
            custom_aids: default_isdr_aids(),
            remocard_servers: Vec::new(),
            proxy_url: None,
            device_imei: DEFAULT_DEVICE_IMEI.to_string(),
            redaction: RedactionSettings::default(),
            profile_nicknames_cache: BTreeMap::new(),
            profile_tags: BTreeMap::new(),
            notifications_history: Vec::new(),
            gm_psk_cache: BTreeMap::new(),
        }
    }
}

impl PersistedState {
    /// Clamps persisted fields into safe operational bounds (e.g., `wallpaper_opacity`
    /// within `0.05..=0.95` and `mss` within `60..=240`).
    pub fn normalize(&mut self) {
        self.wallpaper_opacity = clamp_wallpaper_opacity(self.wallpaper_opacity);
        self.mss = self.mss.clamp(60, 240);
    }

    /// Parses [`PersistedState`] from JSON and normalizes out-of-range values.
    ///
    /// # Errors
    /// Returns a `serde_json::Error` if the JSON payload is malformed.
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        let mut state: Self = serde_json::from_str(json)?;
        state.normalize();
        Ok(state)
    }

    /// Serializes [`PersistedState`] into pretty-printed JSON.
    ///
    /// # Errors
    /// Returns a `serde_json::Error` if serialization fails.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Clamps wallpaper opacity into `0.05..=0.95`, defaulting non-finite values to `0.35`.
#[must_use]
pub fn clamp_wallpaper_opacity(opacity: f32) -> f32 {
    if !opacity.is_finite() {
        return DEFAULT_WALLPAPER_OPACITY;
    }
    opacity.clamp(MIN_WALLPAPER_OPACITY, MAX_WALLPAPER_OPACITY)
}

fn default_isdr_aids() -> Vec<String> {
    vec![
        "A06573746B6D65FFFF4953442D522031".to_string(),
        "A06573746B6D65FFFF4953442D522030".to_string(),
        "A0000005591010FFFFFFFF8900000100".to_string(),
        "A0000005591010FFFFFFFF8900050500".to_string(),
        "A0000005591010000000008900000300".to_string(),
        "A0000005591010FFFFFFFF8900000177".to_string(),
    ]
}
