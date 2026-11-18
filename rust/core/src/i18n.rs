//! Localization catalog embedding all 11 ARB translation bundles (`l10n/app_*.arb`),
//! locale preference resolution (including `Tiếng Việt`), and placeholder formatting.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// User-selectable application locale preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LocaleChoice {
    #[default]
    System,
    En,
    Vi,
    Zh,
    ZhTw,
    Ja,
    Ko,
    De,
    Es,
    Fr,
    It,
    Ru,
}

/// Complete list of all supported locale choices for Settings selectors.
pub const ALL_LOCALE_CHOICES: &[LocaleChoice] = &[
    LocaleChoice::System,
    LocaleChoice::En,
    LocaleChoice::Vi,
    LocaleChoice::Zh,
    LocaleChoice::ZhTw,
    LocaleChoice::Ja,
    LocaleChoice::Ko,
    LocaleChoice::De,
    LocaleChoice::Es,
    LocaleChoice::Fr,
    LocaleChoice::It,
    LocaleChoice::Ru,
];

impl LocaleChoice {
    /// Returns the BCP-47/ARB code for this locale choice (`"system"`, `"en"`, `"vi"`, etc.).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::En => "en",
            Self::Vi => "vi",
            Self::Zh => "zh",
            Self::ZhTw => "zh-TW",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::De => "de",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::It => "it",
            Self::Ru => "ru",
        }
    }

    /// Returns the native display label for this locale choice.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::En => "English",
            Self::Vi => "Tiếng Việt",
            Self::Zh => "简体中文",
            Self::ZhTw => "繁體中文",
            Self::Ja => "日本語",
            Self::Ko => "한국어",
            Self::De => "Deutsch",
            Self::Es => "Español",
            Self::Fr => "Français",
            Self::It => "Italiano",
            Self::Ru => "Русский",
        }
    }

    /// Parses a persisted locale code into a [`LocaleChoice`].
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        let normalized = code.trim().replace('_', "-").to_ascii_lowercase();
        match normalized.as_str() {
            "" | "system" => Some(Self::System),
            "en" => Some(Self::En),
            "vi" => Some(Self::Vi),
            "zh" | "zh-cn" | "zh-sg" | "zh-hans" => Some(Self::Zh),
            "zh-tw" | "zh-hk" | "zh-mo" | "zh-hant" => Some(Self::ZhTw),
            "ja" => Some(Self::Ja),
            "ko" => Some(Self::Ko),
            "de" => Some(Self::De),
            "es" => Some(Self::Es),
            "fr" => Some(Self::Fr),
            "it" => Some(Self::It),
            "ru" => Some(Self::Ru),
            _ => None,
        }
    }
}

/// Resolves an explicit [`LocaleChoice`] or `LocaleChoice::System` + BCP-47 `system_tag`
/// into a concrete loaded locale (never returning `LocaleChoice::System`).
#[must_use]
pub fn resolve_locale(choice: LocaleChoice, system_tag: &str) -> LocaleChoice {
    if choice != LocaleChoice::System {
        return choice;
    }

    let normalized = system_tag.trim().replace('_', "-").to_ascii_lowercase();
    if normalized.is_empty() {
        return LocaleChoice::En;
    }

    let mut parts = normalized.split('-');
    let lang = parts.next().unwrap_or("");
    let rest: Vec<&str> = parts.collect();

    match lang {
        "zh" => {
            let is_trad = rest
                .iter()
                .any(|&sub| matches!(sub, "tw" | "hk" | "mo" | "hant"));
            if is_trad {
                LocaleChoice::ZhTw
            } else {
                LocaleChoice::Zh
            }
        }
        "vi" => LocaleChoice::Vi,
        "ja" => LocaleChoice::Ja,
        "ko" => LocaleChoice::Ko,
        "de" => LocaleChoice::De,
        "es" => LocaleChoice::Es,
        "fr" => LocaleChoice::Fr,
        "it" => LocaleChoice::It,
        "ru" => LocaleChoice::Ru,
        _ => LocaleChoice::En,
    }
}

const LOCALE_ARB_SOURCES: [(LocaleChoice, &str); 11] = [
    (LocaleChoice::En, include_str!("../../../l10n/app_en.arb")),
    (LocaleChoice::Vi, include_str!("../../../l10n/app_vi.arb")),
    (LocaleChoice::Zh, include_str!("../../../l10n/app_zh.arb")),
    (
        LocaleChoice::ZhTw,
        include_str!("../../../l10n/app_zh-TW.arb"),
    ),
    (LocaleChoice::Ja, include_str!("../../../l10n/app_ja.arb")),
    (LocaleChoice::Ko, include_str!("../../../l10n/app_ko.arb")),
    (LocaleChoice::De, include_str!("../../../l10n/app_de.arb")),
    (LocaleChoice::Es, include_str!("../../../l10n/app_es.arb")),
    (LocaleChoice::Fr, include_str!("../../../l10n/app_fr.arb")),
    (LocaleChoice::It, include_str!("../../../l10n/app_it.arb")),
    (LocaleChoice::Ru, include_str!("../../../l10n/app_ru.arb")),
];

/// In-memory translation catalog populated once from all 11 embedded `l10n/app_*.arb` files.
#[derive(Clone, Debug)]
pub struct I18nCatalog {
    tables: [BTreeMap<String, String>; 11],
}

impl Default for I18nCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl I18nCatalog {
    /// Parses all 11 embedded ARB localization bundles into memory.
    #[must_use]
    pub fn new() -> Self {
        let tables = std::array::from_fn(|idx| parse_arb_messages(LOCALE_ARB_SOURCES[idx].1));
        Self { tables }
    }

    /// Returns the number of message keys loaded for `locale`.
    #[must_use]
    pub fn len(&self, locale: LocaleChoice) -> usize {
        let idx = locale_table_index(locale);
        self.tables[idx].len()
    }

    /// Returns `true` if no message keys are loaded for `locale`.
    #[must_use]
    pub fn is_empty(&self, locale: LocaleChoice) -> bool {
        self.len(locale) == 0
    }

    /// Looks up a translation string for `locale`, falling back to `LocaleChoice::En`.
    #[must_use]
    pub fn lookup(&self, locale: LocaleChoice, key: &str) -> Option<&str> {
        let idx = locale_table_index(locale);
        self.tables[idx]
            .get(key)
            .map(String::as_str)
            .or_else(|| self.tables[0].get(key).map(String::as_str))
    }

    /// Returns the translation string for `locale`, falling back to `LocaleChoice::En`
    /// and returning `""` if `key` does not exist in any bundle.
    #[must_use]
    pub fn get(&self, locale: LocaleChoice, key: &str) -> &str {
        self.lookup(locale, key).unwrap_or("")
    }

    /// Formats a translated template string by substituting `{name}` placeholders (and simple
    /// ICU `{name, plural, ...}` clauses), falling back to `LocaleChoice::En` then `key`.
    #[must_use]
    pub fn format(&self, locale: LocaleChoice, key: &str, args: &[(&str, &str)]) -> String {
        let template = self.lookup(locale, key).unwrap_or(key);
        render_template(template, args)
    }
}

const fn locale_table_index(locale: LocaleChoice) -> usize {
    match locale {
        LocaleChoice::System | LocaleChoice::En => 0,
        LocaleChoice::Vi => 1,
        LocaleChoice::Zh => 2,
        LocaleChoice::ZhTw => 3,
        LocaleChoice::Ja => 4,
        LocaleChoice::Ko => 5,
        LocaleChoice::De => 6,
        LocaleChoice::Es => 7,
        LocaleChoice::Fr => 8,
        LocaleChoice::It => 9,
        LocaleChoice::Ru => 10,
    }
}

fn parse_arb_messages(src: &str) -> BTreeMap<String, String> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(src) else {
        return BTreeMap::new();
    };
    let mut out = BTreeMap::new();
    for (key, value) in map {
        if key.starts_with('@') {
            continue;
        }
        if let serde_json::Value::String(text) = value {
            out.insert(key, text);
        }
    }
    out
}

fn render_template(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut pos = 0usize;

    while pos < bytes.len() {
        if bytes[pos] == b'{' {
            if let Some(close_rel) = find_matching_brace(&template[pos..]) {
                let expr = &template[pos + 1..pos + close_rel];
                if let Some(rendered) = evaluate_placeholder(expr, args) {
                    out.push_str(&rendered);
                } else {
                    out.push('{');
                    out.push_str(expr);
                    out.push('}');
                }
                pos += close_rel + 1;
                continue;
            }
        }
        let Some(ch) = template[pos..].chars().next() else {
            break;
        };
        out.push(ch);
        pos += ch.len_utf8();
    }

    out
}

fn find_matching_brace(slice: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in slice.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn evaluate_placeholder(expr: &str, args: &[(&str, &str)]) -> Option<String> {
    if let Some((arg_name, rest)) = expr.split_once(',') {
        let trimmed_name = arg_name.trim();
        let rest_trimmed = rest.trim_start();
        if let Some(plural_body) = rest_trimmed.strip_prefix("plural,") {
            let count_str = lookup_arg(trimmed_name, args)?;
            let count = count_str.trim().parse::<i64>().ok()?;
            let chosen = pick_plural_branch(plural_body.trim(), count)?;
            return Some(render_template(chosen, args));
        }
    }

    lookup_arg(expr.trim(), args).map(str::to_string)
}

fn lookup_arg<'a>(name: &str, args: &[(&str, &'a str)]) -> Option<&'a str> {
    args.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
}

fn pick_plural_branch(body: &str, count: i64) -> Option<&str> {
    let mut exact_match: Option<&str> = None;
    let mut one_match: Option<&str> = None;
    let mut other_match: Option<&str> = None;

    let mut pos = 0usize;
    let bytes = body.as_bytes();
    while pos < bytes.len() {
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let Some(open_rel) = body[pos..].find('{') else {
            break;
        };
        let selector = body[pos..pos + open_rel].trim();
        let brace_start = pos + open_rel;
        let close_rel = find_matching_brace(&body[brace_start..])?;
        let branch_text = &body[brace_start + 1..brace_start + close_rel];

        if let Some(num_str) = selector.strip_prefix('=') {
            if num_str.trim().parse::<i64>().ok() == Some(count) {
                exact_match = Some(branch_text);
            }
        } else if selector == "one" {
            one_match = Some(branch_text);
        } else if selector == "other" {
            other_match = Some(branch_text);
        }

        pos = brace_start + close_rel + 1;
    }

    exact_match
        .or(if count == 1 { one_match } else { None })
        .or(other_match)
}
