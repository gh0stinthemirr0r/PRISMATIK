//! # prismatik-prismatik-tsfm
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — TSFM registry/runtime contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use calibration_link::{CalibrationMethod, CalibrationRecord};
pub use pretraining::{ContaminationVerdict, PretrainingRecord};
pub use registry::{ModelFamily, ModelId, ModelRegistration, ModelRegistry, RegistryError};
pub use runtime::{OutOfDomainVerdict, TsfmForecastRequest, TsfmForecastResponse, TsfmRuntime};
pub use tokenizer::{SeriesTokenizer, TokenizerBinding, TokenizerError};

/// Model registry contracts.
pub mod registry {
    /// Stable model id.
    pub type ModelId = String;

    /// Model family.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ModelFamily {
        /// Family id.
        pub id: String,
        /// Human-readable name.
        pub name: String,
    }

    /// Model registration.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ModelRegistration {
        /// Model id.
        pub model_id: ModelId,
        /// Family metadata.
        pub family: ModelFamily,
        /// Model digest.
        pub model_digest: String,
        /// Tokenizer digest.
        pub tokenizer_digest: String,
    }

    /// Registry error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RegistryError {
        /// Error message.
        pub message: String,
    }

    /// Model registry contract.
    pub trait ModelRegistry: Send + Sync {
        /// Register a model.
        fn register(&self, model: ModelRegistration) -> Result<(), RegistryError>;
    }
}

/// Tokenizer contracts.
pub mod tokenizer {
    /// Tokenizer binding metadata.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TokenizerBinding {
        /// Binding id.
        pub id: String,
        /// Vocabulary size.
        pub vocab_size: usize,
    }

    /// Tokenizer error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct TokenizerError {
        /// Error message.
        pub message: String,
    }

    /// Series tokenizer contract.
    pub trait SeriesTokenizer: Send + Sync {
        /// Encode numeric series.
        fn encode(&self, values: &[f64]) -> Result<Vec<i64>, TokenizerError>;
        /// Decode token ids.
        fn decode(&self, tokens: &[i64]) -> Result<Vec<f64>, TokenizerError>;
    }
}

/// Runtime contracts.
pub mod runtime {
    /// Forecast request.
    #[derive(Clone, Debug, PartialEq)]
    pub struct TsfmForecastRequest {
        /// Model id.
        pub model_id: String,
        /// Input series.
        pub series: Vec<f64>,
        /// Horizon length.
        pub horizon: usize,
    }

    /// Forecast response.
    #[derive(Clone, Debug, PartialEq)]
    pub struct TsfmForecastResponse {
        /// Predicted values.
        pub values: Vec<f64>,
    }

    /// Out-of-domain verdict.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum OutOfDomainVerdict {
        /// Safe to use.
        InDomain,
        /// Distribution shift detected.
        OutOfDomain,
    }

    /// TSFM runtime contract.
    pub trait TsfmRuntime: Send + Sync {
        /// Generate a forecast.
        fn forecast(&self, request: &TsfmForecastRequest) -> Result<TsfmForecastResponse, String>;
    }
}

/// Pretraining provenance contracts.
pub mod pretraining {
    /// Pretraining provenance record.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PretrainingRecord {
        /// Model id.
        pub model_id: String,
        /// Data cutoff timestamp.
        pub cutoff: String,
    }

    /// Contamination verdict.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ContaminationVerdict {
        /// Clean for evaluation.
        Clean,
        /// Contamination detected.
        Contaminated,
    }
}

/// Calibration linkage contracts.
pub mod calibration_link {
    /// Calibration method label.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum CalibrationMethod {
        /// Adaptive conformal inference.
        Aci,
        /// EnbPI.
        Enbpi,
        /// Split conformal.
        SplitConformal,
    }

    /// Calibration provenance record.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CalibrationRecord {
        /// Model id.
        pub model_id: String,
        /// Method.
        pub method: CalibrationMethod,
        /// Coverage as decimal string.
        pub realized_coverage: String,
    }
}
