//! Data-lineage contracts with eagerly computed invalidation identity.

use serde::{Deserialize, Serialize};

/// Stable transform id.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransformId(pub String);

impl TransformId {
    /// Construct a transform id.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Data lineage metadata pinned to upstream records, code, and artifacts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lineage {
    /// Output record identifier.
    pub record_id: [u8; 32],
    /// Upstream immutable record ids.
    pub upstream_record_ids: Vec<[u8; 32]>,
    /// Transform id.
    pub transform_id: TransformId,
    /// Transform version.
    pub transform_version: String,
    /// Digest of the pinned artifact set used by the transform.
    pub pinned_digest: [u8; 32],
    /// Content-derived invalidation hash.
    pub invalidation_hash: [u8; 32],
}

impl Lineage {
    /// Construct lineage and compute its stable invalidation hash.
    pub fn new(
        record_id: [u8; 32],
        mut upstream_record_ids: Vec<[u8; 32]>,
        transform_id: TransformId,
        transform_version: impl Into<String>,
        pinned_digest: [u8; 32],
    ) -> Self {
        upstream_record_ids.sort_unstable();
        let transform_version = transform_version.into();
        let mut hasher = blake3::Hasher::new();
        hasher.update(&record_id);
        for upstream in &upstream_record_ids {
            hasher.update(upstream);
        }
        hasher.update(transform_id.0.as_bytes());
        hasher.update(transform_version.as_bytes());
        hasher.update(&pinned_digest);
        let invalidation_hash = *hasher.finalize().as_bytes();
        Self {
            record_id,
            upstream_record_ids,
            transform_id,
            transform_version,
            pinned_digest,
            invalidation_hash,
        }
    }
}
