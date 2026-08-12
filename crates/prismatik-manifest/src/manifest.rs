//! Manifest schema — versioning wrapper plus re-export surface.
//!
//! The canonical v1 schema lives in [`crate::types`] (schema v1.0, richer
//! determinism / lineage / audit blocks). This module defines the top-level
//! [`Manifest`] versioning enum and re-exports the v1 types under the
//! historical `manifest::` paths so existing call sites keep working against
//! the single current contract.
//!
//! Spec: `DOCS/spec/MANIFEST_SCHEMA.md` / Wave 0 `P0-DK-09`.

use serde::{Deserialize, Serialize};

pub use crate::types::{
    sample_artifact, AuditBlock, DeterminismBlock, EntropyStreamSeed, LineageBlock,
    ManifestBuilder, ManifestKind, ManifestV1, ProducerInfo, ProducerProfile, SCHEMA_VERSION,
};

/// Top-level manifest envelope for schema evolution. The on-disk JSON form is
/// a plain `ManifestV1`; this enum is the in-memory handle used when a caller
/// needs to discriminate on schema version (e.g. the research bundle and the
/// standalone verifier).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Manifest {
    /// Manifest schema v1.0.0.
    V1(ManifestV1),
}

impl Manifest {
    /// Borrow the inner v1 payload.
    pub fn as_v1(&self) -> &ManifestV1 {
        let Manifest::V1(v1) = self;
        v1
    }
}

impl From<ManifestV1> for Manifest {
    fn from(v1: ManifestV1) -> Self {
        Self::V1(v1)
    }
}
