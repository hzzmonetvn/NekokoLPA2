//! Mobile Country Code (MCC), PLMN and regional indicator mapping.
//!
//! Maps ITU-T E.212 Mobile Country Codes to ISO 3166-1 alpha-2 country codes
//! and generates Unicode regional indicator flag emojis for eSIM profile visualization.

/// Maps an ITU-T E.212 MCC (with optional MNC/ICCID disambiguation) to an ISO 3166-1 alpha-2 code.
/// Returns `"UN"` for unrecognized, unknown, or international codes.
#[must_use]
pub fn mcc_to_country_code(mcc: &str, mnc: Option<&str>, _iccid: Option<&str>) -> &'static str {
    match mcc {
        // --- Europe (200-299) ---
        "202" => "GR",
        "204" => "NL",
        "206" => "BE",
        "208" => "FR",
        "212" => "MC",
        "213" => "AD",
        "214" => "ES",
        "216" => "HU",
        "218" => "BA",
        "219" => "HR",
        "220" => "RS",
        "222" => "IT",
        "225" => "VA",
        "226" => "RO",
        "228" => "CH",
        "230" => "CZ",
        "231" => "SK",
        "232" => "AT",
        "234" => match mnc {
            Some("03" | "28" | "50") => "JE",
            Some("18" | "36" | "58" | "73") => "IM",
            _ => "GB",
        },
        "235" => "GB",
        "238" => "DK",
        "240" => "SE",
        "242" => "NO",
        "244" => "FI",
        "246" => "LT",
        "247" => "LV",
        "248" => "EE",
        "250" => "RU",
        "255" => "UA",
        "257" => "BY",
        "259" => "MD",
        "260" => "PL",
        "262" => "DE",
        "266" => "GI",
        "268" => "PT",
        "270" => "LU",
        "272" => "IE",
        "274" => "IS",
        "276" => "AL",
        "278" => "MT",
        "280" => "CY",
        "282" => "GE",
        "283" => "AM",
        "284" => "BG",
        "286" => "TR",
        "288" => "FO",
        "290" => "GL",
        "292" => "SM",
        "293" => "SI",
        "294" => "MK",
        "295" => "LI",
        "297" => "ME",

        // --- North America & Caribbean (300-399) ---
        "302" => "CA",
        "310" | "311" | "312" | "313" | "314" | "315" | "316" => "US",
        "330" => "PR",
        "334" => "MX",
        "338" => "JM",
        "340" => match mnc {
            Some("01" | "02" | "03") => "GP",
            Some("08" | "09") => "BL",
            Some("11" | "12") => "MF",
            _ => "MQ",
        },
        "342" => "BB",
        "344" => "AG",
        "346" => "KY",
        "348" => "VG",
        "350" => "BM",
        "352" => "GD",
        "354" => "MS",
        "356" => "KN",
        "358" => "LC",
        "360" => "VC",
        "362" => "CW",
        "363" => "AW",
        "364" => "BS",
        "365" => "AI",
        "366" => "DM",
        "368" => "CU",
        "370" => "DO",
        "372" => "HT",
        "374" => "TT",
        "376" => "TC",

        // --- Asia (400-499) ---
        "400" => "AZ",
        "401" => "KZ",
        "402" => "BT",
        "404" | "405" | "406" => "IN",
        "410" => "PK",
        "412" => "AF",
        "413" => "LK",
        "414" => "MM",
        "415" => "LB",
        "416" => "JO",
        "417" => "SY",
        "418" => "IQ",
        "419" => "KW",
        "420" => "SA",
        "421" => "YE",
        "422" => "OM",
        "424" | "430" | "431" => "AE",
        "425" => "IL",
        "426" => "BH",
        "427" => "QA",
        "428" => "MN",
        "429" => "NP",
        "432" => "IR",
        "434" => "UZ",
        "436" => "TJ",
        "437" => "KG",
        "438" => "TM",
        "440" | "441" => "JP",
        "450" => "KR",
        "452" => "VN",
        "454" => "HK",
        "455" => "MO",
        "456" => "KH",
        "457" => "LA",
        "460" | "461" => "CN",
        "466" => "TW",
        "467" => "KP",
        "470" => "BD",
        "472" => "MV",

        // --- Oceania (500-599) ---
        "502" => "MY",
        "505" => "AU",
        "510" => "ID",
        "514" => "TL",
        "515" => "PH",
        "520" => "TH",
        "525" => "SG",
        "528" => "BN",
        "530" => "NZ",
        "536" => "NR",
        "537" => "PG",
        "539" => "TO",
        "540" => "SB",
        "541" => "VU",
        "542" => "FJ",
        "543" => "WF",
        "544" => "AS",
        "545" => "KI",
        "546" => "NC",
        "547" => "PF",
        "548" => "CK",
        "549" => "WS",
        "550" => "FM",
        "551" => "MH",
        "552" => "PW",
        "553" => "TV",
        "555" => "NU",

        // --- Africa (600-699) ---
        "602" => "EG",
        "603" => "DZ",
        "604" => "MA",
        "605" => "TN",
        "606" => "LY",
        "607" => "GM",
        "608" => "SN",
        "609" => "MR",
        "610" => "ML",
        "611" => "GN",
        "612" => "CI",
        "613" => "BF",
        "614" => "NE",
        "615" => "TG",
        "616" => "BJ",
        "617" => "MU",
        "618" => "LR",
        "619" => "SL",
        "620" => "GH",
        "621" => "NG",
        "622" => "TD",
        "623" => "CF",
        "624" => "CM",
        "625" => "CV",
        "626" => "ST",
        "627" => "GQ",
        "628" => "GA",
        "629" => "CG",
        "630" => "CD",
        "631" => "AO",
        "632" => "GW",
        "633" => "SC",
        "634" => "SD",
        "635" => "RW",
        "636" => "ET",
        "637" => "SO",
        "638" => "DJ",
        "639" => "KE",
        "640" => "TZ",
        "641" => "UG",
        "642" => "BI",
        "643" => "MZ",
        "645" => "ZM",
        "646" => "MG",
        "647" => match mnc {
            Some("02") => "YT",
            _ => "RE",
        },
        "648" => "ZW",
        "649" => "NA",
        "650" => "MW",
        "651" => "LS",
        "652" => "BW",
        "653" => "SZ",
        "654" => "KM",
        "655" => "ZA",
        "657" => "ER",
        "658" => "SH",
        "659" => "SS",

        // --- South & Central America (700-799) ---
        "702" => "BZ",
        "704" => "GT",
        "706" => "SV",
        "708" => "HN",
        "710" => "NI",
        "712" => "CR",
        "714" => "PA",
        "716" => "PE",
        "722" => "AR",
        "724" => "BR",
        "730" => "CL",
        "732" => "CO",
        "734" => "VE",
        "736" => "BO",
        "738" => "GY",
        "740" => "EC",
        "744" => "PY",
        "746" => "SR",
        "748" => "UY",
        "750" => "FK",

        _ => "UN",
    }
}

/// Generates a Unicode regional indicator symbol flag emoji from a 2-letter ISO country code.
/// Returns `"🌐"` if the code is invalid or not 2 uppercase letters.
#[must_use]
pub fn get_flag_emoji(country_code: &str) -> String {
    let upper = country_code.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    if bytes.len() != 2 || !bytes[0].is_ascii_alphabetic() || !bytes[1].is_ascii_alphabetic() {
        return "🌐".to_string();
    }

    let c1 = char::from_u32(127397 + u32::from(bytes[0])).unwrap_or('🌐');
    let c2 = char::from_u32(127397 + u32::from(bytes[1])).unwrap_or('🌐');
    format!("{c1}{c2}")
}

/// Parses MCC and MNC from GSMA/3GPP 3-byte PLMN format.
/// Byte 0: MCC digit 2 | MCC digit 1
/// Byte 1: MNC digit 3 | MCC digit 3
/// Byte 2: MNC digit 2 | MNC digit 1
#[must_use]
pub fn parse_operator_id(data: &[u8]) -> Option<(String, String)> {
    if data.len() < 3 {
        return None;
    }

    let m1 = data[0] & 0x0F;
    let m2 = (data[0] >> 4) & 0x0F;
    let m3 = data[1] & 0x0F;
    let n3 = (data[1] >> 4) & 0x0F;
    let n1 = data[2] & 0x0F;
    let n2 = (data[2] >> 4) & 0x0F;

    let mcc = format!("{m1}{m2}{m3}");
    let mnc = if n3 == 0x0F {
        format!("{n1}{n2}")
    } else {
        format!("{n1}{n2}{n3}")
    };

    Some((mcc, mnc))
}
