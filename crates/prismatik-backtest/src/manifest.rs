//! Signed reproducibility manifest for stub backtest runs (Wave 3 DoD #16 floor).
//!
//! Builds a schema-v1 [`ManifestV1`] with backtest-specific payload in
//! `extensions["io.prismatik.backtest"]` and signs with a test-only Ed25519 key.

use crate::{
    BacktestConfig, BacktestMetrics, BacktestResult, BacktestWindow, ExecutionAssumptions,
};
use prismatik_determinism::{
    signing_key_from_seed, ContentHash, PinnedArtifactSet, RunId, SigningIdentity,
    SigningIdentityKind,
};
use prismatik_manifest::{
    sign_manifest_ed25519, AuditBlock, DeterminismBlock, EntropyStreamSeed, ManifestBuilder,
    ManifestKind, ManifestV1, ProducerInfo, ProducerProfile,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Test-only Ed25519 seed for stub backtest manifests (CI / floor only).
pub const BACKTEST_TEST_SIGNING_SEED: [u8; 32] = [42u8; 32];

/// Backtest-specific extension payload (required DoD #16 fields).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BacktestExtension {
    /// Strategy IR id from config.
    pub strategy_id: String,
    /// Backtest window boundaries.
    pub window: BacktestWindow,
    /// Execution assumptions echoed from config.
    pub execution_assumptions: ExecutionAssumptions,
    /// BLAKE3 digest of canonical result metrics JSON.
    pub result_metrics_hash: ContentHash,
}

/// Bundles stub run output with its signed manifest.
#[derive(Clone, Debug, PartialEq)]
pub struct BacktestStubRun {
    /// Stub engine result.
    pub result: BacktestResult,
    /// Signed reproducibility manifest.
    pub manifest: ManifestV1,
}

/// Builds a signed backtest manifest from config + result.
#[derive(Clone, Debug)]
pub struct BacktestManifestBuilder {
    config: BacktestConfig,
    result: BacktestResult,
    produced_at: OffsetDateTime,
    manifest_id: String,
    run_id: RunId,
}

impl BacktestManifestBuilder {
    /// Create a builder from a completed stub run.
    pub fn new(
        config: BacktestConfig,
        result: BacktestResult,
        produced_at: OffsetDateTime,
    ) -> Self {
        let run_id = run_id_from_config(&config);
        let manifest_id = format!("backtest-stub-{}", hex::encode(run_id.as_bytes()));
        Self {
            config,
            result,
            produced_at,
            manifest_id,
            run_id,
        }
    }

    /// Canonical BLAKE3 digest of the result metrics block.
    pub fn result_metrics_hash(metrics: &BacktestMetrics) -> ContentHash {
        let json = serde_json::to_string(metrics).expect("BacktestMetrics serializes");
        ContentHash::from_bytes(json.as_bytes())
    }

    /// Build the backtest extension block (unsigned fields).
    pub fn extension(&self) -> BacktestExtension {
        BacktestExtension {
            strategy_id: self.config.strategy_id.clone(),
            window: self.config.window.clone(),
            execution_assumptions: self.config.execution.clone(),
            result_metrics_hash: Self::result_metrics_hash(&self.result.metrics),
        }
    }

    /// Produce a signed manifest (Ed25519-only transitional).
    pub fn build_signed(&self) -> Result<ManifestV1, String> {
        let key = signing_key_from_seed(BACKTEST_TEST_SIGNING_SEED);
        let identity = SigningIdentity {
            kind: SigningIdentityKind::Ci,
            id: "prismatik-backtest-stub".into(),
            signed_at: Some(self.produced_at.to_string()),
        };

        let trace_seed = blake3::hash(
            format!(
                "{}:{}:{:?}",
                self.config.strategy_id, self.result.bars_processed, self.result.status
            )
            .as_bytes(),
        );
        let trace_digest = ContentHash(trace_seed);

        let strategy_ir_hash = ContentHash::from_bytes(self.config.strategy_id.as_bytes());

        let mut manifest = ManifestBuilder::new(
            self.manifest_id.clone(),
            self.run_id,
            ManifestKind::Backtest,
            self.produced_at,
            ProducerInfo {
                prismatik_version: env!("CARGO_PKG_VERSION").into(),
                build_hash: "backtest-stub".into(),
                build_attestation: None,
                profile: ProducerProfile::Desktop,
            },
            DeterminismBlock {
                root_seed: 8675309,
                clock_kind: "utc_bar_step".into(),
                clock_start: Some(self.config.window.start),
                clock_end: Some(self.config.window.end),
                clock_tick_step_micros: Some(
                    self.config.bar_interval_seconds.saturating_mul(1_000_000),
                ),
                entropy_streams: vec![EntropyStreamSeed {
                    label: "backtest.engine".into(),
                    seed: 12345,
                }],
                thread_count: 1,
                rayon_parallel: false,
                libc_overrides: vec![],
                trace_digest,
            },
        )
        .pinned(PinnedArtifactSet::default())
        .lineage("backtest.stub", "1.0.0")
        .audit(AuditBlock {
            tree_size: 0,
            tree_root: ContentHash::from([0u8; 32]),
            leaf_position: None,
            leaf_hash: None,
        })
        .build_unsigned();

        manifest.lineage.strategy_ir_hash = Some(strategy_ir_hash);
        manifest.extensions.insert(
            "io.prismatik.backtest".into(),
            serde_json::to_value(self.extension()).map_err(|error| error.to_string())?,
        );

        sign_manifest_ed25519(&mut manifest, &key, identity).map_err(|error| error.to_string())?;
        Ok(manifest)
    }

    /// Produce pretty-printed signed manifest JSON.
    pub fn build_signed_json(&self) -> Result<String, String> {
        let manifest = self.build_signed()?;
        serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())
    }
}

fn run_id_from_config(config: &BacktestConfig) -> RunId {
    let mut h = blake3::Hasher::new();
    h.update(config.strategy_id.as_bytes());
    h.update(&config.window.start.unix_timestamp().to_le_bytes());
    h.update(&config.window.end.unix_timestamp().to_le_bytes());
    let digest = h.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    RunId::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BacktestEngine, FillModel};
    use prismatik_manifest::StandaloneVerifier;
    use time::macros::datetime;
    use time::Duration;

    fn sample_config() -> BacktestConfig {
        BacktestConfig {
            strategy_id: "strategy/demo-v1".into(),
            window: BacktestWindow {
                start: OffsetDateTime::UNIX_EPOCH,
                end: OffsetDateTime::UNIX_EPOCH + Duration::days(1),
            },
            starting_capital_micros: 100_000_000_000,
            execution: ExecutionAssumptions {
                fill_model: FillModel::NextOpen,
                slippage_bps: 5,
                commission_per_share_micros: 5_000,
                assignment_enabled: false,
            },
            bar_interval_seconds: 3_600,
        }
    }

    #[test]
    fn extension_carries_required_fields() {
        let config = sample_config();
        let engine = BacktestEngine::new(config.clone()).unwrap();
        let result = engine.run_stub();
        let at = datetime!(2026-07-26 14:32:00 UTC);
        let builder = BacktestManifestBuilder::new(config, result.clone(), at);
        let ext = builder.extension();
        assert_eq!(ext.strategy_id, "strategy/demo-v1");
        assert_eq!(ext.window.start, OffsetDateTime::UNIX_EPOCH);
        assert_eq!(ext.execution_assumptions.fill_model, FillModel::NextOpen);
        assert_eq!(
            ext.result_metrics_hash,
            BacktestManifestBuilder::result_metrics_hash(&result.metrics)
        );
    }

    #[test]
    fn signed_manifest_passes_standalone_verifier() {
        let config = sample_config();
        let engine = BacktestEngine::new(config.clone()).unwrap();
        let result = engine.run_stub();
        let at = datetime!(2026-07-26 14:32:00 UTC);
        let manifest = BacktestManifestBuilder::new(config, result, at)
            .build_signed()
            .unwrap();

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let verifier = StandaloneVerifier::transitional();
        let report = verifier.verify_json(&json, None, None, at).unwrap();
        assert!(report.overall, "{report:?}");
        assert!(report.signature_ed25519_ok);
        assert_eq!(manifest.kind, ManifestKind::Backtest);
    }
}
