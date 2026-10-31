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

