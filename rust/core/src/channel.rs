//! APDU transport, ISO/IEC 7816-4 logical channel mapping, request chaining,
//! response reassembly, and OMAPI safety state tracking.

use core::fmt;
use std::collections::BTreeMap;
use std::error::Error;

/// Error arising during logical channel management, APDU chaining, or OMAPI transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelError {
    /// Logical channel number exceeds supported ISO 7816-4 range (0..=19).
    UnsupportedLogicalChannel(u8),
    /// Logical channel 0 (Basic Channel) cannot be closed via MANAGE CHANNEL.
    CannotCloseBasicChannel,
    /// APDU buffer length is too short to contain header (less than 4 bytes).
    InvalidApduLength(usize),
    /// APDU response is too short to contain status bytes (less than 2 bytes).
    ResponseTooShort(usize),
    /// Exceeded maximum allowed GET RESPONSE steps during reassembly.
    TooManyReassemblySteps,
    /// Logical channel already claimed by an active owner.
    ChannelAlreadyOwned { channel: u8 },
    /// Token mismatch or stale channel access.
    StaleChannelToken { channel: u8, token: u64 },
    /// OMAPI session has been poisoned by catastrophic transport corruption.
    SessionPoisoned(String),
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedLogicalChannel(ch) => {
                write!(f, "unsupported logical channel number: {ch} (max is 19)")
            }
            Self::CannotCloseBasicChannel => {
                write!(f, "cannot close basic channel 0")
            }
            Self::InvalidApduLength(len) => {
                write!(f, "invalid APDU length: {len} bytes (minimum 4)")
            }
            Self::ResponseTooShort(len) => {
                write!(f, "response too short: {len} bytes (minimum 2)")
            }
            Self::TooManyReassemblySteps => {
                write!(f, "exceeded maximum APDU response reassembly steps")
            }
            Self::ChannelAlreadyOwned { channel } => {
                write!(f, "logical channel {channel} already has an active owner")
            }
            Self::StaleChannelToken { channel, token } => {
                write!(
                    f,
                    "channel handle {channel} is stale or token {token} mismatch"
                )
            }
            Self::SessionPoisoned(reason) => {
                write!(f, "OMAPI session is poisoned: {reason}")
            }
        }
    }
}

impl Error for ChannelError {}

/// Maps an APDU CLA byte to target a specific ISO/IEC 7816-4 logical channel.
///
/// Channels 0..=3 are mapped via bits 1 and 2: `(cla & 0xFC) | channel`.
/// Channels 4..=19 are mapped via bits 6 and 0..=3: `(cla & 0xF0) | 0x40 | (channel - 4)`.
///
/// # Errors
/// Returns [`ChannelError::UnsupportedLogicalChannel`] if `channel >= 20`.
pub fn map_logical_channel_cla(cla: u8, channel: u8) -> Result<u8, ChannelError> {
    if channel < 4 {
        Ok((cla & 0xFC) | channel)
    } else if channel < 20 {
        Ok((cla & 0xF0) | 0x40 | (channel - 4))
    } else {
        Err(ChannelError::UnsupportedLogicalChannel(channel))
    }
}

/// Builds a MANAGE CHANNEL [Open] APDU (`[0x00, 0x70, 0x00, 0x00, 0x01]`).
#[must_use]
pub fn build_manage_channel_open_apdu() -> [u8; 5] {
    [0x00, 0x70, 0x00, 0x00, 0x01]
}

/// Builds a MANAGE CHANNEL [Close] APDU (`[0x00, 0x70, 0x80, channel, 0x00]`).
///
/// # Errors
/// Returns [`ChannelError::CannotCloseBasicChannel`] if `channel == 0`.
pub fn build_manage_channel_close_apdu(channel: u8) -> Result<[u8; 5], ChannelError> {
    if channel == 0 {
        return Err(ChannelError::CannotCloseBasicChannel);
    }
    Ok([0x00, 0x70, 0x80, channel, 0x00])
}

/// Formats a single APDU command frame with header, optional data, and expected length (Le).
#[must_use]
pub fn frame_apdu_command(
    cla: u8,
    ins: u8,
    p1: u8,
    p2: u8,
    data: Option<&[u8]>,
    le: Option<usize>,
) -> Vec<u8> {
    let mut apdu = Vec::with_capacity(4 + data.map_or(0, |d| d.len() + 3) + 2);
    apdu.push(cla);
    apdu.push(ins);
    apdu.push(p1);
    apdu.push(p2);

    if let Some(payload) = data {
        if !payload.is_empty() {
            if payload.len() <= 255 {
                apdu.push(payload.len() as u8);
                apdu.extend_from_slice(payload);
            } else {
                // Extended length APDU
                apdu.push(0x00);
                apdu.push(((payload.len() >> 8) & 0xFF) as u8);
                apdu.push((payload.len() & 0xFF) as u8);
                apdu.extend_from_slice(payload);
            }
        }
    }

    if let Some(expected_le) = le {
        if expected_le > 255 {
            if data.is_none() || data.is_some_and(|d| d.is_empty()) {
                apdu.push(0x00); // Extended length preamble
            }
            apdu.push(((expected_le >> 8) & 0xFF) as u8);
            apdu.push((expected_le & 0xFF) as u8);
        } else {
            apdu.push(expected_le as u8);
        }
    } else if ins != 0xE2 && (data.is_none() || data.is_some_and(|d| d.is_empty())) {
        // Standard ISO commands default Le to 0x00 unless STORE DATA (0xE2)
        apdu.push(0x00);
    }

    apdu
}

/// Splits an outgoing APDU payload exceeding Maximum Segment Size (MSS) into chained APDUs.
///
/// For `INS == 0xE2` (STORE DATA):
/// - P1 bit 7 indicates the last block: `is_last ? (p1 | 0x80) : (p1 & 0x7F)`.
/// - P2 carries the 8-bit sequential block number.
///
/// For standard commands:
/// - CLA bit 4 (0x10) indicates chaining (`current_cla |= 0x10` if not last).
#[must_use]
pub fn chain_apdu_request(
    cla: u8,
    ins: u8,
    p1: u8,
    p2: u8,
    data: &[u8],
    le: Option<usize>,
    mss: usize,
) -> Vec<Vec<u8>> {
    let effective_mss = if mss == 0 { 255 } else { mss };
    if data.len() <= effective_mss {
        return vec![frame_apdu_command(cla, ins, p1, p2, Some(data), le)];
    }

    let mut chained_packets = Vec::new();
    let mut offset = 0;
    let mut block_num: u8 = 0;

    while offset < data.len() {
        let remaining = data.len() - offset;
        let is_last = remaining <= effective_mss;
        let chunk_size = if is_last { remaining } else { effective_mss };
        let chunk = &data[offset..offset + chunk_size];

        let mut current_cla = cla;
        let mut current_p1 = p1;
        let mut current_p2 = p2;

        if ins == 0xE2 {
            current_p1 = if is_last { p1 | 0x80 } else { p1 & 0x7F };
            current_p2 = block_num;
            block_num = block_num.wrapping_add(1);
        } else if !is_last {
            current_cla |= 0x10;
        }

        let mut b = Vec::with_capacity(5 + chunk.len() + 1);
        b.push(current_cla);
        b.push(ins);
        b.push(current_p1);
        b.push(current_p2);
        b.push(chunk.len() as u8);
        b.extend_from_slice(chunk);

        if is_last && ins != 0xE2 {
            if let Some(expected_le) = le {
                if expected_le > 255 {
                    b.push(0x00);
                } else {
                    b.push(expected_le as u8);
                }
            } else {
                b.push(0x00);
            }
        }

        chained_packets.push(b);
        offset += chunk_size;
    }

    chained_packets
}

/// Action determined by the [`ResponseReassembler`] after evaluating an APDU response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReassemblyAction {
    /// Full response reassembly complete.
    Complete(Vec<u8>),
    /// Additional response bytes available on card; transmit GET RESPONSE APDU.
    SendGetResponse([u8; 5]),
    /// Incorrect Le status received (SW1=0x6C); re-issue previous command with exact Le.
    ReissueWithLe(u8),
}

/// State machine for reassembling multi-frame APDU responses (`SW1 == 0x61` and `SW1 == 0x6C`).
#[derive(Debug, Clone)]
pub struct ResponseReassembler {
    accumulated: Vec<u8>,
    steps: usize,
    max_steps: usize,
}

impl Default for ResponseReassembler {
    fn default() -> Self {
        Self::new(2048)
    }
}

impl ResponseReassembler {
    /// Creates a new reassembler with a maximum step bound to prevent infinite loops.
    #[must_use]
    pub fn new(max_steps: usize) -> Self {
        Self {
            accumulated: Vec::new(),
            steps: 0,
            max_steps,
        }
    }

    /// Evaluates one APDU response slice.
    ///
    /// # Errors
    /// Returns [`ChannelError::ResponseTooShort`] if response is shorter than 2 bytes.
    /// Returns [`ChannelError::TooManyReassemblySteps`] if loop threshold is exceeded.
    pub fn process_chunk(
        &mut self,
        current_apdu: &[u8],
        response: &[u8],
    ) -> Result<ReassemblyAction, ChannelError> {
        if response.len() < 2 {
            return Err(ChannelError::ResponseTooShort(response.len()));
        }
        self.steps += 1;
        if self.steps > self.max_steps {
            return Err(ChannelError::TooManyReassemblySteps);
        }

        let sw1 = response[response.len() - 2];
        let sw2 = response[response.len() - 1];

        // SW1 == 0x61: More bytes available; send GET RESPONSE
        if sw1 == 0x61 {
            if response.len() > 2 {
                self.accumulated
                    .extend_from_slice(&response[..response.len() - 2]);
            }
            let cla = current_apdu.first().copied().unwrap_or(0x00);
            let le = if sw2 == 0 { 0x00 } else { sw2 };
            return Ok(ReassemblyAction::SendGetResponse([
                cla, 0xC0, 0x00, 0x00, le,
            ]));
        }

        // SW1 == 0x6C: Reissue with requested Le
        if sw1 == 0x6C {
            return Ok(ReassemblyAction::ReissueWithLe(sw2));
        }

        // Final payload chunk
        if !self.accumulated.is_empty() {
            self.accumulated.extend_from_slice(response);
            let complete = std::mem::take(&mut self.accumulated);
            Ok(ReassemblyAction::Complete(complete))
        } else {
            Ok(ReassemblyAction::Complete(response.to_vec()))
        }
    }

    /// Resets the reassembler buffer and step counter.
    pub fn reset(&mut self) {
        self.accumulated.clear();
        self.steps = 0;
    }
}

/// Tracks channel ownership tokens to guard against stale handles and multi-owner race conditions.
#[derive(Debug, Default, Clone)]
pub struct OmapiChannelTracker {
    next_token: u64,
    owners: BTreeMap<u8, u64>,
}

impl OmapiChannelTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Claims an OMAPI logical channel, returning a unique ownership token.
    ///
    /// # Errors
    /// Returns [`ChannelError::ChannelAlreadyOwned`] if the channel is currently claimed.
    pub fn claim(&mut self, channel: u8) -> Result<u64, ChannelError> {
        if self.owners.contains_key(&channel) {
            return Err(ChannelError::ChannelAlreadyOwned { channel });
        }
        self.next_token = self.next_token.wrapping_add(1);
        let token = self.next_token;
        self.owners.insert(channel, token);
        Ok(token)
    }

    /// Ensures that the caller holds the valid active ownership token for this channel.
    ///
    /// # Errors
    /// Returns [`ChannelError::StaleChannelToken`] if the token does not match or is absent.
    pub fn ensure_owned(&self, channel: u8, token: u64) -> Result<(), ChannelError> {
        match self.owners.get(&channel) {
            Some(&current) if current == token => Ok(()),
            _ => Err(ChannelError::StaleChannelToken { channel, token }),
        }
    }

    /// Releases a claimed channel if the supplied token matches.
    pub fn release_if_owned(&mut self, channel: u8, token: u64) -> bool {
        if self.owners.get(&channel) == Some(&token) {
            self.owners.remove(&channel);
            true
        } else {
            false
        }
    }

    /// Clears all channel ownership records.
    pub fn clear(&mut self) {
        self.owners.clear();
    }
}

/// Latch tracking whether an OMAPI session has suffered catastrophic transport corruption.
#[derive(Debug, Default, Clone)]
pub struct OmapiSafetyLatch {
    poison_reason: Option<String>,
}

impl OmapiSafetyLatch {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Poisons the latch with a description of the transport corruption.
    pub fn poison(&mut self, reason: impl Into<String>) {
        self.poison_reason = Some(reason.into());
    }

    /// Returns `true` if the session is currently marked poisoned.
    #[must_use]
    pub fn is_poisoned(&self) -> bool {
        self.poison_reason.is_some()
    }

    /// Checks if the session is safe for commands.
    ///
    /// # Errors
    /// Returns [`ChannelError::SessionPoisoned`] if the session has been poisoned.
    pub fn check_safety(&self) -> Result<(), ChannelError> {
        if let Some(ref reason) = self.poison_reason {
            Err(ChannelError::SessionPoisoned(reason.clone()))
        } else {
            Ok(())
        }
    }

    /// Clears the poison state, recovering session safety.
    pub fn recover(&mut self) {
        self.poison_reason = None;
    }
}
