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
| `cargo fmt --check` in workspace | PASS | Completed successfully. |
| `cargo test --workspace` | PASS | Completed successfully across all crates (`nlpa_core` modem tests: 7 passed; `nlpa_core` telecom/asn1/profile tests: 10 passed; `nlpa_ui` virtual DOM tests: 1 passed; total 18 tests passed). |
| `cargo clippy --all-targets -- -D warnings` | PASS | Completed successfully without warnings across all workspace crates. |
| `cargo doc --no-deps` | PASS | Generated documentation cleanly with zero broken links or doc test warnings. |
| Android ABI configuration inspection | PASS | `android/app/build.gradle.kts` filters `arm64-v8a` and `armeabi-v7a`; `android/gradle.properties` requests `android-arm,android-arm64`; CI privileged target is ARM64 only. No x86/x86_64 app ABI target appears in these active configurations. |
| Android APK build | NOT RUN | No successful APK build was performed; APK production is unverified. |

## Ported Rust Modules (`nlpa_core` & `rust_lib_nlpa2`)

| Original Dart Module | Rust Module | Porting Details |
|---|---|---|
| `lib/utils/hex_utils.dart` | `nlpa_core::hex` | High-efficiency ASCII hex encoder/decoder (`bytes_to_hex`, `hex_to_bytes`), space-insensitive parsing, odd nibble left-padding, and `swap_nibbles` utilizing `as_chunks::<2>()`. |
| `lib/models/activation_code.dart` | `nlpa_core::activation_code` | Robust GSMA SGP.22 Activation Code parser/formatter (`LPA:1$<SMDP>$<MatchingId>[$<SMDP_OID>][$<CC>]`) with typed `ActivationCodeError`. |
| `lib/utils/iccid_formatter.dart` | `nlpa_core::iccid` | ICCID BCD trailing filler stripper (`iccid_for_display`) and 4-digit grouped industrial formatter (`format_iccid_grouped`). |
| `lib/utils/apdu_exception.dart` | `nlpa_core::apdu` | ISO 7816 / GSMA SGP.22 APDU status word validator returning typed `Result<(), ApduError>` (0x6985, 0x6881, 0x6A80, 0x6A88, 0x91XX proactive refresh). |
| `lib/utils/imei_codec.dart` | `nlpa_core::imei` | 3GPP IMEI Luhn checksum algorithm (`calculate_imei_luhn_checksum`) and 8-byte packed BCD encoder/decoder (`imei_digits_from_stored_bytes`, `encode_stored_imei_bytes`). |
| `lib/models/asn1/simple_reader.dart` | `nlpa_core::asn1` | ASN.1 DER stream parser and TLV hierarchy (`Asn1Reader`, `Asn1Node`), multi-byte tag parsing, length handling, and recursive sequence navigation. |
| `lib/models/euicc_profile.dart` | `nlpa_core::profile` | GSMA eUICC profile representation (`EuiccProfile`), raw vs display ICCID separation, display name resolution logic. |
| Bridge Layer | `rust_lib_nlpa2::api::telecom` | Flutter Rust Bridge exports (`parse_activation_code`, `format_activation_code`, `hex_encode`, `hex_decode`, `nibble_swap`, `format_iccid`, `clean_iccid`, `validate_apdu`, `calculate_imei_luhn`, `decode_stored_imei`, `encode_stored_imei`, `parse_asn1_tlvs`, `create_euicc_profile`). |

## Remaining issues

- Run Flutter formatting, analysis, and tests with an installed Flutter SDK.
- Build and inspect the Android APK using a supported Flutter/Android/JDK environment; no APK success is claimed here.

