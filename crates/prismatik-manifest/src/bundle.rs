//! Research bundle contract.

use crate::manifest::Manifest;
use thiserror::Error;

/// Bundle parse/validation error.
#[derive(Debug, Error)]
pub enum BundleError {
    /// Manifest not found.
    #[error("bundle missing manifest")]
    MissingManifest,
    /// Bundle content invalid.
    #[error("invalid bundle content: {0}")]
    InvalidContent(String),
}

/// Portable research bundle.
#[derive(Clone, Debug)]
pub struct ResearchBundle {
    /// Bundle manifest.
    pub manifest: Manifest,
    /// Referenced artifact payloads `(artifact_id, bytes)`.
    pub artifacts: Vec<(String, Vec<u8>)>,
}

impl ResearchBundle {
    /// Validate minimal bundle structure.
    pub fn validate(&self) -> Result<(), BundleError> {
        if self.artifacts.iter().any(|(id, _)| id.is_empty()) {
            return Err(BundleError::InvalidContent(
                "artifact identifier must be non-empty".into(),
            ));
        }
        Ok(())
    }
}
