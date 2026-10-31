![NekokoLPA2 — Trình quản lý eSIM đa nền tảng](docs/assets/readme-banner.png)

# NekokoLPA2
[![Crowdin](https://badges.crowdin.net/nekokolpa2/localized.svg)](https://crowdin.com/project/nekokolpa2)

[![Tải trên App Store](https://img.shields.io/badge/App%20Store-Download-0D96F6?logo=appstore&logoColor=white&style=for-the-badge)](https://apps.apple.com/en/app/nekokolpa-2/id6757540723)
[![Tải trên Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)

**Ngôn ngữ:** [English](./README.md) | **Tiếng Việt** | [日本語](./README_ja-JP.md) | [简体中文](./README_zh-CN.md)

NekokoLPA2 là ứng dụng quản lý eSIM đa nền tảng mạnh mẽ dùng để làm việc với các chip eUICC nội bộ, đầu đọc thẻ ngoài và các điểm cuối đọc thẻ từ xa. Ứng dụng được thiết kế dành cho người dùng cần toàn quyền kiểm soát các thao tác cấu hình (profile), tùy chọn giao thức truyền tải và khả năng hiển thị thẻ chi tiết hơn so với các ứng dụng nhà mạng thông thường.

[Bắt đầu](#bắt-đầu) · [Đầu đọc và nền tảng](#hỗ-trợ-nền-tảng) · [Tính năng](#tính-năng) · [Ghi chú cho Android](#ghi-chú-cho-android) · [Biên dịch từ mã nguồn](#biên-dịch-từ-mã-nguồn) · [Xử lý sự cố](#xử-lý-sự-cố)

Tùy chọn tải về: [Các biến thể Android và bản build](docs/build-variants.md) · [So sánh OTBridge và ứng dụng Rooted](docs/android-root.md).

Khôi phục OMAPI trên Android: [Hồi phục lỗi khởi động lại](docs/omapi-recovery.md).

## Bắt đầu

1. Cài đặt ứng dụng từ [kênh tải về chính thức](#tải-về).
2. Kết nối đầu đọc eUICC và bật kiểu kết nối tương ứng trong cài đặt đầu đọc thẻ. Đối với Bluetooth, cấp quyền Bluetooth được yêu cầu và quét tìm tên đầu đọc hiển thị.
3. Chọn đầu đọc thẻ và tải danh sách hồ sơ (profile). Sử dụng mã kích hoạt (activation code) hoặc mã QR để tải hồ sơ; bật, đổi tên hoặc xóa hồ sơ ngay từ danh sách hồ sơ của đầu đọc.

*Lưu ý: Đầu đọc bên ngoài chỉ quản lý thẻ gắn vào đầu đọc đó. Nó không biến eSIM bên trong của iPhone thành thẻ có thể truy cập từ bên ngoài. Khả năng truy cập thẻ nội bộ trên Android phụ thuộc vào giao diện OMAPI, bridge hoặc modem có sẵn cùng với các quy tắc truy cập của thẻ.*

## Điểm nổi bật

- Tối ưu hóa sâu cho nền tảng Android, hỗ trợ kiến trúc ARM (ARM64 và ARMv7)
- Hỗ trợ đa dạng đầu đọc: BLE, đầu đọc USB CCID, OMAPI, Telephony/TMAPI, đầu đọc từ xa và WebUSB
- Công cụ tổ chức hồ sơ thông minh: biểu tượng tùy chỉnh, ghi chú, thẻ (tags) và thông báo theo lịch
- Trên các thiết bị Android đã Root, `OTBridge` cho phép kích hoạt hỗ trợ Telephony và bỏ qua các giới hạn OMAPI ARA-M mà không cần cài đặt ứng dụng chính dưới dạng đặc quyền
- Hỗ trợ giao diện AMOLED tối ưu pin và trải nghiệm hiển thị

## Hỗ trợ nền tảng

| Kiểu kết nối | Android | iOS | macOS | Linux | Windows | Chrome |
| --- | --- | --- | --- | --- | --- | --- |
| Đầu đọc BLE | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ |
| Đầu đọc USB CCID | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Đầu đọc từ xa (Remote) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| OMAPI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Telephony / TMAPI | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `OTBridge` Provider | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| SE HAL (Shizuku) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| WebUSB SCRP / WebCard | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |

Ghi chú:
- `Telephony / TMAPI` và `OTBridge` là các đường dẫn dành riêng cho Android.
- `OTBridge` dành cho các thiết bị Android đã root cần quyền truy cập Telephony hoặc vượt qua chính sách OMAPI.
- `SE HAL` là tính năng thử nghiệm. Cần Shizuku và chỉ hoạt động khi thiết bị cho phép `shell` gọi Secure Element HAL.

## Bản địa hóa & Dịch thuật

Bản dịch và ngôn ngữ được quản lý tập trung trên [Crowdin](https://crowdin.com/project/nekokolpa2).

## Tính năng

- **Hỗ trợ đa đầu đọc thẻ**: BLE, USB CCID, OMAPI, Telephony API, đầu đọc từ xa
- **Đầu đọc TYBLE**: Đầu đọc Tianyu với tiền tố tên như `SCT-BT-<UID>`, `TES-BT-<UID>`, và đầu đọc 9eSIM B0 Plus mang tên `9eSIM` theo sau là 8 chữ số.
- **Dò thông tin SIM (SIM Probing)**: Thẻ SIM vật lý thông thường có thể hiển thị thông tin nhà mạng và danh tính trên màn hình khi đầu đọc kết nối cho phép gửi APDU thô.
- **Đường dẫn Android đã Root**: `OTBridge` giúp kích hoạt hỗ trợ Telephony và vượt qua giới hạn OMAPI ARA-M trong khi vẫn giữ ứng dụng chính ở chế độ không đặc quyền.
- **Tích hợp nhà cung cấp**: Hệ thống tích hợp phần thưởng/cổng eSIM.cc, trạng thái số/số dư/gói cước eSIM.gg và 3HK được duy trì trực tiếp trong kho mã nguồn.
- **Quản lý hồ sơ eSIM**: Đặt tên gợi nhớ, biểu tượng tùy chỉnh, ghi chú và gắn thẻ phân loại.
- **Thông báo nhắc nhở**: Lên lịch nhắc nhở hạn sử dụng gói cước và các sự kiện liên quan đến profile.

## Ghi chú cho Android

- NekokoLPA2 ưu tiên sử dụng `OTBridge` bên ngoài trên Android khi được cài đặt.
- Trên các thiết bị Android đã root, `OTBridge` cung cấp quyền truy cập Telephony/TMAPI và vượt qua hạn chế ARA-M của OMAPI.
- Mã nguồn và các bản dựng OTBridge nằm trong kho lưu trữ này: [mã nguồn](otbridge/), [bản tải về phát hành](https://github.com/hzzmonetvn/NekokoLPA2/releases).
- Bản ứng dụng/module đặc quyền độc lập cũng được duy trì. [So sánh OTBridge và bản Rooted đầy đủ kèm hướng dẫn cài đặt](docs/android-root.md).

## Biên dịch từ mã nguồn

Dự án hỗ trợ build cho Android với các tùy chọn Split APKs và Universal APK. Phiên bản Flutter hiện được ghim là **3.47.2**. Phần giao tiếp QRTR/Modem sử dụng **Rust 1.89 trở lên**.

```sh
git clone --branch master https://github.com/hzzmonetvn/NekokoLPA2.git
cd NekokoLPA2
flutter pub get
```

### Kiểm tra tính hợp lệ

```sh
cargo check --workspace
dart analyze lib test
flutter test --no-pub
```

## Tải về

- [![Tải trên App Store](https://img.shields.io/badge/App%20Store-Download-0D96F6?logo=appstore&logoColor=white&style=for-the-badge)](https://apps.apple.com/en/app/nekokolpa-2/id6757540723)
- [![Tải trên Google Play](https://img.shields.io/badge/Google%20Play-Download-34A853?logo=googleplay&logoColor=white&style=for-the-badge)](https://play.google.com/store/apps/details?id=ee.nekoko.nlpa)
- **GitHub Releases**: [Bản phát hành NekokoLPA2](https://github.com/hzzmonetvn/NekokoLPA2/releases)
- **Bản Split APKs**: Xem chi tiết trong danh sách file đính kèm của mỗi Release (hỗ trợ `arm64-v8a`, `armeabi-v7a` và bản `universal`).

## Nhật ký thay đổi (Changelog)

Xem [CHANGELOG.md](CHANGELOG.md) để biết chi tiết lịch sử phát hành và các tính năng mới qua từng phiên bản.

## Hỗ trợ & Đóng góp

Nếu bạn gặp sự cố, muốn yêu cầu tính năng hoặc có câu hỏi, vui lòng mở phản hồi tại [GitHub Issues](https://github.com/hzzmonetvn/NekokoLPA2/issues).

## Giấy phép

NekokoLPA2 được phát hành theo giấy phép [MIT License](LICENSE).

Hình ảnh linh vật mèo NekokoLPA được sáng tác bởi @sanzennami. Vui lòng tham khảo [NOTICE](NOTICE) và [BRANDING.md](BRANDING.md) để biết thêm về quy định sử dụng thương hiệu.

---

© 2026 Nekoko.
