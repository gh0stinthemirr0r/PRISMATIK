//! # prismatik-prismatik-analog-store
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — analog store contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use disclosures::{SensitivityReport, SurvivorshipWarning};
pub use query::{AnalogFilter, AnalogNeighbour, AnalogQuery, AnalogResult, DistanceMetric};
pub use store::{AnalogError, AnalogStore, DatasetVersionRef, EmbeddingBatch};

/// Store contracts.
pub mod store {
    /// Dataset version pointer.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct DatasetVersionRef {
        /// Version identifier.
        pub version_id: String,
    }

    /// Embedding batch.
    #[derive(Clone, Debug, PartialEq)]
    pub struct EmbeddingBatch {
        /// Entity identifiers.
        pub entity_ids: Vec<String>,
        /// Dense vectors aligned with entities.
        pub vectors: Vec<Vec<f32>>,
    }

    /// Analog store error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct AnalogError {
        /// Error message.
        pub message: String,
    }

    /// Analog storage contract.
    pub trait AnalogStore: Send + Sync {
        /// Insert embeddings batch.
        fn upsert(
            &self,
            dataset: &DatasetVersionRef,
            batch: &EmbeddingBatch,
        ) -> Result<(), AnalogError>;
    }
}

/// Query contracts.
pub mod query {
    /// Distance metric.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum DistanceMetric {
        /// Cosine distance.
        Cosine,
        /// Euclidean distance.
        Euclidean,
    }

    /// Query filter.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct AnalogFilter {
        /// Filter expression.
        pub expression: String,
    }

    /// Neighbor result.
    #[derive(Clone, Debug, PartialEq)]
    pub struct AnalogNeighbour {
        /// Entity id.
        pub entity_id: String,
        /// Distance score.
        pub distance: f32,
    }

    /// Query request.
    #[derive(Clone, Debug, PartialEq)]
    pub struct AnalogQuery {
        /// Query vector.
        pub vector: Vec<f32>,
        /// Number of neighbors.
        pub k: usize,
        /// Distance metric.
        pub metric: DistanceMetric,
        /// Optional filter.
        pub filter: Option<AnalogFilter>,
    }

    /// Query result with mandatory disclosures.
    #[derive(Clone, Debug, PartialEq)]
    pub struct AnalogResult {
        /// Neighbors.
        pub neighbours: Vec<AnalogNeighbour>,
        /// Sample size.
        pub sample_size: usize,
        /// Filters applied.
        pub filters_applied: Vec<String>,
        /// Survivorship warning.
        pub survivorship_warning: crate::disclosures::SurvivorshipWarning,
        /// Leave-N-out sensitivity summary.
        pub leave_n_out_sensitivity: crate::disclosures::SensitivityReport,
    }
}

/// Disclosure contracts.
pub mod disclosures {
    /// Survivorship warning status.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SurvivorshipWarning {
        /// No warning.
        None,
        /// Warning present.
        Present,
    }

    /// Sensitivity report.
    #[derive(Clone, Debug, PartialEq)]
    pub struct SensitivityReport {
        /// Relative sensitivity.
        pub sensitivity: f32,
    }
}
