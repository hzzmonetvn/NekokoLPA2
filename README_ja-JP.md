# NekokoLPA2 - 高度な eSIM マネージメント
[![Crowdin](https://badges.crowdin.net/nekokolpa2/localized.svg)](https://crowdin.com/project/nekokolpa2)

[![Get it on Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)

**Language:** [English](./README_en.md) | [Tiếng Việt](./README.md) | **日本語** | [简体中文](./README_zh-CN.md)

NekokoLPA2 は、Android プラットフォーム（ARM64 および ARMv7 アーキテクチャ）に特化した高性能な eSIM 管理アプリケーションです。ネイティブ Rust コアとプロトコル統合により、本体内蔵 eUICC、USB CCID リーダー、Qualcomm QRTR モデム、SE HAL、およびリモートリーダーとの通信を直接処理します。

[はじめに](#はじめに) · [リーダーとプラットフォーム](#プラットフォームサポート) · [主な機能](#主な機能) · [ソースコードからのビルド](#ソースコードからのビルド)

## はじめに

1. [GitHub Releases](https://github.com/hzzmonetvn/NekokoLPA2/releases) または Google Play からアプリをインストールします。
2. eUICC カードリーダーを接続し、リーダー設定で該当する接続タイプを有効にします（USB CCID、OMAPI、TMAPI、QRTR、SE HAL）。
3. リーダーを選択してプロファイル一覧を読み込みます。アクティベーションコードまたは QR コードスキャンでプロファイルをダウンロードし、有効化・名前変更・削除などの操作を行えます。

## 主な特徴

- **Android & ARM 専用設計**: `arm64-v8a` および `armeabi-v7a` アーキテクチャの Android に特化。
- **Rust コア & プロトコル実装**: QRTR モデム通信、プロトコルエンジン、UI バインディングを単一の Cargo ワークスペース（`rust`、`rust/core`、`rust/ui`）で管理。
- **Split APK & Universal APK**: 単一 ABI 別スプリット APK および統合ユニバーサル APK の自動ビルド対応。
- **AMOLED ピュアブラックテーマ**: OLED/AMOLED 画面で優れた省電力性とハイコントラストを実現する完全な黒背景デザイン。
- **ベトナム語および多言語ローカライズ**: 多言語インターフェースおよびドキュメントに対応。
- **Root & Magisk モジュール対応**: Root 端末向け `OTBridge` モジュールによる Telephony サポートと OMAPI ARA-M 回避。

## プラットフォームサポート

| 接続タイプ | Android (ARM64 / ARMv7) | 備考 |
| --- | --- | --- |
| USB CCID リーダー | ✅ | USB OTG 経由の標準スマートカードリーダー |
| リモートリーダー | ✅ | クラウドリーダーエンドポイント経由での接続 |
| OMAPI | ✅ | Android Open Mobile API |
| Telephony / TMAPI | ✅ | Shizuku / Bridge 経由での接続 |
| `OTBridge` Provider | ✅ | Root 端末向け拡張機能 |
| SE HAL (Shizuku) | ✅ | Shizuku 経由の Secure Element HAL |
| QRTR モデム | ✅ | Qualcomm 基盤モデムバスとの直接通信 |

## 主な機能

- **多彩なハードウェアリーダー対応**: USB CCID、OMAPI、Telephony API、QRTR モデム、SE HAL、リモートリーダー。
- **Root 環境サポート**: `OTBridge` により、特権なしアプリから Telephony/TMAPI にアクセスし、OMAPI ARA-M 制限を回避。
- **AMOLED ダークテーマ**: 深い黒（#000000）を基調とした高コントラストかつ目に優しいデザイン。
- **プロファイル管理**: カスタムアイコン、メモ、タグ分類、有効期限通知機能。

## ソースコードからのビルド

Flutter **3.47.2**、Android NDK r28c、**Rust 1.89+**（`aarch64-linux-android` および `armv7-linux-androideabi` ターゲット）が必要です。

```sh
git clone --branch master https://github.com/hzzmonetvn/NekokoLPA2.git
cd NekokoLPA2
flutter pub get
flutter gen-l10n
```

### 検証とテスト

```sh
dart analyze
flutter test
cargo test --workspace
```

### APK パッケージング (Split APK per-ABI & Universal)

```sh
# Community フレーバー
flutter build apk --release --flavor community --target lib/main.dart
cd android && ./gradlew assembleCommunityRelease && cd ..

# Privileged (Root 端末用) フレーバー
flutter build apk --release --flavor privileged --target lib/main.dart --dart-define=PRIVILEGED_BUILD=true
cd android && ./gradlew assemblePrivilegedRelease && cd ..
```

ビルドされた APK は `build/app/outputs/flutter-apk/` に出力されます:
- `app-arm64-v8a-<flavor>-release.apk`: 64-bit ARM 端末用
- `app-armeabi-v7a-<flavor>-release.apk`: 32-bit ARM 端末用
- `app-universal-<flavor>-release.apk`: 統合ユニバーサルパッケージ

## ライセンス

NekokoLPA2 は [MIT License](LICENSE) の下で公開されています。
作者: **hzzmonetvn**。
