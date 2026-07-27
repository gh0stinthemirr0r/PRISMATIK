//! Data lineage contracts.

use prismatik_determinism::{ContentHash, PinnedArtifactSet};
use serde::{Deserialize, Serialize};

/// Stable transform id.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TransformId(pub String);

/// Data lineage metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lineage {
    /// Upstream immutable record ids.
    pub upstream_record_ids: Vec<String>,
    /// Transform id.
    pub transform_id: TransformId,
    /// Transform version.
    pub transform_version: String,
}

impl Lineage {
    /// Compute invalidation hash including pinned artifact set.
    pub fn invalidation_hash(&self, pinned: &PinnedArtifactSet) -> ContentHash {
        let mut h = blake3::Hasher::new();
        let mut ids = self.upstream_record_ids.clone();
        ids.sort_unstable();
        for id in ids {
            h.update(id.as_bytes());
        }
        h.update(self.transform_id.0.as_bytes());
        h.update(self.transform_version.as_bytes());
        h.update(pinned.stable_digest().as_slice());
        ContentHash(h.finalize())
    }
}
