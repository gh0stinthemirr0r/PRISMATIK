//! Manifest schema types and builder.

use prismatik_audit::TreeHead;
use prismatik_determinism::{ContentHash, DualSignature, PinnedArtifactSet, RunId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Top-level manifest enum for schema evolution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "schema_version", content = "payload")]
pub enum Manifest {
    /// Manifest schema v1.0.0.
    #[serde(rename = "1.0.0")]
    V1(ManifestV1),
}

/// Manifest v1 payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestV1 {
    /// Stable manifest identifier.
    pub manifest_id: String,
    /// Deterministic run identifier.
    pub run_id: RunId,
    /// Run kind.
    pub kind: String,
    /// Manifest production timestamp.
    pub produced_at: OffsetDateTime,
    /// Producer metadata.
    pub producer: ProducerInfo,
    /// Determinism block.
    pub determinism: DeterminismBlock,
    /// Pinned artifact set.
    pub pinned_artifacts: PinnedArtifactSet,
    /// Lineage metadata hash.
    pub lineage_hash: ContentHash,
    /// Referenced audit tree head.
    pub audit_head: TreeHead,
    /// Dual signature.
    pub signature: DualSignature,
}

/// Manifest producer details.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProducerInfo {
    /// Prismatik version.
    pub prismatik_version: String,
    /// Build hash.
    pub build_hash: String,
    /// Build profile.
    pub profile: String,
}

/// Determinism metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterminismBlock {
    /// Root seed.
    pub root_seed: u64,
    /// Clock kind.
    pub clock_kind: String,
    /// Optional trace digest.
    pub trace_digest: Option<String>,
}

/// Builder for `ManifestV1`.
#[derive(Clone, Debug)]
pub struct ManifestBuilder {
    inner: ManifestV1,
}

impl ManifestBuilder {
    /// Start a new v1 manifest builder.
    pub fn new(
        manifest_id: impl Into<String>,
        run_id: RunId,
        kind: impl Into<String>,
        producer: ProducerInfo,
        determinism: DeterminismBlock,
        pinned_artifacts: PinnedArtifactSet,
        lineage_hash: ContentHash,
        audit_head: TreeHead,
    ) -> Self {
        Self {
            inner: ManifestV1 {
                manifest_id: manifest_id.into(),
                run_id,
                kind: kind.into(),
                produced_at: OffsetDateTime::UNIX_EPOCH,
                producer,
                determinism,
                pinned_artifacts,
                lineage_hash,
                audit_head,
                signature: DualSignature::default(),
            },
        }
    }

    /// Set manifest timestamp.
    pub fn produced_at(mut self, produced_at: OffsetDateTime) -> Self {
        self.inner.produced_at = produced_at;
        self
    }

    /// Set manifest signature.
    pub fn signature(mut self, signature: DualSignature) -> Self {
        self.inner.signature = signature;
        self
    }

    /// Build final manifest.
    pub fn build(self) -> ManifestV1 {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_determinism::ClockKind;

    #[test]
    fn builder_creates_manifest() {
        let m = ManifestBuilder::new(
            "m1",
            RunId::test(),
            "backtest",
            ProducerInfo {
                prismatik_version: "0.1.0".into(),
                build_hash: "abc".into(),
                profile: "desktop".into(),
            },
            DeterminismBlock {
                root_seed: 42,
                clock_kind: format!("{:?}", ClockKind::Simulated),
                trace_digest: None,
            },
            PinnedArtifactSet::default(),
            ContentHash::from_bytes(b"lineage"),
            TreeHead {
                tree_size: 0,
                root_hash: ContentHash::from_bytes(b"root"),
            },
        )
        .produced_at(OffsetDateTime::UNIX_EPOCH)
        .build();

        assert_eq!(m.manifest_id, "m1");
        assert_eq!(m.kind, "backtest");
    }
}
