//! GlobalPlatform Secure Channel Protocol 03 (SCP03) mutual authentication and APDU wrapping.
//!
//! Implemented per GlobalPlatform Card Specification v2.3 Amendment D for
//! Issuer Security Domain - Profile (ISD-P) and eUICC secure channel operations.
//!
//! Provides:
//! - NIST SP 800-108 counter mode session key derivation using AES-CMAC (S-ENC, S-MAC, S-RMAC)
//! - Card and Host Cryptogram generation and verification
//! - `INITIALIZE UPDATE` command construction and response parsing
//! - `EXTERNAL AUTHENTICATE` command construction with C-MAC chaining
//! - APDU command payload encryption (C-ENC) and MAC computation (C-MAC)

use std::fmt;

use crate::crypto::{aes_cbc_encrypt, aes_cmac, aes_cmac_8, AesCipher, CryptoError};

/// Secure channel security levels per GlobalPlatform Card Specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecurityLevel {
    /// No security (plain APDU transport)
    NoSecurity = 0x00,
    /// Command integrity only (C-MAC)
    #[default]
    CMac = 0x01,
    /// Command integrity and payload encryption (C-MAC + C-DEC)
    CDecCMac = 0x03,
    /// Command integrity, payload encryption, and response integrity (C-MAC + C-DEC + R-MAC)
    CDecCMacRMac = 0x33,
}

impl SecurityLevel {
    /// Parses security level from a byte.
    pub fn from_u8(val: u8) -> Result<Self, Scp03Error> {
        match val {
            0x00 => Ok(Self::NoSecurity),
            0x01 => Ok(Self::CMac),
            0x03 => Ok(Self::CDecCMac),
            0x33 => Ok(Self::CDecCMacRMac),
            other => Err(Scp03Error::InvalidSecurityLevel(other)),
        }
    }

    /// Returns `true` if command integrity (C-MAC) is required.
    #[must_use]
    pub fn has_c_mac(self) -> bool {
        matches!(self, Self::CMac | Self::CDecCMac | Self::CDecCMacRMac)
    }

    /// Returns `true` if command payload encryption (C-DEC) is required.
    #[must_use]
    pub fn has_c_dec(self) -> bool {
        matches!(self, Self::CDecCMac | Self::CDecCMacRMac)
    }

    /// Returns `true` if response integrity (R-MAC) is required.
    #[must_use]
    pub fn has_r_mac(self) -> bool {
        matches!(self, Self::CDecCMacRMac)
    }
}

/// Errors occurring during SCP03 operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scp03Error {
    InvalidKeyLength(usize),
    InvalidChallengeLength(usize),
    InvalidResponseLength(usize),
    CardCryptogramMismatch { expected: [u8; 8], actual: [u8; 8] },
    InvalidState(String),
    Crypto(CryptoError),
    InvalidSecurityLevel(u8),
    InvalidPadding,
    ApduTooLong(usize),
}

impl fmt::Display for Scp03Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKeyLength(len) => {
                write!(
                    f,
                    "Invalid SCP03 key length: {len} bytes (expected 16 or 32)"
                )
            }
            Self::InvalidChallengeLength(len) => {
                write!(f, "Invalid challenge length: {len} bytes (expected 8)")
            }
            Self::InvalidResponseLength(len) => {
                write!(
                    f,
                    "Invalid INITIALIZE UPDATE response length: {len} bytes (expected >= 29)"
                )
            }
            Self::CardCryptogramMismatch { expected, actual } => {
                write!(
                    f,
                    "Card cryptogram mismatch: expected {expected:02X?}, got {actual:02X?}"
                )
            }
            Self::InvalidState(msg) => write!(f, "Invalid SCP03 session state: {msg}"),
            Self::Crypto(err) => write!(f, "Cryptographic operation failed: {err}"),
            Self::InvalidSecurityLevel(lvl) => write!(f, "Unsupported security level: 0x{lvl:02X}"),
            Self::InvalidPadding => write!(f, "Invalid APDU padding"),
            Self::ApduTooLong(len) => write!(f, "APDU payload exceeds 255 bytes: {len}"),
        }
    }
}

impl std::error::Error for Scp03Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Crypto(err) => Some(err),
            _ => None,
        }
    }
}

impl From<CryptoError> for Scp03Error {
    fn from(err: CryptoError) -> Self {
        Self::Crypto(err)
    }
}

/// Static SCP03 key set configured on the card or ISD-P.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scp03KeySet {
    pub k_enc: Vec<u8>,
    pub k_mac: Vec<u8>,
    pub key_version: u8,
    pub key_id: u8,
}

impl Scp03KeySet {
    /// Creates a new SCP03 key set from 16-byte (AES-128) or 32-byte (AES-256) keys.
    pub fn new(
        k_enc: &[u8],
        k_mac: &[u8],
        key_version: u8,
        key_id: u8,
    ) -> Result<Self, Scp03Error> {
        if k_enc.len() != 16 && k_enc.len() != 32 {
            return Err(Scp03Error::InvalidKeyLength(k_enc.len()));
        }
        if k_mac.len() != k_enc.len() {
            return Err(Scp03Error::InvalidKeyLength(k_mac.len()));
        }
        Ok(Self {
            k_enc: k_enc.to_vec(),
            k_mac: k_mac.to_vec(),
            key_version,
            key_id,
        })
    }
}

/// Derived session keys established during SCP03 mutual authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scp03SessionKeys {
    pub s_enc: Vec<u8>,
    pub s_mac: Vec<u8>,
    pub s_rmac: Vec<u8>,
}

/// Derives an SCP03 session key using NIST SP 800-108 counter mode with AES-CMAC.
///
/// Constants per GlobalPlatform Card Specification v2.3 Amendment D:
/// - `0x04`: S-ENC
/// - `0x06`: S-MAC
/// - `0x07`: S-RMAC
pub fn derive_session_key(
    static_key: &[u8],
    constant: u8,
    host_challenge: &[u8; 8],
    card_challenge: &[u8; 8],
) -> Result<Vec<u8>, Scp03Error> {
    let key_len_bits: u16 = (static_key.len() as u16) * 8;
    let l_bytes = key_len_bits.to_be_bytes();

    // Block 1 (Counter 0x01)
    let mut derivation_data = [0u8; 32];
    derivation_data[11] = constant;
    derivation_data[12] = 0x00;
    derivation_data[13] = l_bytes[0];
    derivation_data[14] = l_bytes[1];
    derivation_data[15] = 0x01; // Counter 1
    derivation_data[16..24].copy_from_slice(host_challenge);
    derivation_data[24..32].copy_from_slice(card_challenge);

    let part1 = aes_cmac(static_key, &derivation_data)?;

    if static_key.len() == 16 {
        return Ok(part1.to_vec());
    }

    // Block 2 (Counter 0x02) for 256-bit keys
    derivation_data[15] = 0x02;
    let part2 = aes_cmac(static_key, &derivation_data)?;

    let mut full = Vec::with_capacity(32);
    full.extend_from_slice(&part1);
    full.extend_from_slice(&part2);
    Ok(full)
}

/// Computes the 8-byte Card Cryptogram per GlobalPlatform SCP03.
pub fn compute_card_cryptogram(
    s_mac: &[u8],
    host_challenge: &[u8; 8],
    card_challenge: &[u8; 8],
) -> Result<[u8; 8], Scp03Error> {
    let mut data = [0u8; 32];
    data[11] = 0x00; // Constant 0x00 for Card Cryptogram
    data[12] = 0x00;
    data[13] = 0x00;
    data[14] = 0x40; // 64 bits length
    data[15] = 0x01; // Counter 1
    data[16..24].copy_from_slice(host_challenge);
    data[24..32].copy_from_slice(card_challenge);

    Ok(aes_cmac_8(s_mac, &data)?)
}

/// Computes the 8-byte Host Cryptogram per GlobalPlatform SCP03.
pub fn compute_host_cryptogram(
    s_mac: &[u8],
    host_challenge: &[u8; 8],
    card_challenge: &[u8; 8],
) -> Result<[u8; 8], Scp03Error> {
    let mut data = [0u8; 32];
    data[11] = 0x01; // Constant 0x01 for Host Cryptogram
    data[12] = 0x00;
    data[13] = 0x00;
    data[14] = 0x40; // 64 bits length
    data[15] = 0x01; // Counter 1
    data[16..24].copy_from_slice(host_challenge);
    data[24..32].copy_from_slice(card_challenge);

    Ok(aes_cmac_8(s_mac, &data)?)
}

/// Parsed response from `INITIALIZE UPDATE` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializeUpdateResponse {
    pub key_diversification_data: [u8; 10],
    pub key_information: [u8; 3],
    pub card_challenge: [u8; 8],
    pub card_cryptogram: [u8; 8],
    pub sequence_counter: Option<[u8; 3]>,
}

impl InitializeUpdateResponse {
    /// Parses from the raw APDU response data (excluding 0x9000 status word).
    pub fn parse(data: &[u8]) -> Result<Self, Scp03Error> {
        if data.len() < 29 {
            return Err(Scp03Error::InvalidResponseLength(data.len()));
        }
        let mut key_diversification_data = [0u8; 10];
        key_diversification_data.copy_from_slice(&data[0..10]);

        let mut key_information = [0u8; 3];
        key_information.copy_from_slice(&data[10..13]);

        let mut card_challenge = [0u8; 8];
        card_challenge.copy_from_slice(&data[13..21]);

        let mut card_cryptogram = [0u8; 8];
        card_cryptogram.copy_from_slice(&data[21..29]);

        let sequence_counter = if data.len() >= 32 {
            let mut sc = [0u8; 3];
            sc.copy_from_slice(&data[29..32]);
            Some(sc)
        } else {
            None
        };

        Ok(Self {
            key_diversification_data,
            key_information,
            card_challenge,
            card_cryptogram,
            sequence_counter,
        })
    }
}

/// Lifecycle states of an SCP03 channel session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionState {
    #[default]
    Uninitialized,
    UpdateInitialized,
    Authenticated,
    Terminated,
}

/// State machine managing GlobalPlatform SCP03 mutual authentication and APDU wrapping.
pub struct Scp03Session {
    state: SessionState,
    security_level: SecurityLevel,
    session_keys: Option<Scp03SessionKeys>,
    host_challenge: [u8; 8],
    card_challenge: [u8; 8],
    mac_chaining_value: [u8; 16],
    encryption_counter: u64,
}

impl Default for Scp03Session {
    fn default() -> Self {
        Self::new(SecurityLevel::CMac)
    }
}

impl Scp03Session {
    /// Creates a new SCP03 session with the desired security level.
    #[must_use]
    pub fn new(security_level: SecurityLevel) -> Self {
        Self {
            state: SessionState::Uninitialized,
            security_level,
            session_keys: None,
            host_challenge: [0u8; 8],
            card_challenge: [0u8; 8],
            mac_chaining_value: [0u8; 16],
            encryption_counter: 1,
        }
    }

    /// Returns the current lifecycle state of the session.
    #[must_use]
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Returns the active security level.
    #[must_use]
    pub fn security_level(&self) -> SecurityLevel {
        self.security_level
    }

    /// Builds the `INITIALIZE UPDATE` APDU command.
    ///
    /// APDU format: `80 50 <key_version> <key_id> 08 <host_challenge> 00`
    pub fn build_initialize_update(
        &mut self,
        host_challenge: [u8; 8],
        key_version: u8,
        key_id: u8,
    ) -> Vec<u8> {
        self.host_challenge = host_challenge;
        self.state = SessionState::UpdateInitialized;

        let mut apdu = Vec::with_capacity(16);
        apdu.extend_from_slice(&[0x80, 0x50, key_version, key_id, 0x08]);
        apdu.extend_from_slice(&host_challenge);
        apdu.push(0x00); // Le
        apdu
    }

    /// Processes `INITIALIZE UPDATE` response, derives session keys, validates Card Cryptogram,
    /// and returns the derived 8-byte Host Cryptogram.
    pub fn process_initialize_update_response(
        &mut self,
        response_bytes: &[u8],
        key_set: &Scp03KeySet,
    ) -> Result<[u8; 8], Scp03Error> {
        if self.state != SessionState::UpdateInitialized {
            return Err(Scp03Error::InvalidState(
                "Cannot process INITIALIZE UPDATE response when session is not in UpdateInitialized state"
                    .to_string(),
            ));
        }

        let resp = InitializeUpdateResponse::parse(response_bytes)?;
        self.card_challenge = resp.card_challenge;

        // 1. Derive S-ENC, S-MAC, S-RMAC
        let s_enc = derive_session_key(
            &key_set.k_enc,
            0x04,
            &self.host_challenge,
            &self.card_challenge,
        )?;
        let s_mac = derive_session_key(
            &key_set.k_mac,
            0x06,
            &self.host_challenge,
            &self.card_challenge,
        )?;
        let s_rmac = derive_session_key(
            &key_set.k_mac,
            0x07,
            &self.host_challenge,
            &self.card_challenge,
        )?;

        // 2. Validate Card Cryptogram
        let expected_card_cryptogram =
            compute_card_cryptogram(&s_mac, &self.host_challenge, &self.card_challenge)?;
        if expected_card_cryptogram != resp.card_cryptogram {
            self.state = SessionState::Terminated;
            return Err(Scp03Error::CardCryptogramMismatch {
                expected: expected_card_cryptogram,
                actual: resp.card_cryptogram,
            });
        }

        // 3. Compute Host Cryptogram
        let host_cryptogram =
            compute_host_cryptogram(&s_mac, &self.host_challenge, &self.card_challenge)?;

        self.session_keys = Some(Scp03SessionKeys {
            s_enc,
            s_mac,
            s_rmac,
        });

        Ok(host_cryptogram)
    }

    /// Builds the `EXTERNAL AUTHENTICATE` APDU command with Host Cryptogram and initial C-MAC.
    ///
    /// APDU format: `84 82 <security_level> 00 10 <host_cryptogram> <c_mac>`
    pub fn build_external_authenticate(
        &mut self,
        host_cryptogram: [u8; 8],
    ) -> Result<Vec<u8>, Scp03Error> {
        let keys = self
            .session_keys
            .as_ref()
            .ok_or_else(|| Scp03Error::InvalidState("Session keys not established".to_string()))?;

        // Format command header: CLA=0x84, INS=0x82, P1=security_level, P2=0x00, Lc=0x10 (16 bytes)
        let cla = 0x84;
        let ins = 0x82;
        let p1 = self.security_level as u8;
        let p2 = 0x00;
        let lc = 0x10;

        // Input for initial C-MAC: chaining_value (16 zeros) || header (5 bytes) || data (8 bytes host_cryptogram)
        let mut cmac_input = Vec::with_capacity(16 + 5 + 8);
        cmac_input.extend_from_slice(&self.mac_chaining_value);
        cmac_input.extend_from_slice(&[cla, ins, p1, p2, lc]);
        cmac_input.extend_from_slice(&host_cryptogram);

        let mac16 = aes_cmac(&keys.s_mac, &cmac_input)?;
        self.mac_chaining_value = mac16;

        let mut apdu = Vec::with_capacity(5 + 16);
        apdu.extend_from_slice(&[cla, ins, p1, p2, lc]);
        apdu.extend_from_slice(&host_cryptogram);
        apdu.extend_from_slice(&mac16[..8]);

        self.state = SessionState::Authenticated;
        Ok(apdu)
    }

    /// Wraps a plain APDU command into an SCP03 protected APDU with C-MAC and optional C-ENC.
    pub fn wrap_apdu(
        &mut self,
        cla: u8,
        ins: u8,
        p1: u8,
        p2: u8,
        data: &[u8],
    ) -> Result<Vec<u8>, Scp03Error> {
        if self.state != SessionState::Authenticated {
            return Err(Scp03Error::InvalidState(
                "Cannot wrap APDU before successful authentication".to_string(),
            ));
        }

        let keys = self
            .session_keys
            .as_ref()
            .ok_or_else(|| Scp03Error::InvalidState("Session keys missing".to_string()))?;

        // 1. If payload encryption (C-DEC) is active, encrypt data using AES-CBC with S-ENC
        let encrypted_payload = if self.security_level.has_c_dec() && !data.is_empty() {
            // Compute Initial Chaining Vector (ICV): encrypt 16-byte counter with S-ENC
            let mut counter_block = [0u8; 16];
            counter_block[8..16].copy_from_slice(&self.encryption_counter.to_be_bytes());
            self.encryption_counter += 1;

            let cipher = AesCipher::new(&keys.s_enc)?;
            let mut icv = counter_block;
            cipher.encrypt_block(&mut icv);

            // Pad data with 0x80 followed by zeros to a multiple of 16
            let pad_len = 16 - (data.len() % 16);
            let mut padded = Vec::with_capacity(data.len() + pad_len);
            padded.extend_from_slice(data);
            padded.push(0x80);
            while (padded.len() % 16) != 0 {
                padded.push(0x00);
            }

            aes_cbc_encrypt(&keys.s_enc, &icv, &padded)?
        } else {
            data.to_vec()
        };

        // 2. Modified CLA byte: bit 3 (0x04) set indicates secure messaging
        let modified_cla = cla | 0x04;

        // 3. Compute C-MAC if required
        if self.security_level.has_c_mac() {
            let total_data_len = encrypted_payload.len() + 8;
            if total_data_len > 255 {
                return Err(Scp03Error::ApduTooLong(total_data_len));
            }
            let lc = total_data_len as u8;

            let mut cmac_input = Vec::with_capacity(16 + 5 + encrypted_payload.len());
            cmac_input.extend_from_slice(&self.mac_chaining_value);
            cmac_input.extend_from_slice(&[modified_cla, ins, p1, p2, lc]);
            cmac_input.extend_from_slice(&encrypted_payload);

            let mac16 = aes_cmac(&keys.s_mac, &cmac_input)?;
            self.mac_chaining_value = mac16;

            let mut wrapped = Vec::with_capacity(5 + total_data_len);
            wrapped.extend_from_slice(&[modified_cla, ins, p1, p2, lc]);
            wrapped.extend_from_slice(&encrypted_payload);
            wrapped.extend_from_slice(&mac16[..8]);
            Ok(wrapped)
        } else {
            let lc = encrypted_payload.len() as u8;
            let mut wrapped = Vec::with_capacity(5 + encrypted_payload.len());
            wrapped.extend_from_slice(&[modified_cla, ins, p1, p2, lc]);
            wrapped.extend_from_slice(&encrypted_payload);
            Ok(wrapped)
        }
    }
}
