//! GSMA SGP.22 ES9+ and ES10 interaction flow for eSIM Profile Download.
//!
//! Manages protocol session state across eUICC and SM-DP+ exchanges:
//! 1. `EUICCInfo1` (Tag `0xBF20`) and `EuiccChallenge` (Tag `0xBF2E`) acquisition.
//! 2. `InitiateAuthentication` ES9+ request payload formatting.
//! 3. `transactionId` octet normalization and exact hex preservation.
//! 4. SGP.22 `HashCc` computation: `SHA256(SHA256(CC) || TransactionID)`.
//! 5. `AuthenticateServer` ES10 APDU handling and `AuthenticateClient` ES9+ response validation.

use std::fmt;

use crate::crypto::compute_hash_cc;
use crate::hex::hex_to_bytes;

/// Errors arising during profile download session operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadSessionError {
    MissingEuiccInfo1,
    MissingEuiccChallenge,
    MissingTransactionId,
    MissingServerCertificate,
    MissingAuthenticateServerResponse,
    MissingConfirmationCode,
    InvalidTransactionIdHex(String),
    InvalidTransactionIdLength(usize),
}

impl fmt::Display for DownloadSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEuiccInfo1 => write!(f, "Must process EUICCInfo1 before proceeding"),
            Self::MissingEuiccChallenge => {
                write!(f, "Must process EuiccChallenge before proceeding")
            }
            Self::MissingTransactionId => write!(f, "Transaction ID has not been provided"),
            Self::MissingServerCertificate => write!(f, "Server certificate is missing"),
            Self::MissingAuthenticateServerResponse => {
                write!(f, "Must process AuthenticateServerResponse first")
            }
            Self::MissingConfirmationCode => write!(f, "Confirmation code is required but missing"),
            Self::InvalidTransactionIdHex(err) => {
                write!(f, "Failed to decode transaction ID hex: {err}")
            }
            Self::InvalidTransactionIdLength(len) => {
                write!(
                    f,
                    "Transaction ID is {len} bytes (SGP.22 requires 1..=16 octets)"
                )
            }
        }
    }
}

impl std::error::Error for DownloadSessionError {}

/// Manages the state and flow of a GSMA SGP.22 eSIM profile download session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileDownloadSession {
    smdp_address: String,
    matching_id: Option<String>,
    confirmation_code: Option<String>,

    euicc_info1_bytes: Option<Vec<u8>>,
    euicc_challenge: Option<Vec<u8>>,

    transaction_id_hex: Option<String>,
    transaction_id_octets: Option<Vec<u8>>,
    server_certificate: Option<Vec<u8>>,

    authenticate_server_response: Option<Vec<u8>>,
    authenticate_client_status: Option<String>,

    smdp_signed2: Option<Vec<u8>>,
    smdp_signature2: Option<Vec<u8>>,
    profile_metadata: Option<Vec<u8>>,
    smdp_certificate: Option<Vec<u8>>,
}

impl ProfileDownloadSession {
    /// Creates a new profile download session targeting a given SM-DP+ server address.
    #[must_use]
    pub fn new(
        smdp_address: impl Into<String>,
        matching_id: Option<String>,
        confirmation_code: Option<String>,
    ) -> Self {
        Self {
            smdp_address: smdp_address.into(),
            matching_id,
            confirmation_code,
            euicc_info1_bytes: None,
            euicc_challenge: None,
            transaction_id_hex: None,
            transaction_id_octets: None,
            server_certificate: None,
            authenticate_server_response: None,
            authenticate_client_status: None,
            smdp_signed2: None,
            smdp_signature2: None,
            profile_metadata: None,
            smdp_certificate: None,
        }
    }

    #[must_use]
    pub fn smdp_address(&self) -> &str {
        &self.smdp_address
    }

    #[must_use]
    pub fn matching_id(&self) -> Option<&str> {
        self.matching_id.as_deref()
    }

    #[must_use]
    pub fn confirmation_code(&self) -> Option<&str> {
        self.confirmation_code.as_deref()
    }

    pub fn set_confirmation_code(&mut self, code: impl Into<String>) {
        self.confirmation_code = Some(code.into());
    }

    /// Step 1: Ingests raw DER bytes for `EUICCInfo1` (Tag `0xBF20`).
    pub fn process_euicc_info1(&mut self, bytes: &[u8]) {
        self.euicc_info1_bytes = Some(bytes.to_vec());
    }

    #[must_use]
    pub fn euicc_info1(&self) -> Option<&[u8]> {
        self.euicc_info1_bytes.as_deref()
    }

    /// Step 2: Ingests `EuiccChallenge` (Tag `0xBF2E`) from the card.
    pub fn process_euicc_challenge(&mut self, bytes: &[u8]) {
        self.euicc_challenge = Some(bytes.to_vec());
    }

    #[must_use]
    pub fn euicc_challenge(&self) -> Option<&[u8]> {
        self.euicc_challenge.as_deref()
    }

    /// Validates that prerequisites for `InitiateAuthentication` are present.
    pub fn validate_initiate_authentication_prerequisites(
        &self,
    ) -> Result<(), DownloadSessionError> {
        if self.euicc_info1_bytes.is_none() {
            return Err(DownloadSessionError::MissingEuiccInfo1);
        }
        if self.euicc_challenge.is_none() {
            return Err(DownloadSessionError::MissingEuiccChallenge);
        }
        Ok(())
    }

    /// Step 4: Processes `InitiateAuthenticationResponse` from ES9+.
    ///
    /// `transaction_id_hex` is the hex text sent by the SM-DP+. It is decoded
    /// into octets per SGP.22 (`TransactionId ::= OCTET STRING (SIZE(1..16))`)
    /// so downstream cryptographic calculations (like `HashCc`) hash the exact
    /// octets, while the verbatim hex spelling is kept for ES9+ JSON bodies.
    pub fn process_initiate_authentication_response(
        &mut self,
        transaction_id_hex: &str,
        server_certificate: &[u8],
    ) -> Result<(), DownloadSessionError> {
        let octets = hex_to_bytes(transaction_id_hex)
            .map_err(|e| DownloadSessionError::InvalidTransactionIdHex(e.to_string()))?;

        if octets.is_empty() || octets.len() > 16 {
            return Err(DownloadSessionError::InvalidTransactionIdLength(
                octets.len(),
            ));
        }

        self.transaction_id_hex = Some(transaction_id_hex.to_string());
        self.transaction_id_octets = Some(octets);
        self.server_certificate = Some(server_certificate.to_vec());
        Ok(())
    }

    /// The transaction ID as raw octets, required for `HashCc` and ASN.1 structures.
    #[must_use]
    pub fn get_transaction_id(&self) -> Option<&[u8]> {
        self.transaction_id_octets.as_deref()
    }

    /// The transaction ID as the SM-DP+ server spelled it, for ES9+ JSON payloads.
    #[must_use]
    pub fn get_transaction_id_hex(&self) -> Option<&str> {
        self.transaction_id_hex.as_deref()
    }

    /// Computes the GSMA SGP.22 Confirmation Code hash:
    /// `HashCc = SHA256(SHA256(CC) || TransactionID)`
    pub fn compute_hash_cc(&self) -> Result<[u8; 32], DownloadSessionError> {
        let cc = self
            .confirmation_code
            .as_deref()
            .ok_or(DownloadSessionError::MissingConfirmationCode)?;
        let tx_id = self
            .transaction_id_octets
            .as_deref()
            .ok_or(DownloadSessionError::MissingTransactionId)?;
        Ok(compute_hash_cc(cc, tx_id))
    }

    /// Step 6: Processes `AuthenticateServerResponse` (Tag `0xBF38`) from the eUICC.
    pub fn process_authenticate_server_response(&mut self, response_bytes: &[u8]) {
        self.authenticate_server_response = Some(response_bytes.to_vec());
    }

    #[must_use]
    pub fn authenticate_server_response(&self) -> Option<&[u8]> {
        self.authenticate_server_response.as_deref()
    }

    /// Step 8: Processes `AuthenticateClientResponse` from ES9+.
    pub fn process_authenticate_client_response(
        &mut self,
        function_execution_status: impl Into<String>,
        smdp_signed2: Option<&[u8]>,
        smdp_signature2: Option<&[u8]>,
        profile_metadata: Option<&[u8]>,
        smdp_certificate: Option<&[u8]>,
    ) {
        self.authenticate_client_status = Some(function_execution_status.into());
        self.smdp_signed2 = smdp_signed2.map(|b| b.to_vec());
        self.smdp_signature2 = smdp_signature2.map(|b| b.to_vec());
        self.profile_metadata = profile_metadata.map(|b| b.to_vec());
        self.smdp_certificate = smdp_certificate.map(|b| b.to_vec());
    }

    /// Returns `true` if the client was authenticated successfully by the SM-DP+.
    #[must_use]
    pub fn is_client_authenticated_successfully(&self) -> bool {
        self.authenticate_client_status.as_deref() == Some("Executed-Success")
            && self.smdp_signed2.is_some()
    }

    #[must_use]
    pub fn get_profile_metadata(&self) -> Option<&[u8]> {
        self.profile_metadata.as_deref()
    }

    #[must_use]
    pub fn get_smdp_signed2(&self) -> Option<&[u8]> {
        self.smdp_signed2.as_deref()
    }

    #[must_use]
    pub fn get_smdp_signature2(&self) -> Option<&[u8]> {
        self.smdp_signature2.as_deref()
    }

    #[must_use]
    pub fn authenticate_client_status(&self) -> Option<&str> {
        self.authenticate_client_status.as_deref()
    }

    #[must_use]
    pub fn smdp_certificate(&self) -> Option<&[u8]> {
        self.smdp_certificate.as_deref()
    }

    #[must_use]
    pub fn server_certificate(&self) -> Option<&[u8]> {
        self.server_certificate.as_deref()
    }

    #[must_use]
    pub fn get_server_certificate(&self) -> Option<&[u8]> {
        self.smdp_certificate
            .as_deref()
            .or(self.server_certificate.as_deref())
    }
}
