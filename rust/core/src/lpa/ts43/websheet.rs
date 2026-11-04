//! TS.43 websheet callback parser.
//!
//! Ported from `websheet_result.go` / `websheet.dart`.

use std::collections::HashMap;

use super::enums::Operation;
use super::errors::WebsheetException;
use super::types::{first_non_empty, WebsheetCallbackEvent, WebsheetResult};

/// Parses a websheet callback line into a `WebsheetResult`.
pub fn parse_websheet_result(text: &str) -> Result<WebsheetResult, WebsheetException> {
    let mut line = text.trim();
    if line.is_empty() {
        return Err(WebsheetException(
            "websheet callback is required".to_string(),
        ));
    }
    let vowifi_callback = line.starts_with("TS43_VOWIFI_CALLBACK");
    line = trim_callback_log_prefix(line, "TS43_ODSA_CALLBACK");
    line = trim_callback_log_prefix(line, "TS43_VOWIFI_CALLBACK");

    if line.starts_with('{') {
        return from_json(line, vowifi_callback);
    }
    if line.contains("ODSAServiceFlow.") {
        return from_js(line);
    }
    if line.contains("VoWiFiWebServiceFlow.") {
        return from_vowifi_js(line);
    }
    if line.contains("/_callback") {
        return from_url(line);
    }
    Err(WebsheetException(format!(
        "unknown websheet callback \"{line}\""
    )))
}

fn trim_callback_log_prefix<'a>(line: &'a str, prefix: &str) -> &'a str {
    let trimmed = line.trim();
    if let Some(stripped) = trimmed.strip_prefix(prefix) {
        stripped.trim()
    } else {
        trimmed
    }
}

fn from_json(line: &str, vowifi_callback: bool) -> Result<WebsheetResult, WebsheetException> {
    let payload: serde_json::Value = serde_json::from_str(line)
        .map_err(|e| WebsheetException(format!("invalid JSON callback: {e}")))?;
    let obj = payload
        .as_object()
        .ok_or_else(|| WebsheetException("JSON callback must be an object".to_string()))?;

    let s = |k: &str| -> String {
        obj.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    if vowifi_callback || s("source").eq_ignore_ascii_case("vowifi") {
        return from_vowifi_name(&s("event"));
    }

    let default_smdp = first_non_empty(&[&s("defaultSmdpAddress"), &s("smdpFqdn")]);
    let mut query = HashMap::new();
    query.insert("activationCode".to_string(), s("activationCode"));
    query.insert("defaultSmdpAddress".to_string(), default_smdp);
    query.insert("iccid".to_string(), s("iccid"));
    query.insert("imei".to_string(), s("imei"));
    query.insert("nextAction".to_string(), s("nextAction"));

    from_name(&s("event"), &[], &query)
}

fn from_js(line: &str) -> Result<WebsheetResult, WebsheetException> {
    let idx = line
        .find("ODSAServiceFlow.")
        .ok_or_else(|| WebsheetException("ODSAServiceFlow callback is required".to_string()))?;
    let after = &line[idx + "ODSAServiceFlow.".len()..];
    let paren = after
        .find('(')
        .ok_or_else(|| WebsheetException("callback argument list is required".to_string()))?;
    let name = &after[..paren];
    let rest = &after[paren + 1..];
    let close = rest
        .rfind(')')
        .ok_or_else(|| WebsheetException("callback argument list is not closed".to_string()))?;
    let args = split_js_args(&rest[..close]);
    let query = HashMap::new();
    from_name(name, &args, &query)
}

fn from_vowifi_js(line: &str) -> Result<WebsheetResult, WebsheetException> {
    let idx = line.find("VoWiFiWebServiceFlow.").ok_or_else(|| {
        WebsheetException("VoWiFiWebServiceFlow callback is required".to_string())
    })?;
    let after = &line[idx + "VoWiFiWebServiceFlow.".len()..];
    let paren = after
        .find('(')
        .ok_or_else(|| WebsheetException("callback argument list is required".to_string()))?;
    let name = &after[..paren];
    from_vowifi_name(name)
}

fn from_vowifi_name(name: &str) -> Result<WebsheetResult, WebsheetException> {
    match normalize_callback_name(name).as_str() {
        "entitlementchanged" => Ok(WebsheetResult {
            event: WebsheetCallbackEvent::VoWiFiEntitlementChanged,
            ..Default::default()
        }),
        "dismissflow" => Ok(WebsheetResult {
            event: WebsheetCallbackEvent::VoWiFiDismissFlow,
            ..Default::default()
        }),
        _ => Err(WebsheetException(format!(
            "unknown VoWiFi websheet callback \"{name}\""
        ))),
    }
}

fn from_url(raw: &str) -> Result<WebsheetResult, WebsheetException> {
    let query_idx = raw.find('?');
    let path_part = if let Some(idx) = query_idx {
        &raw[..idx]
    } else {
        raw
    };
    let query_str = if let Some(idx) = query_idx {
        &raw[idx + 1..]
    } else {
        ""
    };

    let mut query = HashMap::new();
    for pair in query_str.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let val = parts.next().unwrap_or("").trim();
        if !key.is_empty() {
            query.insert(key.to_string(), val.to_string());
        }
    }

    let mut name = first_value(&query, &["event", "callback", "action", "method"]);
    if name.is_empty() {
        let parts: Vec<&str> = path_part.split('/').filter(|p| !p.is_empty()).collect();
        if let Some(last) = parts.last() {
            name = last.to_string();
        }
    }
    let normalized = normalize_callback_name(&name);
    if normalized == "callback" || normalized == "esim" {
        name = callback_name_from_query(&query);
    }
    from_name(&name, &[], &query)
}

fn from_name(
    name: &str,
    args: &[String],
    query: &HashMap<String, String>,
) -> Result<WebsheetResult, WebsheetException> {
    match normalize_callback_name(name).as_str() {
        "profilereadywithactivationcode" | "activation" => {
            let ac = arg_or_query(args, query, 0, &["activationCode", "activation_code"]);
            if ac.is_empty() {
                return Err(WebsheetException("activation code is required".to_string()));
            }
            Ok(WebsheetResult {
                event: WebsheetCallbackEvent::ProfileReadyWithActivationCode,
                activation_code: ac,
                iccid: arg_or_query(args, query, 1, &["iccid", "ICCID"]),
                imei: arg_or_query(args, query, 2, &["imei", "IMEI"]),
                ..Default::default()
            })
        }
        "profilereadywithdefaultsmdp" | "defaultsmdp" => {
            let smdp = arg_or_query(args, query, 0, &["defaultSmdpAddress", "smdp", "smdpFqdn"]);
            let iccid = arg_or_query(args, query, 1, &["iccid", "ICCID"]);
            if smdp.is_empty() || iccid.is_empty() {
                return Err(WebsheetException(
                    "default SM-DP+ and ICCID are required".to_string(),
                ));
            }
            Ok(WebsheetResult {
                event: WebsheetCallbackEvent::ProfileReadyWithDefaultSmdp,
                smdp_fqdn: smdp,
                iccid,
                imei: arg_or_query(args, query, 2, &["imei", "IMEI"]),
                ..Default::default()
            })
        }
        "selectioncompleted" | "select" => {
            let iccid = arg_or_query(args, query, 0, &["iccid", "ICCID"]);
            let imei = arg_or_query(args, query, 1, &["imei", "IMEI"]);
            if iccid.is_empty() && imei.is_empty() {
                return Err(WebsheetException(
                    "selected ICCID or IMEI is required".to_string(),
                ));
            }
            Ok(WebsheetResult {
                event: WebsheetCallbackEvent::SelectionCompleted,
                iccid,
                imei,
                ..Default::default()
            })
        }
        "finishflow" | "done" => Ok(WebsheetResult {
            event: WebsheetCallbackEvent::FinishFlow,
            next_action: operation_from_callback_text(&arg_or_query(
                args,
                query,
                0,
                &["next_action", "nextAction"],
            )),
            ..Default::default()
        }),
        "dismissflow" | "dismiss" => Ok(WebsheetResult {
            event: WebsheetCallbackEvent::DismissFlow,
            ..Default::default()
        }),
        "deletetoken" => Ok(WebsheetResult {
            event: WebsheetCallbackEvent::DeleteToken,
            ..Default::default()
        }),
        "checkprofileservicestatus" | "status" => Ok(WebsheetResult {
            event: WebsheetCallbackEvent::CheckProfileServiceStatus,
            ..Default::default()
        }),
        "deleteprofileinuse" | "deleteprofile" => Ok(WebsheetResult {
            event: WebsheetCallbackEvent::DeleteProfileInUse,
            iccid: arg_or_query(args, query, 0, &["iccid", "ICCID"]),
            ..Default::default()
        }),
        "entitlementchanged" => Ok(WebsheetResult {
            event: WebsheetCallbackEvent::VoWiFiEntitlementChanged,
            ..Default::default()
        }),
        _ => Err(WebsheetException(format!(
            "unknown websheet callback \"{name}\""
        ))),
    }
}

fn callback_name_from_query(query: &HashMap<String, String>) -> String {
    if !first_value(query, &["activationCode", "activation_code"]).is_empty() {
        return "profileReadyWithActivationCode".to_string();
    }
    if !first_value(query, &["defaultSmdpAddress", "smdp", "smdpFqdn"]).is_empty() {
        return "profileReadyWithDefaultSmdp".to_string();
    }
    if !first_value(query, &["next_action", "nextAction"]).is_empty() {
        return "finishFlow".to_string();
    }
    String::new()
}

fn operation_from_callback_text(s: &str) -> Operation {
    match normalize_callback_name(s).as_str() {
        "" => Operation::None,
        "acquireconfiguration" | "acquireconf" | "configuration" => Operation::AcquireConfiguration,
        "managesubscription" | "manage" => Operation::ManageSubscription,
        "checkeligibility" | "check" => Operation::CheckEligibility,
        "acquiretemporarytoken" | "temptoken" => Operation::AcquireTemporaryToken,
        _ => s.parse().unwrap_or_default(),
    }
}

fn split_js_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in text.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                current.push(ch);
            }
        } else if ch == '\'' || ch == '"' {
            quote = Some(ch);
        } else if ch == ',' {
            args.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    let last = current.trim();
    if !last.is_empty() || !text.is_empty() {
        args.push(last.to_string());
    }
    args
}

fn normalize_callback_name(name: &str) -> String {
    let mut s = name.trim();
    if let Some(stripped) = s.strip_suffix("()") {
        s = stripped;
    }
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn arg_or_query(
    args: &[String],
    query: &HashMap<String, String>,
    index: usize,
    keys: &[&str],
) -> String {
    if index < args.len() && !args[index].is_empty() {
        return args[index].clone();
    }
    first_value(query, keys)
}

fn first_value(values: &HashMap<String, String>, keys: &[&str]) -> String {
    for &key in keys {
        if let Some(v) = values.get(key) {
            if !v.is_empty() {
                return v.clone();
            }
        }
    }
    String::new()
}
