# NekokoLPA2 - Pure Rust Android eSIM マネージャー
[![Crowdin](https://badges.crowdin.net/nekokolpa2/localized.svg)](https://crowdin.com/project/nekokolpa2)

[![Get it on Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)

**Language:** [English](./README_en.md) | [Tiếng Việt](./README.md) | **日本語** | [简体中文](./README_zh-CN.md)

NekokoLPA2 (`2.5.0+606`) は、Android プラットフォーム（ARM64 および ARMv7 アーキテクチャ）に特化した 100% Pure Rust 実装の高性能 eSIM 管理アプリケーションです。ゼロ依存のプロトコルコア (`nlpa_core`)、Dioxus `0.6` SSR UI (`nlpa_ui`)、および JNI ブリッジ (`rust_lib_nlpa2`) を Android System `WebView` 上で動作させ、Kotlin ハードウェア層を通じて本体内蔵 eUICC、USB CCID リーダー、Qualcomm QRTR モデム、SE HAL、ADB Bridge、リモートリーダーと通信します。

[はじめに](#はじめに) · [リーダーとプラットフォーム](#プラットフォームサポート) · [主な機能](#主な機能) · [ソースコードからのビルド](#ソースコードからのビルド)

## はじめに

1. [GitHub Releases](https://github.com/hzzmonetvn/NekokoLPA2/releases) または Google Play からアプリをインストールします。
2. eUICC カードリーダーを接続し、リーダー設定で該当する接続タイプを有効にします（USB CCID、OMAPI、TMAPI、OTBridge、ADB Bridge、QRTR、SE HAL）。
3. リーダーを選択してプロファイル一覧を読み込みます。アクティベーションコードまたは QR コードスキャンでプロファイルをダウンロードし、有効化・名前変更・削除などの操作を行えます。

## 主な特徴

- **100% Pure Rust コア & Dioxus 0.6 SSR UI**: Flutter/Dart および `flutter_rust_bridge` を完全に廃止し、`rust`、`rust/core`、`rust/ui` の単一 Cargo ワークスペースへ統合。
- **Android & ARM 専用設計**: `arm64-v8a` および `armeabi-v7a` アーキテクチャの Android に特化。
- **Split APK & Universal APK**: 単一 ABI 別スプリット APK および統合ユニバーサル APK を GitHub Actions で自動ビルド。
- **AMOLED ピュアブラック & カスタム壁紙**: 完全な黒背景 (`#000000`) に加え、読み込み中・設定画面でも透過ガラス効果を維持する壁紙および不透明度スライダーを搭載。
- **11 言語ローカライズ (`l10n/app_*.arb`)**: ベトナム語 (`Tiếng Việt (vi)`) を含む全 11 言語を Rust バイナリへ内蔵。
- **Root & Magisk モジュール対応**: Root 端末向け `OTBridge` モジュールによる Telephony サポートと OMAPI ARA-M 回避。

## プラットフォームサポート

| 接続タイプ | Android (ARM64 / ARMv7) | 備考 |
| --- | --- | --- |
| USB CCID リーダー | ✅ | USB OTG 経由の標準スマートカードリーダー |
| リモートリーダー (RemoCard V2) | ✅ | HTTP JSON + AES-256-CBC 暗号化対応 |
| OMAPI | ✅ | `OmapiSafetyLatch` 保護付き Android Open Mobile API |
| Telephony / TMAPI | ✅ | Privileged ビルドまたは `OTBridge` 経由 |
| `OTBridge` Provider | ✅ | Root 端末向け拡張機能 |
| ADB Bridge | ✅ | Root/Shizuku 不要のワンタイム `adb shell` 接続 |
| SE HAL (Shizuku) | ✅ | Shizuku 経由の Secure Element HAL |
| QRTR モデム | ✅ | Qualcomm 基盤モデムバス (`AF_QIPCRTR`) 直接通信 |

## 主な機能

- **多彩なハードウェアリーダー対応**: USB CCID、OMAPI、Telephony API、ADB Bridge、QRTR モデム、SE HAL、リモートリーダー。
- **内蔵 Rust プラグイン**: eSTK.me デュアルスロット (`estk`)、GM (`BF71`)、`eSIM.cc` / `eSIM.gg` / `3HK` プロバイダー連携。
- **AMOLED ダークテーマ & プライバシー保護**: 深い黒（#000000）を基調としたデザイン、EID/ICCID マスキング設定、JSON 状態エクスポート/インポート。

## ソースコードからのビルド

> [!NOTE]
> **ローカルビルドは無効化されています**: ローカル環境での `cargo` / `gradle` 実行は行わず、すべての検証（`cargo fmt`、`cargo clippy`、`cargo test --workspace`、`cargo doc --no-deps`）および Android APK パッケージングは **GitHub Actions** (`.github/workflows/build.yml`) 上で実行されます。

```sh
# GitHub Actions CI で実行される検証・ビルド手順
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps
cargo ndk -t arm64-v8a -t armeabi-v7a -o android/app/src/main/jniLibs build --manifest-path rust/Cargo.toml --release
cd android && ./gradlew assembleCommunityRelease && cd ..
```

## ライセンス

NekokoLPA2 は [MIT License](LICENSE) の下で公開されています。
作者: **hzzmonetvn**。
