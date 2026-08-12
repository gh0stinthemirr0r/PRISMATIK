//! # prismatik-manifest
//!
//! Layer 1 — Kernel extension.
//!
//! Reproducibility manifests (invariant I3). Every deterministic run produces a
//! signed manifest pinning its inputs, determinism context, lineage, and audit
//! head; the standalone verifier checks one offline with no PRISMATIK install.
//!
//! Spec: `DOCS/spec/MANIFEST_SCHEMA.md` / Wave 0 `P0-DK-09`, `P0-QM-02`.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use bundle::{BundleError, ResearchBundle};
pub use canonical::{canonical_json, unsigned_canonical_bytes, unsigned_canonical_digest};
pub use golden::{build_golden_pair, GOLDEN_DATA_INGEST, GOLDEN_DST_REPLAY};
pub use manifest::{Manifest, ManifestBuilder, ManifestV1};
pub use types::{
    sample_artifact, AuditBlock, DeterminismBlock, EntropyStreamSeed, LineageBlock, ManifestKind,
    ProducerInfo, ProducerProfile, SCHEMA_VERSION,
};
pub use verify::{
    sign_manifest_ed25519, signing_key, unsigned_canonical_digest as verify_canonical_digest,
    StandaloneVerifier, VerificationError, VerificationReport,
};

/// Research bundle contract.
pub mod bundle;
/// Canonical JSON serialization (sorted keys, stable bytes).
pub mod canonical;
/// Golden manifest corpus harness (`P0-QM-02`).
pub mod golden;
/// Manifest versioning envelope and v1 re-exports.
pub mod manifest;
/// Reproducibility manifest schema types (v1.0).
pub mod types;
/// Standalone signing and verification.
pub mod verify;
