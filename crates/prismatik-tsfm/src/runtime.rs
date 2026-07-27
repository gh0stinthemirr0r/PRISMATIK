//! TSFM runtime trait stub — no ONNX link (`P5-QM-08` floor).

use crate::registry::{ModelId, SeriesModality};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Out-of-domain assessment for a forecast request (floor stub).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutOfDomainVerdict {
    /// Request is in-distribution for the loaded model.
    InDomain,
    /// Request is out-of-distribution; caller should widen / suppress.
    OutOfDomain,
    /// Not evaluated at this floor.
    Unevaluated,
}

/// Forecast request envelope (floor).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TsfmForecastRequest {
    /// Asset identifier (opaque string at this floor).
    pub asset: String,
    /// Series modality.
    pub modality: SeriesModality,
    /// Context window values.
    pub context: Vec<f64>,
    /// Horizon length in steps.
    pub horizon: usize,
}

/// Forecast response envelope (floor).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TsfmForecastResponse {
    /// Point forecast path.
    pub point: Vec<f64>,
    /// Optional quantile forecasts keyed by probability micros (e.g. 500_000 = 0.5).
    #[serde(default)]
    pub quantiles: Vec<(u32, Vec<f64>)>,
    /// OOD verdict.
    pub ood: OutOfDomainVerdict,
}

/// Runtime errors.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum TsfmRuntimeError {
    /// No inference backend linked (Wave 3B floor).
    #[error("tsfm runtime stub: {0}")]
    Stub(String),
    /// Request validation failure.
    #[error("invalid forecast request: {0}")]
    InvalidRequest(String),
}

/// Foundation-model inference surface. Floor implementations return stub
/// errors / empty vectors — **no ONNX Runtime dependency**.
pub trait TsfmRuntime: Send + Sync + std::fmt::Debug {
    /// Model id this runtime was loaded for.
    fn model_id(&self) -> &ModelId;

    /// Produce a forecast for the given request.
    fn forecast(
        &self,
        request: &TsfmForecastRequest,
    ) -> Result<TsfmForecastResponse, TsfmRuntimeError>;

    /// Extract an embedding for the context window.
    fn embed(&self, context: &[f64]) -> Result<Vec<f32>, TsfmRuntimeError>;
}

/// Stub runtime returned by [`crate::registry::ModelRegistry::load_tsfm`].
#[derive(Clone, Debug)]
pub struct StubTsfmRuntime {
    model_id: ModelId,
}

impl StubTsfmRuntime {
    /// Construct a stub runtime for `model_id`.
    #[must_use]
    pub fn new(model_id: ModelId) -> Self {
        Self { model_id }
    }
}

impl TsfmRuntime for StubTsfmRuntime {
    fn model_id(&self) -> &ModelId {
        &self.model_id
    }

    fn forecast(
        &self,
        _request: &TsfmForecastRequest,
    ) -> Result<TsfmForecastResponse, TsfmRuntimeError> {
        Err(TsfmRuntimeError::Stub(
            "forecast: ONNX / inference backend not linked at P5-QM-08 floor".into(),
        ))
    }

    fn embed(&self, _context: &[f64]) -> Result<Vec<f32>, TsfmRuntimeError> {
        // Empty vector path — no weights loaded.
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_forecast_errors_and_embed_empty() {
        let rt = StubTsfmRuntime::new(ModelId::new("stub"));
        let req = TsfmForecastRequest {
            asset: "AAPL".into(),
            modality: SeriesModality::Univariate,
            context: vec![1.0, 2.0],
            horizon: 4,
        };
        assert!(matches!(rt.forecast(&req), Err(TsfmRuntimeError::Stub(_))));
        assert!(rt.embed(&[1.0, 2.0]).unwrap().is_empty());
    }
}
