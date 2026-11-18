//! Pure-Rust eUICC session and provider plugin helpers:
//! - eSTK.me dual-slot AID discovery & management channel parsing
//! - GM card EID matching & `BF71` AES-CBC MAC authentication APDU builder
//! - Public profile provider badges (`eSIM.cc`, `eSIM.gg`, `3HK`)

use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::channel::frame_apdu_command;
use crate::crypto::{aes_cbc_encrypt, calc_host_mac, sha256};
use crate::redaction::{format_size_bytes, SizeUnit};

/// ASCII ATR signature embedded by eSTK.me cards (`"estkme"`, hex `6573746B6D65`).
pub const ESTK_ATR_MARKER: &[u8] = b"estkme";

/// eSTK.me management application AID (`A06573746B6D65FFFFFFFFFFFF6D6774`).
pub const ESTK_MGT_AID: &[u8] = &[
    0xA0, 0x65, 0x73, 0x74, 0x6B, 0x6D, 0x65, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x6D, 0x67, 0x74,
];

/// eSTK.me ISD-R slot 1 AID (`SE0`).
pub const ESTK_AID_CHAN1: &str = "A06573746B6D65FFFF4953442D522030";

/// eSTK.me ISD-R slot 2 AID (`SE1`).
pub const ESTK_AID_CHAN2: &str = "A06573746B6D65FFFF4953442D522031";

const ESTK_ATR_HEX_MARKER: &str = "6573746B6D65";
const GM_EID_MIN: &str = "89086029202200002122000030900000";
const GM_EID_MAX: &str = "89086029202200002122000040000000";

/// Device metadata read from the eSTK.me management channel.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EstkCardInfo {
    pub firmware: Option<String>,
    pub bootloader: Option<String>,
    pub serial: Option<String>,
    pub sku: Option<String>,
}

/// Returns extra eSTK.me ISD-R AIDs when `atr_hex` contains the `6573746B6D65` marker.
#[must_use]
pub fn estk_extra_aids(atr_hex: &str) -> Vec<String> {
    let normalized: String = atr_hex
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();

    if normalized.contains(ESTK_ATR_HEX_MARKER) {
        vec![ESTK_AID_CHAN2.to_string(), ESTK_AID_CHAN1.to_string()]
    } else {
        Vec::new()
    }
}

/// Builds the 5-byte eSTK.me MGT APDU (`00 00 <cmd_data> 00 00`) to read serial (`0`),
/// bootloader (`1`), firmware (`2`), SKU (`3`), or language (`4`).
#[must_use]
pub const fn build_estk_mgt_read_apdu(cmd_data: u8) -> [u8; 5] {
    [0x00, 0x00, cmd_data, 0x00, 0x00]
}

/// Extracts a trimmed printable ASCII string from an eSTK.me MGT response, skipping NUL bytes.
#[must_use]
pub fn parse_estk_mgt_string(bytes: &[u8]) -> Option<String> {
    let text: String = bytes
        .iter()
        .copied()
        .filter(|&b| b != 0 && (0x20..=0x7E).contains(&b))
        .map(char::from)
        .collect();
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Checks if an EID belongs to the GM card range
/// (`89086029202200002122000030900000..=89086029202200002122000040000000`).
#[must_use]
pub fn is_gm_eid(eid: &str) -> bool {
    let trimmed = eid.trim();
    trimmed.len() == 32
        && trimmed.bytes().all(|b| b.is_ascii_digit())
        && trimmed >= GM_EID_MIN
        && trimmed <= GM_EID_MAX
}

/// Builds the GM card `BF71` mutual-authentication APDU (`81 E2 91 00`) using a fresh 16-byte
/// host random challenge.
///
/// # Errors
/// Returns an error string if `psk_bytes` is not 16 or 32 bytes or `challenge` is not 16 bytes.
pub fn build_gm_auth_apdu(psk_bytes: &[u8], challenge: &[u8]) -> Result<Vec<u8>, String> {
    let rn_host = generate_host_nonce(challenge);
    build_gm_auth_apdu_with_rn(psk_bytes, challenge, &rn_host)
}

/// Deterministic variant of [`build_gm_auth_apdu`] taking an explicit 16-byte `rn_host` nonce.
///
/// # Errors
/// Returns an error string if `psk_bytes` is not 16 or 32 bytes or `challenge` is not 16 bytes.
pub fn build_gm_auth_apdu_with_rn(
    psk_bytes: &[u8],
    challenge: &[u8],
    rn_host: &[u8; 16],
) -> Result<Vec<u8>, String> {
    if challenge.len() != 16 {
        return Err(format!(
            "GM challenge must be 16 bytes, got {}",
            challenge.len()
        ));
    }

    let mut auth_data = [0u8; 16];
    auth_data[..8].copy_from_slice(&challenge[..8]);
    auth_data[8..16].copy_from_slice(&rn_host[..8]);

    let sk_raw = aes_cbc_encrypt(psk_bytes, &[0u8; 16], &auth_data).map_err(|e| e.to_string())?;
    let mac = calc_host_mac(&sk_raw, challenge).map_err(|e| e.to_string())?;

    let mut inner = Vec::with_capacity(2 + mac.len() + 2 + rn_host.len());
    inner.push(0x80);
    inner.push(0x04);
    inner.extend_from_slice(&mac);
    inner.push(0x81);
    inner.push(rn_host.len() as u8);
    inner.extend_from_slice(rn_host);

    let mut bf71 = Vec::with_capacity(3 + inner.len());
    bf71.push(0xBF);
    bf71.push(0x71);
    bf71.push(inner.len() as u8);
    bf71.extend_from_slice(&inner);

    Ok(frame_apdu_command(
        0x81,
        0xE2,
        0x91,
        0x00,
        Some(&bf71),
        None,
    ))
}

fn generate_host_nonce(challenge: &[u8]) -> [u8; 16] {
    let mut nonce = [0u8; 16];
    if let Ok(mut f) = File::open("/dev/urandom") {
        if f.read_exact(&mut nonce).is_ok() {
            return nonce;
        }
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    let mut seed = Vec::with_capacity(challenge.len() + 20);
    seed.extend_from_slice(challenge);
    seed.extend_from_slice(&nanos.to_le_bytes());
    seed.extend_from_slice(&pid.to_le_bytes());
    let digest = sha256(&seed);
    nonce.copy_from_slice(&digest[..16]);
    nonce
}

/// Supported public profile provider plugins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ProviderPluginKind {
    #[default]
    EsimCc,
    EsimGg,
    ThreeHk,
}

/// Enriched provider badge and optional quota/MSISDN metadata for a profile card.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProfileProviderBadge {
    pub kind: ProviderPluginKind,
    pub label: String,
    pub msisdn: Option<String>,
    pub quota_summary: Option<String>,
    pub portal_url: Option<String>,
}

/// Identifies the matching public provider plugin for an ICCID prefix, if any.
#[must_use]
pub fn match_provider_plugin(iccid: &str) -> Option<ProviderPluginKind> {
    let clean = iccid.trim();
    if clean.starts_with("8985223") {
        Some(ProviderPluginKind::EsimCc)
    } else if clean.starts_with("893720401") {
        Some(ProviderPluginKind::EsimGg)
    } else if clean.starts_with("8985231") || clean.starts_with("8985203") {
        Some(ProviderPluginKind::ThreeHk)
    } else {
        None
    }
}

/// Returns the static eSIM.cc rewards badge when `iccid` matches eSIM.cc.
#[must_use]
pub fn esim_cc_badge(iccid: &str) -> Option<ProfileProviderBadge> {
    if match_provider_plugin(iccid) != Some(ProviderPluginKind::EsimCc) {
        return None;
    }
    Some(ProfileProviderBadge {
        kind: ProviderPluginKind::EsimCc,
        label: "eSIM.cc".to_string(),
        msisdn: None,
        quota_summary: None,
        portal_url: Some("https://www.esim.cc/card-bonus".to_string()),
    })
}

/// Parses an eSIM.gg API or cached status JSON response into a [`ProfileProviderBadge`].
///
/// Supports both single-record JSON objects and bulk `{"results": [...]}` responses.
#[must_use]
pub fn parse_esim_gg_response(json: &str, iccid: &str) -> Option<ProfileProviderBadge> {
    let root: serde_json::Value = serde_json::from_str(json).ok()?;
    let item = select_esim_gg_item(&root, iccid)?;
    let obj = item.as_object()?;

    let raw_number = obj
        .get("number")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let msisdn = raw_number.map(|n| {
        if n.starts_with('+') {
            n.to_string()
        } else if n.starts_with("372") || n.bytes().all(|b| b.is_ascii_digit()) {
            format!("+{n}")
        } else {
            n.to_string()
        }
    });

    let balance = obj.get("balance").and_then(serde_json::Value::as_f64);
    let balance_str = balance.map(|b| format!("{b:.2} €"));

    let package_name = obj
        .get("package_name")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let remaining = obj.get("remaining_data").and_then(value_as_u64);
    let total = obj.get("total_data").and_then(value_as_u64);
    let expiry = obj
        .get("package_expiry")
        .or_else(|| obj.get("expire_date"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let mut summary_parts = Vec::new();
    if let Some(ref b) = balance_str {
        summary_parts.push(b.clone());
    }
    if let (Some(name), Some(rem), Some(tot)) = (package_name, remaining, total) {
        let rem_str = format_size_bytes(rem, SizeUnit::AdaptiveSi);
        let tot_str = format_size_bytes(tot, SizeUnit::AdaptiveSi);
        let mut pkg = format!("{name}: {rem_str} / {tot_str}");
        if let Some(exp) = expiry {
            let _ = write!(pkg, " ({exp})");
        }
        summary_parts.push(pkg);
    }

    if msisdn.is_none() && summary_parts.is_empty() {
        return None;
    }

    let label = match &balance_str {
        Some(b) => format!("eSIM.gg · {b}"),
        None => "eSIM.gg".to_string(),
    };

    Some(ProfileProviderBadge {
        kind: ProviderPluginKind::EsimGg,
        label,
        msisdn,
        quota_summary: (!summary_parts.is_empty()).then(|| summary_parts.join(" · ")),
        portal_url: Some("https://esim.gg/".to_string()),
    })
}

/// Parses a 3HK (`three.com.hk`) status JSON response into a [`ProfileProviderBadge`].
#[must_use]
pub fn parse_three_hk_response(json: &str, _iccid: &str) -> Option<ProfileProviderBadge> {
    let root: serde_json::Value = serde_json::from_str(json).ok()?;
    let obj = root.as_object()?;

    let raw_msisdn = obj
        .get("msisdn")
        .or_else(|| obj.get("number"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let msisdn = raw_msisdn.map(|m| {
        if m.starts_with('+') {
            m.to_string()
        } else {
            format!("+852{m}")
        }
    });

    let line_expiry = obj
        .get("line_expiry")
        .or_else(|| obj.get("expireDate"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let packages: &[serde_json::Value] = obj
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .map_or(&[], Vec::as_slice);

    if msisdn.is_none() && line_expiry.is_none() && packages.is_empty() {
        return None;
    }

    let mut summary_parts = Vec::new();
    for pkg in packages {
        if let Some(pkg_obj) = pkg.as_object() {
            let name = pkg_obj
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Data")
                .trim();
            let total_gb = pkg_obj
                .get("total")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let used_gb = pkg_obj
                .get("used")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let total_bytes = gb_to_bytes(total_gb);
            let used_bytes = gb_to_bytes(used_gb);
            let mut text = format!(
                "{name}: {} / {}",
                format_size_bytes(used_bytes, SizeUnit::AdaptiveBi),
                format_size_bytes(total_bytes, SizeUnit::AdaptiveBi),
            );
            if let Some(exp) = pkg_obj
                .get("expiry")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let _ = write!(text, " ({exp})");
            }
            summary_parts.push(text);
        }
    }
    if let Some(exp) = line_expiry {
        summary_parts.push(format!("Exp {exp}"));
    }

    Some(ProfileProviderBadge {
        kind: ProviderPluginKind::ThreeHk,
        label: "3HK".to_string(),
        msisdn,
        quota_summary: (!summary_parts.is_empty()).then(|| summary_parts.join(" · ")),
        portal_url: Some("https://www.three.com.hk/".to_string()),
    })
}

fn select_esim_gg_item<'a>(
    root: &'a serde_json::Value,
    iccid: &str,
) -> Option<&'a serde_json::Value> {
    if let Some(results) = root.get("results").and_then(serde_json::Value::as_array) {
        let target = strip_trailing_f(iccid);
        if target.is_empty() {
            return results.first();
        }
        return results.iter().find(|entry| {
            entry
                .get("iccid")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| strip_trailing_f(id).eq_ignore_ascii_case(target))
        });
    }
    Some(root)
}

fn strip_trailing_f(iccid: &str) -> &str {
    let trimmed = iccid.trim();
    trimmed
        .strip_suffix('F')
        .or_else(|| trimmed.strip_suffix('f'))
        .unwrap_or(trimmed)
}

fn value_as_u64(v: &serde_json::Value) -> Option<u64> {
    if let Some(u) = v.as_u64() {
        Some(u)
    } else {
        v.as_f64()
            .filter(|&f| f.is_finite() && f >= 0.0)
            .map(|f| f.round() as u64)
    }
}

fn gb_to_bytes(gb: f64) -> u64 {
    if !gb.is_finite() || gb <= 0.0 {
        return 0;
    }
    (gb * 1024.0 * 1024.0 * 1024.0).round() as u64
}
