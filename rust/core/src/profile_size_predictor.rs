//! Profile size predictor matching eSIM profile order attributes (SM-DP+ FQDN,
//! PLMN, Service Provider Name, and EUM prefix from EID) against reference profile size catalogs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An entry in the reference profile sizes catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceSizeEntry {
    pub rsp: String,
    #[serde(default)]
    pub plmn: Option<String>,
    #[serde(default, rename = "serviceProviderName")]
    pub service_provider_name: Option<String>,
    pub reference_size: i64,
    #[serde(default)]
    pub eum_sizes: HashMap<String, i64>,
}

#[derive(Deserialize)]
struct RawCatalog {
    #[serde(default)]
    reference_eum: Option<String>,
    #[serde(default)]
    results: Vec<ReferenceSizeEntry>,
}

/// In-memory catalog of reference profile sizes for prediction.
#[derive(Debug, Clone, Default)]
pub struct ProfileSizeCatalog {
    pub reference_eum: Option<String>,
    entries: Vec<ReferenceSizeEntry>,
    by_rsp: HashMap<String, Vec<usize>>,
}

impl ProfileSizeCatalog {
    /// Creates a new catalog from raw entries.
    #[must_use]
    pub fn new(reference_eum: Option<String>, entries: Vec<ReferenceSizeEntry>) -> Self {
        let mut by_rsp: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, entry) in entries.iter().enumerate() {
            by_rsp
                .entry(entry.rsp.to_ascii_lowercase())
                .or_default()
                .push(idx);
        }
        Self {
            reference_eum,
            entries,
            by_rsp,
        }
    }

    /// Returns the catalog entries in their original order.
    #[must_use]
    pub fn entries(&self) -> &[ReferenceSizeEntry] {
        &self.entries
    }

    /// Parses a profile size catalog from GSMA/reference JSON text.
    ///
    /// # Errors
    /// Returns a `serde_json::Error` if the JSON structure is malformed.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let raw: RawCatalog = serde_json::from_str(json_str)?;
        Ok(Self::new(raw.reference_eum, raw.results))
    }

    /// Predicts the profile size in bytes for the specified order parameters.
    #[must_use]
    pub fn predict_size(
        &self,
        eid: Option<&str>,
        smdp_address: Option<&str>,
        plmn: Option<&str>,
        service_provider_name: Option<&str>,
    ) -> Option<i64> {
        if self.entries.is_empty() {
            return None;
        }

        let eum_prefix = eid.and_then(|e| (e.len() >= 8 && e.is_ascii()).then(|| &e[..8]));

        // Determine candidate entry indices
        let candidates: Vec<&ReferenceSizeEntry> = if let Some(smdp) = smdp_address {
            let smdp_lower = smdp.trim().to_ascii_lowercase();
            if let Some(indices) = self.by_rsp.get(&smdp_lower) {
                indices.iter().map(|&idx| &self.entries[idx]).collect()
            } else {
                Vec::new()
            }
        } else {
            self.entries.iter().collect()
        };

        let mut best_match: Option<&ReferenceSizeEntry> = None;
        let mut best_score: u32 = 0;

        for entry in candidates {
            let mut score = 0;

            if let Some(smdp) = smdp_address {
                if entry.rsp.eq_ignore_ascii_case(smdp.trim()) {
                    score += 1;
                } else {
                    continue;
                }
            }

            if let (Some(target_plmn), Some(entry_plmn)) = (plmn, &entry.plmn) {
                if entry_plmn.trim() == target_plmn.trim() {
                    score += 2;
                }
            }

            if let (Some(target_spn), Some(entry_spn)) =
                (service_provider_name, &entry.service_provider_name)
            {
                let t_lower = target_spn.trim().to_ascii_lowercase();
                let e_lower = entry_spn.trim().to_ascii_lowercase();
                if e_lower == t_lower || t_lower.contains(&e_lower) {
                    score += 3;
                }
            }

            // If no SMDP was supplied, require at least one strong match (SPN or PLMN)
            if smdp_address.is_none() && score == 0 {
                continue;
            }

            if score > best_score {
                best_score = score;
                best_match = Some(entry);
            }
        }

        let best = best_match?;

        // 1. If EUM prefix exists in eum_sizes, use it
        if let Some(prefix) = eum_prefix {
            if let Some(&size) = best.eum_sizes.get(prefix) {
                return Some(size);
            }
        }

        // 2. If EUM prefix matches the catalog reference EUM, return reference_size
        if let (Some(prefix), Some(ref_eum)) = (eum_prefix, &self.reference_eum) {
            if prefix == ref_eum.as_str() {
                return Some(best.reference_size);
            }
        }

        // 3. Fallback to reference_size
        Some(best.reference_size)
    }
}
