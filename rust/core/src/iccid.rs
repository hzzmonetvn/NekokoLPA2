//! Utilities for cleaning and formatting SIM ICCID identifiers.

/// Strips the BCD filler `'F'` or `'f'` from the end of an ICCID for display.
///
/// Only trailing filler is removed. An `'F'` in the middle of an ICCID is real
/// and occurs in the wild on certain telecommunications test profiles.
#[must_use]
pub fn iccid_for_display(iccid: &str) -> &str {
    iccid.trim_end_matches(['F', 'f'])
}

/// Formats an ICCID into 4-digit groups for industrial readability (e.g. `8984 0400 0000 0000 001`).
#[must_use]
pub fn format_iccid_grouped(iccid: &str) -> String {
    let cleaned = iccid_for_display(iccid);
    let mut out = String::with_capacity(cleaned.len() + cleaned.len() / 4);
    for (idx, ch) in cleaned.chars().enumerate() {
        if idx > 0 && idx % 4 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}
