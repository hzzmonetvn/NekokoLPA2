# NekokoLPA2
[![Crowdin](https://badges.crowdin.net/nekokolpa2/localized.svg)](https://crowdin.com/project/nekokolpa2)


[![Download on the App Store](https://img.shields.io/badge/App%20Store-Download-0D96F6?logo=appstore&logoColor=white&style=for-the-badge)](https://apps.apple.com/en/app/nekokolpa-2/id6757540723)
[![Get it on Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)


**Language:** [English](./README.md) | [Tiếng Việt](./README_vi-VN.md) | [日本語](./README_ja-JP.md) | **简体中文**


NekokoLPA2 是一款跨平台 eSIM 管理应用，可操作本机 eUICC、外置读卡器以及远程读卡器端点。它面向需要比运营商自带应用更精细地掌控配置文件操作、传输方式和卡片可见性的用户。

## 亮点

- 支持 Android、iOS、macOS、Linux、Windows 和 Chrome
- 支持 BLE 读卡器、USB CCID 读卡器、OMAPI、Telephony/TMAPI、远程读卡器，以及浏览器侧的 WebUSB 通道
- 提供自定义图标、备注、标签和定时通知等配置文件管理工具
- 在已 root 的 Android 设备上，`OTBridge` 可启用 Telephony 支持并绕过 OMAPI ARA-M 限制，而无需将 NekokoLPA2 本身安装为特权应用

## 平台支持

| 连接方式 | Android | iOS | macOS | Linux | Windows | Chrome |
| --- | --- | --- | --- | --- | --- | --- |
| BLE 读卡器 | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ |
| USB CCID 读卡器 | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| 远程读卡器 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| OMAPI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Telephony / TMAPI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `OTBridge` 提供方 | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| WebUSB SCRP / WebCard | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |

说明：
- `Telephony / TMAPI` 和 `OTBridge` 仅适用于 Android。
- `OTBridge` 适用于需要 Telephony 访问权限或绕过 OMAPI 策略的已 root Android 环境。
- Chrome 支持指的是在具备所需浏览器 API 的 Chromium 浏览器中运行的 Web 版本。

## 翻译

翻译工作在 [Crowdin](https://crowdin.com/project/nekokolpa2) 上进行。

## 功能

- **多读卡器支持**：BLE、USB CCID、OMAPI、Telephony API、远程读卡器以及基于浏览器的传输方式
- **已 root 的 Android 通道**：`OTBridge` 可启用 Telephony 支持并绕过 OMAPI ARA-M 限制，同时让主应用保持非特权状态
- **配置文件管理**：自定义图标、备注、标签和简洁的管理工具
- **定时通知**：针对到期日和其他配置文件相关事件的提醒
- **跨平台界面**：自适应布局，并针对各平台调整读卡器操作流程

## Android 说明

- 在 Android 上，若已安装外部 `OTBridge` 提供方，NekokoLPA2 会优先使用它。
- 在已 root 的 Android 设备上，`OTBridge` 可提供 Telephony/TMAPI 访问权限并绕过 OMAPI ARA-M 限制，同时让 NekokoLPA2 本身保持普通应用的安装方式。
- `OTBridge` 的版本发布见：[iebb/OTBridge](https://github.com/iebb/OTBridge/releases)

## 下载

- [![Download on the App Store](https://img.shields.io/badge/App%20Store-Download-0D96F6?logo=appstore&logoColor=white&style=for-the-badge)](https://apps.apple.com/en/app/nekokolpa-2/id6757540723)
- [![Get it on Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)
- **GitHub Releases**：[iebb/NekokoLPA2 releases](https://github.com/iebb/NekokoLPA2/releases)
- **TestFlight**：[加入 TestFlight](https://testflight.apple.com/join/bP38fzC4)
- **Web**：[web.lpa.ee](https://web.lpa.ee)

## Star History

[![Star History Chart](https://api.star-history.com/chart?repos=iebb/NekokoLPA2%2Ciebb/NekokoLPA&type=date&legend=top-left)](https://www.star-history.com/?repos=iebb%2FNekokoLPA2%2Ciebb%2FNekokoLPA&type=date&legend=top-left)

## 更新日志

版本历史和面向用户的变更请参阅 [CHANGELOG.md](CHANGELOG.md)。

## Wiki

- [Wiki 首页](https://github.com/iebb/NekokoLPA2/wiki)
- [快速上手](https://github.com/iebb/NekokoLPA2/wiki/Getting-Started)
- [功能介绍](https://github.com/iebb/NekokoLPA2/wiki/Features)
- [读卡器与平台](https://github.com/iebb/NekokoLPA2/wiki/Readers-and-Platforms)
- [问题排查](https://github.com/iebb/NekokoLPA2/wiki/Troubleshooting)

## 支持

如需报告问题、提出功能建议或咨询，请访问我们的 [GitHub Issues](https://github.com/iebb/NekokoLPA2/issues)。

## 许可证

NekokoLPA2 基于 [MIT License](LICENSE) 发布。

NekokoLPA 的猫咪吉祥物美术作品由 @sanzennami 创作。可以引用，但未经同意不得将该吉祥物用作你自己的标志、应用图标、店铺头像或主要品牌标识。仅当该美术作品占整个标志或图片的比例不超过 10%，且其使用不会让人误以为获得官方认可时，才可将其作为标志或图片的一部分。详见 [NOTICE](NOTICE) 和 [BRANDING.md](BRANDING.md)。

---

© 2026 Nekoko.
