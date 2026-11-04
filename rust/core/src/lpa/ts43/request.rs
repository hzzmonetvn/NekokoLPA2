//! TS.43 Service Entitlement Request builders, parameters, and ODSA operation dispatch.
//!
//! Ported from `request.go` / `google_request.go` / `samsung_request.go` / `operation.go`.

use super::constants::Ts43Constants;
use super::enums::{EntitlementChannel, MessageButton, Operation, OperationType, TransferType};
use super::sim::{imsi_eap_identity, Identity};
use super::types::{first_non_empty, Device, Entitlement, UserInputResult};
use super::Ts43Error;
use crate::base64_encode;

/// Service-entitlement request fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServiceEntitlementRequest {
    pub entitlement_version: String,
    pub request_version: String,
    pub authentication_token: String,
    pub temporary_token: String,
    pub app: String,
    pub terminal_id: String,
    pub terminal_vendor: String,
    pub terminal_model: String,
    pub terminal_software: String,
    pub app_name: String,
    pub app_version: String,
    pub provisioning_version: String,
    pub accept: String,
    pub gid1: String,
    pub client_ts43: String,
    pub omit_eap_id: bool,
    pub cookies: Vec<String>,
}

/// ODSA operation parameters.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OdsaOperation {
    pub operation: Operation,
    pub operation_type: OperationType,
    pub operation_targets: Vec<Operation>,
    pub terminal_iccid: String,
    pub terminal_eid: String,
    pub target_terminal_id: String,
    pub target_terminal_iccid: String,
    pub target_terminal_eid: String,
    pub target_terminal_model: String,
    pub target_terminal_sn: String,
    pub target_terminal_imeis: Vec<String>,
    pub target_terminal_entitlement_proto: String,
    pub companion_terminal_id: String,
    pub companion_terminal_vendor: String,
    pub companion_terminal_model: String,
    pub companion_terminal_software: String,
    pub companion_terminal_friendly_name: String,
    pub companion_terminal_service: String,
    pub companion_terminal_iccid: String,
    pub companion_terminal_eid: String,
    pub old_terminal_id: String,
    pub old_terminal_iccid: String,
    pub old_terminal_entitlement_proto: String,
    pub message_response: String,
    pub message_button: i32,
    pub message_button_set: bool,
}

/// HTTP-agnostic prepared request: parameters + headers + method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRequest {
    pub use_post: bool,
    pub params: Vec<(String, Vec<String>)>,
    pub accept: String,
    pub user_agent: String,
    pub cookies: Vec<String>,
}

impl PreparedRequest {
    /// Returns the first value associated with `key` in `params`.
    #[must_use]
    pub fn get_param(&self, key: &str) -> Option<&str> {
        for (k, vals) in &self.params {
            if k == key {
                return vals.first().map(|s| s.as_str());
            }
        }
        None
    }

    /// Form-urlencoded query string for GET requests.
    #[must_use]
    pub fn query_string(&self) -> String {
        let mut parts = Vec::new();
        for (key, values) in &self.params {
            for v in values {
                parts.push(format!("{}={}", url_encode(key), url_encode(v)));
            }
        }
        parts.join("&")
    }

    /// JSON body for POST requests.
    #[must_use]
    pub fn json_body(&self) -> String {
        let mut map = serde_json::Map::new();
        for (key, values) in &self.params {
            if key == "target_terminal_imeis" {
                if let Some(last) = values.last() {
                    map.insert(key.clone(), serde_json::Value::String(last.clone()));
                }
            } else {
                map.insert(key.clone(), serde_json::Value::String(values.join(",")));
            }
        }
        serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
    }
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                use core::fmt::Write;
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

#[derive(Default)]
struct ParamBuilder {
    params: Vec<(String, Vec<String>)>,
}

impl ParamBuilder {
    fn set(&mut self, name: &str, value: impl Into<String>) {
        let val = value.into();
        if let Some(pos) = self.params.iter().position(|(k, _)| k == name) {
            self.params[pos].1 = vec![val];
        } else {
            self.params.push((name.to_string(), vec![val]));
        }
    }

    fn set_optional(&mut self, name: &str, value: &str) {
        if !value.is_empty() {
            self.set(name, value);
        }
    }

    fn add(&mut self, name: &str, value: impl Into<String>) {
        let val = value.into();
        if let Some(pos) = self.params.iter().position(|(k, _)| k == name) {
            self.params[pos].1.push(val);
        } else {
            self.params.push((name.to_string(), vec![val]));
        }
    }

    fn remove(&mut self, name: &str) {
        self.params.retain(|(k, _)| k != name);
    }
}

pub fn build_prepared_request(
    entitlement: &Entitlement,
    identity: &Identity,
    req: &ServiceEntitlementRequest,
    op: &OdsaOperation,
    client_ts43: &str,
    realm: &str,
) -> Result<PreparedRequest, Ts43Error> {
    let mut shaped_req = req.clone();
    if entitlement.channel == EntitlementChannel::Samsung {
        shaped_req.app = Ts43Constants::APP_ODSA_PRIMARY.to_string();
    }
    if shaped_req.app.is_empty() {
        return Err(Ts43Error::MalformedPayload("app is required".to_string()));
    }

    let mut params = ParamBuilder::default();
    add_auth_identity(&mut params, identity, &shaped_req, realm)?;
    add_common(&mut params, &shaped_req);
    if op.operation != Operation::None {
        add_operation(&mut params, op);
    }
    add_samsung_operation_fields(&mut params, entitlement, identity, op, realm)?;

    let accept = request_accept(entitlement, &shaped_req);
    let ua = user_agent(
        &shaped_req,
        if shaped_req.client_ts43.is_empty() {
            client_ts43
        } else {
            &shaped_req.client_ts43
        },
    );

    Ok(PreparedRequest {
        use_post: entitlement.use_post,
        params: params.params,
        accept,
        user_agent: ua,
        cookies: shaped_req.cookies,
    })
}

fn request_accept(e: &Entitlement, req: &ServiceEntitlementRequest) -> String {
    let accept = response_accept(e, req);
    if initial_eap_request(req) {
        eap_relay_accept(&accept)
    } else {
        accept
    }
}

pub fn response_accept(e: &Entitlement, req: &ServiceEntitlementRequest) -> String {
    if !req.accept.is_empty() {
        return req.accept.clone();
    }
    if e.channel == EntitlementChannel::Samsung {
        Ts43Constants::SAMSUNG_RESPONSE_ACCEPT.to_string()
    } else {
        Ts43Constants::DEFAULT_RESPONSE_ACCEPT.to_string()
    }
}

fn initial_eap_request(req: &ServiceEntitlementRequest) -> bool {
    req.authentication_token.is_empty() && req.temporary_token.is_empty() && !req.omit_eap_id
}

pub fn eap_relay_accept(accept: &str) -> String {
    let base = if accept.is_empty() {
        Ts43Constants::DEFAULT_RESPONSE_ACCEPT
    } else {
        accept
    };
    if accept_has_media_type(base, Ts43Constants::EAP_RELAY_CONTENT_TYPE) {
        return base.to_string();
    }
    format!("{}, {base}", Ts43Constants::EAP_RELAY_CONTENT_TYPE)
}

fn accept_has_media_type(accept: &str, media_type: &str) -> bool {
    for part in accept.split(',') {
        let value = part.split(';').next().unwrap_or("").trim();
        if value.eq_ignore_ascii_case(media_type) {
            return true;
        }
    }
    false
}

fn add_auth_identity(
    params: &mut ParamBuilder,
    identity: &Identity,
    req: &ServiceEntitlementRequest,
    realm: &str,
) -> Result<(), Ts43Error> {
    if !req.authentication_token.is_empty() {
        params.set("IMSI", &identity.imsi);
        params.set("token", &req.authentication_token);
        return Ok(());
    }
    if !req.temporary_token.is_empty() {
        params.set("temporary_token", &req.temporary_token);
        return Ok(());
    }
    if req.omit_eap_id {
        return Ok(());
    }
    let eap_id = imsi_eap_identity(&identity.mcc_mnc(), &identity.imsi, realm)
        .map_err(|e| Ts43Error::MalformedPayload(e.to_string()))?;
    params.set("EAP_ID", eap_id);
    Ok(())
}

fn add_common(params: &mut ParamBuilder, req: &ServiceEntitlementRequest) {
    let version = if req.request_version.is_empty() {
        "0"
    } else {
        &req.request_version
    };
    params.set("terminal_id", &req.terminal_id);
    params.set("app", &req.app);
    params.set("terminal_vendor", truncate(&req.terminal_vendor, 4));
    params.set("terminal_model", truncate(&req.terminal_model, 10));
    params.set("terminal_sw_version", truncate(&req.terminal_software, 20));
    params.set("vers", version);
    params.set("entitlement_version", &req.entitlement_version);
    params.set_optional("GID1", &req.gid1);
    params.set_optional("app_version", &req.app_version);
    params.set_optional("app_name", &req.app_name);
    params.set_optional("provisioning_version", &req.provisioning_version);
}

fn add_operation(params: &mut ParamBuilder, op: &OdsaOperation) {
    params.set("operation", op.operation.as_str());
    if op.operation_type != OperationType::Omit {
        params.set("operation_type", op.operation_type.text());
    }
    params.set_optional(
        "operation_targets",
        &operation_targets_str(&op.operation_targets),
    );
    params.set_optional("terminal_iccid", &op.terminal_iccid);
    params.set_optional("terminal_eid", &op.terminal_eid);
    params.set_optional("target_terminal_id", &op.target_terminal_id);
    params.set_optional("target_terminal_iccid", &op.target_terminal_iccid);
    params.set_optional("target_terminal_eid", &op.target_terminal_eid);
    params.set_optional("target_terminal_model", &op.target_terminal_model);
    params.set_optional("target_terminal_sn", &op.target_terminal_sn);
    params.set_optional(
        "target_terminal_entitlement_protocol",
        &op.target_terminal_entitlement_proto,
    );
    params.set_optional("companion_terminal_id", &op.companion_terminal_id);
    params.set_optional("companion_terminal_vendor", &op.companion_terminal_vendor);
    params.set_optional("companion_terminal_model", &op.companion_terminal_model);
    params.set_optional(
        "companion_terminal_sw_version",
        &op.companion_terminal_software,
    );
    params.set_optional(
        "companion_terminal_friendly_name",
        &op.companion_terminal_friendly_name,
    );
    params.set_optional("companion_terminal_service", &op.companion_terminal_service);
    params.set_optional("companion_terminal_iccid", &op.companion_terminal_iccid);
    params.set_optional("companion_terminal_eid", &op.companion_terminal_eid);
    params.set_optional("old_terminal_id", &op.old_terminal_id);
    params.set_optional("old_terminal_iccid", &op.old_terminal_iccid);
    params.set_optional(
        "old_terminal_entitlement_protocol",
        &op.old_terminal_entitlement_proto,
    );
    params.set_optional("MSG_response", &op.message_response);
    if op.message_button_set {
        params.set("MSG_btn", MessageButton::text(op.message_button));
    }
    for v in &op.target_terminal_imeis {
        if !v.is_empty() {
            params.add("target_terminal_imeis", v);
        }
    }
}

fn add_samsung_operation_fields(
    params: &mut ParamBuilder,
    e: &Entitlement,
    identity: &Identity,
    op: &OdsaOperation,
    realm: &str,
) -> Result<(), Ts43Error> {
    if e.channel != EntitlementChannel::Samsung || op.operation == Operation::None {
        return Ok(());
    }
    if op.operation == Operation::ManageSubscription && samsung_digitalized_temp_token_4ff(e) {
        params.remove("IMSI");
        return Ok(());
    }
    if !samsung_operation_uses_eap_identity(op.operation) {
        params.remove("IMSI");
        return Ok(());
    }
    params.remove("IMSI");
    let eap_id = imsi_eap_identity(&identity.mcc_mnc(), &identity.imsi, realm)
        .map_err(|err| Ts43Error::MalformedPayload(err.to_string()))?;
    params.set("IMSI", base64_encode(eap_id.as_bytes()));
    Ok(())
}

fn samsung_operation_uses_eap_identity(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::CheckEligibility
            | Operation::ManageSubscription
            | Operation::AcquireConfiguration
    )
}

fn samsung_digitalized_temp_token_4ff(e: &Entitlement) -> bool {
    e.channel == EntitlementChannel::Samsung
        && e.transfer_type == TransferType::TEMPORARY_TOKEN_OLD_TERMINAL_IDS
        && e.use_temp_token_conversion
}

fn operation_targets_str(ops: &[Operation]) -> String {
    if ops.is_empty() {
        return String::new();
    }
    ops.iter().map(|o| o.as_str()).collect::<Vec<_>>().join(",")
}

#[must_use]
pub fn user_agent(req: &ServiceEntitlementRequest, client_ts43: &str) -> String {
    let client = if client_ts43.is_empty() {
        Ts43Constants::DEFAULT_ODSA_CLIENT_TS43
    } else {
        client_ts43
    };
    format!(
        "PRD-TS43 term-{}/{} {}/{} OS-Android/{}",
        truncate(&req.terminal_vendor, 4),
        truncate(&req.terminal_model, 10),
        client,
        req.app_version,
        truncate(&req.terminal_software, 20),
    )
}

fn truncate(s: &str, n: usize) -> &str {
    if s.len() <= n {
        s
    } else {
        &s[..n]
    }
}

#[must_use]
pub fn entitlement_protocol(protocol: i32) -> String {
    if protocol == 0 {
        String::new()
    } else {
        protocol.to_string()
    }
}

#[must_use]
pub fn old_terminal_entitlement_protocol(source: &Device) -> String {
    if source.entitlement_protocol != 0 {
        entitlement_protocol(source.entitlement_protocol)
    } else {
        String::new()
    }
}

#[must_use]
pub fn terminal_imeis(device: &Device) -> Vec<String> {
    device.imeis.clone()
}

// ---- Google channel operation builders ----

#[must_use]
pub fn google_temporary_token_operation(e: &Entitlement, target: &Device) -> OdsaOperation {
    let mut op = OdsaOperation {
        operation: Operation::AcquireTemporaryToken,
        operation_type: OperationType::Omit,
        operation_targets: vec![
            Operation::ManageSubscription,
            Operation::AcquireConfiguration,
        ],
        ..Default::default()
    };
    if google_include_target_terminal_identity(e) {
        set_google_target_terminal_fields(&mut op, target, true);
    }
    op
}

#[must_use]
pub fn google_check_eligibility_operation(e: &Entitlement, target: &Device) -> OdsaOperation {
    let mut op = OdsaOperation {
        operation: Operation::CheckEligibility,
        operation_type: OperationType::Omit,
        ..Default::default()
    };
    if google_include_target_terminal_identity(e) {
        set_google_target_terminal_fields(&mut op, target, true);
    }
    op
}

#[must_use]
pub fn google_manage_subscription_operation(
    e: &Entitlement,
    source: &Device,
    target: &Device,
    answer: &UserInputResult,
) -> OdsaOperation {
    let mut op = base_manage_subscription_operation(OperationType::TransferSubscription, answer);
    if TransferType::is_source_credential(e.transfer_type) {
        op.target_terminal_id = target.imei.clone();
        op.target_terminal_eid = target.eid.clone();
    } else {
        op.terminal_eid = target.eid.clone();
        op.terminal_iccid = target.iccid.clone();
        if google_include_target_terminal_identity(e) {
            set_google_target_terminal_fields(&mut op, target, true);
        }
    }
    add_google_old_terminal_fields(&mut op, e, source);
    op
}

#[must_use]
pub fn google_acquire_configuration_operation(
    e: &Entitlement,
    source: &Device,
    target: &Device,
) -> OdsaOperation {
    let mut op = OdsaOperation {
        operation: Operation::AcquireConfiguration,
        operation_type: OperationType::Omit,
        ..Default::default()
    };
    if TransferType::is_source_credential(e.transfer_type) {
        op.target_terminal_id = target.imei.clone();
        op.target_terminal_eid = target.eid.clone();
        op.old_terminal_entitlement_proto = old_terminal_entitlement_protocol(source);
        return op;
    }
    op.terminal_eid = target.eid.clone();
    op.terminal_iccid = target.iccid.clone();
    if google_include_target_terminal_identity(e) {
        set_google_target_terminal_fields(&mut op, target, true);
    }
    op
}

#[must_use]
pub fn google_update_subscription_operation(
    e: &Entitlement,
    source: &Device,
    target: &Device,
) -> OdsaOperation {
    let mut op = OdsaOperation {
        operation: Operation::ManageSubscription,
        operation_type: OperationType::UpdateSubscription,
        terminal_eid: target.eid.clone(),
        terminal_iccid: target.iccid.clone(),
        ..Default::default()
    };
    if google_include_target_terminal_identity(e) {
        set_google_target_terminal_fields(&mut op, target, true);
    }
    add_google_old_terminal_fields(&mut op, e, source);
    op
}

#[must_use]
pub fn google_activation_configuration_operation(
    e: &Entitlement,
    target: &Device,
) -> OdsaOperation {
    let mut op = OdsaOperation {
        operation: Operation::AcquireConfiguration,
        operation_type: OperationType::Omit,
        terminal_eid: target.eid.clone(),
        ..Default::default()
    };
    if google_include_target_terminal_identity(e) {
        set_google_target_terminal_fields(&mut op, target, true);
    }
    op
}

fn set_google_target_terminal_fields(
    op: &mut OdsaOperation,
    target: &Device,
    include_identity: bool,
) {
    if include_identity {
        op.target_terminal_id = target.imei.clone();
        op.target_terminal_eid = target.eid.clone();
    }
    op.target_terminal_iccid = target.iccid.clone();
    op.target_terminal_model = target.model.clone();
    op.target_terminal_sn = target.serial_number.clone();
    op.target_terminal_imeis = terminal_imeis(target);
    op.target_terminal_entitlement_proto = entitlement_protocol(target.entitlement_protocol);
}

fn add_google_old_terminal_fields(op: &mut OdsaOperation, e: &Entitlement, source: &Device) {
    if !google_include_old_terminal_ids(e) {
        return;
    }
    op.old_terminal_id = source.imei.clone();
    op.old_terminal_iccid = source.iccid.clone();
    op.old_terminal_entitlement_proto = old_terminal_entitlement_protocol(source);
}

fn google_include_target_terminal_identity(e: &Entitlement) -> bool {
    match e.transfer_type {
        TransferType::TEMPORARY_TOKEN | TransferType::TEMPORARY_TOKEN_VARIANT11 => {
            e.set_target_terminal_fields
        }
        _ => true,
    }
}

fn google_include_old_terminal_ids(e: &Entitlement) -> bool {
    if !e.restrict_old_terminal_fields {
        return true;
    }
    matches!(
        e.transfer_type,
        TransferType::OLD_TERMINAL_IDS | TransferType::TEMPORARY_TOKEN_OLD_TERMINAL_IDS
    )
}

#[must_use]
pub fn base_manage_subscription_operation(
    operation_type: OperationType,
    answer: &UserInputResult,
) -> OdsaOperation {
    OdsaOperation {
        operation: Operation::ManageSubscription,
        operation_type,
        message_response: answer.response.clone(),
        message_button: answer.button,
        message_button_set: answer.button != MessageButton::UNKNOWN,
        ..Default::default()
    }
}

// ---- Samsung channel operation builders ----

#[must_use]
pub fn samsung_temporary_token_operation(source: &Device) -> OdsaOperation {
    OdsaOperation {
        operation: Operation::AcquireTemporaryToken,
        operation_type: OperationType::Omit,
        operation_targets: vec![
            Operation::ManageSubscription,
            Operation::AcquireConfiguration,
        ],
        terminal_iccid: source.iccid.clone(),
        ..Default::default()
    }
}

#[must_use]
pub fn samsung_check_eligibility_operation(source: &Device, target: &Device) -> OdsaOperation {
    OdsaOperation {
        operation: Operation::CheckEligibility,
        operation_type: OperationType::Omit,
        terminal_iccid: source.iccid.clone(),
        target_terminal_id: target.imei.clone(),
        ..Default::default()
    }
}

#[must_use]
pub fn samsung_manage_subscription_operation(
    e: &Entitlement,
    source: &Device,
    target: &Device,
    answer: &UserInputResult,
) -> OdsaOperation {
    let mut op = base_manage_subscription_operation(
        samsung_manage_subscription_operation_type(e.transfer_type),
        answer,
    );
    if TransferType::is_source_credential(e.transfer_type) {
        op.target_terminal_id = target.imei.clone();
        op.target_terminal_eid = target.eid.clone();
        op.terminal_iccid = source.iccid.clone();
        op.target_terminal_iccid = target.iccid.clone();
    } else {
        op.terminal_eid = target.eid.clone();
        op.terminal_iccid = target.iccid.clone();
        op.target_terminal_id = target.imei.clone();
        op.target_terminal_iccid = target.iccid.clone();
        if samsung_digitalized_temp_token_4ff(e) {
            op.terminal_iccid.clear();
        }
    }
    add_samsung_old_terminal_fields(&mut op, source);
    op
}

#[must_use]
pub fn samsung_acquire_configuration_operation(
    e: &Entitlement,
    source: &Device,
    target: &Device,
) -> OdsaOperation {
    let mut op = OdsaOperation {
        operation: Operation::AcquireConfiguration,
        operation_type: OperationType::Omit,
        ..Default::default()
    };
    if TransferType::is_source_credential(e.transfer_type) {
        op.target_terminal_id = target.imei.clone();
        op.terminal_eid = first_non_empty(&[&source.eid, &target.eid]);
        op.terminal_iccid = source.iccid.clone();
        op.target_terminal_iccid = target.iccid.clone();
        return op;
    }
    op.terminal_iccid = target.iccid.clone();
    set_samsung_acquire_configuration_eid(&mut op, e, target);
    op.target_terminal_id = target.imei.clone();
    op.target_terminal_iccid = target.iccid.clone();
    op
}

#[must_use]
pub fn samsung_update_subscription_operation(source: &Device, target: &Device) -> OdsaOperation {
    OdsaOperation {
        operation: Operation::ManageSubscription,
        operation_type: OperationType::UpdateSubscription,
        terminal_eid: target.eid.clone(),
        terminal_iccid: target.iccid.clone(),
        target_terminal_id: target.imei.clone(),
        target_terminal_iccid: target.iccid.clone(),
        old_terminal_id: source.imei.clone(),
        old_terminal_iccid: source.iccid.clone(),
        ..Default::default()
    }
}

#[must_use]
pub fn samsung_activation_configuration_operation(target: &Device) -> OdsaOperation {
    OdsaOperation {
        operation: Operation::AcquireConfiguration,
        operation_type: OperationType::Omit,
        terminal_eid: target.eid.clone(),
        target_terminal_id: target.imei.clone(),
        target_terminal_iccid: target.iccid.clone(),
        ..Default::default()
    }
}

fn samsung_manage_subscription_operation_type(transfer_type: i32) -> OperationType {
    match transfer_type {
        TransferType::REUSABLE_ACTIVATION_CODE
        | TransferType::TS43_V8
        | TransferType::OLD_TERMINAL_IDS
        | TransferType::TEMPORARY_TOKEN_OLD_TERMINAL_IDS
        | TransferType::TEMPORARY_TOKEN_VARIANT11 => OperationType::TransferSubscription,
        _ => OperationType::SamsungD2DTransfer,
    }
}

fn set_samsung_acquire_configuration_eid(op: &mut OdsaOperation, e: &Entitlement, target: &Device) {
    if e.transfer_type == TransferType::OLD_TERMINAL_IDS {
        op.terminal_eid.clear();
        op.target_terminal_eid = target.eid.clone();
    } else {
        op.terminal_eid = target.eid.clone();
        op.target_terminal_eid.clear();
    }
}

fn add_samsung_old_terminal_fields(op: &mut OdsaOperation, source: &Device) {
    op.old_terminal_id = source.imei.clone();
    op.old_terminal_iccid = source.iccid.clone();
}

// ---- Dispatch operations (operation.go) ----

#[must_use]
pub fn temporary_token_operation(
    e: &Entitlement,
    source: &Device,
    target: &Device,
) -> OdsaOperation {
    if e.channel == EntitlementChannel::Samsung {
        samsung_temporary_token_operation(source)
    } else {
        google_temporary_token_operation(e, target)
    }
}

#[must_use]
pub fn check_eligibility_operation(
    e: &Entitlement,
    source: &Device,
    target: &Device,
) -> OdsaOperation {
    if e.channel == EntitlementChannel::Samsung {
        samsung_check_eligibility_operation(source, target)
    } else {
        google_check_eligibility_operation(e, target)
    }
}

#[must_use]
pub fn manage_subscription_request_device<'a>(
    e: &Entitlement,
    source: &'a Device,
    target: &'a Device,
) -> &'a Device {
    if TransferType::is_source_credential(e.transfer_type)
        || e.transfer_type == TransferType::TEMPORARY_TOKEN
    {
        source
    } else {
        target
    }
}

#[must_use]
pub fn manage_subscription_operation(
    e: &Entitlement,
    source: &Device,
    target: &Device,
    answer: &UserInputResult,
) -> OdsaOperation {
    if e.channel == EntitlementChannel::Samsung {
        samsung_manage_subscription_operation(e, source, target, answer)
    } else {
        google_manage_subscription_operation(e, source, target, answer)
    }
}

#[must_use]
pub fn acquire_configuration_request_device<'a>(
    e: &Entitlement,
    source: &'a Device,
    target: &'a Device,
) -> &'a Device {
    if TransferType::is_source_credential(e.transfer_type) {
        source
    } else {
        target
    }
}

#[must_use]
pub fn acquire_configuration_operation(
    e: &Entitlement,
    source: &Device,
    target: &Device,
) -> OdsaOperation {
    if e.channel == EntitlementChannel::Samsung {
        samsung_acquire_configuration_operation(e, source, target)
    } else {
        google_acquire_configuration_operation(e, source, target)
    }
}

#[must_use]
pub fn update_subscription_operation(
    e: &Entitlement,
    source: &Device,
    target: &Device,
) -> OdsaOperation {
    if e.channel == EntitlementChannel::Samsung {
        samsung_update_subscription_operation(source, target)
    } else {
        google_update_subscription_operation(e, source, target)
    }
}

#[must_use]
pub fn activation_configuration_operation(e: &Entitlement, target: &Device) -> OdsaOperation {
    if e.channel == EntitlementChannel::Samsung {
        samsung_activation_configuration_operation(target)
    } else {
        google_activation_configuration_operation(e, target)
    }
}
