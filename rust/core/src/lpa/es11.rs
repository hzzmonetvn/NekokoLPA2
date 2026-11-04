//! GSMA SGP.22 ES11 SM-DS (Root and Cascaded Discovery Server) protocol models,
//! request generators, event entry parsers, and activation code encoders.
//!
//! See SGP.22 v2.2.x §5.8.2 (ES11.AuthenticateClient and event discovery).

use crate::es9::{
    build_authenticate_client_body, build_initiate_authentication_body, parse_json,
    verify_es9_header, Es9Error, JsonValue,
};

/// The default GSMA Root SM-DS server FQDN used when an eUICC does not supply
/// an operator-configured discovery server address.
pub const ROOT_SMDS_ADDRESS: &str = "lpa.ds.gsma.com";

/// A profile-availability event discovered from an SM-DS (ES11), referencing
/// the remote SM-DP+ that holds the pending profile order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiscoveredEvent {
    /// The matching ID or transaction identifier for retrieving the profile.
    pub event_id: String,
    /// The SM-DP+ FQDN that holds the profile.
    pub address: String,
}

impl DiscoveredEvent {
    /// Creates a new [`DiscoveredEvent`] with the address normalized to a clean host FQDN.
    #[must_use]
    pub fn new(event_id: impl Into<String>, address: impl AsRef<str>) -> Self {
        Self {
            event_id: event_id.into(),
            address: clean_smds_host(address.as_ref()),
        }
    }

    /// Formats the GSMA LPA activation code corresponding to this discovered event.
    ///
    /// Form: `LPA:1$<address>$<eventId>`
    #[must_use]
    pub fn activation_code(&self) -> String {
        format!("LPA:1${}${}", self.address, self.event_id)
    }
}

/// Normalizes a raw SM-DS or SM-DP+ URL/host string into an FQDN hostname.
///
/// Strips schemes (`http://`, `https://`), userinfo, paths, queries, fragments,
/// and trailing port designations.
#[must_use]
pub fn clean_smds_host(raw_address: &str) -> String {
    let mut s = raw_address.trim();
    if let Some(stripped) = s.strip_prefix("https://") {
        s = stripped;
    } else if let Some(stripped) = s.strip_prefix("http://") {
        s = stripped;
    }

    // Strip path, query or fragment
    if let Some((host_part, _)) = s.split_once('/') {
        s = host_part;
    }
    if let Some((host_part, _)) = s.split_once('?') {
        s = host_part;
    }
    if let Some((host_part, _)) = s.split_once('#') {
        s = host_part;
    }

    // Strip port if present
    if let Some((host_part, _)) = s.split_once(':') {
        s = host_part;
    }

    s.trim().to_ascii_lowercase()
}

/// Builds an ES11 `InitiateAuthentication` JSON request body for SM-DS discovery.
#[must_use]
pub fn build_smds_initiate_auth_request(
    smds_address: &str,
    euicc_challenge_b64: &str,
    euicc_info1_b64: &str,
) -> String {
    let clean_host = clean_smds_host(smds_address);
    build_initiate_authentication_body(&clean_host, euicc_challenge_b64, euicc_info1_b64)
}

/// Builds an ES11 `AuthenticateClient` JSON request body for SM-DS discovery.
#[must_use]
pub fn build_smds_auth_client_request(
    transaction_id: &str,
    authenticate_server_response_b64: &str,
) -> String {
    build_authenticate_client_body(transaction_id, authenticate_server_response_b64)
}

/// Parses the list of [`DiscoveredEvent`] entries from an SM-DS `AuthenticateClient`
/// JSON response tree.
///
/// # Errors
/// Returns [`Es9Error::MissingField`] if the `eventEntries` array is absent or malformed.
pub fn parse_smds_event_entries(root: &JsonValue) -> Result<Vec<DiscoveredEvent>, Es9Error> {
    let entries = root
        .get("eventEntries")
        .and_then(|v| match v {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        })
        .ok_or(Es9Error::MissingField("eventEntries"))?;

    let mut events = Vec::new();
    for entry in entries {
        if let Some(map) = entry.as_object() {
            let event_id = map
                .get("eventId")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .trim();
            let address = map
                .get("rspServerAddress")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .trim();

            if !event_id.is_empty() && !address.is_empty() {
                events.push(DiscoveredEvent::new(event_id, address));
            }
        }
    }

    Ok(events)
}

/// Parses and validates a complete ES11 `AuthenticateClient` JSON response string,
/// returning the list of discovered pending profile events.
///
/// # Errors
/// Returns [`Es9Error`] if status check fails, JSON is invalid, or `eventEntries` is missing.
pub fn parse_smds_discovery_response(
    json_str: &str,
    http_status: Option<u16>,
) -> Result<Vec<DiscoveredEvent>, Es9Error> {
    let root = parse_json(json_str)?;
    verify_es9_header(&root, http_status)?;
    parse_smds_event_entries(&root)
}
