//! Pure profile-data redaction and display helpers.

use std::fmt::Write;

const MASK: char = '*';
const SHUFFLED_COUNTRY_CODES: [&str; 20] = [
    "US", "JP", "GB", "DE", "FR", "CA", "AU", "KR", "SG", "NL", "CH", "SE", "NO", "DK", "FI", "ES",
    "IT", "PT", "IE", "NZ",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NicknameRedactionMode {
    #[default]
    None,
    RedactPhoneNumbers,
    HideNickname,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OperatorRedactionMode {
    #[default]
    None,
    CountryName,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagsRedactionMode {
    #[default]
    None,
    HideTags,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlagRedactionMode {
    #[default]
    None,
    HideFlag,
    ShuffledFlags,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconRedactionMode {
    #[default]
    None,
    HideIcon,
    RandomIcon,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SizeUnit {
    B,
    Kb,
    Kib,
    AdaptiveBi,
    #[default]
    AdaptiveSi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionSettings {
    pub reveal_sensitive_profile_data: bool,
    pub redact_eid_first: bool,
    pub redact_eid_middle: bool,
    pub redact_eid_last: bool,
    pub redact_iccid_first: bool,
    pub redact_iccid_middle: bool,
    pub redact_iccid_last: bool,
    pub nickname_redaction_mode: NicknameRedactionMode,
    pub operator_redaction_mode: OperatorRedactionMode,
    pub tags_redaction_mode: TagsRedactionMode,
    pub flag_redaction_mode: FlagRedactionMode,
    pub icon_redaction_mode: IconRedactionMode,
}
impl Default for RedactionSettings {
    fn default() -> Self {
        Self {
            reveal_sensitive_profile_data: false,
            redact_eid_first: true,
            redact_eid_middle: false,
            redact_eid_last: true,
            redact_iccid_first: true,
            redact_iccid_middle: true,
            redact_iccid_last: true,
            nickname_redaction_mode: NicknameRedactionMode::None,
            operator_redaction_mode: OperatorRedactionMode::None,
            tags_redaction_mode: TagsRedactionMode::None,
            flag_redaction_mode: FlagRedactionMode::None,
            icon_redaction_mode: IconRedactionMode::None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProfileView {
    pub nickname: Option<String>,
    pub name: Option<String>,
    pub service_provider_name: Option<String>,
    pub flag: String,
    pub iccid: String,
    pub display_name: String,
    pub flag_url: String,
    pub flag_emoji: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorPlaceholder {
    pub country_code: String,
    pub label: String,
}

#[must_use]
pub fn stable_index(seed: &str, modulo: usize) -> usize {
    if modulo == 0 {
        return 0;
    }
    let hash = seed.as_bytes().iter().fold(0i64, |hash, byte| {
        (hash * 31 + i64::from(*byte)) & 0x7fff_ffff
    });
    hash as usize % modulo
}

#[must_use]
pub fn redact_eid(eid: &str, reveal_full: bool, settings: &RedactionSettings) -> String {
    if eid.is_empty() {
        return String::new();
    }
    if reveal_full || settings.reveal_sensitive_profile_data {
        return eid.to_owned();
    }
    redact_eid_preview(eid, settings)
}
#[must_use]
pub fn redact_eid_preview(eid: &str, settings: &RedactionSettings) -> String {
    redact_ranges(
        eid,
        &[
            (0, 8, settings.redact_eid_first),
            (8, 24, settings.redact_eid_middle),
            (
                eid.chars().count().saturating_sub(10),
                eid.chars().count(),
                settings.redact_eid_last,
            ),
        ],
    )
}
#[must_use]
pub fn redact_iccid(iccid: &str, settings: &RedactionSettings) -> String {
    if settings.reveal_sensitive_profile_data {
        iccid.to_owned()
    } else {
        redact_iccid_preview(iccid, settings)
    }
}
#[must_use]
pub fn redact_iccid_preview(iccid: &str, settings: &RedactionSettings) -> String {
    let len = iccid.chars().count();
    redact_ranges(
        iccid,
        &[
            (0, 8, settings.redact_iccid_first),
            (8, 14, settings.redact_iccid_middle),
            (len.saturating_sub(6), len, settings.redact_iccid_last),
        ],
    )
}
fn redact_ranges(source: &str, ranges: &[(usize, usize, bool)]) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut result = String::new();
    let mut stars = 0;
    for (index, ch) in chars.iter().enumerate() {
        let keep = ranges
            .iter()
            .any(|(start, end, enabled)| *enabled && index >= *start && index < *end);
        if keep {
            flush_stars(&mut result, &mut stars);
            result.push(*ch);
        } else {
            stars += 1;
        }
    }
    flush_stars(&mut result, &mut stars);
    result
}
fn flush_stars(out: &mut String, stars: &mut usize) {
    if *stars > 0 {
        out.extend(std::iter::repeat_n(MASK, (*stars).min(8)));
        *stars = 0;
    }
}

#[must_use]
pub fn display_name(profile: &ProfileView, settings: &RedactionSettings) -> String {
    if settings.reveal_sensitive_profile_data {
        return profile.display_name.clone();
    }
    let nickname_visible = profile
        .nickname
        .as_deref()
        .is_some_and(|n| !n.trim().is_empty())
        && settings.nickname_redaction_mode != NicknameRedactionMode::HideNickname;
    let base = if nickname_visible {
        profile.display_name.clone()
    } else if settings.operator_redaction_mode == OperatorRedactionMode::CountryName {
        operator_title(profile, &[])
    } else if settings.nickname_redaction_mode == NicknameRedactionMode::HideNickname {
        fallback_display_name(profile)
    } else {
        profile.display_name.clone()
    };
    if settings.nickname_redaction_mode == NicknameRedactionMode::RedactPhoneNumbers {
        mask_phone_numbers(&base)
    } else {
        base
    }
}
fn fallback_display_name(profile: &ProfileView) -> String {
    profile
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            profile
                .service_provider_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("Unnamed Profile")
        .to_owned()
}
fn mask_phone_numbers(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut output = String::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '+' || chars[index].is_ascii_digit() {
            let start = index;
            let mut digits = 0;
            while index < chars.len()
                && (chars[index].is_ascii_digit()
                    || (digits > 0 && " \t().-".contains(chars[index])))
            {
                if chars[index].is_ascii_digit() {
                    digits += 1;
                }
                index += 1;
            }
            if digits >= 7 {
                output.extend(std::iter::repeat_n(MASK, index - start));
                continue;
            }
            output.extend(&chars[start..index]);
        } else {
            output.push(chars[index]);
            index += 1;
        }
    }
    output
}
#[must_use]
pub fn operator_summary(
    profile: &ProfileView,
    settings: &RedactionSettings,
    placeholders: &[OperatorPlaceholder],
) -> Option<String> {
    if settings.reveal_sensitive_profile_data {
        return original_operator_summary(profile);
    }
    if settings.operator_redaction_mode == OperatorRedactionMode::CountryName {
        return Some(operator_placeholder(profile, settings, placeholders));
    }
    if let Some(p) = shuffled_placeholder(profile, settings, placeholders) {
        return Some(p.label.clone());
    }
    original_operator_summary(profile)
}
fn original_operator_summary(profile: &ProfileView) -> Option<String> {
    let provider = profile
        .service_provider_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let name = profile
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter(|n| Some(*n) != provider);
    match (provider, name) {
        (Some(p), Some(n)) => Some(format!("{p} / {n}")),
        (Some(p), None) => Some(p.into()),
        (None, Some(n)) => Some(n.into()),
        _ => None,
    }
}
#[must_use]
pub fn filter_tags(source: &[String], settings: &RedactionSettings) -> Vec<String> {
    if !settings.reveal_sensitive_profile_data
        && settings.tags_redaction_mode == TagsRedactionMode::HideTags
    {
        Vec::new()
    } else {
        source.to_vec()
    }
}
#[must_use]
pub fn flag_url(
    profile: &ProfileView,
    settings: &RedactionSettings,
    placeholders: &[OperatorPlaceholder],
) -> String {
    if settings.reveal_sensitive_profile_data {
        return profile.flag_url.clone();
    }
    if let Some(p) = shuffled_placeholder(profile, settings, placeholders) {
        return flag_url_from_code(&p.country_code);
    }
    if settings.flag_redaction_mode == FlagRedactionMode::ShuffledFlags {
        return flag_url_from_code(shuffled_country_code(&profile.iccid));
    }
    profile.flag_url.clone()
}
#[must_use]
pub fn flag_emoji(code: &str) -> String {
    if code.chars().count() != 2 {
        return "🏳".into();
    }
    code.to_ascii_uppercase()
        .chars()
        .map(|c| char::from_u32(u32::from(c) + 127397).unwrap_or('�'))
        .collect()
}
#[must_use]
pub fn profile_flag_emoji(
    profile: &ProfileView,
    settings: &RedactionSettings,
    placeholders: &[OperatorPlaceholder],
) -> String {
    if settings.reveal_sensitive_profile_data {
        return profile.flag_emoji.clone();
    }
    if let Some(p) = shuffled_placeholder(profile, settings, placeholders) {
        return flag_emoji(&p.country_code);
    }
    if settings.flag_redaction_mode == FlagRedactionMode::ShuffledFlags {
        return flag_emoji(shuffled_country_code(&profile.iccid));
    }
    profile.flag_emoji.clone()
}
#[must_use]
pub fn hide_flag(settings: &RedactionSettings) -> bool {
    !settings.reveal_sensitive_profile_data
        && settings.flag_redaction_mode == FlagRedactionMode::HideFlag
}
#[must_use]
pub fn hide_icon(settings: &RedactionSettings) -> bool {
    !settings.reveal_sensitive_profile_data
        && settings.icon_redaction_mode == IconRedactionMode::HideIcon
}
#[must_use]
pub fn random_icon(settings: &RedactionSettings) -> bool {
    !settings.reveal_sensitive_profile_data
        && settings.icon_redaction_mode == IconRedactionMode::RandomIcon
}
#[must_use]
pub fn country_name(code: &str) -> String {
    let code = code.to_ascii_uppercase();
    country_names()
        .iter()
        .find(|(key, _)| *key == code)
        .map_or(code.clone(), |(_, value)| (*value).to_owned())
}
#[must_use]
pub fn operator_title(profile: &ProfileView, placeholders: &[OperatorPlaceholder]) -> String {
    shuffled_placeholder(
        profile,
        &RedactionSettings {
            flag_redaction_mode: FlagRedactionMode::ShuffledFlags,
            ..Default::default()
        },
        placeholders,
    )
    .map_or_else(|| country_name(&profile.flag), |p| p.label.clone())
}
#[must_use]
pub fn operator_placeholder(
    profile: &ProfileView,
    settings: &RedactionSettings,
    placeholders: &[OperatorPlaceholder],
) -> String {
    if !settings.reveal_sensitive_profile_data
        && settings.flag_redaction_mode == FlagRedactionMode::HideFlag
    {
        return "Carrier".into();
    }
    if let Some(p) = shuffled_placeholder(profile, settings, placeholders) {
        return p.label.clone();
    }
    let code = if !settings.reveal_sensitive_profile_data
        && settings.flag_redaction_mode == FlagRedactionMode::ShuffledFlags
    {
        shuffled_country_code(&profile.iccid).to_owned()
    } else {
        profile.flag.to_ascii_uppercase()
    };
    format!("{} Carrier", country_name(&code))
}
#[must_use]
pub fn placeholder_label_from_metadata(
    provider_name: Option<&str>,
    name: Option<&str>,
    display_name: &str,
) -> Option<String> {
    provider_name
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .or_else(|| name.map(str::trim).filter(|v| !v.is_empty()))
        .or_else(|| {
            let n = display_name.trim();
            (!n.is_empty() && n != "Unknown Profile").then_some(n)
        })
        .map(str::to_owned)
}
fn shuffled_placeholder<'a>(
    profile: &ProfileView,
    settings: &RedactionSettings,
    placeholders: &'a [OperatorPlaceholder],
) -> Option<&'a OperatorPlaceholder> {
    if settings.reveal_sensitive_profile_data
        || settings.flag_redaction_mode != FlagRedactionMode::ShuffledFlags
        || placeholders.is_empty()
    {
        None
    } else {
        placeholders.get(stable_index(&profile.iccid, placeholders.len()))
    }
}
fn shuffled_country_code(seed: &str) -> &'static str {
    SHUFFLED_COUNTRY_CODES[stable_index(seed, SHUFFLED_COUNTRY_CODES.len())]
}
fn flag_url_from_code(code: &str) -> String {
    let normalized = if code.len() == 2 {
        code.to_ascii_uppercase()
    } else {
        "UN".into()
    };
    let mut chars = normalized.chars();
    let a = chars.next().unwrap_or('U') as u32 + 127397;
    let b = chars.next().unwrap_or('N') as u32 + 127397;
    format!("https://static.toss.im/2d-emojis/png/4x/u{a:X}_u{b:X}.png")
}
fn country_names() -> &'static [(&'static str, &'static str)] {
    &[
        ("AD", "Andorra"),
        ("AE", "United Arab Emirates"),
        ("AF", "Afghanistan"),
        ("AG", "Antigua and Barbuda"),
        ("AI", "Anguilla"),
        ("AL", "Albania"),
        ("AM", "Armenia"),
        ("AO", "Angola"),
        ("AR", "Argentina"),
        ("AT", "Austria"),
        ("AU", "Australia"),
        ("AW", "Aruba"),
        ("AZ", "Azerbaijan"),
        ("BA", "Bosnia and Herzegovina"),
        ("BB", "Barbados"),
        ("BD", "Bangladesh"),
        ("BE", "Belgium"),
        ("BF", "Burkina Faso"),
        ("BG", "Bulgaria"),
        ("BH", "Bahrain"),
        ("BJ", "Benin"),
        ("BM", "Bermuda"),
        ("BN", "Brunei"),
        ("BO", "Bolivia"),
        ("BR", "Brazil"),
        ("BS", "Bahamas"),
        ("BT", "Bhutan"),
        ("BW", "Botswana"),
        ("BY", "Belarus"),
        ("BZ", "Belize"),
        ("CA", "Canada"),
        ("CD", "Democratic Republic of the Congo"),
        ("CF", "Central African Republic"),
        ("CG", "Republic of the Congo"),
        ("CH", "Switzerland"),
        ("CI", "Cote d'Ivoire"),
        ("CL", "Chile"),
        ("CM", "Cameroon"),
        ("CN", "China"),
        ("CO", "Colombia"),
        ("CR", "Costa Rica"),
        ("CU", "Cuba"),
        ("CV", "Cape Verde"),
        ("CW", "Curacao"),
        ("CY", "Cyprus"),
        ("CZ", "Czech Republic"),
        ("DE", "Germany"),
        ("DJ", "Djibouti"),
        ("DK", "Denmark"),
        ("DM", "Dominica"),
        ("DO", "Dominican Republic"),
        ("DZ", "Algeria"),
        ("EC", "Ecuador"),
        ("EE", "Estonia"),
        ("EG", "Egypt"),
        ("ES", "Spain"),
        ("ET", "Ethiopia"),
        ("FI", "Finland"),
        ("FJ", "Fiji"),
        ("FK", "Falkland Islands"),
        ("FO", "Faroe Islands"),
        ("FR", "France"),
        ("GA", "Gabon"),
        ("GB", "United Kingdom"),
        ("GD", "Grenada"),
        ("GE", "Georgia"),
        ("GF", "French Guiana"),
        ("GG", "Guernsey"),
        ("GH", "Ghana"),
        ("GI", "Gibraltar"),
        ("GL", "Greenland"),
        ("GM", "Gambia"),
        ("GN", "Guinea"),
        ("GP", "Guadeloupe"),
        ("GQ", "Equatorial Guinea"),
        ("GR", "Greece"),
        ("GT", "Guatemala"),
        ("GU", "Guam"),
        ("GW", "Guinea-Bissau"),
        ("GY", "Guyana"),
        ("HK", "Hong Kong"),
        ("HN", "Honduras"),
        ("HR", "Croatia"),
        ("HT", "Haiti"),
        ("HU", "Hungary"),
        ("ID", "Indonesia"),
        ("IE", "Ireland"),
        ("IL", "Israel"),
        ("IM", "Isle of Man"),
        ("IN", "India"),
        ("IQ", "Iraq"),
        ("IR", "Iran"),
        ("IS", "Iceland"),
        ("IT", "Italy"),
        ("JE", "Jersey"),
        ("JM", "Jamaica"),
        ("JO", "Jordan"),
        ("JP", "Japan"),
        ("KE", "Kenya"),
        ("KG", "Kyrgyzstan"),
        ("KH", "Cambodia"),
        ("KN", "Saint Kitts and Nevis"),
        ("KP", "North Korea"),
        ("KR", "South Korea"),
        ("KW", "Kuwait"),
        ("KY", "Cayman Islands"),
        ("KZ", "Kazakhstan"),
        ("LA", "Laos"),
        ("LB", "Lebanon"),
        ("LC", "Saint Lucia"),
        ("LI", "Liechtenstein"),
        ("LK", "Sri Lanka"),
        ("LR", "Liberia"),
        ("LS", "Lesotho"),
        ("LT", "Lithuania"),
        ("LU", "Luxembourg"),
        ("LV", "Latvia"),
        ("LY", "Libya"),
        ("MA", "Morocco"),
        ("MC", "Monaco"),
        ("MD", "Moldova"),
        ("ME", "Montenegro"),
        ("MF", "Saint Martin"),
        ("MG", "Madagascar"),
        ("MK", "North Macedonia"),
        ("ML", "Mali"),
        ("MM", "Myanmar"),
        ("MN", "Mongolia"),
        ("MO", "Macao"),
        ("MQ", "Martinique"),
        ("MR", "Mauritania"),
        ("MS", "Montserrat"),
        ("MT", "Malta"),
        ("MU", "Mauritius"),
        ("MV", "Maldives"),
        ("MW", "Malawi"),
        ("MX", "Mexico"),
        ("MY", "Malaysia"),
        ("MZ", "Mozambique"),
        ("NA", "Namibia"),
        ("NC", "New Caledonia"),
        ("NE", "Niger"),
        ("NG", "Nigeria"),
        ("NI", "Nicaragua"),
        ("NL", "Netherlands"),
        ("NO", "Norway"),
        ("NP", "Nepal"),
        ("NZ", "New Zealand"),
        ("OM", "Oman"),
        ("PA", "Panama"),
        ("PE", "Peru"),
        ("PG", "Papua New Guinea"),
        ("PH", "Philippines"),
        ("PK", "Pakistan"),
        ("PL", "Poland"),
        ("PM", "Saint Pierre and Miquelon"),
        ("PR", "Puerto Rico"),
        ("PS", "Palestine"),
        ("PT", "Portugal"),
        ("PY", "Paraguay"),
        ("QA", "Qatar"),
        ("RO", "Romania"),
        ("RS", "Serbia"),
        ("RU", "Russia"),
        ("RW", "Rwanda"),
        ("SA", "Saudi Arabia"),
        ("SC", "Seychelles"),
        ("SD", "Sudan"),
        ("SE", "Sweden"),
        ("SG", "Singapore"),
        ("SI", "Slovenia"),
        ("SK", "Slovakia"),
        ("SL", "Sierra Leone"),
        ("SM", "San Marino"),
        ("SN", "Senegal"),
        ("SO", "Somalia"),
        ("SR", "Suriname"),
        ("SV", "El Salvador"),
        ("SX", "Sint Maarten"),
        ("SY", "Syria"),
        ("SZ", "Eswatini"),
        ("TC", "Turks and Caicos Islands"),
        ("TD", "Chad"),
        ("TG", "Togo"),
        ("TH", "Thailand"),
        ("TJ", "Tajikistan"),
        ("TL", "Timor-Leste"),
        ("TM", "Turkmenistan"),
        ("TN", "Tunisia"),
        ("TO", "Tonga"),
        ("TR", "Turkey"),
        ("TT", "Trinidad and Tobago"),
        ("TW", "Taiwan"),
        ("TZ", "Tanzania"),
        ("UA", "Ukraine"),
        ("UG", "Uganda"),
        ("UN", "Unknown"),
        ("US", "United States"),
        ("UY", "Uruguay"),
        ("UZ", "Uzbekistan"),
        ("VA", "Vatican City"),
        ("VC", "Saint Vincent and the Grenadines"),
        ("VE", "Venezuela"),
        ("VG", "British Virgin Islands"),
        ("VI", "United States Virgin Islands"),
        ("VN", "Vietnam"),
        ("YE", "Yemen"),
        ("ZA", "South Africa"),
        ("ZM", "Zambia"),
        ("ZW", "Zimbabwe"),
    ]
}
#[must_use]
pub fn format_size_bytes(bytes: u64, unit: SizeUnit) -> String {
    match unit {
        SizeUnit::B => format!("{bytes} B"),
        SizeUnit::Kb => format!("{} kB", commas(format!("{:.2}", bytes as f64 / 1000.0))),
        SizeUnit::Kib => format!("{} kiB", commas(format!("{:.2}", bytes as f64 / 1024.0))),
        SizeUnit::AdaptiveBi => adaptive_size(bytes, true),
        SizeUnit::AdaptiveSi => adaptive_size(bytes, false),
    }
}
fn adaptive_size(bytes: u64, bi: bool) -> String {
    let base = 1024_u64;
    if bytes < 1024 {
        return format!("{} B", commas(bytes.to_string()));
    }
    let (div, unit) = if bytes < base.pow(2) {
        (base as f64, if bi { "kiB" } else { "KB" })
    } else if bytes < base.pow(3) {
        ((base.pow(2)) as f64, if bi { "MiB" } else { "MB" })
    } else {
        ((base.pow(3)) as f64, if bi { "GiB" } else { "GB" })
    };
    format!("{} {unit}", commas(format!("{:.2}", bytes as f64 / div)))
}
fn commas(value: String) -> String {
    let (whole, fraction) = value.split_once('.').unwrap_or((&value, ""));
    let mut result = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    if !fraction.is_empty() {
        let _ = write!(result, ".{fraction}");
    }
    result
}
