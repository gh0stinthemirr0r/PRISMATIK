//! Content-addressed artifact store port.
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md` / Wave 0 `P0-DK-12`.

use crate::{ArtifactId, ArtifactRef, ContentHash, DualSignature};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors from artifact store operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ArtifactStoreError {
    /// Artifact id is not present.
    #[error("artifact not found: {0}")]
    NotFound(String),
    /// Bytes do not match the registered content hash (security event).
    #[error("artifact hash mismatch for {id}: expected {expected}, got {actual}")]
    HashMismatch {
        /// Artifact id.
        id: String,
        /// Expected hash.
        expected: String,
        /// Actual hash of provided bytes.
        actual: String,
    },
    /// Signature verification failed.
    #[error("artifact signature invalid for {0}")]
    SignatureInvalid(String),
}

/// Content-addressed artifact storage port.
pub trait ArtifactStore: Send + Sync {
    /// Put bytes under `artifact_ref`, verifying `content_hash` matches.
    fn put(&mut self, artifact_ref: ArtifactRef, bytes: Vec<u8>) -> Result<(), ArtifactStoreError>;

    /// Load bytes, verifying hash (and optional signature) on every load.
    fn get(&self, id: &ArtifactId) -> Result<StoredArtifact, ArtifactStoreError>;

    /// Return metadata only.
    fn meta(&self, id: &ArtifactId) -> Result<ArtifactRef, ArtifactStoreError>;
}

/// Bytes plus verified metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredArtifact {
    /// Artifact metadata.
    pub meta: ArtifactRef,
    /// Raw bytes.
    pub bytes: Vec<u8>,
}

/// In-memory store for tests and desktop cold-start caches.
#[derive(Debug, Default)]
pub struct MemoryArtifactStore {
    entries: HashMap<String, StoredArtifact>,
}

impl MemoryArtifactStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ArtifactStore for MemoryArtifactStore {
    fn put(&mut self, artifact_ref: ArtifactRef, bytes: Vec<u8>) -> Result<(), ArtifactStoreError> {
        let actual = ContentHash::from_bytes(&bytes);
        if actual != artifact_ref.content_hash {
            return Err(ArtifactStoreError::HashMismatch {
                id: artifact_ref.artifact_id.0.clone(),
                expected: artifact_ref.content_hash.to_string(),
                actual: actual.to_string(),
            });
        }
        self.entries.insert(
            artifact_ref.artifact_id.0.clone(),
            StoredArtifact {
                meta: artifact_ref,
                bytes,
            },
        );
        Ok(())
    }

    fn get(&self, id: &ArtifactId) -> Result<StoredArtifact, ArtifactStoreError> {
        let stored = self
            .entries
            .get(&id.0)
            .cloned()
            .ok_or_else(|| ArtifactStoreError::NotFound(id.0.clone()))?;
        let actual = ContentHash::from_bytes(&stored.bytes);
        if actual != stored.meta.content_hash {
            return Err(ArtifactStoreError::HashMismatch {
                id: id.0.clone(),
                expected: stored.meta.content_hash.to_string(),
                actual: actual.to_string(),
            });
        }
        // Signature presence is optional at this layer; when present and scheme
        // is not `None`, callers should verify via `crate::signature`.
        let _ = DualSignature::default();
        Ok(stored)
    }

    fn meta(&self, id: &ArtifactId) -> Result<ArtifactRef, ArtifactStoreError> {
        self.entries
            .get(&id.0)
            .map(|s| s.meta.clone())
            .ok_or_else(|| ArtifactStoreError::NotFound(id.0.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArtifactKind, SemanticVersion};

    #[test]
    fn put_rejects_hash_mismatch() {
        let mut store = MemoryArtifactStore::new();
        let meta = ArtifactRef {
            artifact_id: ArtifactId::new("a1"),
            kind: ArtifactKind::Calendar,
            version: SemanticVersion::new(1, 0, 0),
            content_hash: ContentHash::from_bytes(b"expected"),
            signature: DualSignature::default(),
        };
        let err = store.put(meta, b"other".to_vec()).unwrap_err();
        assert!(matches!(err, ArtifactStoreError::HashMismatch { .. }));
    }

    #[test]
    fn put_get_round_trip() {
        let mut store = MemoryArtifactStore::new();
        let bytes = b"calendar-bytes".to_vec();
        let meta = ArtifactRef {
            artifact_id: ArtifactId::new("cal-1"),
            kind: ArtifactKind::Calendar,
            version: SemanticVersion::new(1, 0, 0),
            content_hash: ContentHash::from_bytes(&bytes),
            signature: DualSignature::default(),
        };
        store.put(meta.clone(), bytes.clone()).unwrap();
        let loaded = store.get(&ArtifactId::new("cal-1")).unwrap();
        assert_eq!(loaded.bytes, bytes);
        assert_eq!(loaded.meta, meta);
    }
}
