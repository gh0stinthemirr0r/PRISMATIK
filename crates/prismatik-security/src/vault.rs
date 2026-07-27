//! Vault / KMS secret reference floor (`P8-SS-03`).
//!
//! Typed refs only — no HashiCorp Vault, cloud KMS, or HSM client.
//!
//! # Author-ops residual
//!
//! Live resolve (Vault HTTP, AWS KMS, HSM PKCS#11), auth methods, lease renewal,
//! and rotation remain **author-ops**. [`try_resolve_secret`] always returns
//! [`SecretResolveError::NotLinked`] so callers fail closed until that wiring
//! lands.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Logical reference to a secret in a Vault-style KV mount.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VaultSecretRef {
    /// Secrets engine mount (e.g. `secret`).
    pub mount: String,
    /// Path within the mount (e.g. `prismatik/prod/db`).
    pub path: String,
    /// Field key inside the secret payload (e.g. `password`).
    pub key: String,
}

/// Errors from attempting to materialize a [`VaultSecretRef`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SecretResolveError {
    /// No live Vault/KMS/HSM backend is linked in this build (author-ops residual).
    #[error("vault/kms resolve not linked (author-ops residual): {0}")]
    NotLinked(String),
}

/// Attempt to resolve `r` to secret bytes.
///
/// Always fails with [`SecretResolveError::NotLinked`] — documents the
/// author-ops residual for live Vault/KMS/HSM integration.
pub fn try_resolve_secret(r: &VaultSecretRef) -> Result<Vec<u8>, SecretResolveError> {
    Err(SecretResolveError::NotLinked(format!(
        "mount={} path={} key={}",
        r.mount, r.path, r.key
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_resolve_secret_is_not_linked() {
        let r = VaultSecretRef {
            mount: "secret".into(),
            path: "prismatik/db".into(),
            key: "password".into(),
        };
        let err = try_resolve_secret(&r).expect_err("must stay unlinked");
        assert!(matches!(err, SecretResolveError::NotLinked(_)));
        assert!(err.to_string().contains("author-ops residual"));
        assert!(err.to_string().contains("mount=secret"));
        assert!(err.to_string().contains("path=prismatik/db"));
    }

    #[test]
    fn try_resolve_secret_never_returns_bytes() {
        // Fail-closed floor: any ref shape must refuse materialization.
        for (mount, path, key) in [
            ("secret", "a", "k"),
            ("kv", "prod/api", "token"),
            ("transit", "keys/signing", "ciphertext"),
        ] {
            let r = VaultSecretRef {
                mount: mount.into(),
                path: path.into(),
                key: key.into(),
            };
            assert!(
                matches!(
                    try_resolve_secret(&r),
                    Err(SecretResolveError::NotLinked(_))
                ),
                "ref {mount}/{path}/{key} must fail closed"
            );
        }
    }
}
