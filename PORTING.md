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
| `cargo test --workspace` | PASS | Completed successfully across all crates (`nlpa_core` modem tests: 7 passed; `nlpa_core` telecom/asn1/profile tests: 11 passed; `nlpa_core` RSP tests: 9 passed; `nlpa_core` ES10 tests: 4 passed; `nlpa_core` LPA engine tests: 3 passed; `nlpa_core` CAT tests: 6 passed; `nlpa_core` crypto tests: 9 passed; `nlpa_core` MCC/PLMN tests: 3 passed; `nlpa_core` notification address tests: 2 passed; `nlpa_core` notifications test: 5 passed; `nlpa_core` download session tests: 2 passed; `nlpa_core` state transition tests: 5 passed; `nlpa_core` channel tests: 7 passed; `nlpa_core` es9 tests: 7 passed; `nlpa_core` atr tests: 3 passed; `nlpa_core` nine_esim tests: 4 passed; `nlpa_ui` virtual DOM tests: 1 passed; `rust_lib_nlpa2` LPA bridge tests: 3 passed; `rust_lib_nlpa2` CCID bridge tests: 2 passed; total 80 tests passed). |
| `cargo clippy --all-targets --all-features -- -D warnings` | PASS | Completed successfully without warnings across all workspace crates. |
| `cargo fmt --check` in workspace | PASS | Completed successfully with zero formatting discrepancies. |
| `cargo doc --no-deps` | PASS | Generated documentation cleanly with zero broken links or doc test warnings. |
| Android ABI configuration inspection | PASS | `android/app/build.gradle.kts` filters `arm64-v8a` and `armeabi-v7a`; `android/gradle.properties` requests `android-arm,android-arm64`; CI privileged target is ARM64 only. No x86/x86_64 app ABI target appears in these active configurations. |
| Android APK build | NOT RUN | Local VPS builds are strictly disabled; all APK packaging runs on GitHub Actions CI. |

## Ported Rust Modules (`nlpa_core` & `rust_lib_nlpa2`)

| Original Dart Module | Rust Module | Porting Details |
|---|---|---|
| `lib/utils/hex_utils.dart` | `nlpa_core::hex` | High-efficiency ASCII hex encoder/decoder (`bytes_to_hex`, `hex_to_bytes`), space-insensitive parsing, odd nibble left-padding, and `swap_nibbles` utilizing `as_chunks::<2>()`. |
| `lib/models/activation_code.dart` | `nlpa_core::activation_code` | Robust GSMA SGP.22 Activation Code parser/formatter (`LPA:1$<SMDP>$<MatchingId>[$<SMDP_OID>][$<CC>]`) with typed `ActivationCodeError`. |
| `lib/utils/iccid_formatter.dart` | `nlpa_core::iccid` | ICCID BCD trailing filler stripper (`iccid_for_display`) and 4-digit grouped industrial formatter (`format_iccid_grouped`). |
| `lib/utils/apdu_exception.dart` | `nlpa_core::apdu` | ISO 7816 / GSMA SGP.22 APDU status word validator returning typed `Result<(), ApduError>` (0x6985, 0x6881, 0x6A80, 0x6A88, 0x91XX proactive refresh). |
| `lib/utils/imei_codec.dart` | `nlpa_core::imei` | 3GPP IMEI Luhn checksum algorithm (`calculate_imei_luhn_checksum`) and 8-byte packed BCD encoder/decoder (`imei_digits_from_stored_bytes`, `encode_stored_imei_bytes`). |
| `lib/models/asn1/simple_reader.dart` | `nlpa_core::asn1` | ASN.1 DER stream parser and TLV hierarchy (`Asn1Reader`, `Asn1Node`), multi-byte tag parsing, length handling, and recursive sequence navigation. |
| `lib/models/asn1/rsp_definitions.g.dart` | `nlpa_core::asn1::rsp` | Strongly typed GSMA SGP.22 RSP ASN.1 DER data structures and codecs in Rust (`EuiccConfiguredAddresses`, `EuiccInfo1`, `EuiccInfo2`, `EuiccChallenge`, `RspProfileInfo`, `SetNicknameRequest`, `EnableProfileRequest`, `DisableProfileRequest`, `DeleteProfileRequest`, `NotificationMetadata`, `EuiccMemoryResetRequest`), while the legacy Dart file was pruned by 67% (1.95MB to 649KB) to retain only the 145 types required by Dart UI and tests. |
| `lib/models/asn1/pe.dart` | `nlpa_core::asn1::pe` | GSMA SGP.22 Bound Profile Package (BPP) Profile Element stream parser and structural sequence validator (`parse_profile_package`, `validate_profile_elements`). |
| `lib/adapter/omapi_reader.dart` & `es10.dart` | `nlpa_core::lpa::es10` | ISO 7816-4 APDU STORE DATA builders targeting ISD-R AID (`A0 00 00 05 59 10 10 FF FF FF FF 89 00 00 01 00`) and APDU response status word validator (`parse_es10_response`). |
| `lib/logic/lpa_engine.dart` | `nlpa_core::lpa::engine` | In-memory eUICC operational state machine (`LpaEngine`) enforcing single-enable semantics, profile nickname updates, and deletion safety invariants. |
| `lib/logic/notification_dispatcher.dart` | `nlpa_core::lpa::notifications` | eUICC Notification Queue manager (`NotificationQueue`, `NotificationRecord`) with pending delivery tracking and ES10b.HandleNotification ACK APDU generator. |
| `lib/ts43/` | `nlpa_core::lpa::ts43` | GSMA TS.43 carrier entitlement client configurations and HTTP headers generator (`Ts43CarrierConfig`, `EntitlementStatus`). |
| `lib/models/euicc_profile.dart` | `nlpa_core::profile` | GSMA eUICC profile representation (`EuiccProfile`), raw vs display ICCID separation, display name resolution logic. |
| `lib/utils/cat_proactive.dart` | `nlpa_core::cat` | ETSI TS 102 221 / TS 102 223 / 3GPP TS 31.111 Card Application Toolkit proactive command encoder/decoder (`STATUS`, `FETCH`, `TERMINAL RESPONSE`), BER-TLV / COMPREHENSION-TLV parser, and eUICC `REFRESH` decision engine. |
| `lib/utils/crypto_utils.dart` | `nlpa_core::crypto` | Standard SHA-256 (RFC 6234), GSMA SGP.22 `HashCc` computation, FIPS 197 AES-128/256 block cipher, CBC mode, PKCS#7 padding/unpadding, Base64 codec, and Telecom GM host MAC calculation. |
| `lib/utils/mcc_utils.dart` | `nlpa_core::mcc` | ITU-T E.212 Mobile Country Code mapping to ISO 3166-1 alpha-2, Unicode regional indicator flag emojis, and 3-byte 3GPP PLMN decoder. |
| `lib/utils/notification_address_validator.dart` | `nlpa_core::notification_address` | Strict GSMA SGP.22 notification server address validator guarding against malformed card metadata. |
| `lib/adapter/omapi_channel.dart` & `channel_transport.dart` | `nlpa_core::channel` | ISO/IEC 7816-4 APDU transport engine, logical channel CLA mapping (basic 0..=3 and extended 4..=19), `MANAGE CHANNEL [Open/Close]` APDU generation, APDU frame formatting, STORE DATA & ISO CLA chaining segmenter, multi-frame response reassembly state machine (`0x61` GET RESPONSE & `0x6C` reissue), and OMAPI channel safety tracker/latch. |
| `lib/es9/es9_client.dart` & `es9_models.dart` | `nlpa_core::es9` | GSMA SGP.22 ES9+ protocol endpoints (`initiateAuthentication`, `authenticateClient`, `getBoundProfilePackage`, `cancelSession`, `handleNotification`), standard headers, JSON request body builders, zero-dependency UTF-8 JSON parser, SGP.22 header/status validator, and strongly typed response DTO decoders. |
| `lib/logic/profile_download_session.dart` | `nlpa_core::lpa::download_session` & `rust_lib_nlpa2::api::lpa` | GSMA SGP.22 eSIM profile download session lifecycle controller (`ProfileDownloadSession`, `DownloadSessionStateDto`), SGP.22 HashCc calculation, transaction ID hex/octet normalization, and ES9+/ES10 state exchange bridge. |
| `lib/logic/profile_state_transition.dart` | `nlpa_core::lpa::state_transition` & `rust_lib_nlpa2::api::lpa` | Idempotent profile state transition convergence controller (`ProfileStateTransition`, `StateTransitionResultDto`, `evaluate_profile_convergence`) with independent verification reads, transient error tolerance, and non-convergence diagnostics. |
| `lib/adapter/ccid/nine_esim_usb_protocol.dart` | `nlpa_core::atr`, `nlpa_core::ccid::nine_esim` & `rust_lib_nlpa2::api::ccid` | ISO/IEC 7816-3 Answer to Reset (ATR) parser (`Atr`, `AtrError`), interface byte traversal (T0, TA, TB, TC, TD), protocol T=15 global interface byte detection, eUICC capability flag toggle (bit 1 `0x02`), trailing TCK checksum recalculation (`XOR(T0..TCK) == 0`), 9eSIM CRC-24 checksum (`0x1864CFB` generator, `0xB704CE` seed), password hex normalizer, CCID vendor password command, enable ATR programming command (`30 97 43 18 00`), and set ATR write command builder. |
| Bridge Layer | `rust_lib_nlpa2::api::telecom`, `api::lpa` & `api::ccid` | Flutter Rust Bridge exports (`parse_activation_code`, `format_activation_code`, `hex_encode`, `hex_decode`, `nibble_swap`, `format_iccid`, `clean_iccid`, `validate_apdu`, `calculate_imei_luhn`, `decode_stored_imei`, `encode_stored_imei`, `parse_asn1_tlvs`, `create_euicc_profile`, `map_channel_cla`, `build_open_logical_channel_apdu`, `build_close_logical_channel_apdu`, `frame_apdu`, `chain_apdu_packets`, `build_initiate_auth_req`, `parse_initiate_auth_resp`, `build_authenticate_client_req`, `parse_authenticate_client_resp`, `build_get_bpp_req`, `parse_get_bpp_resp`, `build_cancel_session_req`, `create_download_session`, `session_process_euicc_info1`, `session_process_euicc_challenge`, `session_validate_initiate_auth`, `session_process_initiate_auth_resp`, `session_compute_hash_cc`, `session_process_auth_server_resp`, `session_process_auth_client_resp`, `evaluate_profile_convergence`, `nine_esim_password_hex`, `nine_esim_set_password_command`, `nine_esim_enable_atr_programming_command`, `nine_esim_set_atr_command`, `atr_euicc_capability_enabled`, `atr_set_euicc_capability`). |

## Remaining issues

- Run Flutter formatting, analysis, and tests with an installed Flutter SDK.
- Build and inspect the Android APK using a supported Flutter/Android/JDK environment; no APK success is claimed here.

