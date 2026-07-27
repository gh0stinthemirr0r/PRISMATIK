//! # prismatik-prismatik-identity
//!
//! Layer 0 — Foundational (no workspace deps)
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — foundational identity contracts for Wave 0 floor.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub mod asset_id;
pub mod corporate_action;
pub mod external_id;
pub mod resolver;

pub use asset_id::{AssetId, MicCode, VenueId};
pub use corporate_action::{
    CorporateAction, CorporateActionId, CorporateActionKind, DelistingReason, MergerTerms,
    OccMemoRef, Ratio,
};
pub use external_id::ExternalIdentifier;
pub use resolver::{IdentityTransition, SymbologyError, SymbologyResolver};

/// OpenFIGI mapper helper.
#[derive(Debug, Default, Clone)]
pub struct OpenFigiMapper;

impl OpenFigiMapper {
    /// Normalize raw FIGI text to upper-case canonical form.
    pub fn normalize_figi(raw: &str) -> Result<String, SymbologyError> {
        let normalized = raw.trim().to_ascii_uppercase();
        let is_valid = normalized.len() == 12
            && normalized
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
        if !is_valid {
            return Err(SymbologyError::Storage(format!("invalid FIGI '{raw}'")));
        }
        Ok(normalized)
    }

    /// Convert raw FIGI text into canonical external identifier.
    pub fn to_external_id(raw: &str) -> Result<ExternalIdentifier, SymbologyError> {
        Ok(ExternalIdentifier::Figi(Self::normalize_figi(raw)?))
    }
}

#[cfg(test)]
mod mapper_tests {
    use super::*;

    #[test]
    fn normalizes_valid_figi() {
        let normalized = OpenFigiMapper::normalize_figi("bbg000blnnh6").expect("valid figi");
        assert_eq!(normalized, "BBG000BLNNH6");
    }

    #[test]
    fn rejects_invalid_figi() {
        let error = OpenFigiMapper::normalize_figi("bad").expect_err("invalid figi");
        assert!(error.to_string().contains("invalid FIGI"));
    }
}
