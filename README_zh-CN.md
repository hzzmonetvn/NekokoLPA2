# NekokoLPA2
[![Crowdin](https://badges.crowdin.net/nekokolpa2/localized.svg)](https://crowdin.com/project/nekokolpa2)

[![Get it on Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)

**Language:** [English](./README_en.md) | [Tiếng Việt](./README.md) | [日本語](./README_ja-JP.md) | **简体中文**

NekokoLPA2 (`2.5.0+606`) 是一款 100% 纯 Rust 实现、专为 Android 平台（ARM64 与 ARMv7 架构）打造的高性能 eSIM 管理应用。应用结合了无第三方加密依赖的 SGP.22/TS.43/SCP03 Rust 协议核心 (`nlpa_core`)、基于 Dioxus `0.6` SSR 的原生界面 (`nlpa_ui`) 以及直连 Android System `WebView` 的 JNI 桥接层 (`rust_lib_nlpa2`)，同时保留 Kotlin 硬件传输层对 OMAPI、SE HAL、Telephony、ADB Bridge、USB CCID 与 Qualcomm QRTR 调制解调器总线的全部安全保护机制。

[快速上手](#快速上手) · [读卡器与平台](#平台支持) · [功能特性](#功能特性) · [从源码构建](#从源码构建)

下载说明：[Android 架构变体与构建分包](docs/build-variants.md) · [OTBridge 与 Root 特权方案对比](docs/android-root.md)。

## 快速上手

1. 从 [GitHub Releases](https://github.com/hzzmonetvn/NekokoLPA2/releases) 或 Google Play 下载并安装应用。
2. 连接 eUICC 读卡器，并在读卡器设置中启用对应的连接类型（USB CCID、OMAPI、TMAPI、OTBridge、ADB Bridge、QRTR 或 SE HAL）。
3. 选择读卡器并加载配置文件列表。支持通过激活码或扫描二维码下载 Profile；可直接管理启用、重命名或删除 Profile。

## 亮点

- **100% 纯 Rust 核心与 UI + Kotlin 硬件宿主**: 彻底移除 Flutter/Dart 与 `flutter_rust_bridge`，全部核心逻辑 (`nlpa_core`)、Dioxus `0.6` SSR UI (`nlpa_ui`) 与 JNI 应用控制器 (`rust_lib_nlpa2`) 统一维护于 Cargo 工作区 (`rust`、`rust/core`、`rust/ui`)。
- **专为 Android & ARM 架构优化**: 仅针对 Android 平台设计，支持 `arm64-v8a` 和 `armeabi-v7a`。
- **Split APK 分包构建 & Universal 通用包**: 通过 GitHub Actions 自动化打包单 ABI 分包、多架构通用 APK 以及 Magisk Root 模块包。
- **AMOLED 纯黑主题与自定义壁纸玻璃**: `#000000` 纯黑背景搭配自定义壁纸与透明度滑块，在 Profile 加载遮罩与设置页面中均可透出背景壁纸。
- **11 种语言完整本地化 (`l10n/app_*.arb`)**: Rust 核心内置全部 11 套 ARB 语言资源，设置界面可直接切换 `Tiếng Việt (vi)` 等全部语言。
- **Root 与 Magisk 模块集成**: 内置 `OTBridge` 模块，支持 Telephony 访问与 OMAPI ARA-M 策略绕过。

## 平台支持

| 连接方式 | Android (ARM64 / ARMv7) | 说明 |
| --- | --- | --- |
| USB CCID 读卡器 | ✅ | 通过 USB OTG 访问标准 CCID 读卡器 |
| 远程读卡器 (RemoCard V2) | ✅ | 支持 HTTP JSON 及可选 AES-256-CBC 加密 |
| OMAPI | ✅ | 带 `OmapiSafetyLatch` 保护的 Android Open Mobile API |
| Telephony / TMAPI | ✅ | 特权构建或通过 `OTBridge` 访问 |
| `OTBridge` 提供方 | ✅ | 适用于 Root 设备的扩展模块 |
| ADB Bridge | ✅ | 免 Root / 免 Shizuku 的一次性 `adb shell` 会话连接 |
| SE HAL (Shizuku) | ✅ | 通过 Shizuku 访问 Secure Element HAL |
| QRTR Modem | ✅ | Qualcomm 基带调制解调器 `AF_QIPCRTR` 总线直连 |

## 功能特性

- **多硬件读卡器支持**: USB CCID、OMAPI、Telephony API、ADB Bridge、QRTR Modem、SE HAL 与 RemoCard V2 远程读卡器。
- **内置 Rust 插件**: 支持 eSTK.me 双卡槽 (`estk`)、GM 卡 (`BF71`) 以及 `eSIM.cc` / `eSIM.gg` / `3HK` 运营商状态徽章。
- **安全与隐私**: 保留 `OMAPI_SESSION_CORRUPTED` 熔断锁、`6881` 切换防重放、SE HAL 切换回读校验、EID/ICCID 脱敏规则与原子化 JSON 状态导入导出。

## 从源码构建

> [!NOTE]
> **本地构建已禁用**: 本机禁用 `cargo`、`rustc`、`gradle` 构建命令；所有工作区检查（`cargo fmt`、`cargo clippy`、`cargo test --workspace`、`cargo doc --no-deps`）及 Android APK 打包均在 **GitHub Actions** (`.github/workflows/build.yml`) 上运行。

```sh
# GitHub Actions CI 执行的检查与构建流程
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps
cargo ndk -t arm64-v8a -t armeabi-v7a -o android/app/src/main/jniLibs build --manifest-path rust/Cargo.toml --release
cd android && ./gradlew assembleCommunityRelease && cd ..
```

## 许可证

NekokoLPA2 基于 [MIT License](LICENSE) 发布。
作者：**hzzmonetvn**。
