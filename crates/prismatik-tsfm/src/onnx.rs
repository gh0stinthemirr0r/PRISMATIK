//! ONNX Runtime adapter façade (`P5-QM-08` floor).
//!
//! Documents how an ONNX / INT8 CPU path would plug into [`TsfmRuntime`].
//! The workspace does **not** link `ort` / ONNX Runtime — construction fails closed.

use crate::registry::ModelId;
use crate::runtime::{TsfmForecastRequest, TsfmForecastResponse, TsfmRuntime, TsfmRuntimeError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// How the deferred ONNX backend would be configured.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnnxRuntimeConfig {
    /// Filesystem path to the `.onnx` artifact (opaque at floor).
    pub model_path: String,
    /// Prefer INT8 quantized CPU execution providers when available.
    pub prefer_int8_cpu: bool,
    /// Intra-op thread count hint (0 = runtime default).
    pub intra_op_threads: u32,
}

impl Default for OnnxRuntimeConfig {
    fn default() -> Self {
        Self {
            model_path: String::new(),
            prefer_int8_cpu: true,
            intra_op_threads: 0,
        }
    }
}

/// Errors constructing the deferred ONNX adapter.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum OnnxAdapterError {
    /// ONNX Runtime not linked in this build.
    #[error("onnx runtime not linked at P5-QM-08 floor: {0}")]
    NotLinked(String),
    /// Config rejected.
    #[error("invalid onnx config: {0}")]
    InvalidConfig(String),
}

/// Typed stand-in that implements [`TsfmRuntime`] but always errors on forecast.
#[derive(Clone, Debug)]
pub struct OnnxTsfmRuntime {
    model_id: ModelId,
    config: OnnxRuntimeConfig,
}

impl OnnxTsfmRuntime {
    /// Attempt to load an ONNX-backed runtime. Always fails closed until `ort` lands.
    pub fn try_load(
        model_id: ModelId,
        config: OnnxRuntimeConfig,
    ) -> Result<Self, OnnxAdapterError> {
        if config.model_path.is_empty() {
            return Err(OnnxAdapterError::InvalidConfig(
                "model_path must be non-empty".into(),
            ));
        }
        Err(OnnxAdapterError::NotLinked(format!(
            "model_id={} path={} int8={}",
            model_id.0, config.model_path, config.prefer_int8_cpu
        )))
    }

    /// Test-only constructor that skips the link gate (still errors on forecast).
    #[must_use]
    pub fn unbound_for_tests(model_id: ModelId, config: OnnxRuntimeConfig) -> Self {
        Self { model_id, config }
    }

    /// Config snapshot.
    #[must_use]
    pub fn config(&self) -> &OnnxRuntimeConfig {
        &self.config
    }
}

impl TsfmRuntime for OnnxTsfmRuntime {
    fn model_id(&self) -> &ModelId {
        &self.model_id
    }

    fn forecast(
        &self,
        request: &TsfmForecastRequest,
    ) -> Result<TsfmForecastResponse, TsfmRuntimeError> {
        if request.horizon == 0 {
            return Err(TsfmRuntimeError::InvalidRequest(
                "horizon must be > 0".into(),
            ));
        }
        Err(TsfmRuntimeError::Stub(format!(
            "onnx forecast deferred (model={}, path={})",
            self.model_id.0, self.config.model_path
        )))
    }

    fn embed(&self, _context: &[f64]) -> Result<Vec<f32>, TsfmRuntimeError> {
        Err(TsfmRuntimeError::Stub(
            "onnx embed deferred — no session loaded".into(),
        ))
    }
}

/// Document that INT8 CPU is the preferred production path once linked.
#[must_use]
pub fn preferred_onnx_floor_config(model_path: impl Into<String>) -> OnnxRuntimeConfig {
    OnnxRuntimeConfig {
        model_path: model_path.into(),
        prefer_int8_cpu: true,
        intra_op_threads: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::SeriesModality;

    #[test]
    fn try_load_requires_path_and_fails_closed() {
        let err = OnnxTsfmRuntime::try_load(ModelId::new("m"), OnnxRuntimeConfig::default())
            .err()
            .unwrap();
        assert!(matches!(err, OnnxAdapterError::InvalidConfig(_)));

        let err = OnnxTsfmRuntime::try_load(
            ModelId::new("m"),
            preferred_onnx_floor_config("/models/financial-bar.onnx"),
        )
        .err()
        .unwrap();
        assert!(matches!(err, OnnxAdapterError::NotLinked(_)));
    }

    #[test]
    fn unbound_forecast_stub_errors() {
        let rt = OnnxTsfmRuntime::unbound_for_tests(
            ModelId::new("financial-bar"),
            preferred_onnx_floor_config("/models/financial-bar.onnx"),
        );
        let req = TsfmForecastRequest {
            asset: "AAPL".into(),
            modality: SeriesModality::Univariate,
            context: vec![1.0],
            horizon: 2,
        };
        assert!(matches!(rt.forecast(&req), Err(TsfmRuntimeError::Stub(_))));
    }
}
