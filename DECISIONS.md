# Project decisions

## Product scope

- **Chosen:** Continue development in the `hzzmonetvn/NekokoLPA2` repository; treat the first-generation codebase as out of scope for ongoing changes.
- **Considered and rejected:** Carrying forward first-generation implementation as the active baseline. The current checkout and its `origin` remote are NekokoLPA2, so this repository is the source of truth for this work.

## Platform and architecture

- **Chosen:** Ship the application for Android only, with ARM 32-bit (`armeabi-v7a`) and ARM 64-bit (`arm64-v8a`) application ABIs. The privileged flavor remains 64-bit ARM only.
- **Considered and rejected:** Android x86/x86_64 APKs and non-Android application targets. The active app CI build has Android jobs only, and Android ABI filters and Cargokit target selection are limited to ARM. iOS VoWiFi and web publishing workflows are removed; update-feed publication remains active for Android release artifacts. This is the current scope of active app build/release configuration, not a claim that every library or Rust helper is Android-specific.

## Implementation language

- **Chosen:** Keep the Rust backend and its Flutter/Rust bridge for native functionality.
- **Considered and rejected:** Replacing Rust with another backend language. No requirement or evidence in the current changes calls for a rewrite.

## Language and appearance

- **Chosen:** Offer AMOLED as a third theme type, using the existing custom component-theme architecture with a true-black scaffold and app/navigation bars plus near-black surfaces. Add Vietnamese ARB source messages and maintain parity with English keys and placeholders.
- **Considered and rejected:** Applying black surfaces to every component. Near-black dialogs and cards provide separation while preserving the OLED-black page background. Hand-editing every generated locale file; the Vietnamese ARB is the maintained translation source.

## CI toolchain

- **Chosen:** Keep Java 17, matching the repository's Android Gradle configuration; pin Android Flutter target platforms in `android/gradle.properties` so Cargokit builds ARMv7 and ARM64 native libraries.
- **Considered and rejected:** Relying only on APK output filters, which would not constrain Cargokit's native Rust target builds.

## Verification limitations

- **Chosen:** Record only checks actually completed. Rust format, tests, and clippy completed successfully; Vietnamese ARB JSON, key parity, and ICU placeholder parity passed. Flutter checks were unavailable because Dart and Flutter are absent from `PATH`; no APK build was completed.
- **Considered and rejected:** Treating configuration inspection as proof of a successful APK build. Android packaging remains unverified until a real build completes.
