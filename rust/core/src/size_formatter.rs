//! Formatting helpers for byte sizes.

use crate::redaction::SizeUnit;

#[must_use]
pub fn format_size(bytes: Option<i64>, precision: usize, unit: SizeUnit) -> String {
    let Some(bytes) = bytes else {
        return "Unknown".into();
    };
    let value = bytes.max(0) as u64;
    match unit {
        SizeUnit::B => format!("{} B", with_commas(&value.to_string())),
        SizeUnit::Kb => format!(
            "{} kB",
            with_commas(&format!("{:.precision$}", value as f64 / 1000.0))
        ),
        SizeUnit::Kib => format!(
            "{} kiB",
            with_commas(&format!("{:.precision$}", value as f64 / 1024.0))
        ),
        SizeUnit::AdaptiveBi | SizeUnit::AdaptiveSi => {
            if value < 1024 {
                return format!("{} B", with_commas(&value.to_string()));
            }
            let (divisor, label) = if value < 1024_u64.pow(2) {
                (
                    1024.0,
                    if unit == SizeUnit::AdaptiveBi {
                        "kiB"
                    } else {
                        "KB"
                    },
                )
            } else if value < 1024_u64.pow(3) {
                (
                    1024.0_f64.powi(2),
                    if unit == SizeUnit::AdaptiveBi {
                        "MiB"
                    } else {
                        "MB"
                    },
                )
            } else {
                (
                    1024.0_f64.powi(3),
                    if unit == SizeUnit::AdaptiveBi {
                        "GiB"
                    } else {
                        "GB"
                    },
                )
            };
            format!(
                "{} {label}",
                with_commas(&format!("{:.precision$}", value as f64 / divisor))
            )
        }
    }
}

#[must_use]
pub fn with_commas(value: &str) -> String {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    let mut out = String::new();
    for (index, ch) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if !fraction.is_empty() {
        out.push('.');
        out.push_str(fraction);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_size_is_unknown() {
        assert_eq!(format_size(None, 2, SizeUnit::B), "Unknown");
    }
    #[test]
    fn adaptive_threshold() {
        assert_eq!(format_size(Some(1024), 2, SizeUnit::AdaptiveBi), "1.00 kiB");
    }
    #[test]
    fn groups_commas() {
        assert_eq!(with_commas("1234567"), "1,234,567");
    }
}
