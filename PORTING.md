# Porting and verification status

## Implemented scope

The current changes keep NekokoLPA2 Android-focused for this porting phase, retain the Rust backend, constrain Android packaging to ARM, add Vietnamese localization source assets, and add an AMOLED theme option. The community Android build targets ARMv7 and ARM64; the privileged app build is ARM64-only. The active CI workflows in this checkout include Android build/release and update-feed publication plus the separate `otbridge` Android APK. Removed workflows include iOS VoWiFi and web publishing. The `build.yml` workflow has no web, Linux, or Windows app build jobs. This does not imply every Rust helper or dependency is Android-specific.

## Verification results

Run from `/home/opc/lpa/NekokoLPA2` unless noted.

| Check | Result | Notes |
|---|---|---|
| `git diff --check` | PASS | No whitespace errors in the working-tree diff at time of check. |
| Vietnamese ARB JSON | PASS | Both English and Vietnamese ARB files parse as JSON. |
| ARB key parity | PASS | English and Vietnamese each have 926 message keys; no missing or extra keys. |
| ICU/placeholder parity | PASS | Placeholder metadata and ICU argument names match for all messages, including plural messages. |
| `dart format` | NOT RUN | Dart is not available on `PATH`. |
| `flutter analyze` | NOT RUN | Flutter is not available on `PATH`. |
| `flutter test` | NOT RUN | Flutter is not available on `PATH`. |
| `cargo fmt --check` in `rust` | PASS | Completed successfully. |
| `cargo test` in `rust` | PASS | Completed successfully; the crate has zero unit tests. |
| `cargo clippy --all-targets -- -D warnings` in `rust` | PASS | Completed successfully without warnings. |
| Android ABI configuration inspection | PASS | `android/app/build.gradle.kts` filters `arm64-v8a` and `armeabi-v7a`; `android/gradle.properties` requests `android-arm,android-arm64`; CI privileged target is ARM64 only. No x86/x86_64 app ABI target appears in these active configurations. |
| Android APK build | NOT RUN | No successful APK build was performed; APK production is unverified. |

## Remaining issues

- Run Flutter formatting, analysis, and tests with an installed Flutter SDK.
- Build and inspect the Android APK using a supported Flutter/Android/JDK environment; no APK success is claimed here.
- Rust tests currently provide no unit-test coverage despite a successful `cargo test` result.
