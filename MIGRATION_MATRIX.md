# Rust migration matrix (`2.5.0+606` Pure Rust Architecture)

Baseline: NekokoLPA2 `2.5.0+606` (`100% Pure Rust` core + Dioxus `0.6` SSR UI + direct Android Kotlin hardware host via JNI). Flutter, Dart (`lib/`, `pubspec.yaml`, `local_packages/`, `rust_builder/`), and `flutter_rust_bridge` have been completely excised.

Status vocabulary:

- `ported_pure_rust`: implemented, integrated, and active inside `nlpa_core` / `nlpa_ui` / `rust_lib_nlpa2` with zero Dart or FRB runtime involvement.
- `android_host_jni`: Android OS hardware/permission/system-bar/WebView boundary retained in native Kotlin (`android/app/src/main/kotlin/ee/nekoko/nlpa2/` & `im/nfc/ccid/`) and invoked via `RustBridge` / `NativeHostBridge`.

## Capability mapping

| Former Dart/Flutter Layer | Rust Implementation | Integration Boundary | Status & Notes |
|---|---|---|---|
| `lib/models/asn1/*`, `rsp_definitions.g.dart` | `rust/core/src/asn1/*` | In-process `nlpa_core` | `ported_pure_rust` — DER reader, RSP codecs, PE, FCP, RFC 5280 X.509. |
| `lib/services/es9plus_service.dart`, `smdp_client.dart` | `rust/core/src/es9.rs`, `rust/src/` controller | `http.request` via `NativeHostBridge` | `ported_pure_rust` — ES9+ JSON framing/parsing in Rust; TLS HTTP transport via Android `OkHttp`/`HttpURLConnection`. |
| `lib/logic/profile_manager.dart` (`_decomposeBpp`, `cancelSession`, `resetEuiccMemory`, NXP `DF25`) | `rust/core/src/bpp.rs`, `lpa/es10.rs`, `channel.rs` | In-process `nlpa_core` + `rust_lib_nlpa2` | `ported_pure_rust` — SGP.22 BPP segmentation, `BF41`, `BF34` (`05 E0`), NXP `DF25`, default MSS `120`. |
| `lib/logic/profile_download_session.dart`, `profile_state_transition.dart`, `profile_download_retry.dart` | `rust/core/src/lpa/{download_session,state_transition,engine}.rs` | `AppController` in `rust_lib_nlpa2` | `ported_pure_rust` — Session state, convergence verification, and non-replay `6881` safety. |
| `lib/services/notification_service.dart`, `notification_address.dart` | `rust/core/src/lpa/notifications.rs`, `notification_address.rs`, `storage.rs` | `AppController` + `state.json` | `ported_pure_rust` — Policy evaluation, sync planner, FQDN validation, and history persistence. |
| `lib/utils/crypto_utils.dart`, `aes_cipher.dart`, `scp03.dart` | `rust/core/src/crypto.rs`, `lpa/scp03.rs` | In-process `nlpa_core` | `ported_pure_rust` — Zero-dependency FIPS AES-128/256, CBC, CMAC, SHA-1/256, `HashCc`, `calc_host_mac`. |
| `lib/utils/atr_capabilities.dart`, `nine_esim_usb_protocol.dart` | `rust/core/src/atr.rs`, `ccid/nine_esim.rs` | In-process `nlpa_core` | `ported_pure_rust` — ETSI TS 102 221 V17.1.0 + 9eSIM ATR capability detection and toggling. |
| `lib/ts43/*` | `rust/core/src/lpa/ts43/*` | `AppController` + `ui.openWebsheet` | `ported_pure_rust` — EAP-AKA, Google/Samsung ODSA requests, WAP XML decoding, websheet result parsing. |
| `lib/services/smds_discovery_service.dart`, `profile_size_prediction_service.dart` | `rust/core/src/lpa/es11.rs`, `profile_size_predictor.rs` | `AppController` (`reference_sizes_simple.json`) | `ported_pure_rust` — ES11 discovery and EUM/SPN/PLMN profile size prediction. |
| `lib/plugins/{estk,gm,esim_cc,esim_gg,thk}_plugin.dart` | `rust/core/src/plugins.rs` | `AppController` + `nlpa_ui` | `ported_pure_rust` — eSTK AID/MGT, GM `BF71` APDU, and `eSIM.cc` / `eSIM.gg` / `3HK` badges. |
| `lib/adapter/remote/remocard_adapter.dart` | `rust/core/src/remocard.rs` | `http.request` via `NativeHostBridge` | `ported_pure_rust` — RemoCard V2 endpoints and AES-256-CBC payload encryption/decryption. |
| `lib/settings/app_settings.dart`, `database_service.dart`, `locale_utils.dart` | `rust/core/src/storage.rs`, `i18n.rs`, `redaction.rs` | `files_dir/state.json` + embedded `l10n/*.arb` | `ported_pure_rust` — Atomic JSON persistence, all 11 ARB locales (incl. `vi`), and redaction settings. |
| `lib/services/update_feed.dart`, `update_service.dart` | `rust/core/src/update_feed.rs` | `http.request` + `cert.getInfo` | `ported_pure_rust` — `COMMUNITY_CERT` pinning and `community`/`open-rooted` channel isolation. |
| `lib/pages/*`, `lib/widgets/*`, `lib/theme/*` | `rust/ui/src/*` (`nlpa_ui`) | `dioxus-ssr` -> Android `WebView` | `ported_pure_rust` — AMOLED `#000000`, wallpaper glass across loading & Settings, opacity slider, all 11 locales. |
| Android OMAPI, Telephony, NBridge, ADB Bridge, QRTR, SE HAL, CCID, Cert, WebView/Picker | `android/app/src/main/kotlin/ee/nekoko/nlpa2/*`, `im/nfc/ccid/*`, `rust/src/qrtr_bus.rs` | `RustBridge` JNI + `NativeHostBridge` | `android_host_jni` — All Flutter channels replaced by direct Kotlin host handlers; `OmapiSafetyLatch` & `QrtrUserService` preserved. |
| `otbridge/app/src/main/kotlin/ee/nekoko/nbridge/*` | Separate helper APK | `nbridge.*` ContentProvider IPC | `android_host_jni` — Standalone Magisk/LSPosed helper APK unchanged. |
