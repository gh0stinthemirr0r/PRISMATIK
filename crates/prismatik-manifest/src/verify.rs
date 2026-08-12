//! Standalone manifest signing and verification.
//!
//! Spec: `DOCS/spec/MANIFEST_SCHEMA.md` §9 / Wave 0 `P0-DK-09`, `P0-QM-02`.
//!
//! Two concerns live here:
//! - [`sign_manifest_ed25519`] stamps an Ed25519 signature over the canonical
//!   unsigned bytes of a manifest (invariant I3: same seed → same signature).
//! - [`StandaloneVerifier`] verifies a manifest offline, with no PRISMATIK
//!   installation, against schema version, signature, and (optional) trusted
//!   root / pinned-artifact policy. This is the externally-trustable surface
//!   referenced by `prismatik-cli verify` and the golden corpus.

use crate::canonical::unsigned_canonical_bytes;
use crate::manifest::Manifest;
use crate::types::ManifestV1;
use ed25519_dalek::{Signature, Signer, Verifier as DalekVerifier};
use prismatik_determinism::{
    signing_key_from_seed, ContentHash, DualSignature, Ed25519Signature, SignatureScheme,
    SigningIdentity,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// Verification failure raised by the standalone verifier.
#[derive(Debug, Error)]
pub enum VerificationError {
    /// Manifest JSON could not be parsed.
    #[error("manifest parse error: {0}")]
    Parse(String),
    /// Signature was malformed or could not be checked.
    #[error("signature error: {0}")]
    Signature(String),
    /// Schema version not accepted by this verifier.
    #[error("unsupported schema version: {0}")]
    Schema(String),
}

/// Verification report for standalone verifier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Overall verification result (all required checks passed).
    pub overall: bool,
    /// Schema version accepted by the verifier.
    pub schema_version_ok: bool,
    /// Signature verification result.
    pub signatures_ok: bool,
    /// Whether the Ed25519 signature specifically verified.
    pub signature_ed25519_ok: bool,
    /// Pinned-artifact / lineage linkage (transitional verifier accepts any).
    pub artifacts_ok: bool,
    /// Audit reference linkage (transitional verifier accepts any).
    pub audit_ok: bool,
    /// Human-readable issues.
    pub issues: Vec<String>,
    /// Instant supplied by the verification caller.
    pub verified_at: OffsetDateTime,
}

impl VerificationReport {
    /// Build a passing report.
    pub fn ok() -> Self {
        Self {
            overall: true,
            schema_version_ok: true,
            signatures_ok: true,
            signature_ed25519_ok: true,
            artifacts_ok: true,
            audit_ok: true,
            issues: Vec::new(),
            verified_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    /// Build a failing report from issue strings; per-check flags stay `false`.
    pub fn fail(issues: Vec<String>) -> Self {
        Self {
            overall: false,
            schema_version_ok: false,
            signatures_ok: false,
            signature_ed25519_ok: false,
            artifacts_ok: false,
            audit_ok: false,
            issues,
            verified_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
}

/// Stamp an Ed25519 signature over the canonical unsigned manifest bytes and
/// attach it to the manifest's `signature` field. Deterministic for a fixed
/// signing key (invariant I3).
pub fn sign_manifest_ed25519(
    manifest: &mut ManifestV1,
    key: &ed25519_dalek::SigningKey,
    identity: SigningIdentity,
) -> Result<(), VerificationError> {
    let bytes = unsigned_canonical_bytes(manifest).map_err(|e| {
        VerificationError::Signature(format!("canonical serialization failed: {e}"))
    })?;
    let sig: Signature = key.sign(&bytes);
    let public_key = key.verifying_key();
    manifest.signature = DualSignature {
        scheme: SignatureScheme::Ed25519Only,
        ed25519: Some(Ed25519Signature {
            public_key: public_key.to_bytes(),
            signature: sig.to_bytes(),
            identity,
        }),
        ml_dsa: None,
    };
    Ok(())
}

/// Offline manifest verifier.
///
/// `transitional()` accepts any non-empty signature envelope and any pinned
/// set — it checks schema version, parses the manifest, and confirms a
/// signature is present. Stricter construction (trusted root, required pinned
/// artifacts) lands alongside the CLI verify path.
#[derive(Debug, Default, Clone)]
pub struct StandaloneVerifier {
    /// When `true`, a present-but-unverified signature still passes. Used while
    /// the published-root trust store is being stood up.
    pub transitional: bool,
}

impl StandaloneVerifier {
    /// Transitional verifier: accepts any schema-1.0 manifest with a signature
    /// envelope present.
    pub fn transitional() -> Self {
        Self { transitional: true }
    }

    /// Verify a decoded manifest envelope.
    pub fn verify_manifest(&self, manifest: &Manifest) -> VerificationReport {
        let Manifest::V1(v1) = manifest;
        self.verify_v1(v1)
    }

    /// Verify a JSON manifest against optional trusted root / pinned policy.
    ///
    /// `trusted_root` and `pinned` are accepted positionally for forward
    /// compatibility with the strict verifier; the transitional verifier
    /// ignores them.
    pub fn verify_json(
        &self,
        json: &str,
        _trusted_root: Option<&[u8; 32]>,
        _pinned: Option<&dyn std::fmt::Debug>,
        now: OffsetDateTime,
    ) -> Result<VerificationReport, VerificationError> {
        let manifest: ManifestV1 =
            serde_json::from_str(json).map_err(|e| VerificationError::Parse(e.to_string()))?;
        let mut report = self.verify_v1(&manifest);
        report.verified_at = now;
        Ok(report)
    }

    fn verify_v1(&self, manifest: &ManifestV1) -> VerificationReport {
        let schema_version_ok = manifest.schema_supported();
        let signatures_ok = signature_present_and_valid(self.transitional, manifest);
        let mut issues = Vec::new();
        if !schema_version_ok {
            issues.push(format!(
                "unsupported schema_version: {}",
                manifest.schema_version
            ));
        }
        if !signatures_ok {
            issues.push("signature envelope missing or invalid".into());
        }
        let overall = schema_version_ok && signatures_ok;
        VerificationReport {
            overall,
            schema_version_ok,
            signatures_ok,
            signature_ed25519_ok: signatures_ok,
            artifacts_ok: true,
            audit_ok: true,
            issues,
            verified_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
}

fn signature_present_and_valid(transitional: bool, manifest: &ManifestV1) -> bool {
    let Some(ed) = manifest.signature.ed25519.as_ref() else {
        return false;
    };
    if transitional {
        return true;
    }
    // Strict path: recompute the canonical bytes and verify the signature.
    let Ok(bytes) = unsigned_canonical_bytes(manifest) else {
        return false;
    };
    let Ok(verifying) = ed25519_dalek::VerifyingKey::from_bytes(&ed.public_key) else {
        return false;
    };
    let Ok(sig) = Signature::from_slice(&ed.signature) else {
        return false;
    };
    verifying.verify(&bytes, &sig).is_ok()
}

/// Produce the unsigned canonical digest of a manifest (convenience re-export
/// of [`crate::canonical::unsigned_canonical_digest`] for verifier callers).
pub fn unsigned_canonical_digest(manifest: &ManifestV1) -> Result<ContentHash, serde_json::Error> {
    crate::canonical::unsigned_canonical_digest(manifest)
}

/// Re-export the deterministic signing-key constructor so callers can import
/// everything they need from `prismatik_manifest`.
pub fn signing_key(seed: [u8; 32]) -> ed25519_dalek::SigningKey {
    signing_key_from_seed(seed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        DeterminismBlock, ManifestBuilder, ManifestKind, ProducerInfo, ProducerProfile,
    };
    use prismatik_determinism::{signing_key_from_seed, PinnedArtifactSet, RunId};
    use time::macros::datetime;

    fn sample_manifest() -> ManifestV1 {
        ManifestBuilder::new(
            "test-m1",
            RunId::from_bytes([1u8; 16]),
            ManifestKind::DataIngest,
            datetime!(2026-07-26 14:32:00 UTC),
            ProducerInfo {
                prismatik_version: "0.1.0".into(),
                build_hash: "test".into(),
                build_attestation: None,
                profile: ProducerProfile::Desktop,
            },
            DeterminismBlock {
                root_seed: 1,
                clock_kind: "frozen".into(),
                clock_start: Some(datetime!(2026-07-26 14:32:00 UTC)),
                clock_end: None,
                clock_tick_step_micros: None,
                entropy_streams: Vec::new(),
                thread_count: 1,
                rayon_parallel: false,
                libc_overrides: Vec::new(),
                trace_digest: ContentHash::from_bytes(b"trace"),
            },
        )
        .pinned(PinnedArtifactSet::default())
        .lineage("test.transform", "1.0.0")
        .build_unsigned()
    }

    #[test]
    fn unsigned_manifest_fails_verification() {
        let v = StandaloneVerifier::transitional();
        let m = sample_manifest();
        assert!(!v.verify_v1(&m).overall);
    }

    #[test]
    fn signed_manifest_verifies_transitionally_and_strictly() {
        let key = signing_key_from_seed([7u8; 32]);
        let mut m = sample_manifest();
        sign_manifest_ed25519(
            &mut m,
            &key,
            SigningIdentity {
                kind: prismatik_determinism::SigningIdentityKind::Ci,
                id: "test".into(),
                signed_at: None,
            },
        )
        .unwrap();

        let transitional = StandaloneVerifier { transitional: true };
        assert!(transitional.verify_v1(&m).overall);

        let strict = StandaloneVerifier {
            transitional: false,
        };
        assert!(strict.verify_v1(&m).overall);

        // Round-trip through JSON and verify via verify_json.
        let json = serde_json::to_string(&m).unwrap();
        let report = strict
            .verify_json(&json, None, None, datetime!(2026-07-26 14:32:00 UTC))
            .unwrap();
        assert!(report.overall);
        assert!(report.schema_version_ok);
        assert!(report.signatures_ok);
    }

    #[test]
    fn tampered_signature_fails_strict_verification() {
        let key = signing_key_from_seed([7u8; 32]);
        let mut m = sample_manifest();
        sign_manifest_ed25519(
            &mut m,
            &key,
            SigningIdentity {
                kind: prismatik_determinism::SigningIdentityKind::Ci,
                id: "test".into(),
                signed_at: None,
            },
        )
        .unwrap();
        // Corrupt the signature bytes.
        if let Some(ed) = m.signature.ed25519.as_mut() {
            ed.signature[0] ^= 0xff;
        }
        let strict = StandaloneVerifier {
            transitional: false,
        };
        assert!(!strict.verify_v1(&m).signatures_ok);
        // Transitional still passes because it only checks presence.
        let transitional = StandaloneVerifier::transitional();
        assert!(transitional.verify_v1(&m).overall);
    }
}
