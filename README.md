![NekokoLPA2 — Trình quản lý eSIM đa nền tảng tối ưu cho Android](docs/assets/readme-banner.png)

# NekokoLPA2
[![Crowdin](https://badges.crowdin.net/nekokolpa2/localized.svg)](https://crowdin.com/project/nekokolpa2)

[![Tải trên Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)

**Language:** [English](./README_en.md) | **Tiếng Việt** | [日本語](./README_ja-JP.md) | [简体中文](./README_zh-CN.md)

NekokoLPA2 là ứng dụng quản lý eSIM chuyên biệt dành cho nền tảng Android (kiến trúc ARM64 và ARMv7), tối ưu hóa sâu với lõi Rust tốc độ cao, hỗ trợ làm việc với các chip eUICC nội bộ, đầu đọc thẻ qua USB CCID, modem Qualcomm QRTR, SE HAL, và các điểm cuối đọc thẻ từ xa.

[Bắt đầu](#bắt-đầu) · [Đầu đọc và nền tảng](#hỗ-trợ-nền-tảng) · [Tính năng](#tính-năng) · [Ghi chú cho Android](#ghi-chú-cho-android) · [Biên dịch từ mã nguồn](#biên-dịch-từ-mã-nguồn) · [Xử lý sự cố](#xử-lý-sự-cố)

Tùy chọn tải về: [Các biến thể Android và bản build](docs/build-variants.md) · [So sánh OTBridge và ứng dụng Rooted](docs/android-root.md).

Khôi phục OMAPI trên Android: [Hồi phục lỗi khởi động lại](docs/omapi-recovery.md).

## Bắt đầu

1. Cài đặt ứng dụng từ [kênh phát hành GitHub](https://github.com/hzzmonetvn/NekokoLPA2/releases) hoặc Google Play.
2. Kết nối đầu đọc eUICC và bật kiểu kết nối tương ứng trong cài đặt đầu đọc thẻ (USB CCID, OMAPI, TMAPI, QRTR, hoặc SE HAL).
3. Chọn đầu đọc thẻ và tải danh sách hồ sơ (profile). Sử dụng mã kích hoạt (activation code) hoặc mã QR để tải hồ sơ; bật, đổi tên hoặc xóa hồ sơ ngay từ danh sách hồ sơ của đầu đọc.

## Điểm nổi bật

- **Chuyên biệt cho Android & ARM**: Tối ưu độc quyền cho Android trên các kiến trúc `arm64-v8a` và `armeabi-v7a`.
- **Lõi xử lý viết bằng Rust**: Toàn bộ module giao thức (modem QRTR, protocol engine, UI bindings) được hợp nhất trong workspace Rust (`rust`, `rust/core`, `rust/ui`).
- **Hỗ trợ Split APK per-ABI & Universal APK**: Tự động đóng gói APK tối ưu dung lượng cho từng kiến trúc ARM hoặc bản Universal tổng hợp.
- **Giao diện AMOLED Pure Black**: Tối ưu tiết kiệm pin trên màn hình OLED/AMOLED với hệ màu đen tuyệt đối và độ tương phản cao.
- **Bản địa hóa Tiếng Việt toàn diện**: Hỗ trợ ngôn ngữ tiếng Việt hoàn chỉnh trong toàn bộ ứng dụng và tài liệu.
- **Hỗ trợ đa dạng đầu đọc thẻ**: Đầu đọc USB CCID, OMAPI, Telephony/TMAPI, modem QRTR, SE HAL, đầu đọc từ xa (Remote Reader).
- **Hỗ trợ thiết bị Root & Magisk**: Tích hợp module `OTBridge` để bật hỗ trợ Telephony và bỏ qua các hạn chế OMAPI ARA-M.

## Hỗ trợ nền tảng

| Kiểu kết nối | Android (ARM64 / ARMv7) | Ghi chú |
| --- | --- | --- |
| Đầu đọc USB CCID | ✅ | Giao thức CCID tiêu chuẩn qua USB OTG |
| Đầu đọc từ xa (Remote) | ✅ | Kết nối qua Cloud Reader endpoint |
| OMAPI | ✅ | Giao diện Open Mobile API của hệ thống Android |
| Telephony / TMAPI | ✅ | Hỗ trợ trên các thiết bị tương thích qua Shizuku / Bridge |
| `OTBridge` Provider | ✅ | Module mở rộng dành cho thiết bị Root |
| SE HAL (Shizuku) | ✅ | Giao tiếp Secure Element HAL qua Shizuku |
| Modem QRTR | ✅ | Giao tiếp modem trực tiếp trên Qualcomm SoC |

## Bản địa hóa & Dịch thuật

Ứng dụng hỗ trợ đa ngôn ngữ với tiếng Việt là một trong những ngôn ngữ chính thức. Bản dịch được quản lý tập trung trên [Crowdin](https://crowdin.com/project/nekokolpa2).

## Tính năng

- **Hỗ trợ đa đầu đọc thẻ**: USB CCID, OMAPI, Telephony API, modem QRTR, SE HAL, đầu đọc từ xa.
- **Truy vấn thông tin thẻ (SIM probing)**: Thẻ SIM thông thường có thể hiển thị thông tin nhà mạng và định danh trên màn hình khi đầu đọc cho phép truyền lệnh APDU thô.
- **Giải pháp cho máy Android đã Root**: `OTBridge` cho phép kích hoạt hỗ trợ Telephony và bỏ qua giới hạn OMAPI ARA-M trong khi vẫn giữ ứng dụng chính ở chế độ không đặc quyền.
- **Quản lý hồ sơ thông minh**: Tùy chỉnh biểu tượng, ghi chú, thẻ phân loại (tags) và thông báo nhắc nhở ngày hết hạn gói cước.
- **Giao diện AMOLED Dark Theme**: Trải nghiệm giao diện nền đen sâu hoàn toàn, bảo vệ mắt và tiết kiệm điện năng.

## Ghi chú cho Android

- NekokoLPA2 ưu tiên sử dụng `OTBridge` provider trên Android khi đã được cài đặt.
- Trên các thiết bị đã Root, `OTBridge` mở ra quyền truy cập Telephony/TMAPI và hỗ trợ vượt qua các hạn chế OMAPI ARA-M.
- Mã nguồn và các bản phát hành của OTBridge được duy trì tại: [thư mục otbridge](otbridge/).

## Biên dịch từ mã nguồn

Dự án yêu cầu Flutter phiên bản **3.47.2**, Android NDK r28c, và **Rust 1.89+** với các target `aarch64-linux-android` và `armv7-linux-androideabi`.

```sh
git clone --branch master https://github.com/hzzmonetvn/NekokoLPA2.git
cd NekokoLPA2
flutter pub get
flutter gen-l10n
```

### Kiểm tra và kiểm thử

```sh
dart analyze
flutter test
cargo test --workspace
```

### Đóng gói APK (Split APK per-ABI & Universal)

```sh
# Bản Community
flutter build apk --release --flavor community --target lib/main.dart
cd android && ./gradlew assembleCommunityRelease && cd ..

# Bản Privileged (Rooted)
flutter build apk --release --flavor privileged --target lib/main.dart --dart-define=PRIVILEGED_BUILD=true
cd android && ./gradlew assemblePrivilegedRelease && cd ..
```

Các file APK sẽ được tạo tại `build/app/outputs/flutter-apk/`:
- `app-arm64-v8a-<flavor>-release.apk`: Dành cho thiết bị 64-bit ARM
- `app-armeabi-v7a-<flavor>-release.apk`: Dành cho thiết bị 32-bit ARM
- `app-universal-<flavor>-release.apk`: Bản đa kiến trúc tổng hợp

## Giấy phép

NekokoLPA2 được phân phối theo giấy phép [MIT License](LICENSE).
Tác giả: **hzzmonetvn**.
