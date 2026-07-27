//! Updater provenance verification floor (`P0-SS-07`).
//!
//! This module **refuses invalid artifacts**. Full cosign / SLSA production
//! signing (`P0-SS-06`) is deliberately stubbed behind clear traits — do not
//! treat [`StubCosignVerifier`] / [`StubSlsaProvenanceVerifier`] as production
//! Sigstore or SLSA Level 3.
//!
//! Author release-ops interface (examples, no production keys in-repo):
//! `scripts/cosign-verify-example.md`, `DOCS/waves/P0_SS_06_Release_Signing.md`.
//! SBOM inventory (not a defense): `scripts/generate-sbom.sh`.

use prismatik_determinism::{
    signing_key_from_seed, ContentHash, DualSignature, DualSignaturePolicy, SigningIdentity,
    SigningIdentityKind,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Accept / Reject decision for an update artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyDecision {
    /// Artifact may be installed.
    Accept,
    /// Artifact must not be installed.
    Reject(RejectReason),
}

/// Why verification rejected an artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    /// Artifact bytes do not match the attested digest.
    DigestMismatch,
    /// Signature / cosign check failed.
    SignatureInvalid,
    /// SLSA-style provenance attestation failed checks.
    ProvenanceInvalid,
    /// Required attestation fields missing.
    MissingAttestation,
}

/// Bytes presented to the updater for install.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateArtifact {
    /// Raw artifact payload.
    pub bytes: Vec<u8>,
}

impl UpdateArtifact {
    /// BLAKE3 content hash of the payload.
    pub fn digest(&self) -> ContentHash {
        ContentHash::from_bytes(&self.bytes)
    }
}

/// Provenance / signature bundle attached to an update (floor shape).
///
/// Production cosign + SLSA attestations should map into this envelope or
/// replace it behind the verifier traits below — generation is P0-SS-06.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceAttestation {
    /// Claimed BLAKE3 digest of the artifact (`blake3:…` via [`ContentHash`]).
    pub artifact_digest: ContentHash,
    /// Detached signature over the digest bytes (Ed25519 floor; cosign later).
    pub signature: DualSignature,
    /// Expected builder identity (SLSA predicate subject — stub allowlist).
    pub builder_id: String,
    /// Expected source repository URI.
    pub source_repo: String,
}

/// Errors from the floor verifier (also used as Reject mapping).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProvenanceVerifyError {
    /// Mapped reject.
    #[error("update refused: {0:?}")]
    Rejected(RejectReason),
}

/// Cosign / Sigstore signature verification (P0-SS-06 production path).
pub trait CosignVerifier {
    /// Verify signature material for `digest`.
    fn verify_signature(
        &self,
        digest: &ContentHash,
        attestation: &ProvenanceAttestation,
    ) -> Result<(), RejectReason>;
}

/// SLSA provenance predicate verification (P0-SS-06 production path).
pub trait SlsaProvenanceVerifier {
    /// Verify provenance claims for `digest`.
    fn verify_provenance(
        &self,
        digest: &ContentHash,
        attestation: &ProvenanceAttestation,
    ) -> Result<(), RejectReason>;
}

/// Offline floor: Ed25519 over digest bytes via [`DualSignature`].
///
/// Not cosign keyless / Rekor — clear stub boundary for P0-SS-06.
#[derive(Debug)]
pub struct StubCosignVerifier {
    /// Dual-signature policy (Wave 0 Ed25519-only exception by default).
    pub policy: DualSignaturePolicy,
}

impl Default for StubCosignVerifier {
    fn default() -> Self {
        Self {
            policy: DualSignaturePolicy::Ed25519OnlyException,
        }
    }
}

impl CosignVerifier for StubCosignVerifier {
    fn verify_signature(
        &self,
        digest: &ContentHash,
        attestation: &ProvenanceAttestation,
    ) -> Result<(), RejectReason> {
        if &attestation.artifact_digest != digest {
            return Err(RejectReason::DigestMismatch);
        }
        attestation
            .signature
            .verify(digest.as_slice(), self.policy)
            .map_err(|_| RejectReason::SignatureInvalid)
    }
}

/// Offline floor: require digest binding + allowlisted builder / repo strings.
///
/// Does **not** parse in-toto / SLSA JSON provenance documents.
#[derive(Clone, Debug)]
pub struct StubSlsaProvenanceVerifier {
    /// Allowed builder identity.
    pub expected_builder_id: String,
    /// Allowed source repository.
    pub expected_source_repo: String,
}

impl Default for StubSlsaProvenanceVerifier {
    fn default() -> Self {
        Self {
            expected_builder_id: "prismatik-ci/github-actions".into(),
            expected_source_repo: "https://github.com/mythos/prismatik".into(),
        }
    }
}

impl SlsaProvenanceVerifier for StubSlsaProvenanceVerifier {
    fn verify_provenance(
        &self,
        digest: &ContentHash,
        attestation: &ProvenanceAttestation,
    ) -> Result<(), RejectReason> {
        if &attestation.artifact_digest != digest {
            return Err(RejectReason::DigestMismatch);
        }
        if attestation.builder_id != self.expected_builder_id {
            return Err(RejectReason::ProvenanceInvalid);
        }
        if attestation.source_repo != self.expected_source_repo {
            return Err(RejectReason::ProvenanceInvalid);
        }
        Ok(())
    }
}

/// Composed updater verifier: digest match + signature + provenance.
#[derive(Debug)]
pub struct UpdaterProvenanceVerifier<C, S> {
    /// Signature half (cosign stub or future real impl).
    pub cosign: C,
    /// Provenance half (SLSA stub or future real impl).
    pub slsa: S,
}

impl Default for UpdaterProvenanceVerifier<StubCosignVerifier, StubSlsaProvenanceVerifier> {
    fn default() -> Self {
        Self {
            cosign: StubCosignVerifier::default(),
            slsa: StubSlsaProvenanceVerifier::default(),
        }
    }
}

impl<C: CosignVerifier, S: SlsaProvenanceVerifier> UpdaterProvenanceVerifier<C, S> {
    /// Verify `artifact` against `attestation`.
    ///
    /// Returns [`VerifyDecision::Reject`] on any failure — never Accepts
    /// tampered or unsigned payloads.
    pub fn verify(
        &self,
        artifact: &UpdateArtifact,
        attestation: Option<&ProvenanceAttestation>,
    ) -> VerifyDecision {
        let Some(attestation) = attestation else {
            return VerifyDecision::Reject(RejectReason::MissingAttestation);
        };
        let digest = artifact.digest();
        if let Err(reason) = self.cosign.verify_signature(&digest, attestation) {
            return VerifyDecision::Reject(reason);
        }
        if let Err(reason) = self.slsa.verify_provenance(&digest, attestation) {
            return VerifyDecision::Reject(reason);
        }
        VerifyDecision::Accept
    }

    /// Same as [`Self::verify`], but as `Result`.
    pub fn verify_or_refuse(
        &self,
        artifact: &UpdateArtifact,
        attestation: Option<&ProvenanceAttestation>,
    ) -> Result<(), ProvenanceVerifyError> {
        match self.verify(artifact, attestation) {
            VerifyDecision::Accept => Ok(()),
            VerifyDecision::Reject(reason) => Err(ProvenanceVerifyError::Rejected(reason)),
        }
    }
}

/// Build a valid floor attestation for tests / local tooling (not release signing).
pub fn attest_for_tests(
    artifact: &UpdateArtifact,
    seed: [u8; 32],
    builder_id: impl Into<String>,
    source_repo: impl Into<String>,
) -> ProvenanceAttestation {
    let digest = artifact.digest();
    let key = signing_key_from_seed(seed);
    let signature = DualSignature::sign_ed25519_only(
        digest.as_slice(),
        &key,
        SigningIdentity {
            kind: SigningIdentityKind::Ci,
            id: "prismatik-security/updater-test".into(),
            signed_at: Some("2026-07-26T00:00:00Z".into()),
        },
    );
    ProvenanceAttestation {
        artifact_digest: digest,
        signature,
        builder_id: builder_id.into(),
        source_repo: source_repo.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_pair() -> (UpdateArtifact, ProvenanceAttestation) {
        let artifact = UpdateArtifact {
            bytes: b"prismatik-release-payload-v0.1.0".to_vec(),
        };
        let slsa = StubSlsaProvenanceVerifier::default();
        let attestation = attest_for_tests(
            &artifact,
            [42u8; 32],
            slsa.expected_builder_id.clone(),
            slsa.expected_source_repo.clone(),
        );
        (artifact, attestation)
    }

    #[test]
    fn accepts_valid_artifact() {
        let (artifact, attestation) = valid_pair();
        let verifier = UpdaterProvenanceVerifier::default();
        assert_eq!(
            verifier.verify(&artifact, Some(&attestation)),
            VerifyDecision::Accept
        );
    }

    /// P0-SS-07 negative test: updater must refuse a tampered payload.
    #[test]
    fn refuses_invalid_artifact() {
        let (mut artifact, attestation) = valid_pair();
        artifact.bytes.push(0xff); // tamper after attestation
        let verifier = UpdaterProvenanceVerifier::default();
        let decision = verifier.verify(&artifact, Some(&attestation));
        assert!(
            matches!(
                decision,
                VerifyDecision::Reject(
                    RejectReason::DigestMismatch | RejectReason::SignatureInvalid
                )
            ),
            "expected Reject on tampered payload, got {decision:?}"
        );
        assert!(verifier
            .verify_or_refuse(&artifact, Some(&attestation))
            .is_err());
    }

    #[test]
    fn refuses_missing_attestation() {
        let artifact = UpdateArtifact {
            bytes: b"unsigned".to_vec(),
        };
        let verifier = UpdaterProvenanceVerifier::default();
        assert_eq!(
            verifier.verify(&artifact, None),
            VerifyDecision::Reject(RejectReason::MissingAttestation)
        );
    }

    #[test]
    fn refuses_wrong_builder_identity() {
        let (artifact, mut attestation) = valid_pair();
        attestation.builder_id = "evil-builder".into();
        let verifier = UpdaterProvenanceVerifier::default();
        assert_eq!(
            verifier.verify(&artifact, Some(&attestation)),
            VerifyDecision::Reject(RejectReason::ProvenanceInvalid)
        );
    }
}
