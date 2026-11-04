//! TS.43 string constants.
//!
//! Ported from `types.go` / `constants.dart`.

pub struct Ts43Constants;

impl Ts43Constants {
    pub const APP_ODSA_PRIMARY: &'static str = "ap2009";
    pub const APP_VOWIFI: &'static str = "ap2004";
    pub const DEFAULT_RESPONSE_ACCEPT: &'static str =
        "application/json, text/vnd.wap.connectivity-xml";
    pub const WFC_RESPONSE_ACCEPT: &'static str = "text/vnd.wap.connectivity-xml";
    pub const SAMSUNG_RESPONSE_ACCEPT: &'static str = "application/xml, text/xml";
    pub const EAP_RELAY_CONTENT_TYPE: &'static str = "application/vnd.gsma.eap-relay.v1.0+json";
    pub const DEFAULT_ODSA_CLIENT_TS43: &'static str = "client-Primary-ODSA";
    pub const SAMSUNG_ODSA_CLIENT_TS43: &'static str = "Primary-ODSA";
    pub const DEFAULT_WFC_CLIENT_TS43: &'static str = "client-IMS-Entitlement";
    pub const DEFAULT_EAP_AKA_REALM: &'static str = "nai.epc";
}
