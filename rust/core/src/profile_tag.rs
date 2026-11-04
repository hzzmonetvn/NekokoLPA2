//! Parsing, formatting, and escaping profile nickname tags.

pub const TAG_SEPARATOR: char = '\n';
pub const INTERNAL_SPACE: char = '\u{11}';
pub const INTERNAL_COLON: char = '\u{03}';

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileTag {
    Text(String),
    Date { raw: String },
    Raw(String),
}

impl ProfileTag {
    #[must_use]
    pub fn raw(&self) -> String {
        match self {
            Self::Text(text) => format!("t:{}", escape_content(text)),
            Self::Date { raw } | Self::Raw(raw) => raw.clone(),
        }
    }

    #[must_use]
    pub fn display_date(&self) -> Option<String> {
        let Self::Date { raw } = self else {
            return None;
        };
        let content = raw.strip_prefix("d:").or_else(|| raw.strip_prefix("D:"))?;
        let date = content.split(':').next()?;
        if date.len() == 6 && date.bytes().all(|b| b.is_ascii_digit()) {
            Some(format!("20{}-{}-{}", &date[..2], &date[2..4], &date[4..]))
        } else if date.len() >= 8 && date[..8].bytes().all(|b| b.is_ascii_digit()) {
            Some(format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..8]))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNickname {
    pub name: String,
    pub tags: Vec<ProfileTag>,
    pub display_name: String,
}

#[must_use]
pub fn escape_content(value: &str) -> String {
    value
        .replace(' ', &INTERNAL_SPACE.to_string())
        .replace(':', &INTERNAL_COLON.to_string())
}

#[must_use]
pub fn unescape_content(value: &str) -> String {
    value
        .replace(INTERNAL_SPACE, " ")
        .replace(INTERNAL_COLON, ":")
}

#[must_use]
pub fn parse_tag(raw: &str) -> ProfileTag {
    let prefix = raw.get(..2).unwrap_or("");
    if prefix.eq_ignore_ascii_case("d:") {
        ProfileTag::Date {
            raw: raw.to_owned(),
        }
    } else if prefix.eq_ignore_ascii_case("t:") {
        ProfileTag::Text(unescape_content(&raw[2..]))
    } else {
        ProfileTag::Text(unescape_content(raw.strip_prefix("t:").unwrap_or(raw)))
    }
}

#[must_use]
pub fn parse_nickname(full: Option<&str>) -> ParsedNickname {
    let Some(full) = full.filter(|s| !s.is_empty()) else {
        return ParsedNickname {
            name: String::new(),
            tags: Vec::new(),
            display_name: String::new(),
        };
    };
    let bytes = full.as_bytes();
    let mut tags = Vec::new();
    let mut ranges = Vec::new();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if (i == 0 || !(bytes[i - 1] as char).is_alphanumeric())
            && matches!(bytes[i].to_ascii_lowercase(), b't' | b'd')
            && bytes[i + 1] == b':'
            && (i + 2 == bytes.len() || bytes[i + 2] != b' ')
        {
            let mut end = i + 2;
            while end < bytes.len()
                && (!bytes[end].is_ascii_whitespace() || bytes[end] == INTERNAL_SPACE as u8)
            {
                end += 1;
            }
            let tag = &full[i..end];
            tags.push(parse_tag(tag));
            ranges.push((i, end));
            i = end;
        } else {
            i += 1;
        }
    }
    let mut name = String::new();
    let mut cursor = 0;
    for (start, end) in ranges {
        name.push_str(&full[cursor..start]);
        cursor = end;
    }
    name.push_str(&full[cursor..]);
    name = name
        .replace(TAG_SEPARATOR, "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    ParsedNickname {
        display_name: name.clone(),
        name,
        tags,
    }
}

#[must_use]
pub fn format_nickname(name: &str, tags: &[ProfileTag]) -> String {
    if tags.is_empty() {
        name.to_owned()
    } else {
        format!(
            "{name}{TAG_SEPARATOR}{}",
            tags.iter()
                .map(ProfileTag::raw)
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escape_unescape_roundtrip() {
        let s = "a b:c";
        assert_eq!(unescape_content(&escape_content(s)), s);
    }
    #[test]
    fn parses_tags() {
        let p = parse_nickname(Some("Alice t:hello"));
        assert_eq!(p.name, "Alice");
        assert_eq!(p.tags.len(), 1);
    }
    #[test]
    fn parses_short_date() {
        assert_eq!(
            parse_tag("d:240101").display_date().as_deref(),
            Some("2024-01-01")
        );
    }
    #[test]
    fn unknown_prefix_falls_back_to_text() {
        assert!(matches!(parse_tag("x:abc"), ProfileTag::Text(_)));
    }
    #[test]
    fn empty_nickname() {
        assert_eq!(parse_nickname(None).name, "");
    }
    #[test]
    fn formats_tags() {
        let tag = parse_tag("t:hello");
        assert_eq!(format_nickname("Alice", &[tag]), "Alice\nt:hello");
    }
}
