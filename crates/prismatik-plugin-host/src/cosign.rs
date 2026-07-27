//! Cosign verification at plugin load (`P4-SS-04` residual floor).
//!
//! Typed request / report only — the workspace does **not** link a Sigstore /
//! cosign client. [`try_verify_at_load`] always fails closed.
//!
//! # Author-ops residual
//!
//! Real cosign keyless verify (Fulcio / Rekor), digest pinning against a
//! release identity, and load-time refusal of unsigned plugins remain
//! **author-ops** (`P4-SS-04` residual). Callers must treat
//! [`CosignError::NotLinked`] as deny-by-default until that wiring lands.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Inputs for a load-time cosign verify of a plugin image / artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CosignVerifyRequest {
    /// Image or artifact reference (OCI tag, path, or publisher URI).
    pub image_ref: String,
    /// Expected content digest (e.g. `sha256:…`).
    pub digest: String,
}

/// Outcome of a cosign verify attempt (or the turbo stub report).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CosignVerifyReport {
    /// Whether the artifact was cryptographically verified.
    ///
    /// Always `false` on the turbo floor — verification is not linked.
    pub verified: bool,
    /// Residual documenting author-ops work still required.
    pub residual: String,
}

/// Errors from load-time cosign verification.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum CosignError {
    /// No cosign / Sigstore client is linked in this build (author-ops residual).
    #[error("cosign verify at load not linked (P4-SS-04 author-ops residual): {0}")]
    NotLinked(String),
}

/// Attempt cosign verification before loading a plugin.
///
/// Always fails with [`CosignError::NotLinked`] — fail closed until real
/// cosign is wired (author-ops / `P4-SS-04` residual).
pub fn try_verify_at_load(req: &CosignVerifyRequest) -> Result<CosignVerifyReport, CosignError> {
    Err(CosignError::NotLinked(format!(
        "image_ref={} digest={} — real cosign is author-ops (P4-SS-04 residual)",
        req.image_ref, req.digest
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_verify_at_load_fails_closed_not_linked() {
        let req = CosignVerifyRequest {
            image_ref: "ghcr.io/prismatik/demo-plugin:1.0.0".into(),
            digest: "sha256:deadbeef".into(),
        };
        let err = try_verify_at_load(&req).expect_err("must fail closed until cosign linked");
        assert!(matches!(err, CosignError::NotLinked(_)));
        assert!(err.to_string().contains("P4-SS-04"));
        assert!(err.to_string().contains("author-ops"));
    }
}
