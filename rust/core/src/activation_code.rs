//! GSMA SGP.22 Activation Code parser and formatter (`LPA:1$<SMDP>$<MatchingId>[$<SMDP_OID>][$<CC>]`).

use core::fmt;
use std::error::Error;

/// GSMA SGP.22 Activation Code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActivationCode {
    pub format: String,
    pub smdp_address: String,
    pub matching_id: String,
    pub smdp_oid: Option<String>,
    pub confirmation_code: Option<String>,
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
        Self {
            format: "LPA:1".to_string(),
            smdp_address: smdp_address.into(),
            matching_id: matching_id.into(),
            smdp_oid,
            confirmation_code,
        }
    }

    /// Parses a raw activation code string into an [`ActivationCode`].
    ///
    /// # Errors
    /// Returns [`ActivationCodeError`] if the code does not match GSMA SGP.22 format specifications.
    pub fn parse(code: &str) -> Result<Self, ActivationCodeError> {
        let trimmed = code.trim();
        if !trimmed.starts_with("LPA:1$") {
            return Err(ActivationCodeError::InvalidHeader);
        }

        let parts: Vec<&str> = trimmed.split('$').collect();
        if parts.len() < 2 || parts[1].is_empty() {
            return Err(ActivationCodeError::MissingMandatoryFields);
        }

        let format = parts[0].to_string();
        let smdp_address = parts[1].to_string();
        let matching_id = if parts.len() > 2 {
            parts[2].to_string()
        } else {
            String::new()
        };

        let smdp_oid = if parts.len() > 3 && !parts[3].is_empty() {
            Some(parts[3].to_string())
        } else {
            None
        };

        let confirmation_code = if parts.len() > 4 && !parts[4].is_empty() {
            Some(parts[4].to_string())
        } else {
            None
        };

        Ok(Self {
            format,
            smdp_address,
            matching_id,
            smdp_oid,
            confirmation_code,
        })
    }
}

impl fmt::Display for ActivationCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LPA:1${}${}", self.smdp_address, self.matching_id)?;
        if self.smdp_oid.is_some() || self.confirmation_code.is_some() {
            write!(f, "${}", self.smdp_oid.as_deref().unwrap_or(""))?;
        }
        if let Some(ref cc) = self.confirmation_code {
            write!(f, "${cc}")?;
        }
        Ok(())
    }
}
