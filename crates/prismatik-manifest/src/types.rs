//! Reproducibility manifest types (schema v1.0).
//!
//! Spec: `DOCS/spec/MANIFEST_SCHEMA.md` / Wave 0 `P0-DK-09`.

use prismatik_determinism::{
    ArtifactRef, ContentHash, DualSignature, PinnedArtifactSet, RunId, SemanticVersion,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Manifest schema version string.
pub const SCHEMA_VERSION: &str = "1.0.0";

/// What kind of run produced the manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestKind {
    /// Strategy backtest.
    Backtest,
    /// Walk-forward evaluation.
    WalkForward,
    /// Monte Carlo simulation.
    MonteCarlo,
    /// TSFM forecast.
    TsfmForecast,
    /// Calibration fit.
    CalibrationFit,
    /// Live session.
    LiveSession,
    /// Data ingest pipeline.
    DataIngest,
    /// Trivial DST pipeline (P0-QM-01).
    DstReplay,
}

/// Producer metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProducerInfo {
    /// PRISMATIK version.
    pub prismatik_version: String,
    /// Build hash / commit.
    pub build_hash: String,
    /// Optional cosign attestation ref.
    #[serde(default)]
    pub build_attestation: Option<String>,
    /// Runtime profile.
    pub profile: ProducerProfile,
}

/// Runtime profile recorded in the manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProducerProfile {
    /// Desktop app.
    Desktop,
    /// Cloud worker.
    Cloud,
    /// Enterprise deployment.
    Enterprise,
}

/// Labeled entropy stream seed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntropyStreamSeed {
    /// Stream label.
    pub label: String,
    /// Derived seed.
    pub seed: u64,
}

/// Determinism block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterminismBlock {
    /// Root seed.
    pub root_seed: u64,
    /// Clock kind label.
    pub clock_kind: String,
    /// Clock start (RFC 3339) when applicable.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub clock_start: Option<OffsetDateTime>,
    /// Clock end.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub clock_end: Option<OffsetDateTime>,
    /// Clock tick step in microseconds.
    #[serde(default)]
    pub clock_tick_step_micros: Option<u64>,
    /// Entropy streams.
    pub entropy_streams: Vec<EntropyStreamSeed>,
    /// Thread count (required for Wave 3 DoD 7).
    pub thread_count: u32,
    /// Whether Rayon parallel reduction was used.
    pub rayon_parallel: bool,
    /// Libc override labels.
    #[serde(default)]
    pub libc_overrides: Vec<String>,
    /// TRACE digest.
    pub trace_digest: ContentHash,
}

/// Lineage block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageBlock {
    /// Transform identifier.
    pub transform_id: String,
    /// Transform version.
    pub transform_version: String,
    /// Optional StrategyIR hash.
    #[serde(default)]
    pub strategy_ir_hash: Option<ContentHash>,
    /// Upstream record ids.
    #[serde(default)]
    pub upstream_record_ids: Vec<String>,
    /// Invalidation hash including pinned set.
    pub invalidation_hash: ContentHash,
}

/// Audit reference block.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditBlock {
    /// Tree size at signing.
    pub tree_size: u64,
    /// Tree root hash.
    pub tree_root: ContentHash,
    /// Leaf position of the signing event (if any).
    #[serde(default)]
    pub leaf_position: Option<u64>,
    /// Leaf hash for inclusion.
    #[serde(default)]
    pub leaf_hash: Option<ContentHash>,
}

/// Reproducibility manifest v1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestV1 {
    /// Schema version.
    pub schema_version: String,
    /// Manifest id.
    pub manifest_id: String,
    /// Run id (hex of 16 bytes).
    pub run_id: String,
    /// Kind.
    pub kind: ManifestKind,
    /// Production timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub produced_at: OffsetDateTime,
    /// Producer.
    pub producer: ProducerInfo,
    /// Determinism.
    pub determinism: DeterminismBlock,
    /// Pinned artifacts.
    pub pinned_artifacts: PinnedArtifactSet,
    /// Lineage.
    pub lineage: LineageBlock,
    /// Audit reference.
    pub audit: AuditBlock,
    /// Dual signature over canonical bytes (excluding this field).
    #[serde(default)]
    pub signature: DualSignature,
    /// Optional extension map (ignored by core verifier).
    #[serde(default)]
    pub extensions: serde_json::Map<String, serde_json::Value>,
}

impl ManifestV1 {
    /// Supported schema major.minor prefix.
    pub fn schema_supported(&self) -> bool {
        self.schema_version.starts_with("1.0.")
            || self.schema_version == "1.0"
            || self.schema_version == SCHEMA_VERSION
    }

    /// Run id helper.
    pub fn run_id_from(run: &RunId) -> String {
        hex::encode(run.as_bytes())
    }
}

/// Builder for `ManifestV1`.
#[derive(Debug)]
pub struct ManifestBuilder {
    manifest: ManifestV1,
}

impl ManifestBuilder {
    /// Start a builder with required identity fields.
    pub fn new(
        manifest_id: impl Into<String>,
        run_id: RunId,
        kind: ManifestKind,
        produced_at: OffsetDateTime,
        producer: ProducerInfo,
        determinism: DeterminismBlock,
    ) -> Self {
        Self {
            manifest: ManifestV1 {
                schema_version: SCHEMA_VERSION.into(),
                manifest_id: manifest_id.into(),
                run_id: ManifestV1::run_id_from(&run_id),
                kind,
                produced_at,
                producer,
                determinism,
                pinned_artifacts: PinnedArtifactSet::default(),
                lineage: LineageBlock {
                    transform_id: "unset".into(),
                    transform_version: SemanticVersion::new(0, 0, 0).to_string(),
                    strategy_ir_hash: None,
                    upstream_record_ids: Vec::new(),
                    invalidation_hash: ContentHash::from([0u8; 32]),
                },
                audit: AuditBlock {
                    tree_size: 0,
                    tree_root: ContentHash::from([0u8; 32]),
                    leaf_position: None,
                    leaf_hash: None,
                },
                signature: DualSignature::default(),
                extensions: serde_json::Map::new(),
            },
        }
    }

    /// Set pinned artifacts and recompute lineage invalidation hash.
    pub fn pinned(mut self, pinned: PinnedArtifactSet) -> Self {
        let inv = pinned.stable_digest();
        self.manifest.pinned_artifacts = pinned;
        self.manifest.lineage.invalidation_hash = inv;
        self
    }

    /// Set lineage transform metadata.
    pub fn lineage(
        mut self,
        transform_id: impl Into<String>,
        transform_version: impl Into<String>,
    ) -> Self {
        self.manifest.lineage.transform_id = transform_id.into();
        self.manifest.lineage.transform_version = transform_version.into();
        // Recompute invalidation with current pinned set.
        let mut h = blake3::Hasher::new();
        let mut ids = self.manifest.lineage.upstream_record_ids.clone();
        ids.sort_unstable();
        for id in &ids {
            h.update(id.as_bytes());
        }
        h.update(self.manifest.lineage.transform_id.as_bytes());
        h.update(self.manifest.lineage.transform_version.as_bytes());
        h.update(self.manifest.pinned_artifacts.stable_digest().as_slice());
        self.manifest.lineage.invalidation_hash = ContentHash(h.finalize());
        self
    }

    /// Attach audit block.
    pub fn audit(mut self, audit: AuditBlock) -> Self {
        self.manifest.audit = audit;
        self
    }

    /// Finish without signing.
    pub fn build_unsigned(self) -> ManifestV1 {
        self.manifest
    }
}

/// Optional artifact ref helper for golden fixtures.
pub fn sample_artifact(id: &str) -> ArtifactRef {
    use prismatik_determinism::{ArtifactId, ArtifactKind, SemanticVersion};
    ArtifactRef {
        artifact_id: ArtifactId::new(id),
        kind: ArtifactKind::Calendar,
        version: SemanticVersion::new(1, 0, 0),
        content_hash: ContentHash::from_bytes(id.as_bytes()),
        signature: DualSignature::default(),
    }
}
