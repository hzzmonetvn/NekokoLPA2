![NekokoLPA2 — Trình quản lý eSIM thuần Rust tối ưu cho Android](docs/assets/readme-banner.png)

# NekokoLPA2
[![Crowdin](https://badges.crowdin.net/nekokolpa2/localized.svg)](https://crowdin.com/project/nekokolpa2)

[![Tải trên Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)

**Ngôn ngữ:** [English](./README_en.md) | **Tiếng Việt** | [日本語](./README_ja-JP.md) | [简体中文](./README_zh-CN.md)

NekokoLPA2 (`2.5.0+606`) là ứng dụng quản lý eSIM thuần Rust 100% chuyên biệt dành cho nền tảng Android (kiến trúc ARM64 và ARMv7), tối ưu hóa sâu với lõi giao thức Rust (`nlpa_core`) và giao diện Dioxus `0.6` SSR (`nlpa_ui`), hỗ trợ làm việc với các chip eUICC nội bộ, đầu đọc thẻ USB CCID, modem Qualcomm QRTR, SE HAL, ADB Bridge và các điểm cuối đọc thẻ từ xa RemoCard V2.

[Bắt đầu](#bắt-đầu) · [Đầu đọc và nền tảng](#hỗ-trợ-nền-tảng) · [Tính năng](#tính-năng) · [Ghi chú cho Android](#ghi-chú-cho-android) · [Biên dịch từ mã nguồn](#biên-dịch-từ-mã-nguồn)

Tùy chọn tải về: [Các biến thể Android và bản build](docs/build-variants.md) · [So sánh OTBridge và ứng dụng Rooted](docs/android-root.md).

Khôi phục OMAPI trên Android: [Hồi phục lỗi khởi động lại](docs/omapi-recovery.md).

## Bắt đầu

1. Cài đặt ứng dụng từ [kênh phát hành GitHub](https://github.com/hzzmonetvn/NekokoLPA2/releases) hoặc Google Play.
2. Kết nối đầu đọc eUICC và bật kiểu kết nối tương ứng trong cài đặt đầu đọc thẻ (USB CCID, OMAPI, TMAPI, OTBridge, ADB Bridge, QRTR hoặc SE HAL).
3. Chọn đầu đọc thẻ và tải danh sách hồ sơ (profile). Sử dụng mã kích hoạt (activation code) hoặc mã QR để tải hồ sơ; bật, đổi tên hoặc xóa hồ sơ ngay từ danh sách hồ sơ của đầu đọc.

## Điểm nổi bật

- **Kiến trúc thuần Rust 100% + Kotlin Hardware Host**: Toàn bộ lõi giao thức (`nlpa_core`), giao diện Dioxus `0.6` SSR (`nlpa_ui`) và JNI bridge (`rust_lib_nlpa2`) được hợp nhất trong workspace Rust (`rust`, `rust/core`, `rust/ui`), loại bỏ hoàn toàn Flutter/Dart và `flutter_rust_bridge`.
- **Chuyên biệt cho Android & ARM**: Tối ưu độc quyền cho Android trên các kiến trúc `arm64-v8a` và `armeabi-v7a`.
- **Hỗ trợ Split APK per-ABI & Universal APK**: Tự động đóng gói APK tối ưu dung lượng cho từng kiến trúc ARM hoặc bản Universal tổng hợp.
- **Giao diện AMOLED Pure Black & Hình nền kính mờ**: Hệ màu đen tuyệt đối `#000000` tiết kiệm pin trên màn hình OLED/AMOLED, hỗ trợ chọn hình nền tùy chỉnh với thanh trượt độ mờ kính (glass opacity) hiển thị ngay cả lúc đang tải và trong Cài đặt.
- **Bản địa hóa Tiếng Việt toàn diện**: Hỗ trợ chọn trực tiếp `Tiếng Việt (vi)` cùng 10 ngôn ngữ khác trong Cài đặt từ `l10n/app_*.arb`.
- **Hỗ trợ đa dạng đầu đọc thẻ**: Đầu đọc USB CCID, OMAPI, Telephony/TMAPI, ADB Bridge, modem QRTR, SE HAL, đầu đọc từ xa (RemoCard V2).
- **Hỗ trợ thiết bị Root & Magisk**: Tích hợp module `OTBridge` để bật hỗ trợ Telephony và bỏ qua các hạn chế OMAPI ARA-M.

## Hỗ trợ nền tảng

| Kiểu kết nối | Android (ARM64 / ARMv7) | Ghi chú |
| --- | --- | --- |
| Đầu đọc USB CCID | ✅ | Giao thức CCID tiêu chuẩn qua USB OTG |
| Đầu đọc từ xa (RemoCard V2) | ✅ | Kết nối qua Cloud Reader endpoint kèm tùy chọn AES-256-CBC |
| OMAPI | ✅ | Giao diện Open Mobile API của hệ thống Android |
| Telephony / TMAPI | ✅ | Hỗ trợ trên bản Privileged hoặc qua `OTBridge` |
| `OTBridge` Provider | ✅ | Module mở rộng dành cho thiết bị Root |
| ADB Bridge | ✅ | Kết nối không cần Root qua phiên `adb shell` một lần |
| SE HAL (Shizuku) | ✅ | Giao tiếp Secure Element HAL qua Shizuku |
| Modem QRTR | ✅ | Giao tiếp modem trực tiếp trên Qualcomm SoC |

## Bản địa hóa & Dịch thuật

Ứng dụng hỗ trợ 11 ngôn ngữ với Tiếng Việt là ngôn ngữ chính thức. Bản dịch được quản lý tập trung tại `l10n/app_*.arb` và trên [Crowdin](https://crowdin.com/project/nekokolpa2).

## Tính năng

- **Hỗ trợ đa đầu đọc thẻ**: USB CCID, OMAPI, Telephony API, ADB Bridge, modem QRTR, SE HAL, đầu đọc từ xa.
- **An toàn giao dịch eUICC**: Bảo toàn chốt an toàn `OMAPI_SESSION_CORRUPTED`, phân mảnh APDU `STORE DATA` mặc định `120` byte chuẩn `lpac`, và kiểm tra xác nhận đọc lại cho SE HAL.
- **Giải pháp cho máy Android đã Root**: `OTBridge` cho phép kích hoạt hỗ trợ Telephony và bỏ qua giới hạn OMAPI ARA-M trong khi vẫn giữ ứng dụng chính ở chế độ không đặc quyền.
- **Quản lý hồ sơ thông minh**: Dự đoán dung lượng profile, gắn thẻ phân loại (tags), đổi tên, ẩn dữ liệu nhạy cảm (redaction) và xuất/nhập cấu hình JSON.

## Ghi chú cho Android

- NekokoLPA2 ưu tiên sử dụng `OTBridge` provider trên Android khi đã được cài đặt.
- Trên các thiết bị đã Root, `OTBridge` mở ra quyền truy cập Telephony/TMAPI và hỗ trợ vượt qua các hạn chế OMAPI ARA-M.
- Mã nguồn và các bản phát hành của OTBridge được duy trì tại: [thư mục otbridge](otbridge/).

## Biên dịch từ mã nguồn

> [!NOTE]
> **Biên dịch cục bộ bị tắt**: Toàn bộ quá trình kiểm tra mã Rust (`cargo fmt`, `cargo clippy`, `cargo test --workspace`, `cargo doc --no-deps`) và đóng gói APK/Magisk chạy tự động trên **GitHub Actions** (`.github/workflows/build.yml`).

```sh
# Các bước kiểm tra & đóng gói chạy trên GitHub Actions CI
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps
cargo ndk -t arm64-v8a -t armeabi-v7a -o android/app/src/main/jniLibs build --manifest-path rust/Cargo.toml --release
cd android && ./gradlew assembleCommunityRelease && cd ..
```

## Giấy phép

NekokoLPA2 được phân phối theo giấy phép [MIT License](LICENSE).
Tác giả: **hzzmonetvn**.
