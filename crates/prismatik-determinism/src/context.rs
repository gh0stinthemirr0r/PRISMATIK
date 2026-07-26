//! Determinism context and pinned artifact set.
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md` §1.1, §1.2,
//! `DOCS/spec/MANIFEST_SCHEMA.md` §3.

use crate::{ArtifactRef, Clock, Entropy};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Stable identifier for a deterministic run. Recorded in the manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId([u8; 16]);

impl RunId {
    /// A stable `RunId` for tests. Production callers should derive this
    /// from `Entropy` at run start.
    pub fn test() -> Self {
        Self([0u8; 16])
    }

    /// Construct from raw bytes.
    pub const fn from_bytes(b: [u8; 16]) -> Self {
        Self(b)
    }

    /// Return the underlying 16-byte representation.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// The complete set of external inputs whose change would alter results.
/// If it is not in here, it cannot influence a run. That is the invariant.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedArtifactSet {
    /// Pinned calendar artifact (one per active venue; we keep the union).
    #[serde(default)]
    pub calendar: Option<ArtifactRef>,
    /// Pinned symbology snapshot.
    #[serde(default)]
    pub symbology_snapshot: Option<ArtifactRef>,
    /// Pinned corporate-action ledger.
    #[serde(default)]
    pub corporate_actions: Option<ArtifactRef>,
    /// Pinned tokenizer codebooks.
    #[serde(default)]
    pub codebooks: Vec<ArtifactRef>,
    /// Pinned model weights.
    #[serde(default)]
    pub models: Vec<ArtifactRef>,
    /// Pinned indicator-kernel version. Stored by version, not by hash,
    /// because the kernel is compiled into the binary.
    #[serde(default)]
    pub indicator_kernel_version: Option<crate::SemanticVersion>,
    /// Pinned dataset versions (raw + curated + feature layers).
    #[serde(default)]
    pub dataset_versions: Vec<DatasetVersionRef>,
}

/// Reference to a dataset version (Lance or Parquet).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetVersionRef {
    /// Stable dataset identifier (e.g. `"curated_bar:daily"`).
    pub dataset_id: String,
    /// Lance/Parquet version number.
    pub version: u64,
    /// BLAKE3 of the dataset's content for verification on load.
    #[serde(default)]
    pub content_hash: Option<crate::ContentHash>,
}

impl PinnedArtifactSet {
    /// Compute a stable digest over the full set. Used by
    /// `Lineage::invalidation_hash` to detect any artifact rotation.
    pub fn stable_digest(&self) -> crate::ContentHash {
        let mut h = blake3::Hasher::new();
        if let Some(c) = &self.calendar {
            h.update(c.artifact_id.0.as_bytes());
            h.update(c.content_hash.as_slice());
        }
        if let Some(s) = &self.symbology_snapshot {
            h.update(s.artifact_id.0.as_bytes());
            h.update(s.content_hash.as_slice());
        }
        if let Some(ca) = &self.corporate_actions {
            h.update(ca.artifact_id.0.as_bytes());
            h.update(ca.content_hash.as_slice());
        }
        // Sort codebooks + models by artifact_id for stability.
        let mut codebooks: Vec<_> = self.codebooks.iter().collect();
        codebooks.sort_by_key(|a| &a.artifact_id.0);
        for a in codebooks {
            h.update(a.artifact_id.0.as_bytes());
            h.update(a.content_hash.as_slice());
        }
        let mut models: Vec<_> = self.models.iter().collect();
        models.sort_by_key(|a| &a.artifact_id.0);
        for a in models {
            h.update(a.artifact_id.0.as_bytes());
            h.update(a.content_hash.as_slice());
        }
        if let Some(v) = self.indicator_kernel_version {
            h.update(v.to_string().as_bytes());
        }
        let mut ds: Vec<_> = self.dataset_versions.iter().collect();
        ds.sort_by(|a, b| a.dataset_id.cmp(&b.dataset_id).then_with(|| a.version.cmp(&b.version)));
        for d in ds {
            h.update(d.dataset_id.as_bytes());
            h.update(&d.version.to_le_bytes());
            if let Some(h2) = d.content_hash {
                h.update(h2.as_slice());
            }
        }
        crate::ContentHash(h.finalize())
    }

    /// Has the run pinned any TSFM model? Used to short-circuit TSFM-dependent
    /// surfaces when no model is registered.
    pub fn has_models(&self) -> bool {
        !self.models.is_empty()
    }
}

/// The bundle every deterministic subsystem receives. `StrategyContext` from
/// the strategy crate wraps this plus scoped data access.
pub struct DeterminismContext {
    /// The clock — SystemClock, SimulatedClock, or FrozenClock.
    pub clock: Arc<dyn Clock>,
    /// The entropy stream for this run.
    pub entropy: Box<dyn Entropy>,
    /// Unique run id, recorded in the manifest.
    pub run_id: RunId,
    /// Every pinned artifact this run depends on. Populated at run start,
    /// frozen thereafter, copied verbatim into the manifest.
    pub pinned: PinnedArtifactSet,
}

impl std::fmt::Debug for DeterminismContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeterminismContext")
            .field("clock_kind", &self.clock.kind())
            .field("run_id", &self.run_id)
            .field("pinned_digest", &self.pinned.stable_digest().to_string())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FrozenClock;

    #[test]
    fn pinned_set_digest_is_stable() {
        let set_a = sample_set();
        let set_b = sample_set();
        assert_eq!(set_a.stable_digest(), set_b.stable_digest());
    }

    #[test]
    fn pinned_set_digest_changes_when_codebook_added() {
        let mut a = sample_set();
        let digest_before = a.stable_digest();
        a.codebooks.push(make_ref("cb-X"));
        assert_ne!(a.stable_digest(), digest_before);
    }

    #[test]
    fn pinned_set_digest_independent_of_insertion_order() {
        let mut a = sample_set();
        let mut b = a.clone();
        // Different insertion order, same set.
        a.codebooks = vec![make_ref("cb-1"), make_ref("cb-2"), make_ref("cb-3")];
        b.codebooks = vec![make_ref("cb-3"), make_ref("cb-2"), make_ref("cb-1")];
        assert_eq!(a.stable_digest(), b.stable_digest());
    }

    fn sample_set() -> PinnedArtifactSet {
        let mut set = PinnedArtifactSet::default();
        set.calendar = Some(make_ref("cal-1"));
        set.symbology_snapshot = Some(make_ref("sym-1"));
        set.codebooks = vec![make_ref("cb-A"), make_ref("cb-B")];
        set
    }

    fn make_ref(id: &str) -> ArtifactRef {
        use crate::{ArtifactId, ArtifactKind, ContentHash, SemanticVersion};
        ArtifactRef {
            artifact_id: ArtifactId::new(id),
            kind: ArtifactKind::TokenizerCodebook,
            version: SemanticVersion::new(1, 0, 0),
            content_hash: ContentHash::from_bytes(id.as_bytes()),
            signature: Default::default(),
        }
    }

    #[test]
    fn determinism_context_debug_format() {
        let ctx = DeterminismContext {
            clock: Arc::new(FrozenClock::new(time::OffsetDateTime::UNIX_EPOCH)),
            entropy: Box::new(crate::SplitEntropy::from_seed(42)),
            run_id: RunId::test(),
            pinned: PinnedArtifactSet::default(),
        };
        let s = format!("{:?}", ctx);
        assert!(s.contains("DeterminismContext"));
        assert!(s.contains("clock_kind"));
    }
}
