//! TS.43 client orchestration and protocol state machine.
//!
//! Ported from `client.go` / `service.go` / `websheet.go` / `ts43_client.dart`.

use std::error::Error;
use std::sync::Arc;

use super::constants::Ts43Constants;
use super::entitlement::{
    discover_entitlement_for_channel, EntitlementRecord, DEFAULT_ENTITLEMENT_TIMEOUT,
};
use super::enums::{
    AppEligibility, EntitlementChannel, MessageButton, Operation, OperationResult, SimType,
    SubscriptionResult, TransferType,
};
use super::errors::{
    CarrierDeniedError, Ts43HttpError, WebsheetException, WEBSHEET_DEFAULT_SMDP_UNSUPPORTED,
    WEBSHEET_DISMISSED,
};
use super::request::{
    acquire_configuration_operation, acquire_configuration_request_device, build_prepared_request,
    check_eligibility_operation, eap_relay_accept, manage_subscription_operation,
    manage_subscription_request_device, temporary_token_operation, update_subscription_operation,
    user_agent, OdsaOperation, PreparedRequest, ServiceEntitlementRequest,
};
use super::response::OdsaResponse;
use super::sim::{
    imsi_eap_identity, respond_to_challenge_base64, AkaChallenge, AkaRequest, AkaResponse, Identity,
};
use super::types::{
    first_non_empty, merge_cookie_headers, ActivationResult, AuthToken, ContinueRequest, Device,
    DownloadConfig, Endpoint, Entitlement, TemporaryToken, TransferOffer, TransferSession,
    TransferTokenResult, Ts43Event, Ts43Result, Ts43State, UserInputResult, WebsheetCallbackEvent,
    WebsheetResult,
};
use super::Ts43Error;

/// HTTP method for TS.43 requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ts43Method {
    Get,
    Post,
}

/// A fully resolved TS.43 HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ts43HttpRequest {
    pub method: Ts43Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub cookies: Vec<String>,
}

/// A raw TS.43 HTTP response: body bytes and response cookies.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ts43HttpResponse {
    pub body: Vec<u8>,
    pub cookies: Vec<String>,
}

impl Ts43HttpResponse {
    #[must_use]
    pub fn new(body: Vec<u8>, cookies: Vec<String>) -> Self {
        Self { body, cookies }
    }
}

/// Pluggable transport for TS.43 HTTP calls.
pub trait Ts43Transport: Send + Sync {
    fn do_raw(
        &self,
        request: &Ts43HttpRequest,
    ) -> Result<Ts43HttpResponse, Box<dyn Error + Send + Sync>>;
}

/// Provider interface for source SIM identity and EAP-AKA authentication.
pub trait SourceSimProvider: Send + Sync {
    fn identity(&self) -> Result<Identity, Box<dyn Error + Send + Sync>>;
    fn authenticate_aka(
        &self,
        request: &AkaRequest,
    ) -> Result<AkaResponse, Box<dyn Error + Send + Sync>>;
}

/// Configuration for `Ts43Client`.
pub struct Ts43Config {
    pub transport: Arc<dyn Ts43Transport>,
    pub source_sim: Option<Arc<dyn SourceSimProvider>>,
    pub entitlement_records: Option<Vec<EntitlementRecord>>,
    pub entitlement: Entitlement,
    pub source: Endpoint,
    pub target: Endpoint,
    pub companion: Device,
    pub app_version: String,
    pub client_ts43: String,
    pub realm: String,
}

struct TransferCredential {
    auth: AuthToken,
    temporary: TemporaryToken,
    is_temporary_token: bool,
}

impl TransferCredential {
    fn of_auth(token: AuthToken) -> Self {
        Self {
            auth: token,
            temporary: TemporaryToken::default(),
            is_temporary_token: false,
        }
    }

    fn of_temporary(token: TemporaryToken) -> Self {
        Self {
            auth: AuthToken::default(),
            temporary: token,
            is_temporary_token: true,
        }
    }
}

struct CallResult {
    resp: OdsaResponse,
    cookies: Vec<String>,
}

/// TS.43 carrier-managed transfer client.
pub struct Ts43Client {
    entitlement: Entitlement,
    source: Endpoint,
    target: Endpoint,
    companion: Device,
    app_version: String,
    client_ts43_override: String,
    realm_override: String,
    entitlement_records: Option<Vec<EntitlementRecord>>,
    transport: Arc<dyn Ts43Transport>,
    source_sim: Option<Arc<dyn SourceSimProvider>>,
    events: Vec<Ts43Event>,
}

impl Ts43Client {
    #[must_use]
    pub fn new(cfg: Ts43Config) -> Self {
        let mut e = cfg.entitlement;
        e.transfer_type = selected_transfer_type(&e);
        if e.timeout.is_zero() {
            e.timeout = DEFAULT_ENTITLEMENT_TIMEOUT;
        }
        if e.poll_interval.is_zero() {
            e.poll_interval = core::time::Duration::from_secs(60);
        }
        if e.poll_timeout.is_zero() {
            e.poll_timeout = core::time::Duration::from_secs(600);
        }
        if e.auto_retry_511_limit == 0 {
            e.auto_retry_511_limit = 1;
        }

        Self {
            entitlement: e,
            source: cfg.source,
            target: cfg.target,
            companion: cfg.companion,
            app_version: cfg.app_version,
            client_ts43_override: cfg.client_ts43,
            realm_override: cfg.realm,
            entitlement_records: cfg.entitlement_records,
            transport: cfg.transport,
            source_sim: cfg.source_sim,
            events: Vec::new(),
        }
    }

    pub fn events(&self) -> &[Ts43Event] {
        &self.events
    }

    fn emit(&mut self, event: Ts43Event) {
        self.events.push(event);
    }

    fn uses_temporary_token(&self) -> bool {
        TransferType::uses_temporary_token(self.entitlement.transfer_type)
    }

    // ---- Top-level transfer flow ----

    pub fn transfer(&mut self) -> Result<Ts43Result, Box<dyn Error + Send + Sync>> {
        let sim = self
            .source_sim
            .clone()
            .ok_or_else(|| Ts43Error::AuthenticationFailed("source SIM is required".to_string()))?;
        let identity = sim.identity()?;
        self.discover_entitlement(&identity)?;

        let mut source = self.source.device.clone();
        source.iccid = first_non_empty(&[&source.iccid, &identity.iccid]);
        let target = self.target.device.clone();

        let tokens = self.fetch_transfer_token(&identity, &source, &target)?;
        let offer = TransferOffer {
            source_identity: identity,
            source_device: source,
            target_device: target,
            auth_token: tokens.auth,
            temporary_token: tokens.temporary,
        };

        self.activate(
            offer,
            UserInputResult {
                response: String::new(),
                button: MessageButton::UNKNOWN,
            },
        )
    }

    fn discover_entitlement(
        &mut self,
        identity: &Identity,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.entitlement.server.is_empty() {
            return Ok(());
        }
        let requested = self.entitlement.clone();
        let records = self.entitlement_records.as_deref().ok_or_else(|| {
            Ts43Error::MalformedPayload("no entitlement records provided".to_string())
        })?;
        let mut discovered = discover_entitlement_for_channel(
            records,
            identity,
            requested.source_sim_type,
            requested.channel,
        )?;
        discovered.source_sim_type = requested.source_sim_type;
        if requested.timeout != DEFAULT_ENTITLEMENT_TIMEOUT || discovered.timeout.is_zero() {
            discovered.timeout = requested.timeout;
        }
        discovered.auto_retry_511_limit = requested.auto_retry_511_limit;
        if discovered.poll_interval.is_zero() {
            discovered.poll_interval = core::time::Duration::from_secs(60);
        }
        if discovered.poll_timeout.is_zero() {
            discovered.poll_timeout = core::time::Duration::from_secs(600);
        }
        if discovered.auto_retry_511_limit == 0 {
            discovered.auto_retry_511_limit = 1;
        }
        self.entitlement = discovered;
        Ok(())
    }

    pub fn continue_flow(
        &mut self,
        result: &mut Ts43Result,
        req: &ContinueRequest,
    ) -> Result<Ts43Result, Box<dyn Error + Send + Sync>> {
        match result.state {
            Ts43State::PendingUserInput => {
                let session = require_session(result)?;
                let user_input = req.user_input.as_ref().ok_or_else(|| {
                    Ts43Error::MalformedPayload("user input is required".to_string())
                })?;
                if user_input.button == MessageButton::UNKNOWN {
                    return Err(Box::new(Ts43Error::MalformedPayload(
                        "user input button is required".to_string(),
                    )));
                }
                self.activate(session.offer.clone(), user_input.clone())
            }
            Ts43State::PendingWebsheet => {
                let _ = require_session(result)?;
                let websheet = req.websheet.as_ref().ok_or_else(|| {
                    Ts43Error::MalformedPayload("websheet result is required".to_string())
                })?;
                self.handle_websheet_result(result, websheet)
            }
            Ts43State::PendingSourceProfileDeletion => {
                let session = require_session(result)?;
                if !req.source_profile_deleted {
                    return Err(Box::new(Ts43Error::MalformedPayload(
                        "source profile deletion confirmation is required".to_string(),
                    )));
                }
                if TransferType::is_source_credential(self.entitlement.transfer_type) {
                    let mut offer = session.offer.clone();
                    if offer.source_device.entitlement_protocol == 0 {
                        offer.source_device.entitlement_protocol = 1;
                    }
                    result.session = Some(TransferSession {
                        offer,
                        source_profile_iccid: session.source_profile_iccid.clone(),
                    });
                    return self.poll_after_source_profile_deletion(result);
                }
                self.activate(
                    session.offer.clone(),
                    UserInputResult {
                        response: String::new(),
                        button: MessageButton::UNKNOWN,
                    },
                )
            }
            _ => Err(Box::new(Ts43Error::MalformedPayload(format!(
                "state {:?} cannot continue",
                result.state
            )))),
        }
    }

    pub fn complete_activation(
        &mut self,
        result: &mut Ts43Result,
        activation: &ActivationResult,
    ) -> Result<Ts43Result, Box<dyn Error + Send + Sync>> {
        let session = require_session(result)?;
        let mut offer = session.offer.clone();
        let iccid = first_non_empty(&[&activation.iccid, &result.download_config.profile_iccid]);
        if !iccid.is_empty() {
            offer.target_device.iccid = iccid.clone();
            result.download_config.profile_iccid = iccid;
        }

        if TransferType::is_source_credential(self.entitlement.transfer_type) {
            result.state = Ts43State::Done;
            result.event = Some(Ts43Event::Done);
            self.emit(Ts43Event::Done);
            return Ok(result.clone());
        }

        let mut credential = self.transfer_credential(&offer)?;
        let mut attempt = 0;
        let update_resp = loop {
            match self.update_subscription(
                &offer.source_identity,
                &offer.source_device,
                &offer.target_device,
                &credential,
            ) {
                Ok(resp) => break resp,
                Err(err) => {
                    if is_http_511(err.as_ref()) && attempt < self.entitlement.auto_retry_511_limit
                    {
                        attempt += 1;
                        credential = self.refresh_transfer_credential(&mut offer)?;
                        continue;
                    }
                    return Err(err);
                }
            }
        };

        result.update_subscription = Some(update_resp.clone());
        let sr = update_resp.subscription_result;
        if sr == SubscriptionResult::DONE {
            result.state = Ts43State::Done;
            result.event = Some(Ts43Event::Done);
            self.emit(Ts43Event::Done);
            return Ok(result.clone());
        }
        if sr == SubscriptionResult::DISMISS {
            result.state = Ts43State::Dismissed;
            result.event = Some(Ts43Event::Dismiss(sr));
            self.emit(Ts43Event::Dismiss(sr));
            return Ok(result.clone());
        }
        if sr == SubscriptionResult::DELAYED_DOWNLOAD {
            result.state = Ts43State::PendingActivation;
            let ev = Ts43Event::ActivationPending {
                iccid: offer.target_device.iccid.clone(),
                subscription_result: sr,
            };
            result.event = Some(ev.clone());
            self.emit(ev);
            return Ok(result.clone());
        }

        result.state = Ts43State::Done;
        result.event = Some(Ts43Event::Done);
        self.emit(Ts43Event::Done);
        Ok(result.clone())
    }

    // ---- Token fetch ----

    fn fetch_transfer_token(
        &mut self,
        identity: &Identity,
        source: &Device,
        target: &Device,
    ) -> Result<TransferTokenResult, Box<dyn Error + Send + Sync>> {
        let mut attempt = 0;
        loop {
            match self.fetch_transfer_token_once(identity, source, target) {
                Ok(res) => return Ok(res),
                Err(err) => {
                    if is_http_511(err.as_ref()) && attempt < self.entitlement.auto_retry_511_limit
                    {
                        attempt += 1;
                        continue;
                    }
                    return Err(err);
                }
            }
        }
    }

    fn fetch_transfer_token_once(
        &mut self,
        identity: &Identity,
        source: &Device,
        target: &Device,
    ) -> Result<TransferTokenResult, Box<dyn Error + Send + Sync>> {
        let auth = self.authenticate(identity, source)?;
        let auth = self.check_eligibility(identity, source, auth, target)?;
        self.transfer_tokens(identity, source, target, auth)
    }

    fn transfer_tokens(
        &mut self,
        identity: &Identity,
        source: &Device,
        target: &Device,
        auth: AuthToken,
    ) -> Result<TransferTokenResult, Box<dyn Error + Send + Sync>> {
        if !self.uses_temporary_token() {
            return Ok(TransferTokenResult {
                auth,
                temporary: TemporaryToken::default(),
            });
        }
        self.acquire_temporary_token(identity, source, target, auth)
    }

    fn authenticate(
        &mut self,
        identity: &Identity,
        device: &Device,
    ) -> Result<AuthToken, Box<dyn Error + Send + Sync>> {
        let mut req = self.base_request(device, "", "");
        let op = OdsaOperation::default();
        let result = match self.call(identity, &req, &op) {
            Ok(res) => res,
            Err(err) => {
                if is_http_511(err.as_ref()) {
                    req.omit_eap_id = true;
                    self.call(identity, &req, &op)?
                } else {
                    return Err(err);
                }
            }
        };

        let resp = result.resp;
        if resp.authentication_token.is_empty() {
            return Err(Box::new(Ts43Error::AuthenticationFailed(
                "authentication token is missing".to_string(),
            )));
        }
        Ok(AuthToken {
            value: resp.authentication_token,
            expires_at: None,
            cookies: result.cookies,
        })
    }

    fn acquire_temporary_token(
        &mut self,
        identity: &Identity,
        source: &Device,
        target: &Device,
        auth: AuthToken,
    ) -> Result<TransferTokenResult, Box<dyn Error + Send + Sync>> {
        let req = self.auth_request(source, &auth);
        let mut op = temporary_token_operation(&self.entitlement, source, target);
        self.add_optional_operation_fields(&mut op);
        let result = self.call(identity, &req, &op)?;
        let resp = result.resp;
        if resp.operation_result != OperationResult::UNKNOWN
            && resp.operation_result != OperationResult::SUCCESS
        {
            return Err(Box::new(CarrierDeniedError::new(
                Operation::AcquireTemporaryToken,
                resp.general_error_text,
            )));
        }
        if resp.temporary_token.is_empty() {
            return Err(Box::new(Ts43Error::AuthenticationFailed(
                "temporary token is missing".to_string(),
            )));
        }
        let merged_cookies = merge_cookie_headers(&auth.cookies, &result.cookies);
        Ok(TransferTokenResult {
            auth: auth_token_from_response(&auth, &resp, &merged_cookies),
            temporary: TemporaryToken {
                value: resp.temporary_token,
                expires_at: None,
                cookies: merged_cookies,
                targets: resp.operation_targets,
            },
        })
    }

    fn check_eligibility(
        &mut self,
        identity: &Identity,
        source: &Device,
        auth: AuthToken,
        target: &Device,
    ) -> Result<AuthToken, Box<dyn Error + Send + Sync>> {
        let req = self.auth_request(source, &auth);
        let mut op = check_eligibility_operation(&self.entitlement, source, target);
        self.add_optional_operation_fields(&mut op);
        let result = self.call(identity, &req, &op)?;
        let resp = result.resp;
        if resp.operation_result != OperationResult::SUCCESS
            || resp.primary_app_eligibility != AppEligibility::ENABLED
        {
            return Err(Box::new(CarrierDeniedError::new(
                Operation::CheckEligibility,
                resp.general_error_text,
            )));
        }
        let merged = merge_cookie_headers(&auth.cookies, &result.cookies);
        Ok(auth_token_from_response(&auth, &resp, &merged))
    }

    // ---- Activate & subscription operations ----

    fn activate(
        &mut self,
        mut offer: TransferOffer,
        answer: UserInputResult,
    ) -> Result<Ts43Result, Box<dyn Error + Send + Sync>> {
        let mut credential = self.transfer_credential(&offer)?;
        let mut attempt = 0;
        let manage_resp = loop {
            match self.manage_subscription(
                &offer.source_identity,
                &offer.source_device,
                &offer.target_device,
                &credential,
                &answer,
            ) {
                Ok(resp) => break resp,
                Err(err) => {
                    if is_http_511(err.as_ref()) && attempt < self.entitlement.auto_retry_511_limit
                    {
                        attempt += 1;
                        credential = self.refresh_transfer_credential(&mut offer)?;
                        continue;
                    }
                    return Err(err);
                }
            }
        };

        let mut result = Ts43Result {
            manage_subscription: Some(manage_resp.clone()),
            session: Some(TransferSession {
                offer: offer.clone(),
                source_profile_iccid: String::new(),
            }),
            ..Default::default()
        };

        if source_profile_deletion_required(&manage_resp) {
            let session = result.session.as_mut().unwrap();
            session.source_profile_iccid =
                first_non_empty(&[&offer.source_device.iccid, &offer.source_identity.iccid]);
            result.state = Ts43State::PendingSourceProfileDeletion;
            let ev = Ts43Event::SourceProfileDeletion(session.source_profile_iccid.clone());
            result.event = Some(ev.clone());
            self.emit(ev);
            return Ok(result);
        }
        if user_input_required(&manage_resp) {
            result.state = Ts43State::PendingUserInput;
            let ev = Ts43Event::UserInput(manage_resp.message.clone());
            result.event = Some(ev.clone());
            self.emit(ev);
            return Ok(result);
        }
        if websheet_required(&manage_resp) {
            result.state = Ts43State::PendingWebsheet;
            let ev = Ts43Event::Websheet(manage_resp.websheet.clone());
            result.event = Some(ev.clone());
            self.emit(ev);
            return Ok(result);
        }
        if download_profile_ready(&manage_resp) {
            result.download_config =
                complete_download_config(manage_resp.download.clone(), &offer.target_device);
            result.state = Ts43State::PendingDownload;
            let ev = Ts43Event::DownloadReady(result.download_config.clone());
            result.event = Some(ev.clone());
            self.emit(ev);
            return Ok(result);
        }
        if subscription_done(&manage_resp) {
            result.state = Ts43State::Done;
            result.event = Some(Ts43Event::Done);
            self.emit(Ts43Event::Done);
            return Ok(result);
        }
        if subscription_dismissed(&manage_resp) {
            result.state = Ts43State::Dismissed;
            let ev = Ts43Event::Dismiss(manage_resp.subscription_result);
            result.event = Some(ev.clone());
            self.emit(ev);
            return Ok(result);
        }
        if delayed_download_required(&manage_resp) {
            self.apply_delayed_download_state(&mut result, &offer, manage_resp.subscription_result);
            if let Some(ev) = &result.event {
                self.emit(ev.clone());
            }
            return Ok(result);
        }

        // Poll configuration
        let config = self.acquire_configuration(
            &offer.source_identity,
            &offer.source_device,
            &offer.target_device,
            &credential,
        )?;
        result.configuration = Some(config.clone());
        self.apply_download_configuration(&mut result, config.download)
    }

    fn manage_subscription(
        &mut self,
        identity: &Identity,
        source: &Device,
        target: &Device,
        credential: &TransferCredential,
        answer: &UserInputResult,
    ) -> Result<OdsaResponse, Box<dyn Error + Send + Sync>> {
        let req_device = manage_subscription_request_device(&self.entitlement, source, target);
        let req = self.transfer_request(req_device, credential);
        let mut op = manage_subscription_operation(&self.entitlement, source, target, answer);
        self.add_optional_operation_fields(&mut op);
        let result = self.call(identity, &req, &op)?;
        if !manage_subscription_accepted(&result.resp) {
            return Err(Box::new(CarrierDeniedError::new(
                Operation::ManageSubscription,
                result.resp.general_error_text,
            )));
        }
        Ok(result.resp)
    }

    fn acquire_configuration(
        &mut self,
        identity: &Identity,
        source: &Device,
        target: &Device,
        credential: &TransferCredential,
    ) -> Result<OdsaResponse, Box<dyn Error + Send + Sync>> {
        let req_device = acquire_configuration_request_device(&self.entitlement, source, target);
        let req = self.transfer_request(req_device, credential);
        let mut op = acquire_configuration_operation(&self.entitlement, source, target);
        self.add_optional_operation_fields(&mut op);
        let result = self.call(identity, &req, &op)?;
        Ok(result.resp)
    }

    fn update_subscription(
        &mut self,
        identity: &Identity,
        source: &Device,
        target: &Device,
        credential: &TransferCredential,
    ) -> Result<OdsaResponse, Box<dyn Error + Send + Sync>> {
        let req = self.transfer_request(target, credential);
        let mut op = update_subscription_operation(&self.entitlement, source, target);
        self.add_optional_operation_fields(&mut op);
        let result = self.call(identity, &req, &op)?;
        if !subscription_update_accepted(&result.resp) {
            return Err(Box::new(CarrierDeniedError::new(
                Operation::ManageSubscription,
                result.resp.general_error_text,
            )));
        }
        Ok(result.resp)
    }

    fn poll_after_source_profile_deletion(
        &mut self,
        result: &mut Ts43Result,
    ) -> Result<Ts43Result, Box<dyn Error + Send + Sync>> {
        let session = require_session(result)?;
        let offer = session.offer.clone();
        let credential = self.transfer_credential(&offer)?;
        let config = self.acquire_configuration(
            &offer.source_identity,
            &offer.source_device,
            &offer.target_device,
            &credential,
        )?;
        result.configuration = Some(config.clone());
        self.apply_download_configuration(result, config.download)
    }

    // ---- Websheet callback handler ----

    fn handle_websheet_result(
        &mut self,
        result: &mut Ts43Result,
        callback: &WebsheetResult,
    ) -> Result<Ts43Result, Box<dyn Error + Send + Sync>> {
        let session = require_session(result)?;
        let mut offer = session.offer.clone();

        match callback.event {
            WebsheetCallbackEvent::ProfileReadyWithActivationCode => {
                if callback.activation_code.is_empty() {
                    return Err(Box::new(Ts43Error::MalformedPayload(
                        "activation code is required".to_string(),
                    )));
                }
                result.download_config = complete_download_config(
                    DownloadConfig {
                        activation_code: callback.activation_code.clone(),
                        profile_iccid: callback.iccid.clone(),
                        imei: first_non_empty(&[&callback.imei, &offer.target_device.imei]),
                        ..Default::default()
                    },
                    &offer.target_device,
                );
                result.state = Ts43State::PendingDownload;
                let ev = Ts43Event::DownloadReady(result.download_config.clone());
                result.event = Some(ev.clone());
                self.emit(ev);
                Ok(result.clone())
            }
            WebsheetCallbackEvent::ProfileReadyWithDefaultSmdp => Err(Box::new(WebsheetException(
                WEBSHEET_DEFAULT_SMDP_UNSUPPORTED.to_string(),
            ))),
            WebsheetCallbackEvent::SelectionCompleted => {
                if callback.iccid.is_empty() && callback.imei.is_empty() {
                    return Err(Box::new(Ts43Error::MalformedPayload(
                        "selected ICCID or IMEI is required".to_string(),
                    )));
                }
                if !callback.iccid.is_empty() {
                    offer.source_device.iccid = callback.iccid.clone();
                    offer.source_identity.iccid = callback.iccid.clone();
                }
                if !callback.imei.is_empty() {
                    offer.source_device.imei = callback.imei.clone();
                }
                self.activate(
                    offer,
                    UserInputResult {
                        response: String::new(),
                        button: MessageButton::UNKNOWN,
                    },
                )
            }
            WebsheetCallbackEvent::DismissFlow => {
                Err(Box::new(WebsheetException(WEBSHEET_DISMISSED.to_string())))
            }
            WebsheetCallbackEvent::FinishFlow => {
                self.continue_websheet_next_action(result, callback.next_action)
            }
            WebsheetCallbackEvent::DeleteToken => {
                offer.auth_token = AuthToken::default();
                offer.temporary_token = TemporaryToken::default();
                result.session = Some(TransferSession {
                    offer,
                    source_profile_iccid: session.source_profile_iccid.clone(),
                });
                Ok(result.clone())
            }
            WebsheetCallbackEvent::CheckProfileServiceStatus => {
                let credential = self.transfer_credential(&offer)?;
                let config = self.acquire_configuration(
                    &offer.source_identity,
                    &offer.source_device,
                    &offer.target_device,
                    &credential,
                )?;
                result.configuration = Some(config.clone());
                self.apply_download_configuration(result, config.download)
            }
            WebsheetCallbackEvent::DeleteProfileInUse => {
                let mut session = session.clone();
                session.source_profile_iccid = first_non_empty(&[
                    &callback.iccid,
                    &session.source_profile_iccid,
                    &offer.source_device.iccid,
                    &offer.source_identity.iccid,
                ]);
                result.session = Some(session.clone());
                result.state = Ts43State::PendingSourceProfileDeletion;
                let ev = Ts43Event::SourceProfileDeletion(session.source_profile_iccid);
                result.event = Some(ev.clone());
                self.emit(ev);
                Ok(result.clone())
            }
            _ => Err(Box::new(Ts43Error::MalformedPayload(
                "websheet callback event is required".to_string(),
            ))),
        }
    }

    fn continue_websheet_next_action(
        &mut self,
        result: &mut Ts43Result,
        mut next: Operation,
    ) -> Result<Ts43Result, Box<dyn Error + Send + Sync>> {
        let session = require_session(result)?;
        let offer = session.offer.clone();
        if next == Operation::None {
            next = Operation::AcquireConfiguration;
        }
        if next == Operation::AcquireConfiguration {
            let credential = self.transfer_credential(&offer)?;
            let config = self.acquire_configuration(
                &offer.source_identity,
                &offer.source_device,
                &offer.target_device,
                &credential,
            )?;
            result.configuration = Some(config.clone());
            return self.apply_download_configuration(result, config.download);
        }
        if next == Operation::ManageSubscription {
            return self.activate(
                offer,
                UserInputResult {
                    response: String::new(),
                    button: MessageButton::UNKNOWN,
                },
            );
        }
        Err(Box::new(Ts43Error::MalformedPayload(format!(
            "unsupported websheet next action {:?}",
            next
        ))))
    }

    // ---- Transport & EAP relay ----

    fn call(
        &mut self,
        identity: &Identity,
        req: &ServiceEntitlementRequest,
        op: &OdsaOperation,
    ) -> Result<CallResult, Box<dyn Error + Send + Sync>> {
        let prepared = build_prepared_request(
            &self.entitlement,
            identity,
            req,
            op,
            &self.client_ts43(),
            &self.realm(),
        )?;
        let http_req = self.to_http_request(&prepared);
        let initial = self.transport.do_raw(&http_req)?;
        let relayed = self.eap_relay(identity, initial, req)?;
        let resp = OdsaResponse::decode(&relayed.body);
        Ok(CallResult {
            resp,
            cookies: relayed.cookies,
        })
    }

    fn to_http_request(&self, prepared: &PreparedRequest) -> Ts43HttpRequest {
        let headers = vec![
            ("Accept".to_string(), prepared.accept.clone()),
            ("User-Agent".to_string(), prepared.user_agent.clone()),
        ];
        if prepared.use_post {
            let mut h = headers;
            h.push(("Content-Type".to_string(), "application/json".to_string()));
            return Ts43HttpRequest {
                method: Ts43Method::Post,
                url: self.entitlement.server.clone(),
                headers: h,
                body: prepared.json_body().into_bytes(),
                cookies: prepared.cookies.clone(),
            };
        }

        let query = prepared.query_string();
        let base = &self.entitlement.server;
        let url = if base.contains('?') {
            format!("{base}&{query}")
        } else if !query.is_empty() {
            format!("{base}?{query}")
        } else {
            base.clone()
        };

        Ts43HttpRequest {
            method: Ts43Method::Get,
            url,
            headers,
            body: Vec::new(),
            cookies: prepared.cookies.clone(),
        }
    }

    fn eap_relay(
        &mut self,
        identity: &Identity,
        initial: Ts43HttpResponse,
        req: &ServiceEntitlementRequest,
    ) -> Result<Ts43HttpResponse, Box<dyn Error + Send + Sync>> {
        let mut body = initial.body;
        let mut request_cookies = initial.cookies.clone();
        let mut response_cookies = initial.cookies;

        for _ in 0..3 {
            let packet = relay_packet(&body);
            if packet.is_empty() {
                return Ok(Ts43HttpResponse::new(body, response_cookies));
            }
            let challenge = AkaChallenge::from_base64(&packet)?;
            let sim = self.source_sim.clone().ok_or_else(|| {
                Ts43Error::AuthenticationFailed("SIM required for EAP-AKA".to_string())
            })?;
            let aka_resp = sim.authenticate_aka(&challenge.authentication_request())?;
            let eap_id = imsi_eap_identity(&identity.mcc_mnc(), &identity.imsi, &self.realm())?;
            let response_text = respond_to_challenge_base64(&challenge, &aka_resp, &eap_id)?;
            let next = self.post_relay(&response_text, req, &request_cookies)?;
            request_cookies = next.cookies.clone();
            response_cookies = next.cookies.clone();
            body = next.body;
        }

        Err(Box::new(Ts43Error::AuthenticationFailed(
            "EAP-AKA relay exhausted".to_string(),
        )))
    }

    fn post_relay(
        &self,
        packet: &str,
        req: &ServiceEntitlementRequest,
        cookies: &[String],
    ) -> Result<Ts43HttpResponse, Box<dyn Error + Send + Sync>> {
        let json_data = serde_json::json!({
            "eap-relay-packet": packet
        });
        let headers = vec![
            (
                "Content-Type".to_string(),
                Ts43Constants::EAP_RELAY_CONTENT_TYPE.to_string(),
            ),
            ("Accept".to_string(), self.relay_response_accept(req)),
            (
                "User-Agent".to_string(),
                user_agent(
                    req,
                    &first_non_empty(&[&req.client_ts43, &self.client_ts43()]),
                ),
            ),
        ];

        let http_req = Ts43HttpRequest {
            method: Ts43Method::Post,
            url: self.entitlement.server.clone(),
            headers,
            body: serde_json::to_vec(&json_data).unwrap_or_default(),
            cookies: cookies.to_vec(),
        };

        self.transport.do_raw(&http_req)
    }

    fn relay_response_accept(&self, req: &ServiceEntitlementRequest) -> String {
        if req.app == Ts43Constants::APP_VOWIFI
            || req.client_ts43 == Ts43Constants::DEFAULT_WFC_CLIENT_TS43
        {
            first_non_empty(&[&req.accept, Ts43Constants::WFC_RESPONSE_ACCEPT])
        } else {
            eap_relay_accept(&req.accept)
        }
    }

    // ---- Helpers ----

    fn base_request(
        &self,
        device: &Device,
        auth_token: &str,
        temp_token: &str,
    ) -> ServiceEntitlementRequest {
        ServiceEntitlementRequest {
            entitlement_version: first_non_empty(&[&self.entitlement.version, "9.0"]),
            request_version: if !self.entitlement.request_version.is_empty() {
                self.entitlement.request_version.clone()
            } else if self.entitlement.channel == EntitlementChannel::Samsung {
                "1".to_string()
            } else {
                "0".to_string()
            },
            authentication_token: auth_token.to_string(),
            temporary_token: temp_token.to_string(),
            app: if !self.entitlement.app.is_empty() {
                self.entitlement.app.clone()
            } else {
                Ts43Constants::APP_ODSA_PRIMARY.to_string()
            },
            terminal_id: device.imei.clone(),
            terminal_vendor: device.vendor.clone(),
            terminal_model: device.model.clone(),
            terminal_software: device.software_version.clone(),
            app_name: if !self.entitlement.app_name.is_empty() {
                self.entitlement.app_name.clone()
            } else if self.entitlement.channel == EntitlementChannel::Samsung {
                Ts43Constants::APP_ODSA_PRIMARY.to_string()
            } else {
                "androidTransfer".to_string()
            },
            app_version: self.app_version.clone(),
            provisioning_version: self.entitlement.provisioning_version.clone(),
            accept: if self.entitlement.channel == EntitlementChannel::Samsung {
                String::new()
            } else {
                Ts43Constants::DEFAULT_RESPONSE_ACCEPT.to_string()
            },
            ..Default::default()
        }
    }

    fn auth_request(&self, device: &Device, token: &AuthToken) -> ServiceEntitlementRequest {
        let mut req = self.base_request(device, &token.value, "");
        req.cookies = token.cookies.clone();
        req
    }

    fn transfer_request(
        &self,
        device: &Device,
        credential: &TransferCredential,
    ) -> ServiceEntitlementRequest {
        if credential.is_temporary_token {
            let mut req = self.base_request(device, "", &credential.temporary.value);
            req.cookies = credential.temporary.cookies.clone();
            req
        } else {
            self.auth_request(device, &credential.auth)
        }
    }

    fn add_optional_operation_fields(&self, op: &mut OdsaOperation) {
        let c = &self.companion;
        op.companion_terminal_id = c.imei.clone();
        op.companion_terminal_vendor = c.vendor.clone();
        op.companion_terminal_model = c.model.clone();
        op.companion_terminal_software = c.software_version.clone();
        op.companion_terminal_friendly_name = c.friendly_name.clone();
        op.companion_terminal_service = c.service.clone();
        op.companion_terminal_iccid = c.iccid.clone();
        op.companion_terminal_eid = c.eid.clone();
    }

    fn client_ts43(&self) -> String {
        if !self.client_ts43_override.is_empty() {
            return self.client_ts43_override.clone();
        }
        if self.entitlement.channel == EntitlementChannel::Samsung {
            Ts43Constants::SAMSUNG_ODSA_CLIENT_TS43.to_string()
        } else {
            Ts43Constants::DEFAULT_ODSA_CLIENT_TS43.to_string()
        }
    }

    fn realm(&self) -> String {
        if !self.realm_override.is_empty() {
            self.realm_override.clone()
        } else {
            Ts43Constants::DEFAULT_EAP_AKA_REALM.to_string()
        }
    }

    fn transfer_credential(
        &mut self,
        offer: &TransferOffer,
    ) -> Result<TransferCredential, Box<dyn Error + Send + Sync>> {
        if self.uses_temporary_token() {
            if !offer.temporary_token.value.is_empty() {
                return Ok(TransferCredential::of_temporary(
                    offer.temporary_token.clone(),
                ));
            }
            let mut off = offer.clone();
            return self.refresh_transfer_credential(&mut off);
        }
        if !offer.auth_token.value.is_empty() {
            return Ok(TransferCredential::of_auth(offer.auth_token.clone()));
        }
        let mut off = offer.clone();
        self.refresh_transfer_credential(&mut off)
    }

    fn refresh_transfer_credential(
        &mut self,
        offer: &mut TransferOffer,
    ) -> Result<TransferCredential, Box<dyn Error + Send + Sync>> {
        let tokens = self.fetch_transfer_token(
            &offer.source_identity,
            &offer.source_device,
            &offer.target_device,
        )?;
        offer.auth_token = tokens.auth.clone();
        offer.temporary_token = tokens.temporary.clone();
        if self.uses_temporary_token() {
            Ok(TransferCredential::of_temporary(tokens.temporary))
        } else {
            Ok(TransferCredential::of_auth(tokens.auth))
        }
    }

    fn apply_delayed_download_state(
        &mut self,
        result: &mut Ts43Result,
        offer: &TransferOffer,
        subscription_result: i32,
    ) {
        let source_iccid =
            first_non_empty(&[&offer.source_device.iccid, &offer.source_identity.iccid]);
        if self.entitlement.use_smds_delayed_download {
            result.state = Ts43State::PendingSmdsDiscovery;
            result.event = Some(Ts43Event::SmdsDiscovery {
                source_iccid,
                target_eid: offer.target_device.eid.clone(),
                target_imei: offer.target_device.imei.clone(),
                subscription_result,
            });
        } else {
            result.state = Ts43State::PendingDelayedDownload;
            result.event = Some(Ts43Event::DelayedDownload {
                source_iccid,
                target_eid: offer.target_device.eid.clone(),
                target_imei: offer.target_device.imei.clone(),
                subscription_result,
            });
        }
    }

    fn apply_download_configuration(
        &mut self,
        result: &mut Ts43Result,
        mut cfg: DownloadConfig,
    ) -> Result<Ts43Result, Box<dyn Error + Send + Sync>> {
        let session = require_session(result)?;
        cfg = complete_download_config(cfg, &session.offer.target_device);
        if cfg.activation_code.is_empty() && cfg.smdp_fqdn.is_empty() {
            return Err(Box::new(Ts43Error::MalformedPayload(
                "download info is missing".to_string(),
            )));
        }
        result.download_config = cfg;
        result.state = Ts43State::PendingDownload;
        let ev = Ts43Event::DownloadReady(result.download_config.clone());
        result.event = Some(ev.clone());
        self.emit(ev);
        Ok(result.clone())
    }
}

fn require_session(
    result: &mut Ts43Result,
) -> Result<&mut TransferSession, Box<dyn Error + Send + Sync>> {
    result.session.as_mut().ok_or_else(|| {
        Box::new(Ts43Error::MalformedPayload(
            "transfer session is required".to_string(),
        )) as Box<dyn Error + Send + Sync>
    })
}

fn selected_transfer_type(e: &Entitlement) -> i32 {
    if e.transfer_type != TransferType::DISABLED || e.source_sim_type != SimType::Psim {
        return e.transfer_type;
    }
    if e.use_temp_token_conversion {
        TransferType::TEMPORARY_TOKEN_OLD_TERMINAL_IDS
    } else {
        TransferType::OLD_TERMINAL_IDS
    }
}

fn source_profile_deletion_required(r: &OdsaResponse) -> bool {
    r.operation_result == OperationResult::SUCCESS
        && r.subscription_result == SubscriptionResult::DELETE_PROFILE_IN_USE
}

fn user_input_required(r: &OdsaResponse) -> bool {
    r.operation_result == OperationResult::SUCCESS
        && r.subscription_result == SubscriptionResult::REQUIRES_USER_INPUT
}

fn websheet_required(r: &OdsaResponse) -> bool {
    r.operation_result == OperationResult::SUCCESS
        && r.subscription_result == SubscriptionResult::CONTINUE_TO_WEBSHEET
}

fn download_profile_ready(r: &OdsaResponse) -> bool {
    if r.operation_result != OperationResult::SUCCESS
        || r.subscription_result != SubscriptionResult::DOWNLOAD_PROFILE
    {
        return false;
    }
    !r.download.activation_code.is_empty() || !r.download.smdp_fqdn.is_empty()
}

fn subscription_done(r: &OdsaResponse) -> bool {
    r.operation_result == OperationResult::SUCCESS
        && r.subscription_result == SubscriptionResult::DONE
}

fn subscription_dismissed(r: &OdsaResponse) -> bool {
    r.operation_result == OperationResult::SUCCESS
        && r.subscription_result == SubscriptionResult::DISMISS
}

fn delayed_download_required(r: &OdsaResponse) -> bool {
    r.operation_result == OperationResult::SUCCESS
        && r.subscription_result == SubscriptionResult::DELAYED_DOWNLOAD
}

fn complete_download_config(mut cfg: DownloadConfig, target: &Device) -> DownloadConfig {
    if cfg.imei.is_empty() {
        cfg.imei = target.imei.clone();
    }
    cfg
}

fn manage_subscription_accepted(r: &OdsaResponse) -> bool {
    if r.operation_result != OperationResult::SUCCESS {
        return false;
    }
    matches!(
        r.subscription_result,
        SubscriptionResult::DOWNLOAD_PROFILE
            | SubscriptionResult::DELAYED_DOWNLOAD
            | SubscriptionResult::DONE
            | SubscriptionResult::DISMISS
            | SubscriptionResult::DELETE_PROFILE_IN_USE
            | SubscriptionResult::CONTINUE_TO_WEBSHEET
            | SubscriptionResult::REQUIRES_USER_INPUT
    )
}

fn subscription_update_accepted(r: &OdsaResponse) -> bool {
    if r.operation_result != OperationResult::SUCCESS {
        return false;
    }
    matches!(
        r.subscription_result,
        SubscriptionResult::DONE
            | SubscriptionResult::DELAYED_DOWNLOAD
            | SubscriptionResult::DISMISS
    )
}

fn is_http_511(err: &(dyn Error + 'static)) -> bool {
    if let Some(http_err) = err.downcast_ref::<Ts43HttpError>() {
        http_err.status_code == 511
    } else {
        false
    }
}

fn relay_packet(data: &[u8]) -> String {
    let Ok(s) = std::str::from_utf8(data) else {
        return String::new();
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(s) else {
        return String::new();
    };
    val.get("eap-relay-packet")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn auth_token_from_response(
    current: &AuthToken,
    resp: &OdsaResponse,
    cookies: &[String],
) -> AuthToken {
    let mut next = AuthToken {
        value: current.value.clone(),
        expires_at: current.expires_at,
        cookies: cookies.to_vec(),
    };
    if !resp.authentication_token.is_empty() {
        next.value = resp.authentication_token.clone();
    }
    next
}
