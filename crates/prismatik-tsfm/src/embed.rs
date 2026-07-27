//! Embedding extraction + batch materialization (`P55-QM-01` floor).
//!
//! Series-tokenizer-first path: encode context → request embeddings from a
//! [`TsfmRuntime`]. ONNX weights are not linked — stub runtimes return empty
//! vectors; this module still validates batch shapes and provenance.

use crate::registry::{ModelId, SeriesModality};
use crate::runtime::{TsfmRuntime, TsfmRuntimeError};
use crate::tokenizer::{SeriesTokenizer, TokenizerError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from embedding extraction.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum EmbedError {
    /// Tokenizer failure.
    #[error(transparent)]
    Tokenizer(#[from] TokenizerError),
    /// Runtime failure.
    #[error(transparent)]
    Runtime(#[from] TsfmRuntimeError),
    /// Empty batch.
    #[error("embed batch empty")]
    EmptyBatch,
    /// Inconsistent embedding dimensions across the batch.
    #[error("embed dim mismatch: expected {expected}, got {got}")]
    DimMismatch {
        /// Expected width.
        expected: usize,
        /// Observed width.
        got: usize,
    },
}

/// One context window submitted for embedding.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmbedRequest {
    /// Opaque asset key.
    pub asset: String,
    /// Series modality.
    pub modality: SeriesModality,
    /// Raw context values (pre-tokenize).
    pub context: Vec<f64>,
}

/// Materialized embedding row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmbedRow {
    /// Asset key echoed from the request.
    pub asset: String,
    /// Dense embedding (may be empty on stub runtime).
    pub vector: Vec<f32>,
    /// Model that produced the vector.
    pub model_id: ModelId,
}

/// Batch of embedding rows with shared dimensionality (0 when all empty).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EmbedBatch {
    /// Rows in request order.
    pub rows: Vec<EmbedRow>,
    /// Shared vector width (`0` if all empty stub vectors).
    pub dim: usize,
}

/// Extract embeddings for a batch using Prismatik series tokenization then runtime.
pub fn materialize_embeddings(
    tokenizer: &dyn SeriesTokenizer,
    runtime: &dyn TsfmRuntime,
    requests: &[EmbedRequest],
) -> Result<EmbedBatch, EmbedError> {
    if requests.is_empty() {
        return Err(EmbedError::EmptyBatch);
    }
    let model_id = runtime.model_id().clone();
    let mut rows = Vec::with_capacity(requests.len());
    let mut dim: Option<usize> = None;

    for req in requests {
        // Series-tokenizer-first: encode for validation / future codebook path.
        let _tokens = tokenizer.encode(&req.context)?;
        let vector = runtime.embed(&req.context)?;
        let width = vector.len();
        match dim {
            None => dim = Some(width),
            Some(expected) if expected != width => {
                return Err(EmbedError::DimMismatch {
                    expected,
                    got: width,
                });
            },
            Some(_) => {},
        }
        rows.push(EmbedRow {
            asset: req.asset.clone(),
            vector,
            model_id: model_id.clone(),
        });
    }

    Ok(EmbedBatch {
        rows,
        dim: dim.unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::StubTsfmRuntime;
    use crate::tokenizer::PrismatikSeriesTokenizer;

    #[test]
    fn stub_runtime_yields_empty_vectors() {
        use prismatik_determinism::SemanticVersion;
        let tok = PrismatikSeriesTokenizer::new(SemanticVersion::new(0, 1, 0));
        let rt = StubTsfmRuntime::new(ModelId::new("prismatik-tsfm-stub"));
        let batch = materialize_embeddings(
            &tok,
            &rt,
            &[
                EmbedRequest {
                    asset: "AAPL".into(),
                    modality: SeriesModality::Univariate,
                    context: vec![1.0, 2.0, 3.0],
                },
                EmbedRequest {
                    asset: "MSFT".into(),
                    modality: SeriesModality::Univariate,
                    context: vec![4.0, 5.0],
                },
            ],
        )
        .unwrap();
        assert_eq!(batch.rows.len(), 2);
        assert_eq!(batch.dim, 0);
        assert!(batch.rows.iter().all(|r| r.vector.is_empty()));
    }

    #[test]
    fn empty_batch_rejected() {
        use prismatik_determinism::SemanticVersion;
        let tok = PrismatikSeriesTokenizer::new(SemanticVersion::new(0, 1, 0));
        let rt = StubTsfmRuntime::new(ModelId::new("prismatik-tsfm-stub"));
        let err = materialize_embeddings(&tok, &rt, &[]).err().unwrap();
        assert!(matches!(err, EmbedError::EmptyBatch));
    }
}
