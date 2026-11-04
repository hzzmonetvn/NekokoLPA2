# Architectural Decisions — NekokoLPA2

## 1. Android-Only Platform Target
- **Choice:** Removed all non-Android code (iOS, macOS, Windows, Linux, Web) and stripped out non-Android dependencies to minimize attack surface and binary size.
- **Alternatives Considered:** Keeping cross-platform support.
- **Reasoning:** The application interacts with modem buses (QRTR) and SIM interfaces that are strictly available on target Android devices.

## 2. Full Rust Logic & Workspace Integration
- **Choice:** Structured a pure Rust workspace (`rust`, `rust/core` and `rust/ui`) using Dioxus (v0.6) for lightweight, modular UI components and consolidating core protocol logic.
- **Alternatives Considered:** Keeping business logic embedded within Flutter plugins.
- **Reasoning:** Clear separation of concerns, higher memory safety, native performance, and ability to render interfaces directly with Rust.

## 3. Split APK Deployment & Build Workflow
- **Choice:** Enabled ABI split APKs (`armeabi-v7a`, `arm64-v8a`) via Gradle configuration and updated GitHub Actions CI to package individual split artifacts and universal builds using direct Gradle task execution (`assemble<Flavor>Release`).
- **Alternatives Considered:** Relying strictly on `flutter build apk --split-per-abi`.
- **Reasoning:** Flutter CLI version 3.47.2 fails with `Gradle build failed to produce an .apk file` when product flavors and ABI splits are enabled simultaneously because it expects single un-split APK naming conventions. Running Gradle directly generates all split and universal APKs reliably.

## 4. AMOLED Dark Theme & System Bar Overlay
- **Choice:** Introduced pure black (`#000000`) theme palette with high-contrast surfaces (`#101010`) and system bar overlay style configuration across both Flutter and Dioxus UI components.
- **Alternatives Considered:** Standard dark gray palette.
- **Reasoning:** Maximum battery efficiency on OLED/AMOLED mobile screens and consistent visual hierarchy.

## 5. Vietnamese Primary Localization & Terminology Overhaul
- **Choice:** Standardized full Vietnamese ARB localization (`l10n/app_vi.arb`) and documentation, replacing rough automated translations with precise, natural Vietnamese telecommunications and mobile UI terminology.
- **Alternatives Considered:** Leaving machine translations or using automated Google Translate without telecom domain context.
- **Reasoning:** Telecommunications and eSIM operations require precise domain terminology (eUICC, APDU, ICCID, OMAPI, matching ID, carrier/nhà mạng, đầu đọc thẻ, mượn kênh, ký xác thực) to prevent confusion and errors during critical cryptographic SIM profile tasks.

## 6. Extreme Native Binary Optimization & ProGuard Rules
- **Choice:** Configured `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, and `strip = true` in workspace `[profile.release]`, paired with thorough ProGuard rules protecting JNI methods, Shizuku, Cargokit, and `flutter_rust_bridge`.
- **Alternatives Considered:** Default Cargo dev/release profiles without LTO or stripping.
- **Reasoning:** Minimizes `.so` binary footprint on mobile storage, removes unused dead code across cross-compilation boundaries, and accelerates cold startup on ARM devices.

## 7. Complete Bluetooth / BLE Stack Removal
- **Choice:** Completely excised all Bluetooth Low Energy (BLE) functionality, code, dependencies, and permissions from the application:
  - Removed local package dependency `ble: path: local_packages/ble` and deleted directory `local_packages/ble/`.
  - Removed all Bluetooth hardware permissions (`BLUETOOTH`, `BLUETOOTH_ADMIN`, `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`) and BLE hardware feature flags from `android/app/src/main/AndroidManifest.xml`.
  - Removed `ReaderType::Ble` from Rust models and components (`rust/ui/src/models.rs`, `rust/ui/src/components/reader_selector.rs`).
  - Excised all BLE adapter implementations (`lib/adapter/ble/`), UI settings pages (`ble_devices_settings_page.dart`), scan dialogs (`ble_scan_dialog.dart`), and BLE plugins (`ty_ble_plugin.dart`, `bee_sim_plugin.dart`).
  - Stripped BLE connection, scanning, error states (`ReaderStatus.bleError`), and manager hooks from `lib/pages/profiles_screen.dart`, `lib/adapter/composite_adapter.dart`, `lib/settings/app_settings.dart`, and `lib/widgets/profiles_screen/state_messages.dart`.
  - Cleaned and synchronized all multilingual documentation (`README.md`, `README_en.md`, `README_ja-JP.md`, `README_zh-CN.md`) to reflect strict Android ARM specialization (USB CCID, OMAPI, TMAPI, QRTR, SE HAL, Remote).
- **Alternatives Considered:** Leaving BLE code unused or conditionally disabling it via runtime feature toggles.
- **Reasoning:** The user specifically mandated focusing exclusively on Android ARM native reader capabilities (OMAPI, TMAPI/Telephony, QRTR modem bus, SE HAL, and USB CCID). Retaining the BLE stack added unnecessary background services, Bluetooth permission prompts, code complexity, and binary bloat without aligning with the dedicated native Android target.

## 8. High-Precision Telecom Localization Overhaul (app_vi.arb)
- **Choice:** Conducted an exhaustive review and refinement of `l10n/app_vi.arb`, correcting automated translation defects, removing redundant repeated phrases ("Đầu đầu đọc từ xa" → "Đầu đọc từ xa"), fixing reversed logic ("Trình duyệt Bật" → "Bật trình duyệt"), standardizing domain terminology ("Toán tử" → "Nhà mạng", "Đang đọc SIM nguồn…", "Máy chủ SM-DS"), and neutralizing lingering Apple/iOS/CallKit/Bluetooth copy into clean Android-native terms.
- **Alternatives Considered:** Relying on raw machine translation outputs or keeping generic translations.
- **Reasoning:** Inaccurate and unnatural translations undermine user clarity and trust in critical cryptographic operations such as eUICC profile installation, deletion, and carrier authorization. Refining translations using specialized telecom domain knowledge ensures exact semantic alignment with GSMA eSIM specifications.

## 9. Dioxus Native UI Architecture & AMOLED Optimization
- **Choice:** Expanded the pure Rust UI component tree with Dioxus v0.6 (`nlpa_ui`) to support complete eSIM lifecycle operations natively: added `EuiccInfoDialog` for inspecting card EID and GSMA SGP.22 metadata, `DownloadProfileModal` for activation code input with IMEI targeting, reactive feedback banners, and rigorous test coverage with Virtual DOM headless mounting (`ui_test.rs`).
- **Alternatives Considered:** Leaving Rust UI as an incomplete mockup without modals or dialogues.
- **Reasoning:** Full native Rust UI capabilities ensure modularity, memory safety, and direct compilation to native code or webview integration with zero overhead, styled with the `#000000` AMOLED token design.

## 10. Unslopping & Telecommunications Interface Craft Pass
- **Choice:** Executed a systematic 9-pass design refactoring on the native Dioxus UI:
  - Enforced a strict 4px grid across all margins, paddings, and control heights.
  - Replaced harsh saturated button backgrounds with telecommunications signal dots (`#2EA043` for active profiles, `#484F58` for inactive) and subtle hairline borders (`#202020`).
  - Adopted structured dual typography: system interface typography for text and `ui-monospace` / `Roboto Mono` with 4-digit grouped formatting for EID, ICCID, and APDU hex byte streams.
  - Grouped reader tags (`USB`, `SYS`, `NET`, `MDM`) and nested border radii (modal 12px, cards 8px, tags 4px) to present an industrial, precise, and battery-efficient visual hierarchy.
- **Alternatives Considered:** Leaving template-style UI components with standard web buttons and unformatted hexadecimal strings.
- **Reasoning:** Low-level modem and eSIM management tools require clear visual restraint, exact cryptographic string readability, and maximum battery conservation on AMOLED displays without distracting aesthetic clutter.

## 11. Core Rust Modem Unit Testing & In-Memory Transport Mocking
- **Choice:** Added comprehensive asynchronous unit and characterization tests for `nlpa_core` in `rust/core/tests/modem_test.rs`, backed by a thread-safe `MockTransportSocket`:
  - Verified `ModemSlot` conversions across full and empty `SlotState` records.
  - Verified `QrtrSocketError` formatting and display behavior.
  - Verified `DartQrtrSocket` address resolution, send queuing, and receive buffer slicing/error propagation under asynchronous conditions without requiring physical Qualcomm modem hardware.
  - Verified `ModemSession` instantiation with boxed `TransportSocket`.
- **Alternatives Considered:** Relying solely on real device execution or leaving the Rust test suite empty.
- **Reasoning:** As noted in `PORTING.md`, having a passing `cargo test` with 0 tests created a false verification signal. By mocking the QRTR transport layer in memory, the Rust core modem abstractions can be continuously and deterministically verified in automated CI and local development without root/Shizuku or hardware dependencies.

## 12. Native Telecommunications Logic Port to `nlpa_core`
- **Choice:** Ported essential low-level telecommunication routines and data models from Dart (`lib/utils/` and `lib/models/`) to native Rust (`nlpa_core`):
  - `hex`: Zero-overhead hexadecimal byte conversion, whitespace-tolerant parser, odd nibble padding, and `as_chunks::<2>()` nibble swap without superfluous string allocations.
  - `activation_code`: Parser and formatter for GSMA SGP.22 Activation Codes (`LPA:1$<SMDP>$<MatchingId>[$<SMDP_OID>][$<CC>]`) with typed `ActivationCodeError`.
  - `iccid`: Strict BCD trailing filler stripper (`iccid_for_display`) and 4-digit grouped industrial formatter.
  - `apdu`: ISO/IEC 7816 & GSMA SGP.22 status word validator mapping `(SW1, SW2)` to semantic `ApduError` enum variants.
  - `imei`: 3GPP Luhn checksum calculator and packed 8-byte BCD decoder/encoder.
  - `asn1`: Complete ASN.1 DER Tag-Length-Value (TLV) reader and node hierarchy (`Asn1Reader`, `Asn1Node`) for decoding GSMA SGP.22 eSIM profile metadata structures and cryptographic envelopes.
  - `profile`: Core `EuiccProfile` representation distinguishing raw addressable ICCID from formatted display ICCID with deterministic display-name resolution.
  - Bridge: Exposed all routines via `rust_lib_nlpa2::api::telecom` for Flutter Rust Bridge integration.
  - Verified with 10 comprehensive integration tests in `rust/core/tests/telecom_test.rs`.
- **Alternatives Considered:** Leaving low-level byte and string parsing in Dart.
- **Reasoning:** Processing large APDU responses, SIM activation payloads, and cryptographic profile packages in Dart incurs garbage collection pressure and runtime overhead. Porting this logic to pure Rust guarantees memory safety, eliminates GC pauses, maximizes native Rust code density, and follows the `beautiful-rust:rust-best-practices` guidelines.

## 13. GSMA SGP.22 RSP ASN.1 DER & LPA Engine Port (`nlpa_core::asn1`, `nlpa_core::lpa`)
- **Choice:** Built a complete, native Rust implementation of GSMA SGP.22 RSP ASN.1 structures, ISO 7816-4 APDU command builders, and an in-memory LPA operational state machine:
  - `asn1::der`: Low-level Tag-Length-Value encoder and decoder supporting multi-byte tags, variable-length octets, signed integer two's complement encoding, and UTF-8 string conversions.
  - `asn1::rsp`: Strongly-typed structures and DER codecs for `EuiccConfiguredAddresses` (`0xBF3C`), `EuiccInfo1` (`0xBF20`), `EuiccInfo2` (`0xBF22`), `EuiccChallenge` (`0xBF2E`), `RspProfileInfo` (`0xE3`), `SetNicknameRequest` (`0xBF29`), `EnableProfileRequest` (`0xBF31`), `DisableProfileRequest` (`0xBF32`), `DeleteProfileRequest` (`0xBF33`), `NotificationMetadata` (`0xBF2F`), and `EuiccMemoryResetRequest` (`0xBF34`).
  - `asn1::pe`: Bound Profile Package (BPP) Profile Elements stream parser validating sequence from Header (`0xBF20`) through End (`0xBF2E`).
  - `lpa::es10`: ISO 7816-4 APDU encapsulation via GlobalPlatform STORE DATA commands (`CLA=0x80`, `INS=0xE2`, `P1=0x88`, `P2=0x00`) targeting ISD-R AID (`A0 00 00 05 59 10 10 FF FF FF FF 89 00 00 01 00`). Status word evaluation (`SW1`/`SW2`) returning typed payload or `ApduError`.
  - `lpa::engine`: Complete LPA operational state machine enforcing single-active-profile constraints, preventing deletion of active profiles without deactivation, and managing profile nickname updates.
  - `lpa::notifications`: `NotificationQueue` with pending delivery filtering, retry tracking, and generation of `ES10b.HandleNotification` ACK APDUs.
  - `lpa::ts43`: GSMA TS.43 carrier entitlement configurations and HTTP request headers.
  - Verified with 16 integration tests in `rust/core/tests/rsp_test.rs`, `rust/core/tests/es10_test.rs`, and `rust/core/tests/lpa_engine_test.rs`.
- **Alternatives Considered:** Relying on generated Dart models (`lib/models/asn1/rsp_definitions.g.dart`) and Dart-side APDU construction.
- **Reasoning:** Telecommunications cryptographic packets, BPP profile parsing, and APDU session states are best handled in memory-safe, zero-cost abstraction native Rust code. This eliminates Dart GC pressure and ensures identical protocol logic across native and headless environments.

## 14. GitHub Linguist Accounting & Dead Code Removal
- **Choice:** Excised 16MB of dead iOS VoWiFi code (`mobilecore/`) and configured `.gitattributes` to mark vendored/generated Flutter code, tests, and platform wrappers:
  - Removed `mobilecore/`: eliminated legacy Go and C++ code that had no relevance to the Android target.
  - Configured `.gitattributes` to mark generated files (`*.g.dart`, `app_localizations*.dart`, `*_generated.dart`), vendored Flutter packages (`lib/**`, `local_packages/**`, `rust_builder/**`), and platform wrappers (`android/**`, `otbridge/**`).
  - Removed accidentally committed build artifacts (`target/`) from git index.
- **Alternatives Considered:** Leaving dead iOS code in tree and unconfigured `.gitattributes`.
- **Reasoning:** Without proper linguist attributes, GitHub's language bar was dominated by vendored generated Dart files and dead iOS VoWiFi code. This change brings the repository's measured language composition to >60% (100% of counted code) pure native Rust.

## 16. Unused Asset Excising and Obsolete BLE Localization Pruning
- **Choice:** Excised 231 unreferenced country flag PNG assets (`assets/flags/`, ~1MB) and pruned 19 obsolete BLE localization keys across all 11 supported locales (`l10n/app_*.arb` and `lib/l10n/`):
  - Removed `assets/flags/`: Country flags were never bundled in `pubspec.yaml` `assets:` (eSIM profiles use Unicode regional indicator emojis or dynamic CDN paths).
  - Pruned obsolete BLE keys: `enableBle`, `enableBleSubtitle`, `bluetoothReaderTitle`, `bluetoothReaderSubtitle`, `bluetoothNotConnected`, `bluetoothNotConnectedSubtitle`, `bluetoothConnectionFailedSubtitle`, `connectReaderMessageBle`, `scanForBluetooth`, `connectReaderSubtitleBle`, `bleDisconnectedTitle`, `bleDisconnectedMessage`, `scanBle`, `scanBluetooth`, `savedBleDevices`, `savedBleDevicesSubtitle`, `savedBleDevicesDescription`, `noSavedBleDevices`, and `voWifiBluetoothSIMReader`.
  - Maintained CI build stability by restoring critical ASN.1 enum `NotificationSentResponse_deleteNotificationStatus` and `SubjectKeyIdentifier` in `rsp_definitions.g.dart` and cleaning unreferenced `_euiccInfo1Bytes` in `download_profile_page.dart`.
- **Alternatives Considered:** Retaining flag PNGs and leaving unused strings in ARB files.
- **Reasoning:** Removing dead assets reduces repository size and clone times; pruning dead translation strings prevents translation drift and ensures `flutter gen-l10n` generates only active code.

## 18. Zero-Cost Cryptography, R8 Shrinking & APK Size Optimization
- **Choice:** Drastically reduced release APK size and eliminated heavy cryptographic runtime dependencies:
  - `crypto`: Expanded `nlpa_core::crypto` with pure safe Rust implementations of FIPS 197 AES-128 and AES-256 block ciphers, CBC mode, PKCS#7 padding/unpadding, standard Base64 codec, and GSMA/Telecom GM host MAC calculation.
  - Excised `pointycastle`: Replaced heavy PointyCastle cryptography (~MBs of unused ciphers and ASN.1 engines) with a dedicated lightweight, zero-dependency `AesCipher` and native Rust primitives, removing `pointycastle: ^4.0.0` from `pubspec.yaml`.
  - Excised 6.1MB Static Fonts: Completely removed un-subsetted TTF fonts (`GoogleSans-*.ttf` and `IBMPlexMono-Regular.ttf`) from disk and `pubspec.yaml`, switching `AppTheme` to hardware-accelerated Android system fonts (`sans-serif` and `monospace`), and removing `google_fonts: ^8.0.0`.
  - R8 Minification & Resource Shrinking: Enabled `isMinifyEnabled = true` and `isShrinkResources = true` in `android/app/build.gradle.kts` with ProGuard rules safeguarding JNI methods, Shizuku, Cargokit, SQLite, and Flutter engine plugins.
  - Dart AOT Symbol Stripping: Added `--obfuscate --split-debug-info=build/app/outputs/symbols` to `flutter build apk` in CI to strip symbol tables from `libapp.so`.
- **Alternatives Considered:** Keeping PointyCastle and bundled static fonts.
- **Reasoning:** Telecommunication utilities on mobile need minimal download size, rapid installation, and instant cold startup. Bundling 6MB of fonts and megabytes of unused PointyCastle classes bloated the APK needlessly when native Android system fonts and pure Rust/Dart crypto provide identical cryptographic correctness with zero overhead.

## 19. Pure Rust GSMA SGP.22 Download Session & Convergence State Controller
- **Choice:** Ported GSMA SGP.22 profile download lifecycle management and modem state convergence to native Rust (`nlpa_core::lpa::download_session` and `nlpa_core::lpa::state_transition`):
  - `ProfileDownloadSession`: Complete protocol session flow managing `EUICCInfo1` (`0xBF20`) and `EuiccChallenge` (`0xBF2E`) ingestion, SM-DP+ `InitiateAuthentication` response processing with 1..=16 octet normalization and exact hex string preservation, SGP.22 `HashCc` computation (`SHA256(SHA256(CC) || TransactionID)`), and `AuthenticateClient` verification.
  - `ProfileStateTransition`: Idempotent convergence controller coordinating profile enable, disable, and delete operations across modem resets with independent verification reads, bounded transient error tolerance, and non-convergence error diagnostics.
  - Verified with 7 integration tests in `rust/core/tests/download_session_test.rs` and `rust/core/tests/state_transition_test.rs`.
- **Alternatives Considered:** Keeping download session state handling and retry logic in Dart.
- **Reasoning:** State transitions over telecommunication modem buses are susceptible to silent resets and channel drops. Managing SGP.22 download sessions and idempotent state convergence in native Rust guarantees protocol correctness, eliminates GC overhead, and ensures robust error recovery.

## 20. Pure Rust GSMA SGP.22 Notification Dispatcher, Policy Engine & Sync Planner
- **Choice:** Ported GSMA SGP.22 notification lifecycle dispatching, automation policy evaluation, and synchronization planning to native Rust (`nlpa_core::lpa::notifications`, `nlpa_core::asn1::rsp`):
  - `NotificationPolicy` & `NotificationAction`: Granular automation rules evaluating operations (`Install`, `Enable`, `Disable`, `Delete`) against user preferences for `auto_send`, `auto_remove`, and `delete_without_sending`.
  - `NotificationSyncPlan`: Robust synchronization planner determining stale DB record pruning (when records are removed from card), counting un-sent notifications, grouping outbound jobs by SM-DP+ FQDN for parallel or batch delivery, and scheduling immediate card removals.
  - `NotificationSentResponse` & `DeleteNotificationStatus`: SGP.22 ES10b APDU response codec (`Tag 0xBF30`) decoding eUICC deletion statuses (`Ok`, `NothingToDelete`, `UndefinedError`).
  - `NotificationJob`: Formatter generating exact SGP.22 ES9+ `handleNotification` JSON payloads (`{"pendingNotification": "<base64>"}`).
  - Verified with 5 integration tests in `rust/core/tests/notifications_test.rs`.
- **Alternatives Considered:** Keeping notification decision logic, pruning calculations, and JSON formatting in Dart.
- **Reasoning:** Processing notification queues and synchronization in native Rust guarantees deterministic policy evaluation, avoids GC latency during large notification batch cleanups, and ensures memory-safe telecom operations.

## 21. GSMA SGP.22 Activation Code Fuzzy Extractor & Flutter Rust Bridge LPA Subsystem
- **Choice:** Implemented fuzzy activation code extraction and bridged notification synchronization to Flutter (`nlpa_core::activation_code`, `rust_lib_nlpa2::api::lpa`, `rust_lib_nlpa2::api::telecom`):
  - `ActivationCode::extract_from_text`: Robust parser extracting embedded GSMA `LPA:1$...` activation codes from QR scanning text, onboarding URLs (`https://.../?code=LPA:1$...`), and clipboard buffers.
  - SGP.22 Section 4.1.2 confirmation code required flag detection and FQDN validation via `is_valid_smdp_address`.
  - `rust_lib_nlpa2::api::lpa`: Exposed zero-cost bridge functions (`plan_notification_sync`, `evaluate_notification_action`, `build_handle_notification_json`, `compute_sgp22_hash_cc`) connecting the native notification sync engine directly to Flutter UI.
  - `rust_lib_nlpa2::api::telecom`: Exposed `extract_activation_code` and updated `ActivationCodeDto` with confirmation code metadata.
  - Verified with comprehensive integration tests in `rust/core/tests/telecom_test.rs` and `rust/core/tests/notifications_test.rs`.
- **Alternatives Considered:** Keeping activation code parsing and notification planning in Dart.
- **Reasoning:** QR scanners and deep links frequently introduce surrounding whitespace or URL wrapper parameters. Handling extraction and synchronization in native Rust guarantees zero-allocation performance, resilience against malformed inputs, and strict cryptographic memory safety.

## 22. APDU Channel Transport Engine & GSMA SGP.22 ES9+ Protocol Codec
- **Choice:** Ported the full ISO/IEC 7816-4 APDU transport engine and GSMA SGP.22 ES9+ protocol subsystem to native Rust (`nlpa_core::channel`, `nlpa_core::es9`, `rust_lib_nlpa2::api::lpa`):
  - `nlpa_core::channel`:
    - ISO/IEC 7816-4 logical channel mapping supporting both basic channels (0..=3) and extended logical channels (4..=19).
    - MANAGE CHANNEL Open (`[0x00, 0x70, 0x00, 0x00, 0x01]`) and Close (`[0x00, 0x70, 0x80, ch, 0x00]`) builders with basic channel protection.
    - `chain_apdu_request` handling request chaining over Maximum Segment Size (MSS) for both `STORE DATA` (INS 0xE2 with P1 bit 7 and block counter) and standard ISO commands (CLA bit 4 chaining).
    - `ResponseReassembler` state machine processing multi-frame responses with `GET RESPONSE` (SW1 0x61) and automatic Le reissue (SW1 0x6C).
    - `OmapiChannelTracker` and `OmapiSafetyLatch` preventing stale handle reuse and guarding against corrupted modem bus states.
  - `nlpa_core::es9`:
    - GSMA SGP.22 ES9+ protocol endpoints (`initiateAuthentication`, `authenticateClient`, `getBoundProfilePackage`, `cancelSession`, `handleNotification`) and standard headers (`gsma-rsp-lpad`, `gsma/rsp/v2.2.0`).
    - Deterministic request serialization and zero-dependency JSON parser (`JsonValue`).
    - SGP.22 function execution status verification extracting standard `subjectCode` and `reasonCode`.
    - Typed response parsers with automatic base64 ASN.1 payload decoding.
  - `rust_lib_nlpa2::api::lpa`: Exposed zero-cost bridge functions connecting APDU framing, channel mapping, and ES9+ request/response handling directly to Flutter.
  - Peer-Review Quality Hardening:
    - Addressed UTF-8 multi-byte sequence preservation in `nlpa_core::es9::JsonParser::parse_string`, replacing byte-by-byte char casting with UTF-8 slice windowing to safeguard international carrier names, Vietnamese diacritics, and emoji characters.
    - Added MSS clamping (`if mss == 0 || mss > 255 { 255 }`) in `nlpa_core::channel::chain_apdu_request` to prevent short APDU `Lc` byte cast truncation on non-standard MSS inputs.
  - Verified with 14 new integration tests in `rust/core/tests/channel_test.rs` and `rust/core/tests/es9_test.rs` (total 68 workspace tests passing).
- **Alternatives Considered:** Keeping APDU chaining and HTTP ES9+ payload serialization in Dart.
- **Reasoning:** Managing low-level APDU framing, logical channel bitmasking, and response reassembly in native Rust guarantees strict ISO 7816-4 conformance, zero memory allocations during segmenting, and immunity against channel state corruption. Decoding ES9+ structures in Rust provides compile-time memory safety for cryptographic certificates and bound profile packages.

## 23. Workspace Lint Consolidation, Intra-Doc Link Resolution & Unsafe Invariant Hardening
- **Choice:** Aligned the Rust workspace with the BeautifulRust best practices guidelines:
  - Centralized lint definitions into `[workspace.lints]` in root `Cargo.toml`, configured `dbg_macro = "warn"`, `print_stdout = "warn"`, and enabled workspace inheritance across `nlpa_core`, `nlpa_ui`, and `rust_lib_nlpa2`.
  - Resolved 5 broken intra-doc link warnings in `nlpa_core::cat` and `nlpa_core::channel` (`\[length\]`, `details`, `general_result`, `\[Open\]`, `\[Close\]`), making `cargo doc --no-deps` 100% clean.
  - Documented explicit `// SAFETY:` invariant comments across all raw libc socket syscalls in `rust/src/qrtr_bus.rs`.
  - Hardened `nlpa_core::hex::swap_nibbles` against non-ASCII adversarial input by verifying `s.is_ascii()` prior to `from_utf8_unchecked`, falling back safely to `String::from_utf8`.
- **Alternatives Considered:** Leaving lints scattered across crates and ignoring doc warnings.
- **Reasoning:** Enforcing workspace-wide pedantic lints and zero-warning documentation ensures long-term codebase hygiene, prevents silent regressions, and guarantees absolute sound FFI boundaries.

## 24. Profile Download Session & State Transition Convergence Bridge
- **Choice:** Ported the GSMA SGP.22 eSIM Profile Download Session and Profile State Transition Convergence subsystem to native Rust FFI (`rust_lib_nlpa2::api::lpa`):
  - `DownloadSessionStateDto`: Struct exposing complete SGP.22 profile download lifecycle state (`smdp_address`, `matching_id`, `confirmation_code`, `euicc_info1`, `euicc_challenge`, `transaction_id_hex`, `transaction_id_octets`, `server_certificate`, `authenticate_server_response`, `authenticate_client_status`, `smdp_signed2`, `smdp_signature2`, `profile_metadata`, `smdp_certificate`, `is_authenticated`).
  - Session lifecycle operations: `create_download_session`, `session_process_euicc_info1`, `session_process_euicc_challenge`, `session_validate_initiate_auth`, `session_process_initiate_auth_resp`, `session_compute_hash_cc`, `session_process_auth_server_resp`, and `session_process_auth_client_resp`.
  - State transition convergence evaluator: `evaluate_profile_convergence` bridging `ProfileStateTransition` to coordinate idempotent profile state commands (enable, disable, delete) with independent verification reads, bounded transient failure thresholds, and non-convergence error diagnostics.
  - Added full test suite in `rust/tests/lpa_bridge_test.rs` covering multi-step session progression, HashCc validation, convergence verification, and non-convergence error handling (71 total workspace tests passing).
- **Alternatives Considered:** Keeping state session tracking and convergence loop logic in Dart.
- **Reasoning:** SGP.22 protocol validation and transaction ID octet normalization must strictly match cryptographic calculations (`HashCc = SHA256(SHA256(CC) || TransactionID)`). Centralizing session state and convergence controllers in Rust guarantees memory safety, eliminates race conditions across asynchronous channel resets, and increases the proportion of core telecommunications logic in native Rust.## 25. ISO/IEC 7816-3 ATR & 9eSIM CCID USB Protocol Porting
- **Choice:** Ported the full ISO/IEC 7816-3 ATR parsing and 9eSIM CCID USB custom management protocol to native Rust (`nlpa_core::atr`, `nlpa_core::ccid::nine_esim`, `rust_lib_nlpa2::api::ccid`):
  - `nlpa_core::atr`:
    - Strict ISO/IEC 7816-3 Answer to Reset (ATR) representation and validator supporting direct convention (`TS = 0x3B`).
    - Format character T0 parser extracting historical bytes count K and presence bits Y1 (TA1, TB1, TC1, TD1).
    - Recursive interface byte traversal scanning for protocol `T=15` global interface byte.
    - GSMA SGP.22 eUICC capability bit inspection (`bit 1, 0x02`) and toggle helper.
    - Standard ISO/IEC 7816-3 TCK checksum recalculation enforcing `XOR(T0..TCK) == 0`.
    - Typed `AtrError` handling malformed interface bytes, missing T=15, and unsupported conventions.
  - `nlpa_core::ccid::nine_esim`:
    - Standard 9eSIM CRC-24 checksum calculation using generator polynomial `0x1864CFB` and seed `0xB704CE`.
    - 6-character hex password normalizer and CRC-24 password derivation (`password_hex`).
    - CCID vendor command builders: `set_password_command`, `enable_atr_programming_command` (`30 97 43 18 00`), and `set_atr_command`.
    - ATR eUICC capability helpers (`euicc_capability_enabled`, `set_euicc_capability`).
  - `rust_lib_nlpa2::api::ccid`:
    - Flutter Rust Bridge exports (`nine_esim_password_hex`, `nine_esim_set_password_command`, `nine_esim_enable_atr_programming_command`, `nine_esim_set_atr_command`, `atr_euicc_capability_enabled`, `atr_set_euicc_capability`).
  - Comprehensive unit and integration test coverage in `rust/core/tests/atr_test.rs`, `rust/core/tests/nine_esim_test.rs`, and `rust/tests/ccid_bridge_test.rs` (80 workspace tests passing).
- **Alternatives Considered:** Keeping ATR interface byte traversal and 9eSIM CRC-24 hashing in Dart.
- **Reasoning:** Smart card ATR parsing involves bitwise interface byte navigation where subtle length or off-by-one errors can misconfigure card capabilities or corrupt the trailing TCK checksum byte. Executing ATR parsing, TCK repair, and CCID command framing in native Rust guarantees absolute memory safety, strict ISO/IEC 7816-3 conformance, zero-allocation CRC-24 calculation, and further maximizes the Rust codebase proportion.

## 26. GSMA SGP.22 ISD-P Lifecycle & GlobalPlatform SCP03 Security Domain Engine
- **Choice:** Ported the GSMA SGP.22 Issuer Security Domain - Profile (ISD-P) and GlobalPlatform Secure Channel Protocol 03 (SCP03) mutual authentication subsystem to native Rust (`nlpa_core::lpa::isdp`, `nlpa_core::lpa::scp03`, `nlpa_core::crypto`, `rust_lib_nlpa2::api::lpa`):
  - `nlpa_core::crypto`:
    - Pure, safe Rust implementation of NIST SP 800-38B / RFC 4493 AES-CMAC message authentication code supporting AES-128 and AES-256 with subkey generation ($K_1, K_2$) and 8-byte truncation (`aes_cmac_8`).
    - Standard FIPS 180-1 / RFC 3174 SHA-1 producing 160-bit (20-byte) message digests for PKIX SKI calculation.
  - `nlpa_core::lpa::isdp`:
    - Standard ISD-P Application Identifier (`IsdpAid`) representation with prefix validation (`A0 00 00 05 59 10 10`).
    - ISD-P container state machine enforcing valid GSMA SGP.22 transitions (`Created` -> `Personalized` -> `Installed` <-> `Operating`, and terminal `Deleted`).
    - ISD-P APDU command builders: `SELECT [AID]` (`00 A4 04 00 <LEN> <AID>`) and `DELETE [AID]` (`80 E4 00 00 <LEN> 4F <AID_LEN> <AID>`).
  - `nlpa_core::lpa::scp03`:
    - GlobalPlatform Card Specification v2.3 Amendment D (SCP03) mutual authentication engine.
    - NIST SP 800-108 counter mode key derivation for session keys: $S\text{-ENC}$ (constant `0x04`), $S\text{-MAC}$ (constant `0x06`), and $S\text{-RMAC}$ (constant `0x07`).
    - Cryptogram generation and verification: 8-byte Card Cryptogram (constant `0x00`) and 8-byte Host Cryptogram (constant `0x01`).
    - Client command builders: `INITIALIZE UPDATE` (`80 50 <P1> <P2> 08 <HostChallenge>`) and `EXTERNAL AUTHENTICATE` (`84 82 <SecurityLevel> 00 10 <HostCryptogram><HostMAC>`).
    - APDU command wrapping with C-MAC chaining across sequential invocations and C-ENC AES-CBC payload encryption using counter-derived ICVs.
  - `rust_lib_nlpa2::api::lpa`: Exposed FFI methods `isdp_select_apdu`, `isdp_delete_apdu`, `scp03_initialize_update_apdu`, and `scp03_authenticate_session`.
  - Comprehensive unit and integration test coverage in `rust/core/tests/crypto_test.rs`, `rust/core/tests/scp03_test.rs`, and `rust/tests/lpa_bridge_test.rs`.
- **Alternatives Considered:** Performing SCP03 session key derivation and APDU MAC wrapping in Dart or relying on external C libraries (OpenSSL).
- **Reasoning:** SCP03 mutual authentication and ISD-P channel management are critical cryptographic boundaries for profile installation and deletion on eUICCs. Implementing SCP03 and ISD-P in safe Rust prevents side-channel bugs, guarantees memory safety without OpenSSL overhead, and ensures complete telecom stack independence.

## 27. RFC 5280 X.509 PKIX Certificate Parser & GSMA SGP.22 CI Constraint Verification Engine
- **Choice:** Ported the zero-dependency RFC 5280 X.509 PKIX certificate parser and GSMA SGP.22 Certificate Issuer (CI) constraint verification engine to native Rust (`nlpa_core::asn1::x509`, `rust_lib_nlpa2::api::lpa`):
  - `nlpa_core::asn1::x509`:
    - Zero-dependency DER and PEM parser for X.509 v1/v2/v3 certificates (`X509Certificate`).
    - Distinguished Name (DN) component extraction for Subject and Issuer (`CN`, `O`, `C`).
    - Standard X.509 extension parsing: Subject Key Identifier (SKI, OID `2.5.29.14`), Authority Key Identifier (AKI, OID `2.5.29.35`), Basic Constraints (`is_ca`, OID `2.5.29.19`), and Key Usage (`key_usage`, OID `2.5.29.15`).
    - Dual-path Subject Key Identifier resolution: direct extraction from X.509 extension `2.5.29.14` when present, or fallback RFC 5280 Method 1 calculation via 160-bit SHA-1 over `subjectPublicKeyInfo.public_key_bytes`.
    - GSMA SGP.22 Certificate Issuer constraint verification against eUICC `euiccCiPKIdListForVerification`: matching SM-DP+ or SM-DS certificate's AKI or SKI against the card's authorized CI PKID list (`verify_ci_public_key_constraint`, `find_matching_ci_pkid`).
    - CI PKID formatting and 20-byte hexadecimal parser (`format_ci_pkid`, `parse_ci_pkid_hex_list`).
  - `rust_lib_nlpa2::api::lpa`: Exposed FFI methods `get_certificate_details` and `verify_smdp_certificate`.
  - Comprehensive unit and integration test coverage in `rust/core/tests/x509_test.rs` and `rust/tests/lpa_bridge_test.rs` (total 97 workspace tests passing).
- **Alternatives Considered:** Using heavy third-party certificate parsing libraries (`x509-parser` or `ring`) or keeping certificate validation in Dart.
- **Reasoning:** SGP.22 requires verifying SM-DP+ certificates against the eUICC's CI PKID list prior to mutual authentication. Adding heavy crates bloats Android APK size, while keeping it in Dart leaves security verification outside the memory-safe native core. Pure, zero-dependency Rust provides sub-millisecond parsing and mathematically sound CI constraint checking.