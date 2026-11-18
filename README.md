![NekokoLPA2 — Trình quản lý eSIM thuần Rust tối ưu cho Android](docs/assets/readme-banner.png)

# NekokoLPA2
[![Crowdin](https://badges.crowdin.net/nekokolpa2/localized.svg)](https://crowdin.com/project/nekokolpa2)

[![Tải trên Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)

**Language:** [English](./README_en.md) | **Tiếng Việt** | [日本語](./README_ja-JP.md) | [简体中文](./README_zh-CN.md)

NekokoLPA2 (`2.5.0+606`) là ứng dụng quản lý eSIM thuần Rust 100% chuyên biệt dành cho nền tảng Android (kiến trúc ARM64 và ARMv7), kết hợp lõi giao thức SGP.22/TS.43 viết hoàn toàn bằng Rust (`nlpa_core`), giao diện Dioxus `0.6` SSR (`nlpa_ui`) chạy qua cầu nối JNI (`rust_lib_nlpa2`) trên Android System `WebView`, và lớp điều khiển phần cứng Kotlin giữ nguyên toàn bộ cơ chế an toàn OMAPI/SE HAL/CCID/QRTR.

[Bắt đầu](#bắt-đầu) · [Đầu đọc và nền tảng](#hỗ-trợ-nền-tảng) · [Tính năng](#tính-năng) · [Ghi chú cho Android](#ghi-chú-cho-android) · [Biên dịch từ mã nguồn](#biên-dịch-từ-mã-nguồn)

Tùy chọn tải về: [Các biến thể Android và bản build](docs/build-variants.md) · [So sánh OTBridge và ứng dụng Rooted](docs/android-root.md).

Khôi phục OMAPI trên Android: [Hồi phục lỗi khởi động lại](docs/omapi-recovery.md).

## Bắt đầu

1. Cài đặt ứng dụng từ [kênh phát hành GitHub](https://github.com/hzzmonetvn/NekokoLPA2/releases) hoặc Google Play.
2. Kết nối đầu đọc eUICC và bật kiểu kết nối tương ứng trong cài đặt đầu đọc thẻ (USB CCID, OMAPI, TMAPI, OTBridge, ADB Bridge, QRTR hoặc SE HAL).
3. Chọn đầu đọc thẻ và tải danh sách hồ sơ (profile). Sử dụng mã kích hoạt (activation code) hoặc mã QR để tải hồ sơ; bật, đổi tên hoặc xóa hồ sơ ngay từ danh sách hồ sơ của đầu đọc.

## Điểm nổi bật

- **100% Pure Rust + Android Kotlin Host**: Toàn bộ lõi SGP.22/ES9+/ES10x/ES11/TS.43/SCP03 (`nlpa_core`), giao diện Dioxus `0.6` SSR (`nlpa_ui`), và controller JNI (`rust_lib_nlpa2`) viết hoàn toàn bằng Rust, không dùng Flutter/Dart hay `flutter_rust_bridge`.
- **Chuyên biệt cho Android & ARM**: Tối ưu độc quyền cho Android trên các kiến trúc `arm64-v8a` và `armeabi-v7a` với dung lượng APK tối gọn.
- **Hỗ trợ Split APK per-ABI & Universal APK**: Tự động đóng gói APK tối ưu dung lượng cho từng kiến trúc ARM hoặc bản Universal tổng hợp qua GitHub Actions.
- **Giao diện AMOLED Pure Black & Hình nền tùy chỉnh**: Hệ màu đen tuyệt đối `#000000` tiết kiệm pin OLED/AMOLED, hỗ trợ chọn hình nền tùy chỉnh với thanh trượt độ mờ kính (glass opacity) hiển thị xuyên suốt cả khi đang tải hồ sơ và trong màn hình Cài đặt.
- **Bản địa hóa 11 ngôn ngữ đầy đủ (bao gồm Tiếng Việt)**: Nhúng trực tiếp toàn bộ 11 bộ ngôn ngữ từ `l10n/app_*.arb` trong Rust (`nlpa_core::i18n`), có thể chọn `Tiếng Việt (vi)` trực tiếp trong Cài đặt.
- **Hỗ trợ đa dạng đầu đọc thẻ**: Đầu đọc USB CCID, OMAPI, Telephony/TMAPI, ADB Bridge, modem QRTR, SE HAL, và đầu đọc từ xa RemoCard V2.
- **Hỗ trợ thiết bị Root & Magisk**: Tích hợp module `OTBridge` để bật hỗ trợ Telephony và bỏ qua các hạn chế OMAPI ARA-M.

## Hỗ trợ nền tảng

| Kiểu kết nối | Android (ARM64 / ARMv7) | Ghi chú |
| --- | --- | --- |
| Đầu đọc USB CCID | ✅ | Giao thức CCID tiêu chuẩn qua USB OTG (`im.nfc.ccid`) |
| Đầu đọc từ xa (RemoCard V2) | ✅ | Kết nối HTTP/JSON + tùy chọn mã hóa AES-256-CBC |
| OMAPI | ✅ | Android Open Mobile API với chốt an toàn `OmapiSafetyLatch` |
| Telephony / TMAPI | ✅ | Hỗ trợ trên bản Privileged hoặc qua `OTBridge` |
| `OTBridge` Provider | ✅ | Module mở rộng dành cho thiết bị Root |
| ADB Bridge | ✅ | Kết nối không cần Root/Shizuku qua `adb shell` một lần |
| SE HAL (Shizuku) | ✅ | Giao tiếp Secure Element HAL qua Shizuku |
| Modem QRTR | ✅ | Giao tiếp modem Qualcomm trực tiếp qua socket `AF_QIPCRTR` |

## Bản địa hóa & Dịch thuật

Ứng dụng hỗ trợ 11 ngôn ngữ (`en`, `vi`, `zh`, `zh-TW`, `ja`, `ko`, `de`, `es`, `fr`, `it`, `ru`) với Tiếng Việt là ngôn ngữ chính thức. Bản dịch được quản lý tập trung tại `l10n/app_*.arb` và trên [Crowdin](https://crowdin.com/project/nekokolpa2).

## Tính năng

- **Hỗ trợ đa đầu đọc thẻ**: USB CCID, OMAPI, Telephony API, ADB Bridge, modem QRTR, SE HAL, đầu đọc từ xa RemoCard V2.
- **Plugin tích hợp thuần Rust**: Hỗ trợ thẻ eSTK.me hai khe (`estk`), thẻ GM (`BF71`), và tra cứu thông tin thuê bao/gói cước từ `eSIM.cc`, `eSIM.gg`, `3HK`.
- **An toàn OMAPI & Chuyển đổi hồ sơ**: Bảo toàn chốt an toàn `OMAPI_SESSION_CORRUPTED`, không tự động gửi lại lệnh `6881` sau khi đã truyền, và kiểm tra đọc lại xác nhận khi chuyển đổi qua SE HAL.
- **Quản lý hồ sơ thông minh**: Dự đoán dung lượng profile, gắn thẻ phân loại (tags), đổi tên, ẩn dữ liệu nhạy cảm (redaction), và lưu trữ trạng thái nguyên tử dưới dạng JSON.

## Ghi chú cho Android

- NekokoLPA2 ưu tiên sử dụng `OTBridge` provider trên Android khi đã được cài đặt.
- Trên các thiết bị đã Root, `OTBridge` mở ra quyền truy cập Telephony/TMAPI và hỗ trợ vượt qua các hạn chế OMAPI ARA-M.
- Mã nguồn và các bản phát hành của OTBridge được duy trì tại: [thư mục otbridge](otbridge/).

## Biên dịch từ mã nguồn

> [!NOTE]
> **Lưu ý về môi trường biên dịch**: Toàn bộ quá trình biên dịch Rust/NDK (`cargo ndk`), kiểm thử (`cargo test --workspace`, `cargo clippy`), và đóng gói Android APK/Magisk được cấu hình chạy tự động trên **GitHub Actions** (`.github/workflows/build.yml`). Các bản build/test cục bộ trên máy phát triển bị tắt để đảm bảo tính đồng nhất với toolchain NDK/CI.

Trên GitHub Actions CI, quy trình sử dụng **Rust stable** (`aarch64-linux-android`, `armv7-linux-androideabi`), `cargo-ndk`, JDK 17, và Android SDK/NDK:

```sh
# Kiểm tra mã nguồn Rust trong CI
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps

# Biên dịch thư viện JNI + đóng gói APK trong CI
cargo ndk -t arm64-v8a -t armeabi-v7a -o android/app/src/main/jniLibs build --manifest-path rust/Cargo.toml --release
cd android && ./gradlew assembleCommunityRelease && cd ..
```

## Giấy phép

NekokoLPA2 được phân phối theo giấy phép [MIT License](LICENSE).
Tác giả: **hzzmonetvn**.
