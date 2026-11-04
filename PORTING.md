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
| `cargo test --workspace` | PASS | Completed successfully across all crates (97 tests passed before profile-redaction tests were added; 11 redaction tests + error_codes + profile_tag added after). |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS | Completed successfully without warnings across all workspace crates after redaction integration. |
| `cargo fmt --check` in workspace | PASS | Completed successfully with zero formatting discrepancies after redaction integration. |
| `cargo doc --no-deps` | PASS | Generated documentation cleanly with zero broken links or doc test warnings prior to redaction changes. |
| Android ABI configuration inspection | PASS | `android/app/build.gradle.kts` filters `arm64-v8a` and `armeabi-v7a`; `android/gradle.properties` requests `android-arm,android-arm64`; CI privileged target is ARM64 only. No x86/x86_64 app ABI target appears in these active configurations. |
| Android APK build | NOT RUN | Local VPS builds are strictly disabled; all APK packaging runs on GitHub Actions CI. |
| `cargo test -p nlpa_core --test redaction_test` | PASS | 11 redaction tests pass. |
| `cargo clippy -p nlpa_core -- -D warnings` | PASS | Zero warnings (field-reassign lint fixed). |

## Ported Rust Modules (`nlpa_core` & `rust_lib_nlpa2`)

| Original Dart Module | Rust Module | Porting Details |
|---|---|---|
| `lib/utils/profile_redaction.dart` and `lib/utils/size_formatter.dart` | `nlpa_core::redaction` | Pure redaction settings, profile display and operator helpers, country/flag formatting, tag filtering, and byte-size formatting. Async metadata warmup is represented by caller-supplied placeholders. |
| `lib/utils/hex_utils.dart` | `nlpa_core::hex` | High-efficiency ASCII hex encoder/decoder (`bytes_to_hex`, `hex_to_bytes`), space-insensitive parsing, odd nibble left-padding, and `swap_nibbles` utilizing `as_chunks::<2>()`. |
| `lib/models/activation_code.dart` | `nlpa_core::activation_code` | Robust GSMA SGP.22 Activation Code parser/formatter (`LPA:1$<SMDP>$<MatchingId>[$<SMDP_OID>][$<CC>]`) with typed `ActivationCodeError`. |
| `lib/utils/iccid_formatter.dart` | `nlpa_core::iccid` | ICCID BCD trailing filler stripper (`iccid_for_display`) and 4-digit grouped industrial formatter (`format_iccid_grouped`). |
| `lib/utils/apdu_exception.dart` | `nlpa_core::apdu` | ISO 7816 / GSMA SGP.22 APDU status word validator returning typed `Result<(), ApduError>` (0x6985, 0x6881, 0x6A80, 0x6A88, 0x91XX proactive refresh). |
| `lib/utils/imei_codec.dart` | `nlpa_core::imei` | 3GPP IMEI Luhn checksum algorithm (`calculate_imei_luhn_checksum`) and 8-byte packed BCD encoder/decoder (`imei_digits_from_stored_bytes`, `encode_stored_imei_bytes`). |
| `lib/models/asn1/simple_reader.dart` | `nlpa_core::asn1` | ASN.1 DER stream parser and TLV hierarchy (`Asn1Reader`, `Asn1Node`), multi-byte tag parsing, length handling, and recursive sequence navigation. |
| `lib/models/asn1/rsp_definitions.g.dart` | `nlpa_core::asn1::rsp` | Strongly typed GSMA SGP.22 RSP ASN.1 DER data structures and codecs in Rust. |
| `lib/models/asn1/pe.dart` | `nlpa_core::asn1::pe` | GSMA SGP.22 Bound Profile Package (BPP) Profile Element stream parser and structural sequence validator. |
| `lib/adapter/omapi_reader.dart` & `es10.dart` | `nlpa_core::lpa::es10` | ISO 7816-4 APDU STORE DATA builders targeting ISD-R AID and response validation. |
| `lib/logic/lpa_engine.dart` | `nlpa_core::lpa::engine` | In-memory eUICC operational state machine with profile operation safety. |
| `lib/logic/notification_dispatcher.dart` | `nlpa_core::lpa::notifications` | eUICC Notification Queue manager and ES10b.HandleNotification ACK APDU generator. |
| `lib/ts43/` | `nlpa_core::lpa::ts43` | GSMA TS.43 carrier entitlement client configurations and HTTP headers generator. |
| `lib/models/euicc_profile.dart` | `nlpa_core::profile` | GSMA eUICC profile representation, raw vs display ICCID separation, display name resolution logic. |
| `lib/utils/cat_proactive.dart` | `nlpa_core::cat` | ETSI TS 102 221 / TS 102 223 / 3GPP TS 31.111 Card Application Toolkit proactive command encoder/decoder. |
| `lib/utils/crypto_utils.dart` | `nlpa_core::crypto` | Standard SHA-256, GSMA SGP.22 HashCc, FIPS AES, CBC mode, PKCS#7, Base64, and telecom MAC support. |
| `lib/utils/mcc_utils.dart` | `nlpa_core::mcc` | ITU-T E.212 MCC mapping, Unicode regional indicator flag emojis, and PLMN decoder. |
| `lib/utils/notification_address_validator.dart` | `nlpa_core::notification_address` | Strict GSMA SGP.22 notification server address validator. |
| `lib/adapter/omapi_channel.dart` & `channel_transport.dart` | `nlpa_core::channel` | ISO/IEC 7816-4 APDU transport, channel mapping, APDU framing and reassembly. |
| `lib/es9/es9_client.dart` & `es9_models.dart` | `nlpa_core::es9` | GSMA SGP.22 ES9+ protocol endpoints and typed response DTO decoders. |
| `lib/logic/profile_download_session.dart` | `nlpa_core::lpa::download_session` & `rust_lib_nlpa2::api::lpa` | GSMA SGP.22 eSIM profile download session lifecycle controller and bridge. |
| `lib/logic/profile_state_transition.dart` | `nlpa_core::lpa::state_transition` & `rust_lib_nlpa2::api::lpa` | Idempotent profile state transition convergence controller. |
| `lib/adapter/ccid/nine_esim_usb_protocol.dart` | `nlpa_core::atr`, `nlpa_core::ccid::nine_esim` & `rust_lib_nlpa2::api::ccid` | ATR parser, capability toggles, CRC-24, and vendor commands. |
| `lib/models/isdp.dart` & `scp03.dart` | `nlpa_core::lpa::isdp`, `nlpa_core::lpa::scp03` & `rust_lib_nlpa2::api::lpa` | ISD-P lifecycle, SELECT/DELETE APDU builders, and GlobalPlatform SCP03 support. |
| `lib/models/x509.dart` & `ci_verification.dart` | `nlpa_core::asn1::x509` & `rust_lib_nlpa2::api::lpa` | RFC 5280 X.509 certificate parsing and GSMA CI constraint verification. |
| Bridge Layer | `rust_lib_nlpa2::api::telecom`, `api::lpa` & `api::ccid` | Flutter Rust Bridge exports for telecom, profile, LPA, and CCID operations. |

## Profile tag and utility port notes

| Dart API | Rust equivalent | Notes |
|---|---|---|
| `ProfileTagUtils.escapeContent`, `unescapeContent` | `profile_tag::{escape_content,unescape_content}` | Preserves replacement order. |
| `ProfileTagUtils.parse`, `format`; `ProfileTag`, `TextTag`, `DateTag` | `profile_tag::{parse_nickname,format_nickname,ProfileTag}` | Hand-scanned tag matching; display names equal parsed names. |
| `Tlv.parse`, `FileInfo.parse`, `SimApplication.parseRecord` | `asn1::fcp::{parse_fcp,parse_application_record}` | Reuses DER reader; rejects indefinite lengths in DER parser. |
| `SizeFormatter.format` | `size_formatter::format_size` | Reuses `redaction::SizeUnit`. |
| `locale_utils.dart` | Not ported | Flutter `Locale` resolution is UI/framework-specific. |

Deliberate differences: Phone-number formatting/detection is omitted; `display_name` equals `name`. Date parsing avoids `chrono`; valid common date forms are formatted directly, and invalid dates have no display date instead of substituting the current date. Tag detection uses a byte scan rather than a regex crate. Existing Dart sources are retained.

## Profile redaction port notes

### API mapping

| Dart API | Rust equivalent |
|---|---|
| `redactEid`, `redactEidPreview` | `redact_eid`, `redact_eid_preview` |
| `redactIccid`, `redactIccidPreview` | `redact_iccid`, `redact_iccid_preview` |
| `displayName`, `operatorSummary` | `display_name`, `operator_summary` |
| `tags` | `filter_tags` |
| `flagUrl`, `flagEmoji` | `flag_url`, `profile_flag_emoji` |
| `hideFlag`, `hideIcon`, `randomIcon` | `hide_flag`, `hide_icon`, `random_icon` |
| `stableIndex` | `stable_index` |
| `countryName`, `operatorTitle`, `operatorPlaceholder` | `country_name`, `operator_title`, `operator_placeholder` |
| `_placeholderLabelFromMetadata` | `placeholder_label_from_metadata` |
| `SizeFormatter.format` | `size_formatter::format_size` (shares `redaction::SizeUnit`) |

### Test mapping

| Characterization behavior | Rust integration test |
|---|---|
| Stable index determinism and zero modulo | `stable_index_is_deterministic_and_zero_modulo_is_zero` |
| All-disabled EID preview mask collapse | `eid_redaction_preview_collapses_mask_runs` |
| EID full reveal | `eid_reveal_full_returns_unchanged` |
| ICCID preview ranges | `iccid_preview_keeps_enabled_ranges` |
| Hidden nickname fallback | `display_name_hides_nickname_with_fallback` |
| Country-name operator summary | `operator_summary_uses_country_name_mode` |
| Hidden tags | `hide_tags_returns_empty` |
| Shuffled flag URL form | `shuffled_flag_url_has_expected_shape` |
| Regional indicator emoji | `flag_emoji_uses_regional_indicators` |
| Known and unknown country codes | `country_name_known_and_unknown_codes` |
| Flag/icon visibility and size units | `redaction_visibility_modes_and_size_formatter` |

### Deliberate differences and preserved quirks

- `stable_index` hashes UTF-8 bytes, unlike Dart's UTF-16 code units; results may differ for non-ASCII seeds.
- Phone number detection is a hand-scanned approximation rather than Dart's regular expression; unusual Unicode or punctuation-heavy candidates may differ.
- Async profile-metadata warmup and `AppSettings` singleton access are omitted; callers provide placeholder slices and a settings value explicitly.
- Redaction mask runs of nine or more stars collapse to exactly eight, as in the Dart implementation.
- The size formatter uses byte units and 1024-based adaptive thresholds, including the Dart `adaptive_si` quirk that labels binary-scaled values as KB/MB/GB.
- The ICCID test pins Dart range behavior for the 20-character example: first 8 and final 6 positions are retained when enabled; overlap is clipped by source length.

## Remaining issues

- Run Flutter formatting, analysis, and tests with an installed Flutter SDK.
- Build and inspect the Android APK using a supported Flutter/Android/JDK environment; no APK success is claimed here.
