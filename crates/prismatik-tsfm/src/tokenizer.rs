//! Series tokenizer trait and binding validation (`P5-QM-07` floor).

use crate::registry::{ModelId, SeriesModality};
use prismatik_determinism::{ArtifactRef, ContentHash, SemanticVersion};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from tokenizer binding validation or encode/decode.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum TokenizerError {
    /// Model / tokenizer version pair does not match.
    #[error("tokenizer mismatch for model {model_id}: expected version {expected}, found {found}")]
    VersionMismatch {
        /// Model under validation.
        model_id: ModelId,
        /// Expected semantic version.
        expected: String,
        /// Found semantic version.
        found: String,
    },
    /// Codebook content hash does not match the model's expected hash.
    #[error(
        "tokenizer codebook hash mismatch for model {model_id}: expected {expected}, found {found}"
    )]
    CodebookHashMismatch {
        /// Model under validation.
        model_id: ModelId,
        /// Expected codebook hash.
        expected: ContentHash,
        /// Found codebook hash.
        found: ContentHash,
    },
    /// Stub encode/decode path (no real codebook loaded).
    #[error("tokenizer stub: {0}")]
    Stub(String),
}

/// Registered pairing of a model artifact and the tokenizer codebook it was
/// trained against. Loading a model with an unpaired codebook is a hard deny.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenizerBinding {
    /// Model this codebook is bound to.
    pub model_id: ModelId,
    /// Model version the codebook was trained with.
    pub model_version: SemanticVersion,
    /// Codebook artifact reference.
    pub codebook: ArtifactRef,
    /// Maximum context length in tokens.
    pub max_context_tokens: usize,
    /// Modalities this tokenizer accepts.
    pub modalities: Vec<SeriesModality>,
    /// Disclosed reconstruction MAE (v0.3 §3.1).
    pub reconstruction_mae: f64,
}

impl TokenizerBinding {
    /// Validate that this binding matches the expected model version and
    /// codebook content hash. Version mismatch fails closed.
    pub fn validate(
        &self,
        expected_model_version: SemanticVersion,
        expected_codebook_hash: ContentHash,
    ) -> Result<(), TokenizerError> {
        if self.model_version != expected_model_version {
            return Err(TokenizerError::VersionMismatch {
                model_id: self.model_id.clone(),
                expected: expected_model_version.to_string(),
                found: self.model_version.to_string(),
            });
        }
        if self.codebook.content_hash != expected_codebook_hash {
            return Err(TokenizerError::CodebookHashMismatch {
                model_id: self.model_id.clone(),
                expected: expected_codebook_hash,
                found: self.codebook.content_hash,
            });
        }
        Ok(())
    }
}

/// Series → token encode/decode surface. Implementations are per-family
/// (upstream codebook / binning schemes when linked).
pub trait SeriesTokenizer: Send + Sync {
    /// Stable tokenizer identifier.
    fn tokenizer_id(&self) -> &str;

    /// Semantic version of this tokenizer / codebook pair.
    fn version(&self) -> SemanticVersion;

    /// Encode a float series into discrete tokens.
    fn encode(&self, series: &[f64]) -> Result<Vec<u32>, TokenizerError>;

    /// Decode tokens back to a float series (best-effort reconstruction).
    fn decode(&self, tokens: &[u32]) -> Result<Vec<f64>, TokenizerError>;

    /// Disclosed reconstruction MAE for this codebook.
    fn reconstruction_mae(&self) -> f64;
}

/// Floor stub tokenizer — always fails encode/decode with [`TokenizerError::Stub`].
#[derive(Clone, Debug)]
pub struct StubSeriesTokenizer {
    id: String,
    version: SemanticVersion,
    mae: f64,
}

impl StubSeriesTokenizer {
    /// Create a stub tokenizer.
    #[must_use]
    pub fn new(id: impl Into<String>, version: SemanticVersion) -> Self {
        Self {
            id: id.into(),
            version,
            mae: 0.0,
        }
    }
}

impl SeriesTokenizer for StubSeriesTokenizer {
    fn tokenizer_id(&self) -> &str {
        &self.id
    }

    fn version(&self) -> SemanticVersion {
        self.version
    }

    fn encode(&self, _series: &[f64]) -> Result<Vec<u32>, TokenizerError> {
        Err(TokenizerError::Stub(
            "encode not implemented at floor".into(),
        ))
    }

    fn decode(&self, _tokens: &[u32]) -> Result<Vec<f64>, TokenizerError> {
        Err(TokenizerError::Stub(
            "decode not implemented at floor".into(),
        ))
    }

    fn reconstruction_mae(&self) -> f64 {
        self.mae
    }
}

/// PRISMATIK pass-through series tokenizer (`P55-QM-01` floor).
///
/// Emits a deterministic token-per-bucket sketch (no real codebook). Used so
/// embedding materialization can exercise the Prismatik encode → embed path
/// without linking ONNX weights.
#[derive(Clone, Debug)]
pub struct PrismatikSeriesTokenizer {
    version: SemanticVersion,
    mae: f64,
}

impl PrismatikSeriesTokenizer {
    /// Construct the Prismatik floor tokenizer.
    #[must_use]
    pub fn new(version: SemanticVersion) -> Self {
        Self { version, mae: 0.0 }
    }
}

impl SeriesTokenizer for PrismatikSeriesTokenizer {
    fn tokenizer_id(&self) -> &str {
        "prismatik-series-floor"
    }

    fn version(&self) -> SemanticVersion {
        self.version
    }

    fn encode(&self, series: &[f64]) -> Result<Vec<u32>, TokenizerError> {
        // Quantize each value into a coarse bucket id (clean-room sketch).
        Ok(series
            .iter()
            .map(|v| {
                let bucket = (v * 100.0).round().clamp(0.0, u32::MAX as f64) as i64;
                bucket.rem_euclid(10_000) as u32
            })
            .collect())
    }

    fn decode(&self, tokens: &[u32]) -> Result<Vec<f64>, TokenizerError> {
        Ok(tokens.iter().map(|t| f64::from(*t) / 100.0).collect())
    }

    fn reconstruction_mae(&self) -> f64 {
        self.mae
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_determinism::{ArtifactId, ArtifactKind};

    fn sample_binding(version: SemanticVersion, hash: ContentHash) -> TokenizerBinding {
        TokenizerBinding {
            model_id: ModelId::new("financial-bar-base"),
            model_version: version,
            codebook: ArtifactRef {
                artifact_id: ArtifactId::new("financial-bar-codebook"),
                kind: ArtifactKind::TokenizerCodebook,
                version,
                content_hash: hash,
                signature: Default::default(),
            },
            max_context_tokens: 512,
            modalities: vec![SeriesModality::Ohlcv],
            reconstruction_mae: 0.01,
        }
    }

    #[test]
    fn version_mismatch_fails_validation() {
        let hash = ContentHash::from_bytes(b"codebook-v1");
        let binding = sample_binding(SemanticVersion::new(1, 0, 0), hash);
        let err = binding
            .validate(SemanticVersion::new(1, 1, 0), hash)
            .unwrap_err();
        assert!(matches!(err, TokenizerError::VersionMismatch { .. }));
    }

    #[test]
    fn matching_version_and_hash_pass() {
        let hash = ContentHash::from_bytes(b"codebook-v1");
        let v = SemanticVersion::new(1, 0, 0);
        sample_binding(v, hash).validate(v, hash).unwrap();
    }
}
