//! Update feed selector verifying channel isolation (`open`/`community` vs
//! `open-rooted`/`community-rooted`), APK signing certificate SHA-1 pinning,
//! and anti-downgrade build comparisons.

use serde::{Deserialize, Serialize};

/// SHA-1 certificate fingerprint of the public Community / Open Android signing key.
pub const COMMUNITY_CERT: &str = "c47350c7ba682b34a3e584a0d58463ea42b1ad73";

/// Default NekokoLPA 2 update feed endpoint.
pub const DEFAULT_UPDATE_FEED_URL: &str = "https://updates.lpa.ee/nlpa2.json";

/// Metadata for a newer update candidate matching channel and certificate constraints.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UpdateCandidate {
    pub version: String,
    pub build: u64,
    pub url: String,
    pub download_url: String,
    pub changelog: String,
}

/// Evaluates `feed_json` for a Community/Open Android update strictly pinned to
/// [`COMMUNITY_CERT`] and isolated between normal (`open`/`community`) and privileged
/// (`open-rooted`/`community-rooted`) channels.
#[must_use]
pub fn check_update_feed(
    feed_json: &str,
    is_privileged: bool,
    current_build: u64,
    cert_sha1: &str,
) -> Option<UpdateCandidate> {
    let normalized_cert = normalize_sha1(cert_sha1);
    if !is_sha1_hex(&normalized_cert) || normalized_cert != COMMUNITY_CERT {
        return None;
    }

    let variant = if is_privileged {
        "community-rooted"
    } else {
        "community"
    };

    select_android_update(feed_json, variant, &[normalized_cert.as_str()], current_build)
}

/// General Android update feed selector supporting multi-platform `nlpa2.json` objects or
/// direct Android channel maps, isolating public channels (`open`/`community` and
/// `open-rooted`/`community-rooted`) from unrestricted/vendor entries.
#[must_use]
pub fn select_android_update(
    feed_json: &str,
    variant: &str,
    signatures: &[&str],
    current_build: u64,
) -> Option<UpdateCandidate> {
    let root: serde_json::Value = serde_json::from_str(feed_json).ok()?;
    let root_obj = root.as_object()?;

    let os_config = root_obj
        .get("android")
        .or_else(|| root_obj.get("android-armeabi-v7a"))
        .and_then(serde_json::Value::as_object)
        .unwrap_or(root_obj);

    let channel = match variant {
        "open" => "community",
        "open-rooted" => "community-rooted",
        other => other,
    };

    let mut channels = vec![channel];
    if channel == "community" {
        channels.push("open");
    } else if channel == "community-rooted" {
        channels.push("open-rooted");
    }

    let installed: Vec<String> = signatures
        .iter()
        .map(|s| normalize_sha1(s))
        .filter(|s| !s.is_empty())
        .collect();

    let public_build = matches!(channel, "community" | "community-rooted");
    let mut highest = current_build;
    let mut best: Option<UpdateCandidate> = None;

    for (key, value) in os_config {
        if !matches_channel_key(key, &channels) {
            continue;
        }

        let entries: Vec<&serde_json::Value> = match value {
            serde_json::Value::Array(arr) => arr.iter().collect(),
            serde_json::Value::Object(_) => vec![value],
            _ => continue,
        };

        for entry in entries {
            let Some(obj) = entry.as_object() else {
                continue;
            };
            let Some(build) = obj.get("build").and_then(serde_json::Value::as_u64) else {
                continue;
            };
            if build <= highest {
                continue;
            }

            let allowed = obj.get("signatures").and_then(serde_json::Value::as_array);
            let unrestricted = allowed.is_none_or(Vec::is_empty);
            if unrestricted && public_build {
                continue;
            }

            let signature_matches = unrestricted
                || allowed.is_some_and(|sigs| {
                    sigs.iter()
                        .filter_map(serde_json::Value::as_str)
                        .any(|s| installed.iter().any(|inst| inst == &normalize_sha1(s)))
                });
            if !signature_matches {
                continue;
            }

            highest = build;
            best = Some(UpdateCandidate {
                version: obj
                    .get("version")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                build,
                url: obj
                    .get("url")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                download_url: obj
                    .get("download_url")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                changelog: obj
                    .get("changelog")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }

    best
}

fn matches_channel_key(key: &str, channels: &[&str]) -> bool {
    channels.iter().any(|name| {
        if key == *name {
            return true;
        }
        let Some(suffix) = key
            .strip_prefix(name)
            .and_then(|rest| rest.strip_prefix('-'))
        else {
            return false;
        };
        !suffix.is_empty() && suffix.split('-').all(is_sha1_hex)
    })
}

fn normalize_sha1(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|c| *c != ':')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn is_sha1_hex(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
