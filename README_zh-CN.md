# NekokoLPA2
[![Crowdin](https://badges.crowdin.net/nekokolpa2/localized.svg)](https://crowdin.com/project/nekokolpa2)

[![Get it on Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)

**Language:** [English](./README_en.md) | [Tiếng Việt](./README.md) | [日本語](./README_ja-JP.md) | **简体中文**

NekokoLPA2 是一款专为 Android 平台（ARM64 与 ARMv7 架构）量身定制的高性能 eSIM 管理应用。深度集成 Rust 协议内核，支持操作本地 eUICC、USB CCID 读卡器、Qualcomm QRTR 调制解调器总线、SE HAL 以及远程读卡器端点。

[快速上手](#快速上手) · [读卡器与平台](#平台支持) · [功能特性](#功能特性) · [从源码构建](#从源码构建) · [问题排查](#问题排查)

下载说明：[Android 架构变体与构建分包](docs/build-variants.md) · [OTBridge 与 Root 特权方案对比](docs/android-root.md)。

## 快速上手

1. 从 [GitHub Releases](https://github.com/hzzmonetvn/NekokoLPA2/releases) 或 Google Play 下载并安装应用。
2. 连接 eUICC 读卡器，并在读卡器设置中启用对应的连接类型（USB CCID、OMAPI、TMAPI、QRTR 或 SE HAL）。
3. 选择读卡器并加载配置文件列表。支持通过激活码或扫描二维码下载 Profile；可直接管理启用、重命名或删除 Profile。

## 亮点

- **专为 Android & ARM 架构优化**：仅针对 Android 平台设计，支持 `arm64-v8a` 和 `armeabi-v7a`。
- **Rust 协议核心**：QRTR 调制解调器通信、协议引擎与 UI 绑定整合在单一 Cargo 工作区（`rust`、`rust/core`、`rust/ui`）。
- **Split APK 分包构建 & Universal 通用包**：自动化 CI 打包单 ABI 分包与多架构整合 APK。
- **AMOLED 纯黑主题**：深度黑底色设计，极佳对比度，针对 OLED/AMOLED 屏幕大幅省电。
- **全套越南语与多语言本地化**：界面与文档全面本地化支持。
- **Root 与 Magisk 模块集成**：内置 `OTBridge` 模块，支持 Telephony 访问与 OMAPI ARA-M 策略绕过。

## 平台支持

| 连接方式 | Android (ARM64 / ARMv7) | 说明 |
| --- | --- | --- |
| USB CCID 读卡器 | ✅ | 通过 USB OTG 访问标准 CCID 读卡器 |
| 远程读卡器 (Remote) | ✅ | 通过 Cloud Reader 端点连接 |
| OMAPI | ✅ | Android Open Mobile API |
| Telephony / TMAPI | ✅ | 通过 Shizuku / Bridge 访问支持设备 |
| `OTBridge` 提供方 | ✅ | 适用于 Root 设备的扩展模块 |
| SE HAL (Shizuku) | ✅ | 通过 Shizuku 访问 Secure Element HAL |
| QRTR Modem | ✅ | Qualcomm 基带调制解调器总线直连 |

## 功能特性

- **多硬件读卡器支持**：USB CCID、OMAPI、Telephony API、QRTR Modem、SE HAL 与远程读卡器。
- **Root 方案支持**：`OTBridge` 模块允许普通权限应用调用 Telephony/TMAPI 并突破 OMAPI ARA-M 规则限制。
- **纯黑 AMOLED 主题**：极佳视觉对比度与 OLED 屏幕能效表现。
- **配置文件管理**：支持自定义图标、备注、分类标签与有效期提醒。

## 从源码构建

需要 Flutter **3.47.2**、Android NDK r28c 以及 **Rust 1.89+**（目标 target：`aarch64-linux-android` 与 `armv7-linux-androideabi`）。

```sh
git clone --branch master https://github.com/hzzmonetvn/NekokoLPA2.git
cd NekokoLPA2
flutter pub get
flutter gen-l10n
```

### 代码检查与测试

```sh
dart analyze
flutter test
cargo test --workspace
```

### APK 打包分发 (Split APK per-ABI 与 Universal 通用包)

```sh
# Community 版本
flutter build apk --release --flavor community --target lib/main.dart
cd android && ./gradlew assembleCommunityRelease && cd ..

# Privileged (Root 特权) 版本
flutter build apk --release --flavor privileged --target lib/main.dart --dart-define=PRIVILEGED_BUILD=true
cd android && ./gradlew assemblePrivilegedRelease && cd ..
```

生成的 APK 文件位于 `build/app/outputs/flutter-apk/`:
- `app-arm64-v8a-<flavor>-release.apk`: 适用于 64-bit ARM 设备
- `app-armeabi-v7a-<flavor>-release.apk`: 适用于 32-bit ARM 设备
- `app-universal-<flavor>-release.apk`: 全架构通用安装包

## 许可证

NekokoLPA2 基于 [MIT License](LICENSE) 发布。
作者：**hzzmonetvn**。
