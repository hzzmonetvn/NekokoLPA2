# Architectural Decisions — NekokoLPA2

## 1. Android-Only Platform Target
- **Choice:** Removed all non-Android code (iOS, macOS, Windows, Linux, Web) and stripped out non-Android dependencies to minimize attack surface and binary size.
- **Alternatives Considered:** Keeping cross-platform support.
- **Reasoning:** The application interacts with modem buses (QRTR) and SIM interfaces that are strictly available on target Android devices.

## 2. Full Rust Logic & Workspace Integration
- **Choice:** Structured a pure Rust workspace (`rust/core` and `rust/ui`) using Dioxus for the UI and consolidating core logic.
- **Alternatives Considered:** Keeping business logic embedded within Flutter plugins.
- **Reasoning:** Clear separation of concerns, higher memory safety, and native performance.

## 3. Split APK Deployment & Build Workflow
- **Choice:** Enabled ABI split APKs (`armeabi-v7a`, `arm64-v8a`) via Gradle configuration and updated GitHub Actions CI to package individual split artifacts and universal builds.
- **Alternatives Considered:** Single monolithic universal APK.
- **Reasoning:** Smaller download size per user device architecture and faster delivery.
