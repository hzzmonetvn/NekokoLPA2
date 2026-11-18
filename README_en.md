![NekokoLPA2 — Pure Rust Android eSIM manager](docs/assets/readme-banner.png)

# NekokoLPA2
[![Crowdin](https://badges.crowdin.net/nekokolpa2/localized.svg)](https://crowdin.com/project/nekokolpa2)

[![Get it on Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)

**Language:** **English** | [Tiếng Việt](./README.md) | [日本語](./README_ja-JP.md) | [简体中文](./README_zh-CN.md)

NekokoLPA2 (`2.5.0+606`) is a 100% Pure Rust Android eSIM management application specialized for ARM64 and ARMv7 Android devices. It combines a zero-dependency SGP.22/TS.43/SCP03 Rust protocol core (`nlpa_core`), a Dioxus `0.6` SSR user interface (`nlpa_ui`) rendered inside an Android `WebView` via a direct JNI bridge (`rust_lib_nlpa2`), and the native Kotlin hardware host layer preserving all OMAPI, SE HAL, Telephony, ADB Bridge, CCID, and QRTR safety guarantees.

[Get started](#get-started) · [Readers and platforms](#platform-support) · [Features](#features) · [Build from source](#build-from-source)

Choosing a download: [Android variants and builds](docs/build-variants.md) · [OTBridge versus the rooted app](docs/android-root.md).

## Get started

1. Install the app from [GitHub Releases](https://github.com/hzzmonetvn/NekokoLPA2/releases) or Google Play.
2. Connect an eUICC reader and enable its connection type in reader settings (USB CCID, OMAPI, TMAPI, OTBridge, ADB Bridge, QRTR, or SE HAL).
3. Select the reader and load its profiles. Use an activation code or QR code to download profiles; enable, rename, or delete profiles directly from the profile list.

## Highlights

- **100% Pure Rust Core & UI + Kotlin Hardware Host**: Protocol engines (`nlpa_core`), Dioxus `0.6` SSR UI (`nlpa_ui`), and JNI application controller (`rust_lib_nlpa2`) live in a unified Cargo workspace (`rust`, `rust/core`, `rust/ui`), with Flutter/Dart and `flutter_rust_bridge` completely removed.
- **Android & ARM Exclusive**: Strictly optimized for Android on `arm64-v8a` and `armeabi-v7a`.
- **Split APK per-ABI & Universal APK**: Automated GitHub Actions builds for split APKs per ARM ABI, universal APKs, and rooted Magisk packages.
- **AMOLED Pure Black & Custom Wallpaper Glass**: Deep `#000000` AMOLED theme with customizable background wallpaper and glass opacity slider visible across the profile list, loading overlay, and Settings.
- **11-Locale Localization (`l10n/app_*.arb`)**: All 11 ARB bundles including selectable `Tiếng Việt (vi)` are compiled into `nlpa_core::i18n`.
- **Root & Magisk Module**: Bundled `OTBridge` module for Telephony support and OMAPI ARA-M bypass on rooted devices.

## Platform Support

| Connection Type | Android (ARM64 / ARMv7) | Notes |
| --- | --- | --- |
| USB CCID readers | ✅ | Standard CCID readers via USB OTG (`im.nfc.ccid`) |
| Remote readers (RemoCard V2) | ✅ | HTTP JSON with optional AES-256-CBC encryption |
| OMAPI | ✅ | Android Open Mobile API with `OmapiSafetyLatch` |
| Telephony / TMAPI | ✅ | Privileged builds or via `OTBridge` |
| `OTBridge` Provider | ✅ | Extension module for rooted devices |
| ADB Bridge | ✅ | Rootless/Shizuku-less access via one-time `adb shell` |
| SE HAL (Shizuku) | ✅ | Secure Element HAL via Shizuku |
| QRTR Modem | ✅ | Direct Qualcomm modem `AF_QIPCRTR` socket communication |

## Features

- **Multi-Reader Hardware Support**: USB CCID, OMAPI, Telephony API, ADB Bridge, QRTR modem, SE HAL, and RemoCard V2 remote readers.
- **Built-In Rust Plugins**: eSTK.me dual-slot (`estk`), GM (`BF71`), and `eSIM.cc`, `eSIM.gg`, `3HK` provider badges.
- **Safety Invariants Preserved**: Permanent `OMAPI_SESSION_CORRUPTED` safety latch, no automatic `6881` switch replay after command submission, and SE HAL read-back verification.
- **Profile Tools**: Size prediction, redaction rules, tags, notification history, and atomic JSON state persistence.

## Build from source

> [!NOTE]
> **Local builds disabled**: Local `cargo`, `rustc`, `gradle`, and SDK builds are disabled in this development environment. All verification (`cargo fmt`, `cargo clippy`, `cargo test --workspace`, `cargo doc --no-deps`) and Android APK packaging run exclusively on **GitHub Actions** (`.github/workflows/build.yml`).

CI uses **Rust stable** (`aarch64-linux-android` and `armv7-linux-androideabi`), `cargo-ndk`, Java 17, and Android SDK/NDK:

```sh
# Rust workspace checks executed in GitHub Actions
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps

# Native JNI + Android APK build executed in GitHub Actions
cargo ndk -t arm64-v8a -t armeabi-v7a -o android/app/src/main/jniLibs build --manifest-path rust/Cargo.toml --release
cd android && ./gradlew assembleCommunityRelease && cd ..
```

## License

NekokoLPA2 is released under the [MIT License](LICENSE).
Author: **hzzmonetvn**.
