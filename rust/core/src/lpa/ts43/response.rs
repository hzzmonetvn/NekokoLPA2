//! TS.43 ODSA / service-entitlement response decoder.
//!
//! Ported from `response.go` / `response.dart`, supporting both JSON and
//! WAP-provisioning XML (`characteristic`/`parm`) formats.

use std::collections::HashMap;

use super::enums::{AppEligibility, Operation, OperationResult, ServiceStatus, SubscriptionResult};
use super::types::{DownloadConfig, Message, Websheet};
use crate::base64_decode;

/// Parsed ODSA / service-entitlement response.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OdsaResponse {
    pub operation_result: i32,
    pub primary_app_eligibility: i32,
    pub subscription_result: i32,
    pub service_status: i32,
    pub polling_interval: i32,
    pub polling_interval_set: bool,
    pub polling_interval_unit: i32,
    pub temporary_token: String,
    pub temporary_token_expiry: String,
    pub operation_targets: Vec<Operation>,
    pub register_notif_status: i32,
    pub general_error_text: String,
    pub authentication_token: String,
    pub auth_token_validity: i32,
    pub download: DownloadConfig,
    pub message: Message,
    pub websheet: Websheet,
    pub raw: Vec<u8>,
}

#[derive(Default)]
struct FieldBuckets {
    token: HashMap<String, String>,
    app: HashMap<String, String>,
    primary: HashMap<String, String>,
    download: HashMap<String, String>,
    msg: HashMap<String, String>,
}

impl OdsaResponse {
    #[must_use]
    pub fn decode(data: &[u8]) -> Self {
        let mut resp = Self {
            operation_result: OperationResult::UNKNOWN,
            primary_app_eligibility: AppEligibility::UNKNOWN,
            subscription_result: SubscriptionResult::UNKNOWN,
            service_status: ServiceStatus::UNKNOWN,
            raw: data.to_vec(),
            ..Default::default()
        };

        let text = String::from_utf8_lossy(data);
        let trimmed = text.trim_start();
        let buckets = if looks_like_json(trimmed) {
            json_to_field_map(&text)
        } else {
            xml_to_field_map(&text)
        };
        resp.apply_buckets(&buckets);
        resp
    }

    fn apply_buckets(&mut self, f: &FieldBuckets) {
        self.operation_result =
            OperationResult::parse(f.app.get("OperationResult").map(|s| s.as_str()));
        self.primary_app_eligibility =
            AppEligibility::parse(f.app.get("PrimaryAppEligibility").map(|s| s.as_str()));
        self.subscription_result =
            SubscriptionResult::parse(f.app.get("SubscriptionResult").map(|s| s.as_str()));
        self.service_status =
            ServiceStatus::parse(f.primary.get("ServiceStatus").map(|s| s.as_str()));

        let mut pi = f.primary.get("PollingInterval");
        let mut pi_unit = f.primary.get("PollingIntervalUnit");
        if pi.is_none() || pi.is_some_and(|s| s.trim().is_empty()) {
            pi = f.app.get("PollingInterval");
            pi_unit = f.app.get("PollingIntervalUnit");
        }
        self.polling_interval_set = pi.is_some_and(|s| !s.trim().is_empty());
        self.polling_interval = atoi(pi.map(|s| s.as_str()));
        self.polling_interval_unit = atoi(pi_unit.map(|s| s.as_str()));

        self.temporary_token = f.app.get("TemporaryToken").cloned().unwrap_or_default();
        self.temporary_token_expiry = f
            .app
            .get("TemporaryTokenExpiry")
            .cloned()
            .unwrap_or_default();
        self.operation_targets = split_operations(
            f.app
                .get("OperationTargets")
                .map(|s| s.as_str())
                .unwrap_or(""),
        );
        self.register_notif_status = atoi(f.app.get("RegisterNotifStatus").map(|s| s.as_str()));
        self.general_error_text = f.app.get("GeneralErrorText").cloned().unwrap_or_default();
        self.authentication_token = f.token.get("token").cloned().unwrap_or_default();
        self.auth_token_validity = atoi(f.token.get("Validity").map(|s| s.as_str()));

        self.message = Message {
            title: f.msg.get("Title").cloned().unwrap_or_default(),
            text: f.msg.get("Message").cloned().unwrap_or_default(),
            accept_button: is_one(f.msg.get("Accept_btn").map(|s| s.as_str())),
            accept_button_label: f.msg.get("Accept_btn_label").cloned().unwrap_or_default(),
            reject_button: is_one(f.msg.get("Reject_btn").map(|s| s.as_str())),
            reject_button_label: f.msg.get("Reject_btn_label").cloned().unwrap_or_default(),
            accept_free_text: is_one(f.msg.get("Accept_freetext").map(|s| s.as_str())),
            accept_free_text_hint: f
                .msg
                .get("Accept_freetext_hint")
                .cloned()
                .unwrap_or_default(),
            accept_free_text_validation: encoded_prefixed_field(
                f.msg
                    .get("Accept_freetext_validation")
                    .map(|s| s.as_str())
                    .unwrap_or(""),
            ),
            accept_free_text_validation_failed_error_text: f
                .msg
                .get("Accept_freetext_validation_failed_error_text")
                .cloned()
                .unwrap_or_default(),
        };

        if self.subscription_result == SubscriptionResult::CONTINUE_TO_WEBSHEET {
            self.websheet = Websheet {
                url: f
                    .app
                    .get("SubscriptionServiceURL")
                    .cloned()
                    .unwrap_or_default(),
                user_data: encoded_prefixed_field(
                    f.app
                        .get("SubscriptionServiceUserData")
                        .map(|s| s.as_str())
                        .unwrap_or(""),
                ),
                contents_type: f
                    .app
                    .get("SubscriptionServiceContentsType")
                    .cloned()
                    .unwrap_or_default(),
            };
        }

        self.download = DownloadConfig {
            activation_code: activation_code_field(
                f.download
                    .get("ProfileActivationCode")
                    .map(|s| s.as_str())
                    .unwrap_or(""),
            ),
            smdp_fqdn: String::new(),
            matching_id: f.download.get("MatchingID").cloned().unwrap_or_default(),
            profile_iccid: f.download.get("ProfileIccid").cloned().unwrap_or_default(),
            imei: String::new(),
        };

        let mut smdp = f
            .download
            .get("ProfileSmdpAddress")
            .cloned()
            .unwrap_or_default();
        if smdp.is_empty() {
            smdp = f
                .download
                .get("SubscriptionServiceURL")
                .cloned()
                .unwrap_or_default();
        }
        self.download.smdp_fqdn = smdp_fqdn(&smdp);
    }
}

fn looks_like_json(s: &str) -> bool {
    let first = s.chars().next();
    matches!(first, Some('{') | Some('['))
}

fn json_to_field_map(text: &str) -> FieldBuckets {
    let mut f = FieldBuckets::default();
    let Ok(val) = serde_json::from_str::<serde_json::Value>(text) else {
        return f;
    };
    let Some(root) = val.as_object() else {
        return f;
    };

    let app = root
        .get("APPLICATION")
        .and_then(|v| v.as_object())
        .unwrap_or(root);

    if let Some(token) = root.get("TOKEN").and_then(|v| v.as_object()) {
        copy_json_fields(token, &mut f.token, &["token", "Validity"]);
    }

    copy_json_fields(
        app,
        &mut f.app,
        &[
            "OperationResult",
            "PrimaryAppEligibility",
            "SubscriptionResult",
            "PollingInterval",
            "PollingIntervalUnit",
            "TemporaryToken",
            "TemporaryTokenExpiry",
            "OperationTargets",
            "RegisterNotifStatus",
            "GeneralErrorText",
            "SubscriptionServiceURL",
            "SubscriptionServiceUserData",
            "SubscriptionServiceContentsType",
        ],
    );

    let primary = app.get("PrimaryConfiguration").and_then(|v| v.as_object());
    let mut download = app.get("DownloadInfo").and_then(|v| v.as_object());
    let mut msg = app.get("MSG").and_then(|v| v.as_object());

    if let Some(prim) = primary {
        copy_json_fields(
            prim,
            &mut f.primary,
            &["ServiceStatus", "PollingInterval", "PollingIntervalUnit"],
        );
        if download.is_none() {
            download = prim.get("DownloadInfo").and_then(|v| v.as_object());
        }
        if msg.is_none() {
            msg = prim.get("MSG").and_then(|v| v.as_object());
        }
    }

    if let Some(dl) = download {
        copy_json_fields(
            dl,
            &mut f.download,
            &[
                "ProfileActivationCode",
                "ProfileSmdpAddress",
                "ProfileIccid",
                "SubscriptionServiceURL",
                "MatchingID",
            ],
        );
    }

    if let Some(m) = msg {
        copy_json_fields(
            m,
            &mut f.msg,
            &[
                "Title",
                "Message",
                "Accept_btn",
                "Accept_btn_label",
                "Reject_btn",
                "Reject_btn_label",
                "Accept_freetext",
                "Accept_freetext_hint",
                "Accept_freetext_validation",
                "Accept_freetext_validation_failed_error_text",
            ],
        );
    }

    f
}

fn copy_json_fields(
    src: &serde_json::Map<String, serde_json::Value>,
    dst: &mut HashMap<String, String>,
    keys: &[&str],
) {
    for &k in keys {
        if let Some(v) = src.get(k) {
            match v {
                serde_json::Value::String(s) => {
                    dst.insert(k.to_string(), s.clone());
                }
                serde_json::Value::Number(n) => {
                    dst.insert(k.to_string(), n.to_string());
                }
                serde_json::Value::Bool(b) => {
                    dst.insert(
                        k.to_string(),
                        if *b { "1".to_string() } else { "0".to_string() },
                    );
                }
                _ => {}
            }
        }
    }
}

fn xml_to_field_map(text: &str) -> FieldBuckets {
    let mut f = FieldBuckets::default();
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);

    let mut char_stack: Vec<String> = Vec::new();

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = e.name().as_ref().to_lowercase();
                if name == "characteristic" {
                    let mut char_type = String::new();
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref().eq_ignore_ascii_case("type") {
                            char_type = attr.value.as_ref().to_string();
                            break;
                        }
                    }
                    char_stack.push(char_type);
                } else if name == "parm" || name == "param" {
                    let mut p_name = String::new();
                    let mut p_val = String::new();
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref().eq_ignore_ascii_case("name") {
                            p_name = attr.value.as_ref().to_string();
                        } else if attr.key.as_ref().eq_ignore_ascii_case("value") {
                            p_val = attr.value.as_ref().to_string();
                        }
                    }
                    if !p_name.is_empty() {
                        insert_param_into_bucket(&mut f, &char_stack, p_name, p_val);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name().as_ref().to_lowercase();
                if name == "characteristic" {
                    char_stack.pop();
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    f
}

fn insert_param_into_bucket(f: &mut FieldBuckets, stack: &[String], key: String, val: String) {
    if let Some(top) = stack.last() {
        match top.as_str() {
            "TOKEN" => {
                f.token.insert(key, val);
            }
            "APPLICATION" => {
                f.app.insert(key, val);
            }
            "PrimaryConfiguration" => {
                f.primary.insert(key, val);
            }
            "DownloadInfo" => {
                f.download.insert(key, val);
            }
            "MSG" => {
                f.msg.insert(key, val);
            }
            _ => {
                if stack.iter().any(|s| s == "DownloadInfo") {
                    f.download.insert(key, val);
                } else if stack.iter().any(|s| s == "MSG") {
                    f.msg.insert(key, val);
                } else if stack.iter().any(|s| s == "PrimaryConfiguration") {
                    f.primary.insert(key, val);
                } else if stack.iter().any(|s| s == "APPLICATION") {
                    f.app.insert(key, val);
                }
            }
        }
    }
}

fn atoi(s: Option<&str>) -> i32 {
    s.map(|v| v.trim().parse::<i32>().unwrap_or(0)).unwrap_or(0)
}

fn is_one(s: Option<&str>) -> bool {
    s.map(|v| v.trim() == "1").unwrap_or(false)
}

fn split_operations(s: &str) -> Vec<Operation> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(',')
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.parse().unwrap_or_default())
        .collect()
}

pub fn encoded_prefixed_field(value: &str) -> String {
    let mut val = value.trim();
    if let Some(stripped) = val.strip_prefix("encodedValue=") {
        val = stripped;
    }
    if val.is_empty() {
        return String::new();
    }
    if let Ok(bytes) = base64_decode(val) {
        if let Ok(s) = String::from_utf8(bytes) {
            return s;
        }
    }
    val.to_string()
}

pub fn activation_code_field(value: &str) -> String {
    let decoded = encoded_prefixed_field(value);
    if decoded.is_empty() {
        return String::new();
    }
    if decoded.starts_with("LPA:") {
        decoded
    } else {
        format!("LPA:{decoded}")
    }
}

pub fn smdp_fqdn(value: &str) -> String {
    let decoded = encoded_prefixed_field(value);
    let values: Vec<&str> = decoded
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if values.is_empty() {
        return String::new();
    }
    let fqdn = values[0];
    if let Some(pos) = fqdn.find("://") {
        let after = &fqdn[pos + 3..];
        let host = after.split('/').next().unwrap_or(after);
        return host.to_string();
    }
    fqdn.to_string()
}
