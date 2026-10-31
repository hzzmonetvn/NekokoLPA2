# Architectural Decisions — NekokoLPA2

## 1. Android-Only Platform Target
- **Choice:** Removed all non-Android code (iOS, macOS, Windows, Linux, Web) and stripped out non-Android dependencies to minimize attack surface and binary size.
- **Alternatives Considered:** Keeping cross-platform support.
- **Reasoning:** The application interacts with modem buses (QRTR) and SIM interfaces that are strictly available on target Android devices.

## 2. Full Rust Logic & Workspace Integration
- **Choice:** Structured a pure Rust workspace (`rust`, `rust/core` and `rust/ui`) using Dioxus for UI prototypes and consolidating core protocol logic.
- **Alternatives Considered:** Keeping business logic embedded within Flutter plugins.
- **Reasoning:** Clear separation of concerns, higher memory safety, and native performance.

## 3. Split APK Deployment & Build Workflow
- **Choice:** Enabled ABI split APKs (`armeabi-v7a`, `arm64-v8a`) via Gradle configuration and updated GitHub Actions CI to package individual split artifacts and universal builds using direct Gradle task execution (`assemble<Flavor>Release`).
- **Alternatives Considered:** Relying strictly on `flutter build apk --split-per-abi`.
- **Reasoning:** Flutter CLI version 3.47.2 fails with `Gradle build failed to produce an .apk file` when product flavors and ABI splits are enabled simultaneously because it expects single un-split APK naming conventions. Running Gradle directly generates all split and universal APKs reliably.

## 4. AMOLED Dark Theme & System Bar Overlay
- **Choice:** Introduced pure black (`#000000`) theme palette with high-contrast surfaces (`#101010`) and system bar overlay style configuration.
- **Alternatives Considered:** Standard dark gray palette.
- **Reasoning:** Maximum battery efficiency on OLED/AMOLED mobile screens and consistent visual hierarchy.

## 5. Vietnamese Primary Localization
- **Choice:** Added full Vietnamese ARB localization and Vietnamese README.
- **Alternatives Considered:** English only.
- **Reasoning:** User accessibility and primary market support.
