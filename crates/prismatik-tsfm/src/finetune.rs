//! Fine-tune manifests for organization-specific TSFM training (`P8-QM-02` floor).

use crate::registry::ModelId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Differential privacy posture declared for a fine-tune run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DpPosture {
    /// No DP applied (explicit disclosure).
    None,
    /// DP noise / clipping applied during fine-tuning.
    Applied,
    /// DP required by policy but not yet verified on this artifact.
    Required,
    /// Posture unknown / not disclosed — fails closed for enterprise gates.
    Unknown,
}

/// Errors raised when constructing or validating a fine-tune manifest.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum FineTuneError {
    /// `dataset_hash` must be a non-empty content digest string.
    #[error("fine-tune manifest denied: dataset_hash is empty")]
    EmptyDatasetHash,
}

/// Manifest binding a fine-tuned artifact to its base model, dataset, and DP posture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FineTuneManifest {
    /// Base (pretrained) model being fine-tuned.
    pub base_model: ModelId,
    /// Content digest of the fine-tune dataset (non-empty hex / `blake3:…` string).
    pub dataset_hash: String,
    /// Declared differential privacy posture.
    pub dp_posture: DpPosture,
}

impl FineTuneManifest {
    /// Construct a validated fine-tune manifest.
    ///
    /// Hard-denies an empty (or whitespace-only) `dataset_hash`.
    pub fn new(
        base_model: ModelId,
        dataset_hash: impl Into<String>,
        dp_posture: DpPosture,
    ) -> Result<Self, FineTuneError> {
        let dataset_hash = dataset_hash.into();
        if dataset_hash.trim().is_empty() {
            return Err(FineTuneError::EmptyDatasetHash);
        }
        Ok(Self {
            base_model,
            dataset_hash,
            dp_posture,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_non_empty_dataset_hash() {
        let m = FineTuneManifest::new(
            ModelId::new("financial-bar-base"),
            "blake3:0123456789abcdef",
            DpPosture::Applied,
        )
        .expect("valid manifest");
        assert_eq!(m.base_model, ModelId::new("financial-bar-base"));
        assert_eq!(m.dataset_hash, "blake3:0123456789abcdef");
        assert_eq!(m.dp_posture, DpPosture::Applied);
    }

    #[test]
    fn denies_empty_dataset_hash() {
        let err = FineTuneManifest::new(ModelId::new("financial-bar-base"), "", DpPosture::None)
            .expect_err("empty hash must deny");
        assert_eq!(err, FineTuneError::EmptyDatasetHash);
    }

    #[test]
    fn denies_whitespace_only_dataset_hash() {
        let err = FineTuneManifest::new(
            ModelId::new("financial-bar-base"),
            "   ",
            DpPosture::Unknown,
        )
        .expect_err("whitespace hash must deny");
        assert_eq!(err, FineTuneError::EmptyDatasetHash);
    }
}
