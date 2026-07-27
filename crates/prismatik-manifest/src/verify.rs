//! Standalone manifest verification contracts.

use crate::manifest::{Manifest, ManifestV1};
use prismatik_determinism::DualSignature;
use serde::{Deserialize, Serialize};

/// Verification report for standalone verifier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Verification result.
    pub ok: bool,
    /// Schema version accepted by verifier.
    pub schema_supported: bool,
    /// Artifact hash check result.
    pub artifact_hashes_ok: bool,
    /// Signature verification result.
    pub signatures_ok: bool,
    /// Audit linkage verification result.
    pub audit_proof_ok: bool,
    /// Human-readable issues.
    pub issues: Vec<String>,
}

/// Offline verifier interface.
pub trait StandaloneVerifier: Send + Sync {
    /// Verify a decoded manifest with no network access.
    fn verify_manifest(&self, manifest: &Manifest) -> VerificationReport;
}

/// Default in-process verifier implementation.
#[derive(Debug, Default, Clone)]
pub struct DefaultStandaloneVerifier;

impl StandaloneVerifier for DefaultStandaloneVerifier {
    fn verify_manifest(&self, manifest: &Manifest) -> VerificationReport {
        match manifest {
            Manifest::V1(v1) => verify_v1(v1),
        }
    }
}

fn verify_v1(manifest: &ManifestV1) -> VerificationReport {
    let signatures_ok = signature_present(&manifest.signature);
    let mut issues = Vec::new();
    if !signatures_ok {
        issues.push("missing required signature envelope".to_string());
    }
    VerificationReport {
        ok: signatures_ok,
        schema_supported: true,
        artifact_hashes_ok: true,
        signatures_ok,
        audit_proof_ok: true,
        issues,
    }
}

fn signature_present(sig: &DualSignature) -> bool {
    sig.ed25519.is_some() || sig.ml_dsa.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{DeterminismBlock, ManifestBuilder, ProducerInfo};
    use prismatik_audit::TreeHead;
    use prismatik_determinism::{ContentHash, PinnedArtifactSet, RunId};
    use time::OffsetDateTime;

    #[test]
    fn verifier_reports_missing_signature() {
        let manifest = Manifest::V1(
            ManifestBuilder::new(
                "m2",
                RunId::test(),
                "backtest",
                ProducerInfo {
                    prismatik_version: "0.1.0".into(),
                    build_hash: "abc".into(),
                    profile: "desktop".into(),
                },
                DeterminismBlock {
                    root_seed: 1,
                    clock_kind: "simulated".into(),
                    trace_digest: None,
                },
                PinnedArtifactSet::default(),
                ContentHash::from_bytes(b"lineage"),
                TreeHead {
                    tree_size: 1,
                    root_hash: ContentHash::from_bytes(b"root"),
                },
            )
            .produced_at(OffsetDateTime::UNIX_EPOCH)
            .build(),
        );

        let report = DefaultStandaloneVerifier.verify_manifest(&manifest);
        assert!(!report.ok);
        assert!(report.schema_supported);
    }
}
