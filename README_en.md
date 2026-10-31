![NekokoLPA2 — Android-specialized eSIM manager](docs/assets/readme-banner.png)

# NekokoLPA2
[![Crowdin](https://badges.crowdin.net/nekokolpa2/localized.svg)](https://crowdin.com/project/nekokolpa2)

[![Get it on Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)

**Language:** **English** | [Tiếng Việt](./README.md) | [日本語](./README_ja-JP.md) | [简体中文](./README_zh-CN.md)

NekokoLPA2 is an eSIM management application specialized for Android (ARM64 and ARMv7 architectures). It features high-performance Rust protocol integration for working with local eUICCs, USB CCID readers, Qualcomm QRTR modems, SE HAL, and remote reader endpoints.

[Get started](#get-started) · [Readers and platforms](#platform-support) · [Features](#features) · [Build from source](#build-from-source) · [Troubleshooting](#troubleshooting)

Choosing a download: [Android variants and builds](docs/build-variants.md) · [OTBridge versus the rooted app](docs/android-root.md).

## Get started

1. Install the app from [GitHub Releases](https://github.com/hzzmonetvn/NekokoLPA2/releases) or Google Play.
2. Connect an eUICC reader and enable its connection type in reader settings (USB CCID, OMAPI, TMAPI, QRTR, or SE HAL).
3. Select the reader and load its profiles. Use an activation code or QR code to download profiles; enable, rename, or delete profiles directly from the profile list.

## Highlights

- **Android & ARM Exclusive**: Focused strictly on Android supporting `arm64-v8a` and `armeabi-v7a`.
- **Rust Core & Protocols**: Modem QRTR services, protocol engines, and UI bindings are maintained in a unified Cargo workspace (`rust`, `rust/core`, `rust/ui`).
- **Split APK per-ABI & Universal APK**: Automated builds for split APKs per ARM ABI and universal APK packaging.
- **AMOLED Dark Theme**: Deep black theme optimized for battery saving and high contrast on OLED/AMOLED screens.
- **Vietnamese Localization**: Full Vietnamese language support in the app interface and documentation.
- **Root & Magisk Module**: Bundled `OTBridge` module for Telephony support and OMAPI ARA-M bypass on rooted devices.

## Platform Support

| Connection Type | Android (ARM64 / ARMv7) | Notes |
| --- | --- | --- |
| USB CCID readers | ✅ | Standard CCID readers via USB OTG |
| Remote readers | ✅ | Connection via Cloud Reader endpoints |
| OMAPI | ✅ | Android Open Mobile API |
| Telephony / TMAPI | ✅ | Supported devices via Shizuku / Bridge |
| `OTBridge` Provider | ✅ | Extension module for rooted devices |
| SE HAL (Shizuku) | ✅ | Secure Element HAL via Shizuku |
| QRTR Modem | ✅ | Direct Qualcomm modem bus communication |

## Build from source

Requires Flutter **3.47.2**, Android NDK r28c, and **Rust 1.89+** (`aarch64-linux-android` and `armv7-linux-androideabi`).

```sh
git clone --branch master https://github.com/hzzmonetvn/NekokoLPA2.git
cd NekokoLPA2
flutter pub get
flutter gen-l10n
```

### Validate and Test

```sh
dart analyze
flutter test
cargo test --workspace
```

### Packaging APKs (Split APK per-ABI & Universal)

```sh
# Community flavor
flutter build apk --release --flavor community --target lib/main.dart
cd android && ./gradlew assembleCommunityRelease && cd ..

# Privileged (Rooted) flavor
flutter build apk --release --flavor privileged --target lib/main.dart --dart-define=PRIVILEGED_BUILD=true
cd android && ./gradlew assemblePrivilegedRelease && cd ..
```

The APK files will be generated in `build/app/outputs/flutter-apk/`:
- `app-arm64-v8a-<flavor>-release.apk`: For 64-bit ARM devices
- `app-armeabi-v7a-<flavor>-release.apk`: For 32-bit ARM devices
- `app-universal-<flavor>-release.apk`: Universal multi-ABI package

## License

NekokoLPA2 is released under the [MIT License](LICENSE).
Author: **hzzmonetvn**.
