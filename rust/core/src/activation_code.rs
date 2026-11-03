//! GSMA SGP.22 Activation Code parser and formatter (`LPA:1$<SMDP>$<MatchingId>[$<SMDP_OID>][$<CC>]`).

use core::fmt;
use std::error::Error;

use crate::notification_address::is_valid_notification_address;

/// GSMA SGP.22 Activation Code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActivationCode {
    pub format: String,
    pub smdp_address: String,
    pub matching_id: String,
    pub smdp_oid: Option<String>,
    pub confirmation_code: Option<String>,
    pub confirmation_code_required: bool,
}

/// Error returned when parsing an activation code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationCodeError {
    /// Code does not start with the mandatory GSMA LPA header `LPA:1$`.
    InvalidHeader,
    /// Code has missing mandatory fields (e.g. empty SM-DP+ address).
    MissingMandatoryFields,
}

impl fmt::Display for ActivationCodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader => write!(f, "activation code does not start with 'LPA:1$'"),
            Self::MissingMandatoryFields => {
                write!(f, "activation code missing mandatory SM-DP+ address")
            }
        }
    }
}

impl Error for ActivationCodeError {}

impl ActivationCode {
    /// Creates a new `ActivationCode` with explicit parameters.
    #[must_use]
    pub fn new(
        smdp_address: impl Into<String>,
        matching_id: impl Into<String>,
        smdp_oid: Option<String>,
        confirmation_code: Option<String>,
    ) -> Self {
        let cc = confirmation_code;
        let cc_req = cc.is_some();
        Self {
            format: "LPA:1".to_string(),
            smdp_address: smdp_address.into(),
            matching_id: matching_id.into(),
            smdp_oid,
            confirmation_code: cc,
            confirmation_code_required: cc_req,
        }
    }

    /// Validates whether the SM-DP+ address is a valid FQDN.
    #[must_use]
    pub fn is_valid_smdp_address(&self) -> bool {
        is_valid_notification_address(&self.smdp_address)
    }

    /// Parses a raw activation code string into an [`ActivationCode`].
    ///
    /// # Errors
    /// Returns [`ActivationCodeError`] if the code does not match GSMA SGP.22 format specifications.
    pub fn parse(code: &str) -> Result<Self, ActivationCodeError> {
        let trimmed = code.trim();
        // Allow case-insensitive LPA:1$ check
        let is_upper = trimmed.starts_with("LPA:1$");
        let is_lower = trimmed.starts_with("lpa:1$");
        if !is_upper && !is_lower {
            return Err(ActivationCodeError::InvalidHeader);
        }

        let parts: Vec<&str> = trimmed.split('$').collect();
        if parts.len() < 2 || parts[1].trim().is_empty() {
            return Err(ActivationCodeError::MissingMandatoryFields);
        }

        let format = "LPA:1".to_string();
        let smdp_address = parts[1].trim().to_string();
        let matching_id = if parts.len() > 2 {
            parts[2].trim().to_string()
        } else {
            String::new()
        };

        let smdp_oid = if parts.len() > 3 && !parts[3].trim().is_empty() {
            Some(parts[3].trim().to_string())
        } else {
            None
        };

        let mut confirmation_code = None;
        let mut confirmation_code_required = false;

        if parts.len() > 4 {
            let p4 = parts[4].trim();
            if p4 == "1" {
                confirmation_code_required = true;
                if parts.len() > 5 && !parts[5].trim().is_empty() {
                    confirmation_code = Some(parts[5].trim().to_string());
                }
            } else if !p4.is_empty() {
                confirmation_code = Some(p4.to_string());
                confirmation_code_required = true;
            }
        }

        Ok(Self {
            format,
            smdp_address,
            matching_id,
            smdp_oid,
            confirmation_code,
            confirmation_code_required,
        })
    }

    /// Fuzzy extracts an activation code from unstructured text, such as a QR code
    /// scanning result, clipboard content, email body, or deep link URL.
    ///
    /// Example: `https://rsp.example.com/onboard?code=LPA:1$smdp.io$MATCH-123`
    #[must_use]
    pub fn extract_from_text(text: &str) -> Option<Self> {
        let lower = text.to_ascii_lowercase();
        let marker = "lpa:1$";
        let start_idx = lower.find(marker)?;

        let candidate = &text[start_idx..];
        // Terminate at whitespace, quotes, or URI query separators
        let end_idx = candidate
            .find(|c: char| {
                c.is_whitespace() || c == '"' || c == '\'' || c == '&' || c == '<' || c == '>'
            })
            .unwrap_or(candidate.len());

        let code_str = &candidate[..end_idx];
        Self::parse(code_str).ok()
    }
}

impl fmt::Display for ActivationCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LPA:1${}${}", self.smdp_address, self.matching_id)?;
        if self.smdp_oid.is_some()
            || self.confirmation_code.is_some()
            || self.confirmation_code_required
        {
            write!(f, "${}", self.smdp_oid.as_deref().unwrap_or(""))?;
        }
        if let Some(ref cc) = self.confirmation_code {
            write!(f, "${cc}")?;
        } else if self.confirmation_code_required {
            write!(f, "$1")?;
        }
        Ok(())
    }
}
