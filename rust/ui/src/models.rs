//! Serializable view models and user action schemas shared between `AppController` and Dioxus SSR.

use nlpa_core::i18n::{LocaleChoice, ALL_LOCALE_CHOICES};
use nlpa_core::storage::{ColorMode, ThemeStyle};
use serde::{Deserialize, Serialize};

/// Top-level navigation tabs in the single-page WebView shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabKind {
    #[default]
    Profiles,
    Download,
    Transfer,
    Notifications,
    Settings,
}

impl TabKind {
    /// Wire identifier used in DOM `data-tab` attributes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Profiles => "profiles",
            Self::Download => "download",
            Self::Transfer => "transfer",
            Self::Notifications => "notifications",
            Self::Settings => "settings",
        }
    }

    /// Parses a wire identifier into a [`TabKind`], defaulting to [`TabKind::Profiles`].
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "download" => Self::Download,
            "transfer" => Self::Transfer,
            "notifications" => Self::Notifications,
            "settings" => Self::Settings,
            _ => Self::Profiles,
        }
    }
}

/// Profile ordering options displayed on the Profiles home screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSortKind {
    #[default]
    EnabledFirst,
    NameAsc,
    NameDesc,
    IccidAsc,
}

impl ProfileSortKind {
    /// Wire identifier for `<select>` options.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EnabledFirst => "enabled_first",
            Self::NameAsc => "name_asc",
            Self::NameDesc => "name_desc",
            Self::IccidAsc => "iccid_asc",
        }
    }

    /// Parses a wire string into [`ProfileSortKind`].
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "name_asc" => Self::NameAsc,
            "name_desc" => Self::NameDesc,
            "iccid_asc" => Self::IccidAsc,
            _ => Self::EnabledFirst,
        }
    }
}

/// Descriptor for one item in the Settings language dropdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocaleOptionView {
    pub choice: LocaleChoice,
    pub code: &'static str,
    pub native_name: &'static str,
}

/// Returns the wire tag for a [`LocaleChoice`].
#[must_use]
pub const fn locale_code(choice: LocaleChoice) -> &'static str {
    match choice {
        LocaleChoice::System => "system",
        LocaleChoice::En => "en",
        LocaleChoice::Vi => "vi",
        LocaleChoice::Zh => "zh",
        LocaleChoice::ZhTw => "zh-TW",
        LocaleChoice::Ja => "ja",
        LocaleChoice::Ko => "ko",
        LocaleChoice::De => "de",
        LocaleChoice::Es => "es",
        LocaleChoice::Fr => "fr",
        LocaleChoice::It => "it",
        LocaleChoice::Ru => "ru",
    }
}

/// Parses a wire locale tag (case-insensitive) into [`LocaleChoice`].
#[must_use]
pub fn parse_locale_code(raw: &str) -> LocaleChoice {
    match raw.trim().to_ascii_lowercase().as_str() {
        "en" => LocaleChoice::En,
        "vi" => LocaleChoice::Vi,
        "zh" | "zh-cn" | "zh_cn" | "zh-hans" => LocaleChoice::Zh,
        "zh-tw" | "zh_tw" | "zh-hant" | "zhtw" => LocaleChoice::ZhTw,
        "ja" => LocaleChoice::Ja,
        "ko" => LocaleChoice::Ko,
        "de" => LocaleChoice::De,
        "es" => LocaleChoice::Es,
        "fr" => LocaleChoice::Fr,
        "it" => LocaleChoice::It,
        "ru" => LocaleChoice::Ru,
        _ => LocaleChoice::System,
    }
}

/// Native language label shown in the Settings language selector.
#[must_use]
pub const fn locale_native_label(choice: LocaleChoice) -> &'static str {
    match choice {
        LocaleChoice::System => "System",
        LocaleChoice::En => "English (en)",
        LocaleChoice::Vi => "Tiếng Việt (vi)",
        LocaleChoice::Zh => "简体中文 (zh)",
        LocaleChoice::ZhTw => "繁體中文 (zh-TW)",
        LocaleChoice::Ja => "日本語 (ja)",
        LocaleChoice::Ko => "한국어 (ko)",
        LocaleChoice::De => "Deutsch (de)",
        LocaleChoice::Es => "Español (es)",
        LocaleChoice::Fr => "Français (fr)",
        LocaleChoice::It => "Italiano (it)",
        LocaleChoice::Ru => "Русский (ru)",
    }
}

/// Returns all 12 supported locale options backed by [`ALL_LOCALE_CHOICES`].
#[must_use]
pub fn all_locale_options() -> Vec<LocaleOptionView> {
    ALL_LOCALE_CHOICES
        .iter()
        .copied()
        .map(|choice| LocaleOptionView {
            choice,
            code: locale_code(choice),
            native_name: locale_native_label(choice),
        })
        .collect()
}

/// Converts [`ThemeStyle`] to its wire string.
#[must_use]
pub const fn theme_style_code(style: ThemeStyle) -> &'static str {
    match style {
        ThemeStyle::Amoled => "amoled",
        ThemeStyle::Custom => "custom",
        ThemeStyle::Stock => "stock",
    }
}

/// Parses a wire string into [`ThemeStyle`].
#[must_use]
pub fn parse_theme_style_code(raw: &str) -> ThemeStyle {
    match raw.trim().to_ascii_lowercase().as_str() {
        "custom" => ThemeStyle::Custom,
        "stock" => ThemeStyle::Stock,
        _ => ThemeStyle::Amoled,
    }
}

/// Converts [`ColorMode`] to its wire string.
#[must_use]
pub const fn color_mode_code(mode: ColorMode) -> &'static str {
    match mode {
        ColorMode::Dark => "dark",
        ColorMode::Light => "light",
        ColorMode::System => "system",
    }
}

/// Parses a wire string into [`ColorMode`].
#[must_use]
pub fn parse_color_mode_code(raw: &str) -> ColorMode {
    match raw.trim().to_ascii_lowercase().as_str() {
        "light" => ColorMode::Light,
        "system" => ColorMode::System,
        _ => ColorMode::Dark,
    }
}

/// Render metadata returned from `nativeDispatch`, `nativeRender`, and `nativeOnPlatformEvent`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderMeta {
    pub html: String,
    pub theme_style: String,
    pub dark: bool,
    pub wallpaper_active: bool,
    pub wallpaper_opacity: f32,
    pub status_bar_black: bool,
}

/// Connected or discovered smart-card reader chip.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReaderChipView {
    pub id: String,
    pub label: String,
    pub transport_badge: String,
    pub slot_label: String,
    pub eid_preview: Option<String>,
    pub is_selected: bool,
    pub is_removable: bool,
    pub is_connected: bool,
    pub needs_permission: bool,
    pub is_estk: bool,
    pub is_9esim: bool,
    pub is_gm: bool,
}

/// eSTK card details shown inside the eSTK management dialog.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EstkInfoView {
    pub firmware: Option<String>,
    pub bootloader: Option<String>,
    pub serial: Option<String>,
    pub sku: Option<String>,
}

/// Active reader EID and chip metadata summary.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EuiccInfoView {
    pub reader_name: String,
    pub raw_eid: String,
    pub display_eid: String,
    pub default_smdp: String,
    pub root_smds: String,
    pub free_nvram_bytes: Option<u64>,
    pub free_nvram_text: String,
    pub free_ram_bytes: Option<u64>,
    pub free_ram_text: String,
    pub svn: String,
    pub firmware_ver: String,
    pub profile_version: String,
    pub is_estk: bool,
    pub estk_info: Option<EstkInfoView>,
    pub is_9esim: bool,
    pub is_gm: bool,
    pub gm_authenticated: bool,
    pub gm_status_note: Option<String>,
}

/// Optional carrier plugin badge associated with an installed profile.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProviderBadgeView {
    pub label: String,
    pub msisdn: Option<String>,
    pub quota_summary: Option<String>,
    pub portal_url: Option<String>,
}

/// Single eSIM profile card view model.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProfileCardView {
    pub iccid: String,
    pub display_iccid: String,
    pub display_name: String,
    pub operator_summary: String,
    pub nickname: Option<String>,
    pub service_provider_name: Option<String>,
    pub profile_name: Option<String>,
    pub enabled: bool,
    pub profile_class: u8,
    pub is_test_profile: bool,
    pub is_provisioning_profile: bool,
    pub tags: Vec<String>,
    pub flag_emoji: String,
    pub hide_flag: bool,
    pub hide_icon: bool,
    pub estimated_size_text: Option<String>,
    pub provider_badge: Option<ProviderBadgeView>,
    pub has_provider_plugin: bool,
    pub isdp_aid: Option<String>,
    pub switching: bool,
}

/// Full-screen loading / blocking progress overlay model.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProfileLoadingState {
    pub active: bool,
    pub title: String,
    pub message: String,
    pub error: Option<String>,
    pub can_cancel: bool,
    pub can_retry: bool,
    pub is_card_refreshing: bool,
}

/// Metadata preview fetched before final BPP download.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DownloadMetadataView {
    pub spn: String,
    pub profile_name: String,
    pub iccid: String,
    pub profile_class: u8,
    pub estimated_bytes: Option<i64>,
    pub estimated_text: String,
    pub ci_trusted: bool,
    pub ci_summary: String,
    pub requires_confirmation_code: bool,
    pub insufficient_storage: bool,
}

/// Status row for a single entry in a batch download run.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BatchItemView {
    pub index: usize,
    pub raw_code: String,
    pub smdp: String,
    pub matching_id: String,
    pub status: String,
    pub error: Option<String>,
    pub installed_iccid: Option<String>,
    pub size_text: Option<String>,
}

/// One pending profile event returned by SM-DS discovery.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DiscoveredSmdsEventView {
    pub event_id: String,
    pub address: String,
    pub activation_code: String,
}

/// Download & Batch Download & SM-DS Discovery tab state.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DownloadView {
    pub combined_lpa_input: String,
    pub smdp_input: String,
    pub matching_id_input: String,
    pub oid_input: String,
    pub confirmation_code_input: String,
    pub confirmation_required: bool,
    pub batch_mode: bool,
    pub batch_text: String,
    pub batch_items: Vec<BatchItemView>,
    pub batch_running: bool,
    pub batch_summary: Option<String>,
    pub metadata: Option<DownloadMetadataView>,
    pub step_label: Option<String>,
    pub progress_pct: Option<u8>,
    pub error: Option<String>,
    pub success_message: Option<String>,
    pub downloading: bool,
    pub cancel_requested: bool,
    pub is_se_hal: bool,
    pub se_hal_preview_confirmed: bool,
    pub smds_server_input: String,
    pub discovered_events: Vec<DiscoveredSmdsEventView>,
    pub discovering_smds: bool,
}

/// TS.43 Carrier eSIM Transfer tab state.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TransferView {
    pub carrier_name: String,
    pub source_iccid: String,
    pub source_mcc_mnc: String,
    pub manual_entitlement_server: String,
    pub state_label: String,
    pub step_message: String,
    pub is_busy: bool,
    pub is_eligible: Option<bool>,
    pub error: Option<String>,
    pub prompt_title: Option<String>,
    pub prompt_text: Option<String>,
    pub prompt_accept_label: Option<String>,
    pub prompt_reject_label: Option<String>,
    pub prompt_free_text: bool,
    pub prompt_input: String,
    pub pending_websheet_url: Option<String>,
    pub pending_delete_source_iccid: Option<String>,
    pub activation_code: Option<String>,
    pub delayed_iccid_input: String,
    pub pending_delayed_download: bool,
    pub can_start: bool,
}

/// Notification queued on the physical eUICC.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PendingNotifView {
    pub seq_number: u32,
    pub operation_label: String,
    pub operation_code: u8,
    pub notification_address: String,
    pub iccid: String,
    pub is_install: bool,
}

/// Local history copy of a processed or failed notification.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HistoryNotifView {
    pub seq_number: u32,
    pub operation_label: String,
    pub notification_address: String,
    pub iccid: String,
    pub status_label: String,
    pub timestamp: String,
    pub raw_b64: String,
}

/// Notifications & History tab state.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NotificationsView {
    pub pending: Vec<PendingNotifView>,
    pub history: Vec<HistoryNotifView>,
    pub history_search: String,
    pub show_history: bool,
    pub loading: bool,
    pub auto_send: bool,
    pub auto_remove: bool,
    pub error: Option<String>,
    pub notice: Option<String>,
}

/// Configured RemoCard server item.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RemoCardServerView {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub token: String,
    pub aes_key_hex: String,
    pub enabled: bool,
}

/// Certificate hashes & ABIs reported by the Android host.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AramInfoView {
    pub sha256_hashes: Vec<String>,
    pub sha1_hashes: Vec<String>,
    pub aram_sha256: Vec<String>,
    pub aram_sha1: Vec<String>,
    pub abis: Vec<String>,
}

/// App update candidate details from feed check.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UpdateCandidateView {
    pub version: String,
    pub build: u64,
    pub url: String,
    pub download_url: String,
    pub changelog: String,
}

/// Full Settings tab view model.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SettingsView {
    pub locale_choice: Option<LocaleChoice>,
    pub reveal_sensitive: bool,
    pub redact_eid_first: bool,
    pub redact_eid_middle: bool,
    pub redact_eid_last: bool,
    pub redact_iccid_first: bool,
    pub redact_iccid_middle: bool,
    pub redact_iccid_last: bool,
    pub nickname_mode: String,
    pub operator_mode: String,
    pub tags_mode: String,
    pub flag_mode: String,
    pub icon_mode: String,
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
    pub adb_connected: bool,
    pub adb_waiting: bool,
    pub adb_command: String,
    pub adb_status_text: String,
    pub remocard_servers: Vec<RemoCardServerView>,
    pub mss: u8,
    pub custom_aids: Vec<String>,
    pub proxy_url: String,
    pub device_imei: String,
    pub imei_error: Option<String>,
    pub app_version: String,
    pub version_code: i32,
    pub is_privileged: bool,
    pub cert_sha1: String,
    pub aram_info: Option<AramInfoView>,
    pub update_checking: bool,
    pub update_candidate: Option<UpdateCandidateView>,
    pub update_notice: Option<String>,
    pub logs: Vec<String>,
    pub apdu_history: Vec<(String, String)>,
}

/// Active modal dialog state.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ModalState {
    #[default]
    None,
    EuiccDetails,
    RenameProfile {
        iccid: String,
        current_name: String,
        tags_csv: String,
    },
    ConfirmDeleteProfile {
        iccid: String,
        display_name: String,
    },
    ConfirmSwitchProfile {
        iccid: String,
        display_name: String,
        enable: bool,
        is_se_hal: bool,
    },
    ConfirmEuiccReset,
    EstkManagement {
        atr_hex: String,
    },
    NineEsimManagement {
        device_name: String,
        atr: String,
        euicc_mode: bool,
    },
    GmPskPrompt {
        eid: String,
    },
    BatchExport {
        csv_text: String,
    },
    AramInfo,
    Logs,
    ApduConsole,
}

/// Complete immutable snapshot rendered by Dioxus SSR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiModel {
    pub active_tab: TabKind,
    pub theme_style: ThemeStyle,
    pub color_mode: ColorMode,
    pub system_dark: bool,
    pub locale: LocaleChoice,
    pub wallpaper_enabled: bool,
    pub wallpaper_opacity: f32,
    pub readers: Vec<ReaderChipView>,
    pub selected_reader_id: Option<String>,
    pub omapi_euicc_warning: bool,
    pub omapi_corrupted_banner: Option<String>,
    pub euicc_info: Option<EuiccInfoView>,
    pub profiles: Vec<ProfileCardView>,
    pub profile_search: String,
    pub profile_sort: ProfileSortKind,
    pub loading: ProfileLoadingState,
    pub toast: Option<(String, bool)>,
    pub download: DownloadView,
    pub transfer: TransferView,
    pub notifications: NotificationsView,
    pub settings: SettingsView,
    pub modal: ModalState,
}

impl Default for UiModel {
    fn default() -> Self {
        Self {
            active_tab: TabKind::Profiles,
            theme_style: ThemeStyle::Amoled,
            color_mode: ColorMode::Dark,
            system_dark: true,
            locale: LocaleChoice::System,
            wallpaper_enabled: false,
            wallpaper_opacity: 0.35,
            readers: Vec::new(),
            selected_reader_id: None,
            omapi_euicc_warning: false,
            omapi_corrupted_banner: None,
            euicc_info: None,
            profiles: Vec::new(),
            profile_search: String::new(),
            profile_sort: ProfileSortKind::EnabledFirst,
            loading: ProfileLoadingState::default(),
            toast: None,
            download: DownloadView::default(),
            transfer: TransferView::default(),
            notifications: NotificationsView::default(),
            settings: SettingsView {
                mss: nlpa_core::channel::DEFAULT_STORE_DATA_MSS as u8,
                enable_omapi: true,
                enable_tmapi: true,
                enable_nbridge: true,
                enable_adb_bridge: true,
                enable_qrtr: true,
                enable_se_hal: true,
                enable_ccid: true,
                enable_remote: true,
                ..SettingsView::default()
            },
            modal: ModalState::None,
        }
    }
}

/// Incoming user actions dispatched from the WebView JSON bridge.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UiAction {
    SwitchTab {
        tab: String,
    },
    RefreshReaders,
    SelectReader {
        id: String,
    },
    RefreshProfiles,
    DismissToast,
    CloseModal,
    CancelLoading,
    RetryLoading,
    SetProfileSearch {
        value: String,
    },
    SetProfileSort {
        sort: String,
    },
    RequestSwitchProfile {
        iccid: String,
        enable: bool,
    },
    ConfirmSwitchProfile {
        iccid: String,
        enable: bool,
    },
    OpenRenameProfile {
        iccid: String,
    },
    SubmitRenameProfile {
        iccid: String,
        name: String,
        #[serde(default)]
        tags_csv: String,
    },
    OpenDeleteProfile {
        iccid: String,
    },
    ConfirmDeleteProfile {
        iccid: String,
    },
    OpenEuiccDetails,
    OpenResetEuicc,
    ConfirmResetEuicc,
    CopyText {
        text: String,
    },
    FetchProviderBadge {
        iccid: String,
    },
    OpenUrl {
        url: String,
    },
    OpenEstkModal,
    EstkCommand {
        command: String,
        #[serde(default)]
        arg: String,
    },
    OpenNineEsimModal,
    NineEsimSetPassword {
        device_name: String,
        password: String,
    },
    NineEsimSetAtr {
        atr: String,
        euicc_mode: bool,
    },
    OpenGmPskModal,
    SubmitGmPsk {
        psk: String,
    },
    SetDownloadField {
        field: String,
        value: String,
    },
    PasteLpaClipboard,
    ScanQr,
    ToggleBatchMode {
        enabled: bool,
    },
    StartDownload {
        #[serde(default)]
        combined_lpa: Option<String>,
        #[serde(default)]
        smdp: Option<String>,
        #[serde(default)]
        matching_id: Option<String>,
        #[serde(default)]
        oid: Option<String>,
        #[serde(default)]
        confirmation_code: Option<String>,
    },
    ConfirmDownloadInstall,
    CancelDownload,
    StartBatchDownload {
        #[serde(default)]
        batch_text: Option<String>,
    },
    DiscoverSmds {
        #[serde(default)]
        server: Option<String>,
    },
    SelectDiscoveredEvent {
        activation_code: String,
    },
    ExportBatchCsv,
    StartTransferProbe {
        #[serde(default)]
        source_iccid: Option<String>,
        #[serde(default)]
        manual_server: Option<String>,
    },
    StartTransfer {
        #[serde(default)]
        manual_server: Option<String>,
    },
    SetTransferField {
        field: String,
        value: String,
    },
    SubmitTransferInput {
        accepted: bool,
        #[serde(default)]
        text: String,
    },
    ConfirmTransferDeleteSource,
    CompleteDelayedTransfer {
        #[serde(default)]
        iccid: Option<String>,
    },
    CancelTransfer,
    ToggleNotifHistory {
        show: bool,
    },
    SetNotifHistorySearch {
        value: String,
    },
    SyncNotifications,
    ProcessAllNotifications,
    SendNotification {
        seq: u32,
    },
    RemoveNotification {
        seq: u32,
    },
    ResendHistoryNotification {
        raw_b64: String,
        server: String,
        seq: u32,
    },
    DeleteHistoryEntry {
        index: usize,
    },
    SetAutoNotif {
        #[serde(default)]
        auto_send: Option<bool>,
        #[serde(default)]
        auto_remove: Option<bool>,
    },
    SetThemeStyle {
        style: String,
    },
    SetColorMode {
        mode: String,
    },
    SetLocale {
        locale: String,
    },
    PickWallpaper,
    ClearWallpaper,
    SetWallpaperOpacity {
        opacity: f32,
    },
    ToggleRedactionFlag {
        flag: String,
        value: bool,
    },
    SetRedactionMode {
        field: String,
        value: String,
    },
    ToggleReaderType {
        reader_type: String,
        enabled: bool,
    },
    AdbStartConnect,
    AdbCancelConnect,
    AdbDisconnect,
    AdbCopyCommand,
    AddRemoCardServer {
        name: String,
        base_url: String,
        #[serde(default)]
        token: String,
        #[serde(default)]
        aes_key_hex: String,
    },
    ToggleRemoCardServer {
        id: String,
        enabled: bool,
    },
    DeleteRemoCardServer {
        id: String,
    },
    SetMss {
        mss: u8,
    },
    AddCustomAid {
        aid: String,
    },
    RemoveCustomAid {
        aid: String,
    },
    SetProxyUrl {
        url: String,
    },
    SetDeviceImei {
        imei: String,
    },
    ExportDatabase,
    ImportDatabase,
    OpenAramModal,
    CheckUpdates,
    OpenLogsModal,
    CopyLogs,
    OpenApduConsole,
    SendRawApdu {
        hex: String,
    },
}
