# Porting and verification status (`2.5.0+606` Pure Rust Rewrite)

## Implemented scope

NekokoLPA2 (`2.5.0+606`) is now a **100% Pure Rust** Android-only (`arm64-v8a` / `armeabi-v7a`) application:
- Flutter, Dart (`lib/`, `pubspec.yaml`, `l10n.yaml`, `test/**/*.dart`, `local_packages/`, `rust_builder/`), and `flutter_rust_bridge` are completely removed.
- `nlpa_core` (`rust/core`, `v2.5.0`) implements the complete SGP.22/ES9+/ES10x/ES11/TS.43/SCP03 protocol stack, Bound Profile Package segmentation (`bpp`), ATR capability inspection (`atr`), eSTK/GM/provider plugins (`plugins`), RemoCard V2 codec (`remocard`), 11-locale ARB catalog (`i18n`), atomic JSON settings/history model (`storage`), and pinned update feed checker (`update_feed`).
- `nlpa_ui` (`rust/ui`, `v2.5.0`) implements the Dioxus `0.6` SSR UI rendered inside an Android `WebView` with `#000000` AMOLED theme, translucent wallpaper glass across profile loading and Settings, `0.05..=0.95` opacity control, and all 11 locales (including selectable `Tiếng Việt (vi)`).
- `rust_lib_nlpa2` (`rust`, `v2.5.0`) implements the native `AppController`, hardware transport adapters over `NativeHostBridge`, and direct JNI entry points (`RustBridge` + preserved `qrtr_bus.rs`), while Android Kotlin retains all native hardware and OMAPI/SE HAL/CCID/ADB/OTBridge safety boundaries.

## Verification policy & results

> Local builds/compilers (`cargo`, `rustc`, `rustfmt`, `clippy`, `gradle`) are disabled on the local development machine. All Rust workspace checks (`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo doc --no-deps`) and Android APK builds execute on GitHub Actions (`.github/workflows/build.yml`).

| Check | Result | Notes |
|---|---|---|
| `git diff --check` | PASS | Executed locally; zero whitespace/conflict-marker issues. |
| All 11 ARB JSON files (`json.load`) | PASS | Executed locally across `l10n/app_*.arb`. |
| `python3 scripts/check_l10n.py` | PASS | 927 keys in `app_en.arb`, 10 translation files verified with zero duplicate or unknown keys. |
| `cargo fmt --all -- --check` | CI (GitHub Actions) | Not run locally per local-build restriction. |
| `cargo clippy --workspace --all-targets -- -D warnings` | CI (GitHub Actions) | Not run locally per local-build restriction. |
| `cargo test --workspace` | CI (GitHub Actions) | Not run locally per local-build restriction. |
| `cargo doc --no-deps` | CI (GitHub Actions) | Not run locally per local-build restriction. |
| Android APK & Magisk ZIP packaging | CI (GitHub Actions) | Built via `cargo ndk` + `./gradlew assembleCommunityRelease` / `assemblePrivilegedRelease`. |

## Complete Rust Core Modules (`nlpa_core`)

| Former Dart Module | Rust Module (`rust/core/src/`) | Porting Details |
|---|---|---|
| `lib/models/activation_code.dart` | `activation_code.rs` | SGP.22 activation code parser/formatter (`ActivationCode`, `ActivationCodeError`). |
| `lib/utils/apdu_exception.dart` | `apdu.rs` | ISO 7816 / SGP.22 SW validator (`validate_apdu_sw`, `ApduError`). |
| `lib/models/asn1/*` | `asn1/{der,fcp,pe,rsp,x509}.rs` | DER TLV reader/writer, SGP.22 RSP codecs, PE stream parser, SIM FCP/EF_DIR parser, RFC 5280 X.509 & CI verification. |
| `lib/utils/atr_capabilities.dart` | `atr.rs` | `Atr` T=15/TCK parser plus `supports_euicc` (ETSI TS 102 221 V17.1.0 §6.3.3/Table 6.7) and `is_likely_euicc_atr`. |
| `lib/logic/profile_manager.dart` (`_decomposeBpp`, `cancelSession`, `resetEuiccMemory`, NXP `DF25`) | `bpp.rs` | `decompose_bpp`, `build_cancel_session_request_der`, `EUICC_MEMORY_RESET_APDU_DATA`, `NxpMemoryInfo`, `parse_nxp_memory_tlv`. |
| `lib/utils/cat_proactive.dart` | `cat.rs` | ETSI TS 102 223 / 3GPP TS 31.111 CAT proactive command codec. |
| `lib/adapter/ccid/nine_esim_usb_protocol.dart` | `ccid/nine_esim.rs` | 9eSIM CRC-24, password, and ATR programming commands. |
| `lib/adapter/euicc_adapter.dart`, `omapi_safety.dart` | `channel.rs` | `DEFAULT_STORE_DATA_MSS = 120`, logical channel CLA mapping, `chain_apdu_request`, `ResponseReassembler`, `OmapiChannelTracker`, `OmapiSafetyLatch`. |
| `lib/utils/crypto_utils.dart`, `aes_cipher.dart` | `crypto.rs` | Zero-dependency SHA-1, SHA-256, HMAC-SHA1, FIPS-197 AES-128/256 CBC/PKCS#7, AES-CMAC, Base64, `HashCc`, `calc_host_mac`. |
| `lib/utils/error_codes.dart` | `error_codes.rs` | Typed `AppErrorCode` and `AppException`. |
| `lib/services/es9plus_service.dart` | `es9.rs` | ES9+ endpoint constants, headers, JSON builders, and response parsers. |
| `lib/utils/hex_utils.dart`, `iccid_formatter.dart`, `imei_codec.dart`, `mcc_mapper.dart`, `notification_address.dart` | `hex.rs`, `iccid.rs`, `imei.rs`, `mcc.rs`, `notification_address.rs` | Pure Rust codecs, Luhn check, MCC/flag mapping, FQDN validator. |
| `lib/utils/locale_utils.dart`, `l10n/app_*.arb` | `i18n.rs` | `LocaleChoice` (12 choices incl. `Vi` -> `Tiếng Việt`), `resolve_locale`, `I18nCatalog` embedding all 11 ARB files. |
| `lib/logic/*`, `lib/services/*`, `lib/ts43/*` | `lpa/{download_session,engine,es10,es11,isdp,notifications,scp03,state_transition,ts43/*}.rs` | Download session, LPA state machine, ES10/ES11 builders/parsers, notification planner, SCP03, TS.43 EAP-AKA/ODSA client. |
| `lib/plugins/{estk,gm,esim_cc,esim_gg,thk}_plugin.dart` | `plugins.rs` | `estk_extra_aids` / MGT helpers, `is_gm_eid` / `build_gm_auth_apdu`, `match_provider_plugin` / `eSIM.cc` / `eSIM.gg` / `3HK` badge parsers. |
| `lib/models/euicc_profile.dart`, `profile_size_predictor.dart`, `profile_tag.dart`, `profile_redaction.dart`, `size_formatter.dart` | `profile.rs`, `profile_size_predictor.rs`, `profile_tag.rs`, `redaction.rs`, `size_formatter.rs` | Profile model, size catalog predictor, nickname tag parser, serde-enabled redaction rules, byte formatter. |
| `lib/adapter/remote/remocard_adapter.dart` | `remocard.rs` | `RemoCardServer`, V2 request/response JSON codecs, and AES-256-CBC payload encryption/decryption. |
| `lib/settings/app_settings.dart`, `lib/services/database_service.dart` | `storage.rs` | `ThemeStyle`, `ColorMode`, `PersistedNotification`, and `PersistedState` JSON persistence schema. |
| `lib/services/update_feed.dart`, `update_service.dart` | `update_feed.rs` | `COMMUNITY_CERT` pinning, `open`/`community` vs `open-rooted`/`community-rooted` channel isolation, `check_update_feed`. |
