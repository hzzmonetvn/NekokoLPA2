//! Card Application Toolkit (CAT) proactive command helpers.
//!
//! STATUS, FETCH, and TERMINAL RESPONSE follow ETSI TS 102 221;
//! proactive command encoding follows ETSI TS 102 223, and REFRESH
//! qualifier 0x09 (eUICC profile state change) follows 3GPP TS 31.111.

use std::fmt;

/// Successful command execution result byte.
pub const RESULT_SUCCESS: u8 = 0x00;

/// Terminal currently unable to process command result byte.
pub const RESULT_UNABLE_TO_PROCESS: u8 = 0x20;

/// Errors arising during proactive command parsing or encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatProactiveError {
    MissingStatusWord,
    NotProactiveBerTlv,
    TruncatedBerTlv,
    TruncatedTag,
    TruncatedValue,
    InvalidBerTlvLength,
    InvalidCommandDetailsTlv,
    MissingCommandDetails,
    TerminalResponseBodyTooLong,
}

impl fmt::Display for CatProactiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingStatusWord => write!(f, "FETCH response is missing a status word"),
            Self::NotProactiveBerTlv => {
                write!(f, "FETCH did not return a proactive command BER-TLV (0xD0)")
            }
            Self::TruncatedBerTlv => write!(f, "Truncated proactive command BER-TLV"),
            Self::TruncatedTag => write!(f, "Truncated COMPREHENSION-TLV tag"),
            Self::TruncatedValue => write!(f, "Truncated COMPREHENSION-TLV value"),
            Self::InvalidBerTlvLength => write!(f, "Invalid BER-TLV length"),
            Self::InvalidCommandDetailsTlv => write!(f, "Invalid command details TLV"),
            Self::MissingCommandDetails => {
                write!(f, "Proactive command is missing command details")
            }
            Self::TerminalResponseBodyTooLong => {
                write!(f, "TERMINAL RESPONSE body exceeds 255 bytes")
            }
        }
    }
}

impl std::error::Error for CatProactiveError {}

/// How a reader that tunnels APDUs should respond to a proactive command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatResponseDecision {
    /// Answer TERMINAL RESPONSE with "command performed successfully" (0x00).
    AcknowledgeSuccess,

    /// Drop logical channels so the application session restarts, then answer 0x00.
    RestartNaaSession,

    /// Answer TERMINAL RESPONSE with "terminal currently unable to process" (0x20).
    DeclineUnableToProcess,

    /// Do not answer; the card needs a real hardware reset before settling.
    RequiresCardReset,
}

/// The command details of a fetched proactive command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProactiveCommandDetails {
    pub number: u8,
    pub command_type: u8,
    pub qualifier: u8,
}

impl ProactiveCommandDetails {
    /// REFRESH, the command raised by an eUICC for profile state change.
    pub const TYPE_REFRESH: u8 = 0x01;

    /// REFRESH qualifier "eUICC profile state change" (3GPP TS 31.111).
    pub const QUALIFIER_PROFILE_STATE_CHANGE: u8 = 0x09;

    /// REFRESH qualifier "UICC reset".
    pub const QUALIFIER_UICC_RESET: u8 = 0x04;

    /// REFRESH qualifier "NAA session reset".
    pub const QUALIFIER_NAA_SESSION_RESET: u8 = 0x06;

    #[must_use]
    pub const fn new(number: u8, command_type: u8, qualifier: u8) -> Self {
        Self {
            number,
            command_type,
            qualifier,
        }
    }

    #[must_use]
    pub const fn is_refresh(&self) -> bool {
        self.command_type == Self::TYPE_REFRESH
    }

    /// Evaluates how a tunneling reader should resolve this proactive command.
    #[must_use]
    pub fn decision(&self) -> CatResponseDecision {
        if !self.is_refresh() {
            return CatResponseDecision::DeclineUnableToProcess;
        }
        match self.qualifier {
            Self::QUALIFIER_PROFILE_STATE_CHANGE => CatResponseDecision::AcknowledgeSuccess,
            Self::QUALIFIER_NAA_SESSION_RESET => CatResponseDecision::RestartNaaSession,
            _ => CatResponseDecision::RequiresCardReset,
        }
    }
}

/// Card Application Toolkit proactive command encoder/decoder.
pub struct CatProactive;

impl CatProactive {
    /// Constructs STATUS APDU (0x80 0xF2 0x00 0x00 0x00).
    #[must_use]
    pub fn status() -> Vec<u8> {
        vec![0x80, 0xF2, 0x00, 0x00, 0x00]
    }

    /// Constructs FETCH APDU (0x80 0x12 0x00 0x00 [length]).
    #[must_use]
    pub fn fetch(length: u8) -> Vec<u8> {
        vec![0x80, 0x12, 0x00, 0x00, length]
    }

    /// Constructs TERMINAL RESPONSE APDU carrying [details] and [general_result].
    pub fn terminal_response(
        details: &ProactiveCommandDetails,
        general_result: u8,
    ) -> Result<Vec<u8>, CatProactiveError> {
        let body = [
            // Command details (tag 0x81, len 3)
            0x81,
            0x03,
            details.number,
            details.command_type,
            details.qualifier,
            // Device identities (tag 0x82, len 2): terminal (0x82) -> UICC (0x81)
            0x82,
            0x02,
            0x82,
            0x81,
            // Result (tag 0x83, len 1)
            0x83,
            0x01,
            general_result,
        ];

        if body.len() > 0xFF {
            return Err(CatProactiveError::TerminalResponseBodyTooLong);
        }

        let mut apdu = Vec::with_capacity(5 + body.len());
        apdu.extend_from_slice(&[0x80, 0x14, 0x00, 0x00, body.len() as u8]);
        apdu.extend_from_slice(&body);
        Ok(apdu)
    }

    /// Parses the command details from a raw FETCH response containing SW1/SW2.
    pub fn parse_fetch_response(
        response: &[u8],
    ) -> Result<ProactiveCommandDetails, CatProactiveError> {
        if response.len() < 2 {
            return Err(CatProactiveError::MissingStatusWord);
        }
        let payload = &response[..response.len() - 2];
        if payload.is_empty() || payload[0] != 0xD0 {
            return Err(CatProactiveError::NotProactiveBerTlv);
        }

        let (outer_len, mut offset) = Self::read_length(payload, 1)?;
        let end = offset + outer_len;
        if end > payload.len() {
            return Err(CatProactiveError::TruncatedBerTlv);
        }

        while offset < end {
            let mut tag = u32::from(payload[offset]);
            offset += 1;
            if tag == 0x7F {
                if offset + 2 > end {
                    return Err(CatProactiveError::TruncatedTag);
                }
                tag = (tag << 16)
                    | (u32::from(payload[offset]) << 8)
                    | u32::from(payload[offset + 1]);
                offset += 2;
            }

            let (tlv_len, tlv_val_offset) = Self::read_length(payload, offset)?;
            let tlv_end = tlv_val_offset + tlv_len;
            if tlv_end > end {
                return Err(CatProactiveError::TruncatedValue);
            }

            // Comprehension required bit is bit 8 (0x80); masked tag is tag & 0x7F.
            if (tag & 0x7F) == 0x01 {
                let val = &payload[tlv_val_offset..tlv_end];
                if val.len() != 3 {
                    return Err(CatProactiveError::InvalidCommandDetailsTlv);
                }
                return Ok(ProactiveCommandDetails::new(val[0], val[1], val[2]));
            }

            offset = tlv_end;
        }

        Err(CatProactiveError::MissingCommandDetails)
    }

    fn read_length(data: &[u8], offset: usize) -> Result<(usize, usize), CatProactiveError> {
        if offset >= data.len() {
            return Err(CatProactiveError::TruncatedBerTlv);
        }
        let first = data[offset];
        if first < 0x80 {
            return Ok((usize::from(first), offset + 1));
        }

        let count = (first & 0x7F) as usize;
        if count == 0 || count > 4 || offset + 1 + count > data.len() {
            return Err(CatProactiveError::InvalidBerTlvLength);
        }

        let mut val = 0usize;
        for i in 0..count {
            val = (val << 8) | usize::from(data[offset + 1 + i]);
        }
        Ok((val, offset + 1 + count))
    }
}
