//! Deferred IVF-PQ vector-index backend façade (`P55-QM-02` floor).
//!
//! Typed surface documenting how a production [`crate::AnalogStore`] would open
//! against a persistent embedding table with an IVF-PQ index. Typical upstream
//! backend is LanceDB; the workspace does **not** link that crate yet —
//! [`try_open_ivf_pq`] fails closed. Owned type names stay domain/algorithmic
//! (ADR-0032).

use thiserror::Error;

use crate::AnalogStore;

/// Errors from deferred persistent analog-store backends.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AnalogStoreError {
    /// Persistent vector backend is not linked in this build.
    #[error("analog store backend not linked at P55-QM-02 floor")]
    BackendNotLinked,
    /// Index / table configuration rejected before open.
    #[error("invalid IVF-PQ index config: {0}")]
    InvalidConfig(String),
}

/// IVF-PQ index parameters for a deferred persistent open (`P55-QM-02`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IvfPqIndexConfig {
    /// Dataset URI (directory or object-store path).
    pub uri: String,
    /// Table / dataset name holding the embedding column.
    pub table_name: String,
    /// IVF partition count (`num_partitions`).
    pub num_partitions: u32,
    /// PQ sub-vector count (`num_sub_vectors`).
    pub num_sub_vectors: u32,
    /// Expected embedding dimensionality (must match [`crate::EMBEDDING_DIMENSIONS`] at link time).
    pub dimension: usize,
}

impl IvfPqIndexConfig {
    /// Build a config with common IVF-PQ defaults for the in-memory embedding width.
    #[must_use]
    pub fn ivf_pq(
        uri: impl Into<String>,
        table_name: impl Into<String>,
        num_partitions: u32,
        num_sub_vectors: u32,
    ) -> Self {
        Self {
            uri: uri.into(),
            table_name: table_name.into(),
            num_partitions,
            num_sub_vectors,
            dimension: crate::EMBEDDING_DIMENSIONS,
        }
    }

    /// Validate non-empty URI/table and positive IVF-PQ parameters.
    pub fn validate(&self) -> Result<(), AnalogStoreError> {
        if self.uri.trim().is_empty() {
            return Err(AnalogStoreError::InvalidConfig(
                "uri must be non-empty".into(),
            ));
        }
        if self.table_name.trim().is_empty() {
            return Err(AnalogStoreError::InvalidConfig(
                "table_name must be non-empty".into(),
            ));
        }
        if self.num_partitions == 0 {
            return Err(AnalogStoreError::InvalidConfig(
                "num_partitions must be > 0".into(),
            ));
        }
        if self.num_sub_vectors == 0 {
            return Err(AnalogStoreError::InvalidConfig(
                "num_sub_vectors must be > 0".into(),
            ));
        }
        if self.dimension == 0 {
            return Err(AnalogStoreError::InvalidConfig(
                "dimension must be > 0".into(),
            ));
        }
        Ok(())
    }
}

/// Attempt to open an IVF-PQ-backed [`AnalogStore`].
///
/// Always fails closed until a persistent vector backend is a workspace
/// dependency with cargo-vet coverage. Valid configs still return
/// [`AnalogStoreError::BackendNotLinked`].
pub fn try_open_ivf_pq(config: &IvfPqIndexConfig) -> Result<AnalogStore, AnalogStoreError> {
    config.validate()?;
    let _ = config;
    Err(AnalogStoreError::BackendNotLinked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_open_ivf_pq_fails_closed_backend_not_linked() {
        let config = IvfPqIndexConfig::ivf_pq("file:///tmp/analogs", "embeddings", 256, 16);
        let err = try_open_ivf_pq(&config).expect_err("must fail closed");
        assert_eq!(err, AnalogStoreError::BackendNotLinked);
    }

    #[test]
    fn try_open_ivf_pq_rejects_empty_uri_before_link_gate() {
        let mut config = IvfPqIndexConfig::ivf_pq("file:///tmp/analogs", "embeddings", 256, 16);
        config.uri = String::new();
        let err = try_open_ivf_pq(&config).expect_err("empty uri");
        assert!(matches!(err, AnalogStoreError::InvalidConfig(_)));
    }

    #[test]
    fn ivf_pq_index_config_defaults_dimension() {
        let config = IvfPqIndexConfig::ivf_pq("./data", "analog_hits", 64, 8);
        assert_eq!(config.dimension, crate::EMBEDDING_DIMENSIONS);
        assert_eq!(config.num_partitions, 64);
        assert_eq!(config.num_sub_vectors, 8);
        config.validate().expect("valid");
    }
}
