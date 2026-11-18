//! Application state machine, atomic JSON storage, and UI model projection (`AppController`).

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use nlpa_core::activation_code::ActivationCode;
use nlpa_core::channel::OmapiSafetyLatch;
use nlpa_core::crypto::base64_decode;
use nlpa_core::hex::bytes_to_hex;
use nlpa_core::i18n::resolve_locale;
use nlpa_core::lpa::es11::{clean_smds_host, ROOT_SMDS_ADDRESS};
use nlpa_core::lpa::notifications::ProfileOperation;
use nlpa_core::lpa::ts43::{
    parse_websheet_result, ActivationResult, ContinueRequest, EntitlementRecord, Ts43Client,
    Ts43Event, Ts43Result, Ts43State, UserInputResult,
};
use nlpa_core::mcc::{get_flag_emoji, mcc_to_country_code};
use nlpa_core::plugins::{is_gm_eid, match_provider_plugin, EstkCardInfo, ProfileProviderBadge};
use nlpa_core::profile::EuiccProfile;
use nlpa_core::profile_size_predictor::ProfileSizeCatalog;
use nlpa_core::profile_tag::{format_nickname, parse_nickname, parse_tag, ProfileTag};
use nlpa_core::redaction::{
    display_name, filter_tags, hide_flag, hide_icon, operator_summary, profile_flag_emoji,
    redact_eid, redact_iccid, FlagRedactionMode, IconRedactionMode, NicknameRedactionMode,
    OperatorRedactionMode, ProfileView, SizeUnit, TagsRedactionMode,
};
use nlpa_core::remocard::RemoCardServer;
use nlpa_core::size_formatter::format_size;
use nlpa_core::storage::{PersistedNotification, PersistedState};
use nlpa_core::update_feed::{check_update_feed, DEFAULT_UPDATE_FEED_URL};
use nlpa_ui::models::{
    parse_color_mode_code, parse_locale_code, parse_theme_style_code, AramInfoView, BatchItemView,
    DiscoveredSmdsEventView, DownloadMetadataView, DownloadView, EstkInfoView, EuiccInfoView,
    HistoryNotifView, ModalState, NotificationsView, PendingNotifView, ProfileCardView,
    ProfileLoadingState, ProfileSortKind, ProviderBadgeView, ReaderChipView, RemoCardServerView,
    RenderMeta, SettingsView, TabKind, TransferView, UiAction, UiModel, UpdateCandidateView,
};

use crate::host::{HostClient, HostTransport, HttpRequestArgs};
use crate::lpa_ops::{
    self, delete_profile, discover_smds_events, dispatch_notification_es9, download_profile,
    execute_estk_command, fetch_provider_badge, load_card_snapshot, nine_esim_set_atr,
    nine_esim_set_password, remove_notification_safe, rename_profile, reset_euicc_memory,
    send_raw_apdu, switch_profile, validate_device_imei_input, CardSnapshot, DownloadOutcome,
    DownloadPreview, LpaOpError, RetrievedNotification,
};
use crate::readers::{discover_readers, DiscoveredReader, TransportKind};
use crate::ts43_ops::{build_ts43_client, parse_entitlement_database};

const MAX_LOG_ENTRIES: usize = 300;
const MAX_BATCH_DOWNLOAD_ITEMS: usize = 20;

/// Host runtime metadata supplied during `nativeInit`.
#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub files_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub package_name: String,
    pub app_version: String,
    pub version_code: i32,
    pub is_privileged: bool,
    pub system_locale: String,
    pub entitlement_json: String,
    pub reference_sizes_json: String,
}

/// Core stateful application coordinator shared between JNI and host tests.
pub struct AppController {
    host: Arc<dyn HostTransport>,
    config: AppConfig,
    state: PersistedState,
    size_catalog: ProfileSizeCatalog,
    entitlement_records: Vec<EntitlementRecord>,
    omapi_latch: Arc<Mutex<OmapiSafetyLatch>>,
    foreground_card_session_open: bool,
    readers: Vec<DiscoveredReader>,
    selected_reader_id: Option<String>,
    cached_eids: BTreeMap<String, String>,
    active_snapshot: Option<CardSnapshot>,
    provider_badges: BTreeMap<String, ProfileProviderBadge>,
    omapi_euicc_warning: bool,
    omapi_corrupted_banner: Option<String>,
    active_tab: TabKind,
    system_dark: bool,
    profile_search: String,
    profile_sort: ProfileSortKind,
    loading: ProfileLoadingState,
    toast: Option<(String, bool)>,
    modal: ModalState,
    download: DownloadView,
    pending_download_preview: Option<DownloadPreview>,
    transfer: TransferView,
    active_transfer_client: Option<Ts43Client>,
    active_transfer_result: Option<Ts43Result>,
    notif_show_history: bool,
    notif_history_search: String,
    auto_send_notifs: bool,
    auto_remove_notifs: bool,
    notif_notice: Option<String>,
    notif_error: Option<String>,
    logs: Vec<String>,
    apdu_history: Vec<(String, String)>,
    aram_info: Option<AramInfoView>,
    cert_sha1: String,
    update_checking: bool,
    update_candidate: Option<UpdateCandidateView>,
    update_notice: Option<String>,
    adb_connected: bool,
    adb_waiting: bool,
    adb_command: String,
    imei_error: Option<String>,
}

impl AppController {
    /// Initializes controller state from `files_dir/state.json` and runs initial reader discovery.
    #[must_use]
    pub fn new(host: Arc<dyn HostTransport>, config: AppConfig) -> Self {
        let state = load_state_from_disk(&config.files_dir);
        let size_catalog = if config.reference_sizes_json.trim().is_empty() {
            ProfileSizeCatalog::default()
        } else {
            ProfileSizeCatalog::from_json(&config.reference_sizes_json).unwrap_or_default()
        };
        let entitlement_records = parse_entitlement_database(&config.entitlement_json);

        let mut controller = Self {
            host,
            config,
            state,
            size_catalog,
            entitlement_records,
            omapi_latch: Arc::new(Mutex::new(OmapiSafetyLatch::new())),
            foreground_card_session_open: false,
            readers: Vec::new(),
            selected_reader_id: None,
            cached_eids: BTreeMap::new(),
            active_snapshot: None,
            provider_badges: BTreeMap::new(),
            omapi_euicc_warning: false,
            omapi_corrupted_banner: None,
            active_tab: TabKind::Profiles,
            system_dark: true,
            profile_search: String::new(),
            profile_sort: ProfileSortKind::EnabledFirst,
            loading: ProfileLoadingState::default(),
            toast: None,
            modal: ModalState::None,
            download: DownloadView {
                smds_server_input: ROOT_SMDS_ADDRESS.to_string(),
                ..DownloadView::default()
            },
            pending_download_preview: None,
            transfer: TransferView::default(),
            active_transfer_client: None,
            active_transfer_result: None,
            notif_show_history: false,
            notif_history_search: String::new(),
            auto_send_notifs: false,
            auto_remove_notifs: false,
            notif_notice: None,
            notif_error: None,
            logs: Vec::new(),
            apdu_history: Vec::new(),
            aram_info: None,
            cert_sha1: String::new(),
            update_checking: false,
            update_candidate: None,
            update_notice: None,
            adb_connected: false,
            adb_waiting: false,
            adb_command: String::new(),
            imei_error: None,
        };

        controller.load_host_cert_info();
        controller.sync_adb_status();
        controller.refresh_readers_and_snapshot(false);
        controller
    }

    /// Returns an immutable reference to persisted settings.
    #[must_use]
    pub fn persisted_state(&self) -> &PersistedState {
        &self.state
    }

    /// Returns the currently discovered deduplicated readers.
    #[must_use]
    pub fn discovered_readers(&self) -> &[DiscoveredReader] {
        &self.readers
    }

    /// Returns the currently loaded card snapshot, if any.
    #[must_use]
    pub fn active_snapshot(&self) -> Option<&CardSnapshot> {
        self.active_snapshot.as_ref()
    }

    /// Renders the complete initial HTML5 document string for `WebView` startup.
    #[must_use]
    pub fn render_document(&self) -> String {
        let model = self.build_ui_model();
        nlpa_ui::render_document(&model)
    }

    /// Renders the [`RenderMeta`] payload returned to Android/WebView after updates.
    #[must_use]
    pub fn render_meta(&self) -> RenderMeta {
        let model = self.build_ui_model();
        nlpa_ui::render_meta(&model)
    }

    /// Dispatches a JSON-encoded [`UiAction`] and returns the updated [`RenderMeta`].
    pub fn dispatch_json(&mut self, action_json: &str) -> RenderMeta {
        match serde_json::from_str::<UiAction>(action_json) {
            Ok(action) => self.handle_action(action),
            Err(err) => {
                self.push_log(format!("Action parse error: {err}"));
                self.toast = Some((format!("Unsupported action: {err}"), true));
            }
        }
        self.render_meta()
    }

    /// Processes an asynchronous JSON platform event from Android and returns [`RenderMeta`].
    pub fn on_platform_event_json(&mut self, event_json: &str) -> RenderMeta {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(event_json) else {
            self.push_log("Ignored malformed platform event JSON".into());
            return self.render_meta();
        };
        let event_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
        self.push_log(format!("Platform event: {event_type}"));

        match event_type {
            "sim_state_changed" => {
                self.refresh_readers_and_snapshot(true);
            }
            "se_service_connected" => {
                if !self.is_omapi_poisoned() {
                    self.refresh_readers_and_snapshot(true);
                }
            }
            "usb_permission_granted" => {
                self.refresh_readers_and_snapshot(true);
            }
            "adb_bridge_state" => {
                self.adb_connected = val
                    .get("connected")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                self.adb_waiting = false;
                self.refresh_readers_and_snapshot(true);
            }
            "deep_link" | "qr_scanned" => {
                let raw = val
                    .get("url")
                    .or_else(|| val.get("code"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !raw.is_empty() {
                    self.apply_activation_code_text(raw);
                    self.active_tab = TabKind::Download;
                }
            }
            "wallpaper_changed" => {
                let active = val
                    .get("active")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                self.state.wallpaper_enabled = active;
                self.save_state_best_effort();
            }
            "state_imported" => {
                if let Some(json_str) = val.get("json").and_then(|v| v.as_str()) {
                    match PersistedState::from_json_str(json_str) {
                        Ok(imported) => {
                            self.state = imported;
                            self.save_state_best_effort();
                            self.toast = Some(("Imported settings cleanly".into(), false));
                            self.refresh_readers_and_snapshot(true);
                        }
                        Err(e) => {
                            self.toast = Some((format!("Invalid state backup JSON: {e}"), true));
                        }
                    }
                }
            }
            "websheet_result" => {
                let dismissed = val
                    .get("dismissed")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let raw = val.get("raw").and_then(|v| v.as_str()).unwrap_or("");
                self.handle_websheet_event(raw, dismissed);
            }
            _ => {}
        }

        self.render_meta()
    }

    /// Executes a typed [`UiAction`] against the controller.
    pub fn handle_action(&mut self, action: UiAction) {
        match action {
            UiAction::SwitchTab { tab } => {
                self.active_tab = TabKind::parse(&tab);
            }
            UiAction::RefreshReaders => {
                self.refresh_readers_and_snapshot(true);
            }
            UiAction::SelectReader { id } => {
                self.selected_reader_id = Some(id);
                self.load_active_reader_snapshot();
            }
            UiAction::RefreshProfiles => {
                self.load_active_reader_snapshot();
            }
            UiAction::DismissToast => {
                self.toast = None;
            }
            UiAction::CloseModal => {
                self.modal = ModalState::None;
            }
            UiAction::CancelLoading => {
                self.loading.active = false;
                self.foreground_card_session_open = false;
            }
            UiAction::RetryLoading => {
                self.loading.active = false;
                self.load_active_reader_snapshot();
            }
            UiAction::SetProfileSearch { value } => {
                self.profile_search = value;
            }
            UiAction::SetProfileSort { sort } => {
                self.profile_sort = ProfileSortKind::parse(&sort);
            }
            UiAction::RequestSwitchProfile { iccid, enable } => {
                let is_se_hal = self
                    .selected_reader()
                    .is_some_and(|r| r.kind == TransportKind::SeHal);
                let display_name = self.display_name_for_iccid(&iccid);
                self.modal = ModalState::ConfirmSwitchProfile {
                    iccid,
                    display_name,
                    enable,
                    is_se_hal,
                };
            }
            UiAction::ConfirmSwitchProfile { iccid, enable } => {
                self.modal = ModalState::None;
                self.execute_profile_switch(&iccid, enable);
            }
            UiAction::OpenRenameProfile { iccid } => {
                let (nick, tags) = self.current_nickname_and_tags(&iccid);
                self.modal = ModalState::RenameProfile {
                    iccid,
                    current_name: nick,
                    tags_csv: tags,
                };
            }
            UiAction::SubmitRenameProfile {
                iccid,
                name,
                tags_csv,
            } => {
                self.modal = ModalState::None;
                self.execute_rename_profile(&iccid, &name, &tags_csv);
            }
            UiAction::OpenDeleteProfile { iccid } => {
                let display_name = self.display_name_for_iccid(&iccid);
                self.modal = ModalState::ConfirmDeleteProfile {
                    iccid,
                    display_name,
                };
            }
            UiAction::ConfirmDeleteProfile { iccid } => {
                self.modal = ModalState::None;
                self.execute_delete_profile(&iccid);
            }
            UiAction::OpenEuiccDetails => {
                self.modal = ModalState::EuiccDetails;
            }
            UiAction::OpenResetEuicc => {
                self.modal = ModalState::ConfirmEuiccReset;
            }
            UiAction::ConfirmResetEuicc => {
                self.modal = ModalState::None;
                self.execute_reset_euicc();
            }
            UiAction::CopyText { text } => {
                let client = HostClient::new(self.host.as_ref());
                let _ = client.ui_copy_clipboard(&text);
                self.toast = Some(("Copied to clipboard".into(), false));
            }
            UiAction::FetchProviderBadge { iccid } => {
                if let Some(badge) = fetch_provider_badge(
                    self.host.as_ref(),
                    self.state.proxy_url.as_deref(),
                    &iccid,
                ) {
                    self.provider_badges.insert(iccid, badge);
                } else {
                    self.toast = Some(("Provider status unavailable".into(), true));
                }
            }
            UiAction::OpenUrl { url } => {
                let client = HostClient::new(self.host.as_ref());
                let _ = client.ui_open_url(&url);
            }
            UiAction::OpenEstkModal => {
                let atr_hex = self
                    .selected_reader()
                    .and_then(|r| r.atr_hex.clone())
                    .unwrap_or_default();
                self.modal = ModalState::EstkManagement { atr_hex };
            }
            UiAction::EstkCommand { command, arg } => {
                self.execute_estk(&command, &arg);
            }
            UiAction::OpenNineEsimModal => {
                let atr = self
                    .selected_reader()
                    .and_then(|r| r.atr_hex.clone())
                    .unwrap_or_else(|| nlpa_core::ccid::nine_esim::DEFAULT_9ESIM_ATR.to_string());
                let dev = self
                    .selected_reader()
                    .map(|r| {
                        let clean: String =
                            r.label.chars().filter(char::is_ascii_hexdigit).collect();
                        clean
                            .get(clean.len().saturating_sub(6)..)
                            .unwrap_or("")
                            .to_ascii_uppercase()
                    })
                    .unwrap_or_default();
                let euicc_mode =
                    nlpa_core::ccid::nine_esim::euicc_capability_enabled(&atr).unwrap_or(true);
                self.modal = ModalState::NineEsimManagement {
                    device_name: dev,
                    atr,
                    euicc_mode,
                };
            }
            UiAction::NineEsimSetPassword {
                device_name,
                password,
            } => {
                self.execute_9esim_password(&device_name, &password);
            }
            UiAction::NineEsimSetAtr { atr, euicc_mode } => {
                self.execute_9esim_atr(&atr, euicc_mode);
            }
            UiAction::OpenGmPskModal => {
                let eid = self
                    .active_snapshot
                    .as_ref()
                    .map(|s| s.eid.clone())
                    .unwrap_or_default();
                self.modal = ModalState::GmPskPrompt { eid };
            }
            UiAction::SubmitGmPsk { psk } => {
                if let Some(eid) = self.active_snapshot.as_ref().map(|s| s.eid.clone()) {
                    self.state.gm_psk_cache.insert(eid, psk.trim().to_string());
                    self.save_state_best_effort();
                }
                self.modal = ModalState::None;
                self.load_active_reader_snapshot();
            }
            UiAction::SetDownloadField { field, value } => {
                self.set_download_field(&field, &value);
            }
            UiAction::PasteLpaClipboard => {
                let client = HostClient::new(self.host.as_ref());
                match client.ui_read_clipboard() {
                    Ok(txt) if !txt.trim().is_empty() => {
                        self.apply_activation_code_text(&txt);
                    }
                    _ => self.toast = Some(("Clipboard empty".into(), true)),
                }
            }
            UiAction::ScanQr => {
                let client = HostClient::new(self.host.as_ref());
                if let Err(e) = client.ui_scan_qr() {
                    self.toast = Some((e.to_string(), true));
                }
            }
            UiAction::ToggleBatchMode { enabled } => {
                self.download.batch_mode = enabled;
            }
            UiAction::StartDownload {
                combined_lpa,
                smdp,
                matching_id,
                oid,
                confirmation_code,
            } => {
                self.sync_download_overrides(
                    combined_lpa,
                    smdp,
                    matching_id,
                    oid,
                    confirmation_code,
                );
                let is_se_hal = self
                    .selected_reader()
                    .is_some_and(|r| r.kind == TransportKind::SeHal);
                self.execute_single_download(!is_se_hal);
            }
            UiAction::ConfirmDownloadInstall => {
                self.download.se_hal_preview_confirmed = true;
                self.execute_single_download(true);
            }
            UiAction::CancelDownload => {
                self.download.cancel_requested = true;
                self.download.downloading = false;
                self.download.step_label = Some("Cancelled".into());
            }
            UiAction::StartBatchDownload { batch_text } => {
                if let Some(txt) = batch_text {
                    self.download.batch_text = txt;
                }
                self.execute_batch_download();
            }
            UiAction::DiscoverSmds { server } => {
                if let Some(srv) = server {
                    self.download.smds_server_input = srv;
                }
                self.execute_smds_discovery();
            }
            UiAction::SelectDiscoveredEvent { activation_code } => {
                self.apply_activation_code_text(&activation_code);
                self.download.batch_mode = false;
                self.toast = Some(("Activation code loaded".into(), false));
            }
            UiAction::ExportBatchCsv => {
                let csv = self.format_batch_csv();
                self.modal = ModalState::BatchExport { csv_text: csv };
            }
            UiAction::StartTransferProbe {
                source_iccid,
                manual_server,
            } => {
                if let Some(srv) = manual_server {
                    self.transfer.manual_entitlement_server = srv;
                }
                self.probe_ts43_eligibility(source_iccid);
            }
            UiAction::StartTransfer { manual_server } => {
                if let Some(srv) = manual_server {
                    self.transfer.manual_entitlement_server = srv;
                }
                self.start_ts43_transfer();
            }
            UiAction::SetTransferField { field, value } => match field.as_str() {
                "manual_server" => self.transfer.manual_entitlement_server = value,
                "delayed_iccid" => self.transfer.delayed_iccid_input = value,
                _ => {}
            },
            UiAction::SubmitTransferInput { accepted, text } => {
                self.continue_ts43_user_input(accepted, text);
            }
            UiAction::ConfirmTransferDeleteSource => {
                self.continue_ts43_delete_source();
            }
            UiAction::CompleteDelayedTransfer { iccid } => {
                if let Some(v) = iccid {
                    self.transfer.delayed_iccid_input = v;
                }
                self.complete_ts43_activation(self.transfer.delayed_iccid_input.clone());
            }
            UiAction::CancelTransfer => {
                self.active_transfer_client = None;
                self.active_transfer_result = None;
                self.transfer.is_busy = false;
                self.transfer.step_message = "Cancelled".into();
            }
            UiAction::ToggleNotifHistory { show } => {
                self.notif_show_history = show;
            }
            UiAction::SetNotifHistorySearch { value } => {
                self.notif_history_search = value;
            }
            UiAction::SyncNotifications => {
                self.load_active_reader_snapshot();
            }
            UiAction::ProcessAllNotifications => {
                self.execute_process_all_notifications();
            }
            UiAction::SendNotification { seq } => {
                self.execute_send_pending_notification(seq);
            }
            UiAction::RemoveNotification { seq } => {
                self.execute_remove_pending_notification(seq);
            }
            UiAction::ResendHistoryNotification {
                raw_b64,
                server,
                seq,
            } => {
                self.execute_resend_history(seq, &server, &raw_b64);
            }
            UiAction::DeleteHistoryEntry { index } => {
                if index < self.state.notifications_history.len() {
                    self.state.notifications_history.remove(index);
                    self.save_state_best_effort();
                }
            }
            UiAction::SetAutoNotif {
                auto_send,
                auto_remove,
            } => {
                if let Some(v) = auto_send {
                    self.auto_send_notifs = v;
                }
                if let Some(v) = auto_remove {
                    self.auto_remove_notifs = v;
                }
            }
            UiAction::SetThemeStyle { style } => {
                self.state.theme_style = parse_theme_style_code(&style);
                self.save_state_best_effort();
            }
            UiAction::SetColorMode { mode } => {
                self.state.color_mode = parse_color_mode_code(&mode);
                self.save_state_best_effort();
            }
            UiAction::SetLocale { locale } => {
                self.state.locale = parse_locale_code(&locale);
                self.save_state_best_effort();
            }
            UiAction::PickWallpaper => {
                let client = HostClient::new(self.host.as_ref());
                if let Err(e) = client.ui_pick_wallpaper() {
                    self.toast = Some((e.to_string(), true));
                }
            }
            UiAction::ClearWallpaper => {
                let client = HostClient::new(self.host.as_ref());
                let _ = client.ui_clear_wallpaper();
                self.state.wallpaper_enabled = false;
                self.save_state_best_effort();
            }
            UiAction::SetWallpaperOpacity { opacity } => {
                if opacity.is_finite() {
                    self.state.wallpaper_opacity = opacity.clamp(0.05, 0.95);
                    self.state.normalize();
                    self.save_state_best_effort();
                }
            }
            UiAction::ToggleRedactionFlag { flag, value } => {
                self.update_redaction_flag(&flag, value);
                self.save_state_best_effort();
            }
            UiAction::SetRedactionMode { field, value } => {
                self.update_redaction_mode(&field, &value);
                self.save_state_best_effort();
            }
            UiAction::ToggleReaderType {
                reader_type,
                enabled,
            } => {
                self.update_reader_toggle(&reader_type, enabled);
                self.save_state_best_effort();
                self.refresh_readers_and_snapshot(false);
            }
            UiAction::AdbStartConnect => {
                let client = HostClient::new(self.host.as_ref());
                self.sync_adb_status();
                match client.adb_begin_connect(60_000) {
                    Ok(()) => {
                        self.adb_waiting = true;
                        self.toast = Some(("Waiting for ADB bridge...".into(), false));
                    }
                    Err(e) => {
                        self.toast = Some((e.to_string(), true));
                    }
                }
            }
            UiAction::AdbCancelConnect => {
                let client = HostClient::new(self.host.as_ref());
                let _ = client.adb_cancel_connect();
                self.adb_waiting = false;
            }
            UiAction::AdbDisconnect => {
                let client = HostClient::new(self.host.as_ref());
                let _ = client.adb_disconnect();
                self.adb_connected = false;
                self.adb_waiting = false;
                self.refresh_readers_and_snapshot(false);
            }
            UiAction::AdbCopyCommand => {
                self.sync_adb_status();
                let client = HostClient::new(self.host.as_ref());
                let _ = client.ui_copy_clipboard(&self.adb_command);
                self.toast = Some(("ADB command copied".into(), false));
            }
            UiAction::AddRemoCardServer {
                name,
                base_url,
                token,
                aes_key_hex,
            } => {
                if base_url.trim().is_empty() {
                    self.toast = Some(("Server URL is required".into(), true));
                    return;
                }
                let id = format!("rc-{}", self.state.remocard_servers.len() + 1);
                self.state.remocard_servers.push(RemoCardServer {
                    id,
                    name: if name.trim().is_empty() {
                        base_url.trim().to_string()
                    } else {
                        name.trim().to_string()
                    },
                    base_url: base_url.trim().to_string(),
                    token: (!token.trim().is_empty()).then(|| token.trim().to_string()),
                    aes_key_hex: (!aes_key_hex.trim().is_empty())
                        .then(|| aes_key_hex.trim().to_string()),
                    enabled: true,
                });
                self.save_state_best_effort();
                self.refresh_readers_and_snapshot(false);
            }
            UiAction::ToggleRemoCardServer { id, enabled } => {
                if let Some(srv) = self.state.remocard_servers.iter_mut().find(|s| s.id == id) {
                    srv.enabled = enabled;
                }
                self.save_state_best_effort();
                self.refresh_readers_and_snapshot(false);
            }
            UiAction::DeleteRemoCardServer { id } => {
                self.state.remocard_servers.retain(|s| s.id != id);
                self.save_state_best_effort();
                self.refresh_readers_and_snapshot(false);
            }
            UiAction::SetMss { mss } => {
                self.state.mss = mss.clamp(60, 240);
                self.save_state_best_effort();
                self.toast = Some((format!("Saved MSS {}", self.state.mss), false));
            }
            UiAction::AddCustomAid { aid } => {
                let clean: String = aid
                    .chars()
                    .filter(char::is_ascii_hexdigit)
                    .collect::<String>()
                    .to_ascii_uppercase();
                if clean.len() >= 10 && !self.state.custom_aids.contains(&clean) {
                    self.state.custom_aids.push(clean);
                    self.save_state_best_effort();
                } else {
                    self.toast = Some(("Enter valid hex AID (min 5 bytes)".into(), true));
                }
            }
            UiAction::RemoveCustomAid { aid } => {
                self.state.custom_aids.retain(|a| a != &aid);
                self.save_state_best_effort();
            }
            UiAction::SetProxyUrl { url } => {
                let trimmed = url.trim();
                self.state.proxy_url = (!trimmed.is_empty()).then(|| trimmed.to_string());
                self.save_state_best_effort();
                self.toast = Some(("Proxy updated".into(), false));
            }
            UiAction::SetDeviceImei { imei } => match validate_device_imei_input(&imei) {
                Ok(valid) => {
                    self.imei_error = None;
                    self.state.device_imei = valid;
                    self.save_state_best_effort();
                    self.toast = Some(("Device TAC/IMEI saved".into(), false));
                }
                Err(err) => {
                    self.imei_error = Some(err);
                }
            },
            UiAction::ExportDatabase => {
                if let Ok(json) = self.state.to_json_string() {
                    let client = HostClient::new(self.host.as_ref());
                    if let Err(e) = client.ui_export_state(&json) {
                        self.toast = Some((e.to_string(), true));
                    }
                }
            }
            UiAction::ImportDatabase => {
                let client = HostClient::new(self.host.as_ref());
                if let Err(e) = client.ui_import_state() {
                    self.toast = Some((e.to_string(), true));
                }
            }
            UiAction::OpenAramModal => {
                self.load_host_cert_info();
                self.modal = ModalState::AramInfo;
            }
            UiAction::CheckUpdates => {
                self.execute_check_updates();
            }
            UiAction::OpenLogsModal => {
                self.modal = ModalState::Logs;
            }
            UiAction::CopyLogs => {
                let text = self.logs.join("\n");
                let client = HostClient::new(self.host.as_ref());
                let _ = client.ui_copy_clipboard(&text);
                self.toast = Some(("Logs copied".into(), false));
            }
            UiAction::OpenApduConsole => {
                self.modal = ModalState::ApduConsole;
            }
            UiAction::SendRawApdu { hex } => {
                self.execute_raw_apdu(&hex);
            }
        }
    }

    fn selected_reader(&self) -> Option<&DiscoveredReader> {
        let id = self.selected_reader_id.as_deref()?;
        self.readers.iter().find(|r| r.id == id)
    }

    fn is_omapi_poisoned(&self) -> bool {
        self.omapi_latch.lock().is_ok_and(|g| g.is_poisoned())
    }

    fn push_log(&mut self, msg: String) {
        if self.logs.len() >= MAX_LOG_ENTRIES {
            self.logs.remove(0);
        }
        self.logs.push(msg);
    }

    fn append_apdu_logs(&mut self, entries: Vec<(String, String)>) {
        self.apdu_history.extend(entries);
        if self.apdu_history.len() > 200 {
            let drain = self.apdu_history.len() - 200;
            self.apdu_history.drain(..drain);
        }
    }

    fn save_state_best_effort(&mut self) {
        if let Err(e) = save_state_atomic(&self.config.files_dir, &self.state) {
            self.push_log(format!("State save error: {e}"));
        }
    }

    fn load_host_cert_info(&mut self) {
        let client = HostClient::new(self.host.as_ref());
        if let Ok(info) = client.cert_get_info() {
            if let Some(first_sha1) = info.sha1.first() {
                self.cert_sha1 = first_sha1.clone();
            }
            self.aram_info = Some(AramInfoView {
                sha256_hashes: info.sha256,
                sha1_hashes: info.sha1,
                aram_sha256: info.aram_sha256,
                aram_sha1: info.aram_sha1,
                abis: info.abis,
            });
        }
    }

    fn sync_adb_status(&mut self) {
        let client = HostClient::new(self.host.as_ref());
        self.adb_connected = client.adb_is_connected().unwrap_or(false);
        if let Ok(cmd) = client.adb_get_connect_command() {
            if !cmd.is_empty() {
                self.adb_command = cmd;
            }
        }
    }

    fn refresh_readers_and_snapshot(&mut self, reload_snapshot: bool) {
        let report = discover_readers(
            Arc::clone(&self.host),
            &self.state,
            &self.omapi_latch,
            self.foreground_card_session_open,
            &self.cached_eids,
        );
        self.omapi_euicc_warning = report.omapi_euicc_warning;
        if let Some(banner) = report.omapi_corrupted_banner {
            self.omapi_corrupted_banner = Some(banner);
        }
        for line in report.logs {
            self.push_log(line);
        }
        for r in &report.readers {
            if let Some(ref eid) = r.eid {
                self.cached_eids.insert(r.id.clone(), eid.clone());
            }
        }
        self.readers = report.readers;
        let still_present = self
            .selected_reader_id
            .as_deref()
            .is_some_and(|id| self.readers.iter().any(|r| r.id == id));
        if !still_present {
            self.selected_reader_id = self.readers.first().map(|r| r.id.clone());
        }
        if reload_snapshot || self.active_snapshot.is_none() {
            if self.selected_reader_id.is_some() && !self.foreground_card_session_open {
                self.load_active_reader_snapshot();
            } else if self.selected_reader_id.is_none() {
                self.active_snapshot = None;
            }
        }
    }

    fn load_active_reader_snapshot(&mut self) {
        let Some(reader) = self.selected_reader().cloned() else {
            self.active_snapshot = None;
            return;
        };
        if reader.needs_permission {
            if reader.kind == TransportKind::Ccid {
                let client = HostClient::new(self.host.as_ref());
                let _ = client.ccid_request_permission(&reader.raw_name);
            }
            return;
        }
        self.foreground_card_session_open = true;
        let res = load_card_snapshot(
            Arc::clone(&self.host),
            &reader,
            &self.state,
            &self.omapi_latch,
        );
        self.foreground_card_session_open = false;

        match res {
            Ok(mut snap) => {
                if !snap.eid.is_empty() {
                    self.cached_eids.insert(reader.id.clone(), snap.eid.clone());
                }
                self.append_apdu_logs(core::mem::take(&mut snap.apdu_log));
                self.backup_notifications_to_history(&snap.eid, &snap.notifications);
                self.active_snapshot = Some(snap);
                self.loading.active = false;
                self.loading.error = None;
            }
            Err(LpaOpError::Reader(crate::readers::ReaderError::OmapiCorrupted(msg))) => {
                self.omapi_corrupted_banner = Some(msg.clone());
                self.toast = Some((msg, true));
                self.loading.active = false;
            }
            Err(e) => {
                self.push_log(format!("Load snapshot failed: {e}"));
                self.toast = Some((e.to_string(), true));
                self.loading.active = false;
            }
        }
    }

    fn backup_notifications_to_history(&mut self, eid: &str, pending: &[RetrievedNotification]) {
        if pending.is_empty() {
            return;
        }
        let now_ms = current_time_ms();
        let mut changed = false;
        for item in pending {
            let exists = self.state.notifications_history.iter().any(|h| {
                h.seq_number == item.metadata.seq_number && (h.eid == eid || h.eid.is_empty())
            });
            if !exists {
                self.state
                    .notifications_history
                    .push(PersistedNotification {
                        eid: eid.to_string(),
                        iccid: item.metadata.iccid.clone().unwrap_or_default(),
                        seq_number: item.metadata.seq_number,
                        operation: operation_label(item.metadata.profile_management_operation)
                            .into(),
                        address: item.metadata.notification_address.clone(),
                        content_base64: item.raw_notification_b64.clone(),
                        status: "pending".into(),
                        timestamp_ms: now_ms,
                        response_code: None,
                        response_content: None,
                    });
                changed = true;
            }
        }
        if changed {
            self.save_state_best_effort();
        }
    }

    fn execute_profile_switch(&mut self, iccid: &str, enable: bool) {
        let Some(reader) = self.selected_reader().cloned() else {
            return;
        };
        let target_iccid = self.resolve_raw_iccid(iccid);
        let expected_eid = self.active_snapshot.as_ref().map(|s| s.eid.clone());
        self.foreground_card_session_open = true;
        let res = switch_profile(
            Arc::clone(&self.host),
            &reader,
            &self.state,
            &self.omapi_latch,
            &target_iccid,
            enable,
            expected_eid.as_deref(),
        );
        self.foreground_card_session_open = false;

        match res {
            Ok((logs, updated_profiles)) => {
                self.append_apdu_logs(logs);
                if let Some(snap) = self.active_snapshot.as_mut() {
                    snap.profiles = updated_profiles;
                }
                self.toast = Some((
                    if enable {
                        "Profile enabled".into()
                    } else {
                        "Profile disabled".into()
                    },
                    false,
                ));
            }
            Err(LpaOpError::Reader(crate::readers::ReaderError::OmapiCorrupted(msg))) => {
                self.omapi_corrupted_banner = Some(msg.clone());
                self.toast = Some((msg, true));
            }
            Err(e) => {
                self.toast = Some((e.to_string(), true));
            }
        }
    }

    fn execute_rename_profile(&mut self, iccid: &str, name: &str, tags_csv: &str) {
        let Some(reader) = self.selected_reader().cloned() else {
            return;
        };
        let target_iccid = self.resolve_raw_iccid(iccid);
        let eid = self.active_snapshot.as_ref().map(|s| s.eid.clone());
        let tags: Vec<ProfileTag> = tags_csv
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|raw| {
                if raw.starts_with("t:") || raw.starts_with("d:") || raw.starts_with("D:") {
                    parse_tag(raw)
                } else {
                    ProfileTag::Text(raw.to_string())
                }
            })
            .collect();
        let encoded = format_nickname(name.trim(), &tags);

        match rename_profile(
            Arc::clone(&self.host),
            &reader,
            &self.state,
            &self.omapi_latch,
            &target_iccid,
            &encoded,
            eid.as_deref(),
        ) {
            Ok(logs) => {
                self.append_apdu_logs(logs);
                self.state
                    .profile_nicknames_cache
                    .insert(iccid.to_string(), encoded.clone());
                self.state
                    .profile_tags
                    .insert(iccid.to_string(), tags_csv.trim().to_string());
                self.save_state_best_effort();
                if let Some(snap) = self.active_snapshot.as_mut() {
                    if let Some(prof) = snap.profiles.iter_mut().find(|p| p.iccid == iccid) {
                        prof.nickname = Some(encoded);
                    }
                }
                self.toast = Some(("Profile renamed".into(), false));
            }
            Err(e) => {
                self.toast = Some((e.to_string(), true));
            }
        }
    }

    fn execute_delete_profile(&mut self, iccid: &str) {
        let Some(reader) = self.selected_reader().cloned() else {
            return;
        };
        let target_iccid = self.resolve_raw_iccid(iccid);
        let eid = self.active_snapshot.as_ref().map(|s| s.eid.clone());
        match delete_profile(
            Arc::clone(&self.host),
            &reader,
            &self.state,
            &self.omapi_latch,
            &target_iccid,
            eid.as_deref(),
        ) {
            Ok(logs) => {
                self.append_apdu_logs(logs);
                self.load_active_reader_snapshot();
                self.toast = Some(("Profile deleted".into(), false));
            }
            Err(e) => {
                self.toast = Some((e.to_string(), true));
            }
        }
    }

    fn execute_reset_euicc(&mut self) {
        let Some(reader) = self.selected_reader().cloned() else {
            return;
        };
        match reset_euicc_memory(
            Arc::clone(&self.host),
            &reader,
            &self.state,
            &self.omapi_latch,
        ) {
            Ok(logs) => {
                self.append_apdu_logs(logs);
                self.load_active_reader_snapshot();
                self.toast = Some(("eUICC memory reset completed".into(), false));
            }
            Err(e) => {
                self.toast = Some((e.to_string(), true));
            }
        }
    }

    fn execute_estk(&mut self, command: &str, arg: &str) {
        let Some(reader) = self.selected_reader().cloned() else {
            return;
        };
        match execute_estk_command(
            Arc::clone(&self.host),
            &reader,
            &self.state,
            &self.omapi_latch,
            command,
            arg,
        ) {
            Ok(Some(info)) => {
                if let Some(snap) = self.active_snapshot.as_mut() {
                    snap.estk_info = Some(info);
                }
            }
            Ok(None) => {
                self.toast = Some(("eSTK slot switched".into(), false));
                self.modal = ModalState::None;
                self.load_active_reader_snapshot();
            }
            Err(e) => {
                self.toast = Some((e.to_string(), true));
            }
        }
    }

    fn execute_9esim_password(&mut self, device_name: &str, password: &str) {
        let Some(reader) = self.selected_reader().cloned() else {
            return;
        };
        match nine_esim_set_password(self.host.as_ref(), &reader.raw_name, device_name, password) {
            Ok(()) => self.toast = Some(("9eSIM password updated".into(), false)),
            Err(e) => self.toast = Some((e.to_string(), true)),
        }
    }

    fn execute_9esim_atr(&mut self, atr: &str, euicc_mode: bool) {
        let Some(reader) = self.selected_reader().cloned() else {
            return;
        };
        match nine_esim_set_atr(self.host.as_ref(), &reader.raw_name, atr, euicc_mode) {
            Ok(updated) => {
                self.modal = ModalState::NineEsimManagement {
                    device_name: String::new(),
                    atr: updated,
                    euicc_mode,
                };
                self.toast = Some(("9eSIM ATR updated".into(), false));
            }
            Err(e) => self.toast = Some((e.to_string(), true)),
        }
    }

    fn execute_raw_apdu(&mut self, hex: &str) {
        let Some(reader) = self.selected_reader().cloned() else {
            return;
        };
        match send_raw_apdu(
            Arc::clone(&self.host),
            &reader,
            &self.state,
            &self.omapi_latch,
            hex,
        ) {
            Ok(rx) => {
                self.apdu_history
                    .push((hex.trim().to_ascii_uppercase(), rx));
            }
            Err(e) => {
                self.apdu_history
                    .push((hex.trim().to_ascii_uppercase(), format!("ERROR: {e}")));
            }
        }
    }

    fn set_download_field(&mut self, field: &str, value: &str) {
        match field {
            "combined_lpa" => self.apply_activation_code_text(value),
            "smdp" => self.download.smdp_input = value.to_string(),
            "matching_id" => self.download.matching_id_input = value.to_string(),
            "oid" => self.download.oid_input = value.to_string(),
            "confirmation_code" => self.download.confirmation_code_input = value.to_string(),
            "batch_text" => self.download.batch_text = value.to_string(),
            "smds_server" => self.download.smds_server_input = value.to_string(),
            _ => {}
        }
    }

    fn sync_download_overrides(
        &mut self,
        combined_lpa: Option<String>,
        smdp: Option<String>,
        matching_id: Option<String>,
        oid: Option<String>,
        confirmation_code: Option<String>,
    ) {
        if let Some(c) = combined_lpa {
            if c.trim() != self.download.combined_lpa_input.trim() && !c.trim().is_empty() {
                self.apply_activation_code_text(&c);
            } else {
                self.download.combined_lpa_input = c;
            }
        }
        if let Some(v) = smdp {
            if !v.trim().is_empty() {
                self.download.smdp_input = v.trim().to_string();
            }
        }
        if let Some(v) = matching_id {
            self.download.matching_id_input = v.trim().to_string();
        }
        if let Some(v) = oid {
            self.download.oid_input = v.trim().to_string();
        }
        if let Some(v) = confirmation_code {
            self.download.confirmation_code_input = v;
        }
    }

    fn apply_activation_code_text(&mut self, raw: &str) {
        self.download.combined_lpa_input = raw.trim().to_string();
        if let Some(parsed) = ActivationCode::extract_from_text(raw) {
            self.download.smdp_input = parsed.smdp_address;
            self.download.matching_id_input = parsed.matching_id;
            self.download.oid_input = parsed.smdp_oid.unwrap_or_default();
            if let Some(cc) = parsed.confirmation_code {
                self.download.confirmation_code_input = cc;
            }
            self.download.confirmation_required = parsed.confirmation_code_required;
            self.download.error = None;
        }
    }

    fn execute_single_download(&mut self, confirm_install: bool) {
        let Some(reader) = self.selected_reader().cloned() else {
            self.download.error = Some("No reader selected".into());
            return;
        };
        let smdp = if !self.download.smdp_input.trim().is_empty() {
            self.download.smdp_input.trim().to_string()
        } else if let Some(parsed) =
            ActivationCode::extract_from_text(&self.download.combined_lpa_input)
        {
            parsed.smdp_address
        } else {
            self.download.error =
                Some("Enter a valid SM-DP+ address or LPA activation code".into());
            return;
        };

        let matching = (!self.download.matching_id_input.trim().is_empty())
            .then(|| self.download.matching_id_input.trim().to_string());
        let cc = (!self.download.confirmation_code_input.trim().is_empty())
            .then(|| self.download.confirmation_code_input.trim().to_string());

        self.download.downloading = true;
        self.download.error = None;
        self.download.success_message = None;
        self.foreground_card_session_open = true;

        let res = download_profile(
            Arc::clone(&self.host),
            &reader,
            &self.state,
            &self.omapi_latch,
            &smdp,
            matching.as_deref(),
            cc.as_deref(),
            &self.size_catalog,
            confirm_install,
        );
        self.foreground_card_session_open = false;
        self.download.downloading = false;

        match res {
            Ok(DownloadOutcome::PreviewOnly { preview, apdu_log }) => {
                self.append_apdu_logs(apdu_log);
                let est_text = format_size(preview.estimated_bytes, 1, SizeUnit::AdaptiveSi);
                let insufficient = preview
                    .estimated_bytes
                    .zip(preview.free_nvram_bytes)
                    .is_some_and(|(est, free)| est > 0 && (est as u64) > free);
                self.download.metadata = Some(DownloadMetadataView {
                    spn: preview.spn.clone(),
                    profile_name: preview.profile_name.clone(),
                    iccid: preview.iccid.clone(),
                    profile_class: preview.profile_class,
                    estimated_bytes: preview.estimated_bytes,
                    estimated_text: est_text,
                    ci_trusted: preview.ci_trusted,
                    ci_summary: preview.ci_summary.clone(),
                    requires_confirmation_code: preview.requires_confirmation_code,
                    insufficient_storage: insufficient,
                });
                self.pending_download_preview = Some(preview);
                self.download.se_hal_preview_confirmed = false;
                self.download.step_label = Some("Preview ready".into());
                self.download.progress_pct = Some(50);
            }
            Ok(DownloadOutcome::Installed {
                installed_iccid,
                apdu_log,
            }) => {
                self.append_apdu_logs(apdu_log);
                self.download.progress_pct = Some(100);
                self.download.step_label = Some("Installed".into());
                self.download.success_message = Some(match installed_iccid {
                    Some(iccid) => format!("Profile installed ({iccid})"),
                    None => "Profile installed successfully".into(),
                });
                self.load_active_reader_snapshot();
            }
            Err(e) => {
                self.download.error = Some(e.to_string());
                self.download.step_label = Some("Failed".into());
            }
        }
    }

    fn execute_batch_download(&mut self) {
        let Some(reader) = self.selected_reader().cloned() else {
            self.download.error = Some("No reader selected".into());
            return;
        };
        let lines: Vec<String> = self
            .download
            .batch_text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .take(MAX_BATCH_DOWNLOAD_ITEMS)
            .map(ToString::to_string)
            .collect();

        if lines.is_empty() {
            self.download.error = Some("Paste up to 20 LPA codes first".into());
            return;
        }

        self.download.batch_running = true;
        self.download.error = None;
        self.download.batch_items.clear();

        let mut remaining_lines = Vec::new();
        let mut installed_count = 0_usize;
        let mut halted = false;

        for (idx, line) in lines.iter().enumerate() {
            if halted {
                remaining_lines.push(line.clone());
                self.download.batch_items.push(BatchItemView {
                    index: idx,
                    raw_code: line.clone(),
                    smdp: String::new(),
                    matching_id: String::new(),
                    status: "Skipped".into(),
                    error: Some("Stopped earlier in batch".into()),
                    ..BatchItemView::default()
                });
                continue;
            }
            let Ok(parsed) = ActivationCode::parse(line) else {
                remaining_lines.push(line.clone());
                self.download.batch_items.push(BatchItemView {
                    index: idx,
                    raw_code: line.clone(),
                    status: "Invalid".into(),
                    error: Some("Malformed LPA code".into()),
                    ..BatchItemView::default()
                });
                continue;
            };

            let free_nvram = self.active_snapshot.as_ref().map(|s| {
                s.nxp_memory
                    .as_ref()
                    .and_then(|n| n.free_non_volatile_memory)
                    .unwrap_or(u64::from(s.info2.free_nvram))
            });
            let pred = self.size_catalog.predict_size(
                self.active_snapshot.as_ref().map(|s| s.eid.as_str()),
                Some(&parsed.smdp_address),
                None,
                None,
            );
            if let (Some(est), Some(free)) = (pred, free_nvram) {
                if est > 0 && (est as u64) > free {
                    halted = true;
                    remaining_lines.push(line.clone());
                    self.download.batch_items.push(BatchItemView {
                        index: idx,
                        raw_code: line.clone(),
                        smdp: parsed.smdp_address,
                        matching_id: parsed.matching_id,
                        status: "Insufficient Space".into(),
                        error: Some(format!("Needs ~{} bytes, {} free", est, free)),
                        size_text: Some(format_size(Some(est), 1, SizeUnit::AdaptiveSi)),
                        ..BatchItemView::default()
                    });
                    continue;
                }
            }

            let matching_opt =
                (!parsed.matching_id.is_empty()).then_some(parsed.matching_id.as_str());
            self.foreground_card_session_open = true;
            let res = download_profile(
                Arc::clone(&self.host),
                &reader,
                &self.state,
                &self.omapi_latch,
                &parsed.smdp_address,
                matching_opt,
                parsed.confirmation_code.as_deref(),
                &self.size_catalog,
                true,
            );
            self.foreground_card_session_open = false;

            match res {
                Ok(DownloadOutcome::Installed {
                    installed_iccid,
                    apdu_log,
                }) => {
                    self.append_apdu_logs(apdu_log);
                    installed_count += 1;
                    self.download.batch_items.push(BatchItemView {
                        index: idx,
                        raw_code: line.clone(),
                        smdp: parsed.smdp_address,
                        matching_id: parsed.matching_id,
                        status: "Installed".into(),
                        error: None,
                        installed_iccid,
                        size_text: pred.map(|b| format_size(Some(b), 1, SizeUnit::AdaptiveSi)),
                    });
                    self.load_active_reader_snapshot();
                }
                Ok(DownloadOutcome::PreviewOnly { .. }) => {}
                Err(e) => {
                    let msg = e.to_string();
                    let out_of_space = msg.to_ascii_lowercase().contains("space")
                        || msg.to_ascii_lowercase().contains("memory");
                    if out_of_space {
                        halted = true;
                    }
                    remaining_lines.push(line.clone());
                    self.download.batch_items.push(BatchItemView {
                        index: idx,
                        raw_code: line.clone(),
                        smdp: parsed.smdp_address,
                        matching_id: parsed.matching_id,
                        status: "Failed".into(),
                        error: Some(msg),
                        ..BatchItemView::default()
                    });
                }
            }
        }

        self.download.batch_running = false;
        self.download.batch_text = remaining_lines.join("\n");
        self.download.batch_summary = Some(format!(
            "Batch finished: {installed_count}/{} installed; {} remaining.",
            lines.len(),
            lines.len().saturating_sub(installed_count)
        ));
    }

    fn format_batch_csv(&self) -> String {
        let mut lines = vec!["index,smdp,matching_id,status,iccid,error".to_string()];
        for item in &self.download.batch_items {
            let iccid = item.installed_iccid.as_deref().unwrap_or("");
            let err = item.error.as_deref().unwrap_or("").replace('"', "\"\"");
            lines.push(format!(
                "{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
                item.index + 1,
                item.smdp,
                item.matching_id,
                item.status,
                iccid,
                err
            ));
        }
        lines.join("\n")
    }

    fn execute_smds_discovery(&mut self) {
        let Some(reader) = self.selected_reader().cloned() else {
            self.download.error = Some("No reader selected".into());
            return;
        };
        let smds = if self.download.smds_server_input.trim().is_empty() {
            ROOT_SMDS_ADDRESS.to_string()
        } else {
            clean_smds_host(&self.download.smds_server_input)
        };
        self.download.discovering_smds = true;
        self.download.error = None;
        self.foreground_card_session_open = true;
        let res = discover_smds_events(
            Arc::clone(&self.host),
            &reader,
            &self.state,
            &self.omapi_latch,
            &smds,
        );
        self.foreground_card_session_open = false;
        self.download.discovering_smds = false;

        match res {
            Ok((events, logs)) => {
                self.append_apdu_logs(logs);
                self.download.discovered_events = events
                    .into_iter()
                    .map(|e| DiscoveredSmdsEventView {
                        activation_code: e.activation_code(),
                        event_id: e.event_id,
                        address: e.address,
                    })
                    .collect();
                if self.download.discovered_events.is_empty() {
                    self.toast = Some(("No pending profiles found on SM-DS".into(), false));
                }
            }
            Err(e) => {
                self.download.error = Some(e.to_string());
            }
        }
    }

    fn probe_ts43_eligibility(&mut self, source_iccid: Option<String>) {
        let Some(reader) = self.selected_reader().cloned() else {
            self.transfer.error = Some("No reader selected".into());
            return;
        };
        let source_iccid = source_iccid.or_else(|| {
            self.active_snapshot.as_ref().and_then(|s| {
                s.profiles
                    .iter()
                    .find(|p| p.enabled)
                    .map(|p| p.iccid.clone())
            })
        });
        let sim = crate::ts43_ops::CardSourceSim {
            host: Arc::clone(&self.host),
            reader,
            state: self.state.clone(),
            omapi_latch: Arc::clone(&self.omapi_latch),
            fallback_iccid: source_iccid,
        };
        use nlpa_core::lpa::ts43::{
            discover_entitlement_for_channel, EntitlementChannel, SimType, SourceSimProvider,
        };
        match sim.identity() {
            Ok(id) => {
                self.transfer.source_iccid = id.iccid.clone();
                self.transfer.source_mcc_mnc = id.mcc_mnc();
                match discover_entitlement_for_channel(
                    &self.entitlement_records,
                    &id,
                    SimType::Esim,
                    EntitlementChannel::Google,
                ) {
                    Ok(ent) => {
                        self.transfer.is_eligible = Some(true);
                        self.transfer.can_start = true;
                        self.transfer.step_message =
                            format!("Carrier entitlement found: {}", ent.server);
                        self.transfer.error = None;
                    }
                    Err(e) => {
                        let has_manual = !self.transfer.manual_entitlement_server.trim().is_empty();
                        self.transfer.is_eligible = Some(has_manual);
                        self.transfer.can_start = has_manual;
                        if has_manual {
                            self.transfer.step_message = "Using manual entitlement server".into();
                            self.transfer.error = None;
                        } else {
                            self.transfer.error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(e) => {
                self.transfer.is_eligible = Some(false);
                self.transfer.can_start = false;
                self.transfer.error = Some(format!("Source SIM unreachable for EAP-AKA: {e}"));
            }
        }
    }

    fn start_ts43_transfer(&mut self) {
        let Some(reader) = self.selected_reader().cloned() else {
            self.transfer.error = Some("No reader selected".into());
            return;
        };
        let eid = self
            .active_snapshot
            .as_ref()
            .map(|s| s.eid.clone())
            .unwrap_or_default();
        let source_iccid =
            (!self.transfer.source_iccid.is_empty()).then(|| self.transfer.source_iccid.clone());
        let mut client = build_ts43_client(
            Arc::clone(&self.host),
            reader,
            self.state.clone(),
            Arc::clone(&self.omapi_latch),
            self.entitlement_records.clone(),
            &self.transfer.manual_entitlement_server,
            source_iccid,
            &eid,
            &self.config.app_version,
        );
        self.transfer.is_busy = true;
        self.transfer.error = None;
        self.foreground_card_session_open = true;
        let res = client.transfer();
        self.foreground_card_session_open = false;

        match res {
            Ok(ts_res) => {
                self.apply_ts43_result(client, ts_res);
            }
            Err(e) => {
                self.transfer.is_busy = false;
                self.transfer.error = Some(e.to_string());
            }
        }
    }

    fn continue_ts43_user_input(&mut self, accepted: bool, text: String) {
        let Some(mut client) = self.active_transfer_client.take() else {
            return;
        };
        let Some(mut ts_res) = self.active_transfer_result.take() else {
            return;
        };
        let req = ContinueRequest {
            user_input: Some(UserInputResult {
                response: text,
                button: if accepted { 1 } else { 0 },
            }),
            websheet: None,
            source_profile_deleted: false,
        };
        self.foreground_card_session_open = true;
        let next = client.continue_flow(&mut ts_res, &req);
        self.foreground_card_session_open = false;
        match next {
            Ok(updated) => self.apply_ts43_result(client, updated),
            Err(e) => {
                self.transfer.is_busy = false;
                self.transfer.error = Some(e.to_string());
            }
        }
    }

    fn handle_websheet_event(&mut self, raw: &str, dismissed: bool) {
        if dismissed {
            self.transfer.pending_websheet_url = None;
            self.transfer.is_busy = false;
            self.transfer.step_message = "Carrier dismissed websheet".into();
            return;
        }
        let Some(mut client) = self.active_transfer_client.take() else {
            return;
        };
        let Some(mut ts_res) = self.active_transfer_result.take() else {
            return;
        };
        match parse_websheet_result(raw) {
            Ok(ws_res) => {
                let req = ContinueRequest {
                    user_input: None,
                    websheet: Some(ws_res),
                    source_profile_deleted: false,
                };
                match client.continue_flow(&mut ts_res, &req) {
                    Ok(updated) => self.apply_ts43_result(client, updated),
                    Err(e) => {
                        self.transfer.is_busy = false;
                        self.transfer.error = Some(e.to_string());
                    }
                }
            }
            Err(e) => {
                self.transfer.is_busy = false;
                self.transfer.error = Some(e.to_string());
            }
        }
    }

    fn continue_ts43_delete_source(&mut self) {
        if let Some(iccid) = self.transfer.pending_delete_source_iccid.take() {
            self.execute_delete_profile(&iccid);
        }
        let Some(mut client) = self.active_transfer_client.take() else {
            return;
        };
        let Some(mut ts_res) = self.active_transfer_result.take() else {
            return;
        };
        let req = ContinueRequest {
            user_input: None,
            websheet: None,
            source_profile_deleted: true,
        };
        match client.continue_flow(&mut ts_res, &req) {
            Ok(updated) => self.apply_ts43_result(client, updated),
            Err(e) => {
                self.transfer.is_busy = false;
                self.transfer.error = Some(e.to_string());
            }
        }
    }

    fn complete_ts43_activation(&mut self, iccid: String) {
        let Some(mut client) = self.active_transfer_client.take() else {
            return;
        };
        let Some(mut ts_res) = self.active_transfer_result.take() else {
            return;
        };
        match client.complete_activation(&mut ts_res, &ActivationResult { iccid }) {
            Ok(updated) => self.apply_ts43_result(client, updated),
            Err(e) => {
                self.transfer.is_busy = false;
                self.transfer.error = Some(e.to_string());
            }
        }
    }

    fn apply_ts43_result(&mut self, mut client: Ts43Client, mut res: Ts43Result) {
        self.transfer.prompt_title = None;
        self.transfer.prompt_text = None;
        self.transfer.pending_websheet_url = None;
        self.transfer.pending_delete_source_iccid = None;
        self.transfer.pending_delayed_download = false;

        match res.event.clone() {
            Some(Ts43Event::UserInput(msg)) => {
                self.transfer.state_label = "Pending Input".into();
                self.transfer.prompt_title = Some(msg.title);
                self.transfer.prompt_text = Some(msg.text);
                self.transfer.prompt_accept_label =
                    msg.accept_button.then(|| msg.accept_button_label);
                self.transfer.prompt_reject_label =
                    msg.reject_button.then(|| msg.reject_button_label);
                self.transfer.prompt_free_text = msg.accept_free_text;
                self.active_transfer_client = Some(client);
                self.active_transfer_result = Some(res);
            }
            Some(Ts43Event::Websheet(ws)) => {
                self.transfer.state_label = "Websheet".into();
                self.transfer.pending_websheet_url = Some(ws.url.clone());
                let host_client = HostClient::new(self.host.as_ref());
                let _ = host_client.ui_open_websheet(&ws.url, &ws.user_data);
                self.active_transfer_client = Some(client);
                self.active_transfer_result = Some(res);
            }
            Some(Ts43Event::SourceProfileDeletion(iccid)) => {
                self.transfer.state_label = "Source Deletion".into();
                self.transfer.pending_delete_source_iccid = Some(iccid);
                self.active_transfer_client = Some(client);
                self.active_transfer_result = Some(res);
            }
            Some(Ts43Event::DownloadReady(cfg)) => {
                let code = if !cfg.activation_code.is_empty() {
                    cfg.activation_code.clone()
                } else {
                    format!("LPA:1${}${}", cfg.smdp_fqdn, cfg.matching_id)
                };
                self.transfer.activation_code = Some(code.clone());
                self.apply_activation_code_text(&code);
                self.transfer.state_label = "Installing Profile".into();
                self.execute_single_download(true);
                let installed_iccid = if !cfg.profile_iccid.is_empty() {
                    cfg.profile_iccid
                } else {
                    self.pending_download_preview
                        .as_ref()
                        .map(|p| p.iccid.clone())
                        .unwrap_or_default()
                };
                if let Ok(done_res) = client.complete_activation(
                    &mut res,
                    &ActivationResult {
                        iccid: installed_iccid,
                    },
                ) {
                    self.transfer.state_label = format!("{:?}", done_res.state);
                    self.transfer.is_busy = false;
                    self.transfer.step_message = "Transfer complete".into();
                } else {
                    self.active_transfer_client = Some(client);
                    self.active_transfer_result = Some(res);
                }
            }
            Some(Ts43Event::DelayedDownload { .. }) => {
                self.transfer.state_label = "Delayed Download".into();
                self.transfer.pending_delayed_download = true;
                self.active_transfer_client = Some(client);
                self.active_transfer_result = Some(res);
            }
            Some(Ts43Event::ActivationPending { .. }) => {
                self.transfer.state_label = "Pending Activation".into();
                self.transfer.step_message =
                    "Carrier is activating your subscription; check again shortly.".into();
                self.transfer.is_busy = false;
            }
            Some(Ts43Event::SmdsDiscovery { target_eid: _, .. }) => {
                self.transfer.state_label = "SM-DS Discovery".into();
                self.execute_smds_discovery();
                self.transfer.is_busy = false;
            }
            Some(Ts43Event::Dismiss(_)) => {
                self.transfer.state_label = "Dismissed".into();
                self.transfer.is_busy = false;
            }
            Some(Ts43Event::Done) | None => {
                self.transfer.state_label = match res.state {
                    Ts43State::Done => "Complete".into(),
                    other => format!("{other:?}"),
                };
                self.transfer.step_message = "Transfer finished".into();
                self.transfer.is_busy = false;
            }
        }
    }

    fn execute_process_all_notifications(&mut self) {
        let Some(pending) = self
            .active_snapshot
            .as_ref()
            .map(|s| s.notifications.clone())
        else {
            return;
        };
        for item in pending {
            let seq = item.metadata.seq_number;
            self.execute_send_pending_notification(seq);
        }
    }

    fn execute_send_pending_notification(&mut self, seq: u32) {
        let item = self.active_snapshot.as_ref().and_then(|s| {
            s.notifications
                .iter()
                .find(|n| n.metadata.seq_number == seq)
                .cloned()
        });
        let Some(notif) = item else {
            self.notif_error = Some(format!("Notification #{seq} not found on card"));
            return;
        };
        let eid = self
            .active_snapshot
            .as_ref()
            .map(|s| s.eid.clone())
            .unwrap_or_default();
        self.backup_notifications_to_history(&eid, std::slice::from_ref(&notif));

        match dispatch_notification_es9(
            self.host.as_ref(),
            self.state.proxy_url.as_deref(),
            &notif.metadata.notification_address,
            &notif.raw_notification_b64,
        ) {
            Ok(()) => {
                self.update_history_status(seq, "sent");
                self.notif_error = None;
                self.notif_notice = Some(format!("Notification #{seq} sent"));
                if self.auto_remove_notifs {
                    self.execute_remove_pending_notification(seq);
                }
            }
            Err(e) => {
                self.update_history_status(seq, "failed");
                self.notif_error = Some(e.to_string());
            }
        }
    }

    fn execute_remove_pending_notification(&mut self, seq: u32) {
        let Some(reader) = self.selected_reader().cloned() else {
            return;
        };
        let eid = self
            .active_snapshot
            .as_ref()
            .map(|s| s.eid.clone())
            .unwrap_or_default();
        let backed_up = self
            .state
            .notifications_history
            .iter()
            .any(|h| h.seq_number == seq && (h.eid == eid || h.eid.is_empty()));

        match remove_notification_safe(
            Arc::clone(&self.host),
            &reader,
            &self.state,
            &self.omapi_latch,
            seq,
            backed_up,
            Some(&eid),
        ) {
            Ok(logs) => {
                self.append_apdu_logs(logs);
                if let Some(snap) = self.active_snapshot.as_mut() {
                    snap.notifications.retain(|n| n.metadata.seq_number != seq);
                }
                self.notif_notice = Some(format!("Notification #{seq} removed from card"));
                self.notif_error = None;
            }
            Err(e) => {
                self.notif_error = Some(e.to_string());
            }
        }
    }

    fn execute_resend_history(&mut self, seq: u32, server: &str, raw_b64: &str) {
        match dispatch_notification_es9(
            self.host.as_ref(),
            self.state.proxy_url.as_deref(),
            server,
            raw_b64,
        ) {
            Ok(()) => {
                self.update_history_status(seq, "sent");
                self.notif_notice = Some(format!("Resent notification #{seq}"));
                self.notif_error = None;
            }
            Err(e) => {
                self.notif_error = Some(e.to_string());
            }
        }
    }

    fn update_history_status(&mut self, seq: u32, status: &str) {
        for entry in &mut self.state.notifications_history {
            if entry.seq_number == seq {
                entry.status = status.to_string();
            }
        }
        self.save_state_best_effort();
    }

    fn execute_check_updates(&mut self) {
        self.update_checking = true;
        self.update_notice = None;
        let client = HostClient::new(self.host.as_ref());
        let mut headers = BTreeMap::new();
        headers.insert("Accept".into(), "application/json".into());
        let resp = client.http_request(&HttpRequestArgs {
            method: "GET".into(),
            url: DEFAULT_UPDATE_FEED_URL.into(),
            headers,
            body_base64: None,
            proxy_url: self.state.proxy_url.clone(),
            timeout_ms: 12_000,
            insecure_tls: false,
        });
        self.update_checking = false;

        match resp {
            Ok(http) if (200..300).contains(&http.status) => {
                let body = base64_decode(&http.body_base64)
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok())
                    .unwrap_or_default();
                let current = u64::try_from(self.config.version_code.max(0)).unwrap_or(0);
                if let Some(cand) =
                    check_update_feed(&body, self.config.is_privileged, current, &self.cert_sha1)
                {
                    self.update_candidate = Some(UpdateCandidateView {
                        version: cand.version,
                        build: cand.build,
                        url: cand.url,
                        download_url: cand.download_url,
                        changelog: cand.changelog,
                    });
                } else {
                    self.update_candidate = None;
                    self.update_notice = Some("You are on the latest release".into());
                }
            }
            Ok(http) => {
                self.update_notice = Some(format!("Update check returned HTTP {}", http.status));
            }
            Err(e) => {
                self.update_notice = Some(format!("Update check failed: {e}"));
            }
        }
    }

    fn update_redaction_flag(&mut self, flag: &str, value: bool) {
        let r = &mut self.state.redaction;
        match flag {
            "reveal_sensitive" => r.reveal_sensitive_profile_data = value,
            "redact_eid_first" => r.redact_eid_first = value,
            "redact_eid_middle" => r.redact_eid_middle = value,
            "redact_eid_last" => r.redact_eid_last = value,
            "redact_iccid_first" => r.redact_iccid_first = value,
            "redact_iccid_middle" => r.redact_iccid_middle = value,
            "redact_iccid_last" => r.redact_iccid_last = value,
            _ => {}
        }
    }

    fn update_redaction_mode(&mut self, field: &str, value: &str) {
        let r = &mut self.state.redaction;
        match field {
            "nickname" => {
                r.nickname_redaction_mode = match value {
                    "redact_phone_numbers" => NicknameRedactionMode::RedactPhoneNumbers,
                    "hide_nickname" => NicknameRedactionMode::HideNickname,
                    _ => NicknameRedactionMode::None,
                };
            }
            "operator" => {
                r.operator_redaction_mode = match value {
                    "country_name" => OperatorRedactionMode::CountryName,
                    _ => OperatorRedactionMode::None,
                };
            }
            "tags" => {
                r.tags_redaction_mode = match value {
                    "hide_tags" => TagsRedactionMode::HideTags,
                    _ => TagsRedactionMode::None,
                };
            }
            "flag" => {
                r.flag_redaction_mode = match value {
                    "hide_flag" => FlagRedactionMode::HideFlag,
                    "shuffled_flags" => FlagRedactionMode::ShuffledFlags,
                    _ => FlagRedactionMode::None,
                };
            }
            "icon" => {
                r.icon_redaction_mode = match value {
                    "hide_icon" => IconRedactionMode::HideIcon,
                    "random_icon" => IconRedactionMode::RandomIcon,
                    _ => IconRedactionMode::None,
                };
            }
            _ => {}
        }
    }

    fn update_reader_toggle(&mut self, reader_type: &str, enabled: bool) {
        match reader_type {
            "omapi" => self.state.enable_omapi = enabled,
            "tmapi" => self.state.enable_tmapi = enabled,
            "nbridge" => self.state.enable_nbridge = enabled,
            "adb_bridge" => self.state.enable_adb_bridge = enabled,
            "qrtr" => self.state.enable_qrtr = enabled,
            "se_hal" => self.state.enable_se_hal = enabled,
            "se_hal_borrowed_slot1" => self.state.se_hal_borrowed_slot1 = enabled,
            "se_hal_borrowed_slot2" => self.state.se_hal_borrowed_slot2 = enabled,
            "ccid" => self.state.enable_ccid = enabled,
            "remote" => self.state.enable_remote = enabled,
            _ => {}
        }
    }

    fn resolve_raw_iccid(&self, iccid: &str) -> String {
        self.active_snapshot
            .as_ref()
            .and_then(|s| {
                s.profiles
                    .iter()
                    .find(|p| p.iccid == iccid || p.raw_iccid == iccid)
                    .map(|p| p.raw_iccid.clone())
            })
            .unwrap_or_else(|| iccid.to_string())
    }

    fn display_name_for_iccid(&self, iccid: &str) -> String {
        self.active_snapshot
            .as_ref()
            .and_then(|s| s.profiles.iter().find(|p| p.iccid == iccid))
            .map(|p| self.build_profile_card_view(p).display_name)
            .unwrap_or_else(|| iccid.to_string())
    }

    fn current_nickname_and_tags(&self, iccid: &str) -> (String, String) {
        let card_nick = self
            .active_snapshot
            .as_ref()
            .and_then(|s| s.profiles.iter().find(|p| p.iccid == iccid))
            .and_then(|p| p.nickname.clone())
            .or_else(|| self.state.profile_nicknames_cache.get(iccid).cloned());
        let parsed = parse_nickname(card_nick.as_deref());
        let tags_csv = self
            .state
            .profile_tags
            .get(iccid)
            .cloned()
            .unwrap_or_else(|| {
                parsed
                    .tags
                    .iter()
                    .map(ProfileTag::raw)
                    .collect::<Vec<_>>()
                    .join(", ")
            });
        (parsed.name, tags_csv)
    }

    fn build_profile_card_view(&self, p: &EuiccProfile) -> ProfileCardView {
        let parsed = parse_nickname(p.nickname.as_deref());
        let cc = mcc_to_country_code(
            p.mcc.as_deref().unwrap_or(""),
            p.mnc.as_deref(),
            Some(&p.iccid),
        );
        let flag = if cc.is_empty() {
            String::new()
        } else {
            get_flag_emoji(cc)
        };
        let raw_display = if !parsed.display_name.is_empty() {
            parsed.display_name.clone()
        } else {
            p.name
                .clone()
                .or_else(|| p.service_provider_name.clone())
                .unwrap_or_else(|| p.iccid.clone())
        };
        let view = ProfileView {
            nickname: p.nickname.clone(),
            name: p.name.clone(),
            service_provider_name: p.service_provider_name.clone(),
            flag: flag.clone(),
            iccid: p.iccid.clone(),
            display_name: raw_display,
            flag_url: String::new(),
            flag_emoji: flag,
        };
        let mut tags: Vec<String> = parsed
            .tags
            .iter()
            .map(|t| match t {
                ProfileTag::Text(s) => s.clone(),
                ProfileTag::Date { raw } | ProfileTag::Raw(raw) => {
                    t.display_date().unwrap_or_else(|| raw.clone())
                }
            })
            .collect();
        if tags.is_empty() {
            if let Some(csv) = self.state.profile_tags.get(&p.iccid) {
                tags = csv
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
        }
        let filtered_tags = filter_tags(&tags, &self.state.redaction);
        let estimated_size_text = self
            .size_catalog
            .predict_size(
                self.active_snapshot.as_ref().map(|s| s.eid.as_str()),
                p.smdp_address.as_deref(),
                None,
                p.service_provider_name.as_deref(),
            )
            .map(|b| format_size(Some(b), 1, SizeUnit::AdaptiveSi));

        let badge = self
            .provider_badges
            .get(&p.iccid)
            .map(|b| ProviderBadgeView {
                label: b.label.clone(),
                msisdn: b.msisdn.clone(),
                quota_summary: b.quota_summary.clone(),
                portal_url: b.portal_url.clone(),
            });

        ProfileCardView {
            iccid: p.iccid.clone(),
            display_iccid: redact_iccid(&p.iccid, &self.state.redaction),
            display_name: display_name(&view, &self.state.redaction),
            operator_summary: operator_summary(&view, &self.state.redaction, &[])
                .unwrap_or_default(),
            nickname: p.nickname.clone(),
            service_provider_name: p.service_provider_name.clone(),
            profile_name: p.name.clone(),
            enabled: p.enabled,
            profile_class: p.profile_class,
            is_test_profile: p.profile_class == 0,
            is_provisioning_profile: p.profile_class == 1,
            tags: filtered_tags,
            flag_emoji: profile_flag_emoji(&view, &self.state.redaction, &[]),
            hide_flag: hide_flag(&self.state.redaction),
            hide_icon: hide_icon(&self.state.redaction),
            estimated_size_text,
            provider_badge: badge,
            has_provider_plugin: match_provider_plugin(&p.iccid).is_some(),
            isdp_aid: p.isdp_aid.clone(),
            switching: false,
        }
    }

    /// Projects current controller state into an immutable [`UiModel`] for Dioxus SSR.
    #[must_use]
    pub fn build_ui_model(&self) -> UiModel {
        let readers: Vec<ReaderChipView> = self
            .readers
            .iter()
            .map(|r| ReaderChipView {
                id: r.id.clone(),
                label: r.label.clone(),
                transport_badge: r.kind.badge().to_string(),
                slot_label: r.slot_label.clone(),
                eid_preview: r
                    .eid
                    .as_deref()
                    .map(|e| redact_eid(e, false, &self.state.redaction)),
                is_selected: self.selected_reader_id.as_deref() == Some(r.id.as_str()),
                is_removable: r.is_removable,
                is_connected: true,
                needs_permission: r.needs_permission,
                is_estk: r.looks_like_estk(),
                is_9esim: r.looks_like_9esim(),
                is_gm: r.eid.as_deref().is_some_and(is_gm_eid),
            })
            .collect();

        let euicc_info = self.active_snapshot.as_ref().map(|snap| {
            let sel = self.selected_reader();
            let nvram = snap
                .nxp_memory
                .as_ref()
                .and_then(|n| n.free_non_volatile_memory)
                .unwrap_or(u64::from(snap.info2.free_nvram));
            let ram = snap
                .nxp_memory
                .as_ref()
                .and_then(|n| n.free_volatile_memory)
                .or_else(|| (snap.info2.free_ram > 0).then_some(u64::from(snap.info2.free_ram)));
            EuiccInfoView {
                reader_name: sel.map(|r| r.label.clone()).unwrap_or_default(),
                raw_eid: snap.eid.clone(),
                display_eid: redact_eid(&snap.eid, false, &self.state.redaction),
                default_smdp: snap
                    .configured
                    .default_smdp_address
                    .clone()
                    .unwrap_or_default(),
                root_smds: snap
                    .configured
                    .root_smds_address
                    .clone()
                    .unwrap_or_default(),
                free_nvram_bytes: Some(nvram),
                free_nvram_text: format_size(i64::try_from(nvram).ok(), 1, SizeUnit::AdaptiveBi),
                free_ram_bytes: ram,
                free_ram_text: ram
                    .map(|v| format_size(i64::try_from(v).ok(), 1, SizeUnit::AdaptiveBi))
                    .unwrap_or_default(),
                svn: version_bytes_to_dotted(&snap.info2.svn),
                firmware_ver: version_bytes_to_dotted(&snap.info2.euicc_firmware_ver),
                profile_version: version_bytes_to_dotted(&snap.info2.profile_version),
                is_estk: sel.is_some_and(DiscoveredReader::looks_like_estk),
                estk_info: snap.estk_info.as_ref().map(EstkCardInfo::to_view),
                is_9esim: sel.is_some_and(DiscoveredReader::looks_like_9esim),
                is_gm: is_gm_eid(&snap.eid),
                gm_authenticated: snap.gm_authenticated,
                gm_status_note: None,
            }
        });

        let profiles: Vec<ProfileCardView> = self
            .active_snapshot
            .as_ref()
            .map(|snap| {
                snap.profiles
                    .iter()
                    .map(|p| self.build_profile_card_view(p))
                    .collect()
            })
            .unwrap_or_default();

        let pending_notifs: Vec<PendingNotifView> = self
            .active_snapshot
            .as_ref()
            .map(|snap| {
                snap.notifications
                    .iter()
                    .map(|n| PendingNotifView {
                        seq_number: n.metadata.seq_number,
                        operation_label: operation_label(n.metadata.profile_management_operation)
                            .to_string(),
                        operation_code: n.metadata.profile_management_operation,
                        notification_address: n.metadata.notification_address.clone(),
                        iccid: n.metadata.iccid.clone().unwrap_or_default(),
                        is_install: n.metadata.profile_management_operation == 1,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let history_notifs: Vec<HistoryNotifView> = self
            .state
            .notifications_history
            .iter()
            .map(|h| HistoryNotifView {
                seq_number: h.seq_number,
                operation_label: h.operation.clone(),
                notification_address: h.address.clone(),
                iccid: h.iccid.clone(),
                status_label: h.status.clone(),
                timestamp: format!("{} ms", h.timestamp_ms),
                raw_b64: h.content_base64.clone(),
            })
            .collect();

        let mut download = self.download.clone();
        download.is_se_hal = self
            .selected_reader()
            .is_some_and(|r| r.kind == TransportKind::SeHal);

        let r = &self.state.redaction;
        UiModel {
            active_tab: self.active_tab,
            theme_style: self.state.theme_style,
            color_mode: self.state.color_mode,
            system_dark: self.system_dark,
            locale: if self.state.locale == nlpa_core::i18n::LocaleChoice::System {
                resolve_locale(self.state.locale, &self.config.system_locale)
            } else {
                self.state.locale
            },
            wallpaper_enabled: self.state.wallpaper_enabled,
            wallpaper_opacity: self.state.wallpaper_opacity,
            readers,
            selected_reader_id: self.selected_reader_id.clone(),
            omapi_euicc_warning: self.omapi_euicc_warning,
            omapi_corrupted_banner: self.omapi_corrupted_banner.clone(),
            euicc_info,
            profiles,
            profile_search: self.profile_search.clone(),
            profile_sort: self.profile_sort,
            loading: self.loading.clone(),
            toast: self.toast.clone(),
            download,
            transfer: self.transfer.clone(),
            notifications: NotificationsView {
                pending: pending_notifs,
                history: history_notifs,
                history_search: self.notif_history_search.clone(),
                show_history: self.notif_show_history,
                loading: false,
                auto_send: self.auto_send_notifs,
                auto_remove: self.auto_remove_notifs,
                error: self.notif_error.clone(),
                notice: self.notif_notice.clone(),
            },
            settings: SettingsView {
                locale_choice: Some(self.state.locale),
                reveal_sensitive: r.reveal_sensitive_profile_data,
                redact_eid_first: r.redact_eid_first,
                redact_eid_middle: r.redact_eid_middle,
                redact_eid_last: r.redact_eid_last,
                redact_iccid_first: r.redact_iccid_first,
                redact_iccid_middle: r.redact_iccid_middle,
                redact_iccid_last: r.redact_iccid_last,
                nickname_mode: match r.nickname_redaction_mode {
                    NicknameRedactionMode::RedactPhoneNumbers => "redact_phone_numbers".into(),
                    NicknameRedactionMode::HideNickname => "hide_nickname".into(),
                    NicknameRedactionMode::None => "none".into(),
                },
                operator_mode: match r.operator_redaction_mode {
                    OperatorRedactionMode::CountryName => "country_name".into(),
                    OperatorRedactionMode::None => "none".into(),
                },
                tags_mode: match r.tags_redaction_mode {
                    TagsRedactionMode::HideTags => "hide_tags".into(),
                    TagsRedactionMode::None => "none".into(),
                },
                flag_mode: match r.flag_redaction_mode {
                    FlagRedactionMode::HideFlag => "hide_flag".into(),
                    FlagRedactionMode::ShuffledFlags => "shuffled_flags".into(),
                    FlagRedactionMode::None => "none".into(),
                },
                icon_mode: match r.icon_redaction_mode {
                    IconRedactionMode::HideIcon => "hide_icon".into(),
                    IconRedactionMode::RandomIcon => "random_icon".into(),
                    IconRedactionMode::None => "none".into(),
                },
                enable_omapi: self.state.enable_omapi,
                enable_tmapi: self.state.enable_tmapi,
                enable_nbridge: self.state.enable_nbridge,
                enable_adb_bridge: self.state.enable_adb_bridge,
                enable_qrtr: self.state.enable_qrtr,
                enable_se_hal: self.state.enable_se_hal,
                se_hal_borrowed_slot1: self.state.se_hal_borrowed_slot1,
                se_hal_borrowed_slot2: self.state.se_hal_borrowed_slot2,
                enable_ccid: self.state.enable_ccid,
                enable_remote: self.state.enable_remote,
                adb_connected: self.adb_connected,
                adb_waiting: self.adb_waiting,
                adb_command: self.adb_command.clone(),
                adb_status_text: if self.adb_connected {
                    "Connected".into()
                } else if self.adb_waiting {
                    "Waiting...".into()
                } else {
                    "Disconnected".into()
                },
                remocard_servers: self
                    .state
                    .remocard_servers
                    .iter()
                    .map(|srv| RemoCardServerView {
                        id: srv.id.clone(),
                        name: srv.name.clone(),
                        base_url: srv.base_url.clone(),
                        token: srv.token.clone().unwrap_or_default(),
                        aes_key_hex: srv.aes_key_hex.clone().unwrap_or_default(),
                        enabled: srv.enabled,
                    })
                    .collect(),
                mss: self.state.mss,
                custom_aids: self.state.custom_aids.clone(),
                proxy_url: self.state.proxy_url.clone().unwrap_or_default(),
                device_imei: self.state.device_imei.clone(),
                imei_error: self.imei_error.clone(),
                app_version: self.config.app_version.clone(),
                version_code: self.config.version_code,
                is_privileged: self.config.is_privileged,
                cert_sha1: self.cert_sha1.clone(),
                aram_info: self.aram_info.clone(),
                update_checking: self.update_checking,
                update_candidate: self.update_candidate.clone(),
                update_notice: self.update_notice.clone(),
                logs: self.logs.clone(),
                apdu_history: self.apdu_history.clone(),
            },
            modal: self.modal.clone(),
        }
    }
}

trait EstkViewExt {
    fn to_view(&self) -> EstkInfoView;
}

impl EstkViewExt for EstkCardInfo {
    fn to_view(&self) -> EstkInfoView {
        EstkInfoView {
            firmware: self.firmware.clone(),
            bootloader: self.bootloader.clone(),
            serial: self.serial.clone(),
            sku: self.sku.clone(),
        }
    }
}

fn load_state_from_disk(files_dir: &std::path::Path) -> PersistedState {
    let path = files_dir.join("state.json");
    match fs::read_to_string(path) {
        Ok(raw) => PersistedState::from_json_str(&raw).unwrap_or_default(),
        Err(_) => PersistedState::default(),
    }
}

fn save_state_atomic(files_dir: &std::path::Path, state: &PersistedState) -> std::io::Result<()> {
    if files_dir.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(files_dir)?;
    let tmp_path = files_dir.join("state.json.tmp");
    let final_path = files_dir.join("state.json");
    let json = state
        .to_json_string()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut file = File::create(&tmp_path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;
    fs::rename(&tmp_path, &final_path)
}

fn operation_label(code: u8) -> &'static str {
    match ProfileOperation::from(code) {
        ProfileOperation::Install => "Install",
        ProfileOperation::Enable => "Enable",
        ProfileOperation::Disable => "Disable",
        ProfileOperation::Delete => "Delete",
        ProfileOperation::Other(_) => "Event",
    }
}

fn version_bytes_to_dotted(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if bytes.len() <= 3 {
        return bytes
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(".");
    }
    bytes_to_hex(bytes)
}

fn current_time_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
    )
    .unwrap_or(0)
}
