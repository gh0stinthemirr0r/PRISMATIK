//! Errors raised by the determinism kernel.

use thiserror::Error;

/// Errors returned by `prismatik-determinism` APIs.
#[derive(Debug, Error)]
pub enum DeterminismError {
    /// An ambient-nondeterminism leak was detected at runtime. This is a
    /// programming error and should be treated as a panic in production.
    #[error("ambient nondeterminism leak: {0}")]
    AmbientLeak(String),

    /// An artifact's content hash did not match the pinned hash on load.
    /// Treated as a security event by callers, not an I/O error.
    #[error("artifact hash mismatch for {artifact_id}: expected {expected}, found {found}")]
    HashMismatch {
        /// The artifact whose hash was checked.
        artifact_id: crate::ArtifactId,
        /// The hash recorded in the manifest.
        expected: crate::ContentHash,
        /// The hash actually computed from the loaded bytes.
        found: crate::ContentHash,
    },

    /// A replayed run diverged from the recorded trace. The seed identifies
    /// the failing run for local reproduction.
    #[error("seed replay divergence at seed {seed}")]
    ReplayDivergence {
        /// The seed that diverged.
        seed: u64,
    },

    /// Signature verification failed on an artifact or manifest.
    #[error("signature verification failed: {0}")]
    SignatureInvalid(String),

    /// An entropy operation was attempted on an exhausted or invalid stream.
    #[error("entropy error: {0}")]
    Entropy(#[from] crate::EntropyError),
}
