//! GSMA SGP.22 notification-server FQDN validation.
//!
//! Notification metadata originates from the eUICC. Faulty or non-compliant
//! card implementations may return corrupted or malformed metadata from
//! ListNotification. Strict FQDN checking prevents dispatching network
//! requests to malformed hostnames and allows falling back to signed notifications.

/// Returns whether `address` is a valid, usable GSMA notification-server FQDN.
#[must_use]
pub fn is_valid_notification_address(address: &str) -> bool {
    if address.is_empty() || address != address.trim() {
        return false;
    }

    if address.len() > 253 || !address.is_ascii() {
        return false;
    }

    let fqdn = address.strip_suffix('.').unwrap_or(address);
    let labels: Vec<&str> = fqdn.split('.').collect();
    if labels.len() < 2 {
        return false;
    }

    for label in &labels {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return false;
        }
    }

    let top_level = match labels.last() {
        Some(tld) => *tld,
        None => return false,
    };

    let mut chars = top_level.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
        }
        _ => false,
    }
}
