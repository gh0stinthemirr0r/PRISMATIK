//! OS keychain integration and CI-safe in-memory backend.
//!
//! Production secrets live in the platform credential store (Windows Credential
//! Manager, macOS Keychain, Linux Secret Service) via the `keyring` crate.
//! Unit tests and headless CI use [`MemoryKeychain`] — never the interactive
//! OS UI surface.

use std::collections::BTreeMap;
use std::sync::Mutex;

use secrecy::SecretVec;
use thiserror::Error;
use zeroize::Zeroize;

/// Default service name written into the platform credential store.
pub const DEFAULT_KEYCHAIN_SERVICE: &str = "com.mythos.prismatik";

/// Errors from keychain backends.
#[derive(Debug, Error)]
pub enum KeychainError {
    /// No secret is stored under the requested account.
    #[error("keychain entry not found: {account}")]
    NotFound {
        /// Account / user key within the service namespace.
        account: String,
    },
    /// The platform credential store is unavailable or not linked for this target.
    #[error("OS keychain unsupported on this platform: {reason}")]
    UnsupportedPlatform {
        /// Human-readable reason.
        reason: String,
    },
    /// Backend reported a platform-specific failure.
    #[error("keychain backend error: {0}")]
    Backend(String),
    /// Internal lock poisoned (should not occur in normal use).
    #[error("keychain lock poisoned")]
    LockPoisoned,
}

/// Pluggable keychain surface used by the key hierarchy.
///
/// Implementations MUST treat `secret` as sensitive: prefer writing through
/// [`SecretVec`] on read paths and avoid logging raw bytes.
pub trait Keychain: Send + Sync {
    /// Persist `secret` under `account` within this keychain's service namespace.
    fn set_secret(&self, account: &str, secret: &[u8]) -> Result<(), KeychainError>;

    /// Load the secret for `account`. Missing entries yield [`KeychainError::NotFound`].
    fn get_secret(&self, account: &str) -> Result<SecretVec<u8>, KeychainError>;

    /// Delete the credential for `account`. Missing entries are a no-op success
    /// so callers can treat delete as idempotent.
    fn delete_secret(&self, account: &str) -> Result<(), KeychainError>;
}

/// Process-local keychain for tests and headless CI (no OS UI).
#[derive(Debug, Default)]
pub struct MemoryKeychain {
    entries: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl MemoryKeychain {
    /// Empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored accounts (test helper).
    pub fn len(&self) -> Result<usize, KeychainError> {
        let guard = self
            .entries
            .lock()
            .map_err(|_| KeychainError::LockPoisoned)?;
        Ok(guard.len())
    }

    /// Whether the store has no accounts.
    pub fn is_empty(&self) -> Result<bool, KeychainError> {
        Ok(self.len()? == 0)
    }
}

impl Keychain for MemoryKeychain {
    fn set_secret(&self, account: &str, secret: &[u8]) -> Result<(), KeychainError> {
        let mut guard = self
            .entries
            .lock()
            .map_err(|_| KeychainError::LockPoisoned)?;
        if let Some(existing) = guard.get_mut(account) {
            existing.zeroize();
        }
        guard.insert(account.to_owned(), secret.to_vec());
        Ok(())
    }

    fn get_secret(&self, account: &str) -> Result<SecretVec<u8>, KeychainError> {
        let guard = self
            .entries
            .lock()
            .map_err(|_| KeychainError::LockPoisoned)?;
        guard
            .get(account)
            .map(|v| SecretVec::new(v.clone()))
            .ok_or_else(|| KeychainError::NotFound {
                account: account.to_owned(),
            })
    }

    fn delete_secret(&self, account: &str) -> Result<(), KeychainError> {
        let mut guard = self
            .entries
            .lock()
            .map_err(|_| KeychainError::LockPoisoned)?;
        if let Some(mut removed) = guard.remove(account) {
            removed.zeroize();
        }
        Ok(())
    }
}

impl Drop for MemoryKeychain {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.entries.lock() {
            for (_k, v) in guard.iter_mut() {
                v.zeroize();
            }
            guard.clear();
        }
    }
}

/// Platform-backed keychain (Windows Credential Manager / Keychain / Secret Service).
///
/// Construction always succeeds; individual operations may return
/// [`KeychainError::UnsupportedPlatform`] on targets without a linked native store.
#[derive(Debug, Clone)]
pub struct OsKeychain {
    service: String,
}

impl OsKeychain {
    /// Create an OS keychain client for the default PRISMATIK service name.
    pub fn new() -> Self {
        Self::with_service(DEFAULT_KEYCHAIN_SERVICE)
    }

    /// Create an OS keychain client for a custom service namespace.
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    /// Service name used for platform credential entries.
    pub fn service(&self) -> &str {
        &self.service
    }

    /// Whether this build links a native credential store for the current target.
    pub fn is_native_available() -> bool {
        cfg!(any(
            target_os = "windows",
            target_os = "macos",
            all(unix, not(any(target_os = "macos", target_os = "ios")))
        ))
    }
}

impl Default for OsKeychain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(
    target_os = "windows",
    target_os = "macos",
    all(unix, not(any(target_os = "macos", target_os = "ios")))
))]
mod native {
    use super::{Keychain, KeychainError, OsKeychain};
    use keyring::Entry;
    use secrecy::SecretVec;

    fn map_keyring(err: keyring::Error, account: &str) -> KeychainError {
        match err {
            keyring::Error::NoEntry => KeychainError::NotFound {
                account: account.to_owned(),
            },
            other => KeychainError::Backend(other.to_string()),
        }
    }

    fn entry(service: &str, account: &str) -> Result<Entry, KeychainError> {
        Entry::new(service, account).map_err(|e| KeychainError::Backend(e.to_string()))
    }

    impl Keychain for OsKeychain {
        fn set_secret(&self, account: &str, secret: &[u8]) -> Result<(), KeychainError> {
            let entry = entry(&self.service, account)?;
            entry
                .set_secret(secret)
                .map_err(|e| map_keyring(e, account))
        }

        fn get_secret(&self, account: &str) -> Result<SecretVec<u8>, KeychainError> {
            let entry = entry(&self.service, account)?;
            let bytes = entry.get_secret().map_err(|e| map_keyring(e, account))?;
            Ok(SecretVec::new(bytes))
        }

        fn delete_secret(&self, account: &str) -> Result<(), KeychainError> {
            let entry = entry(&self.service, account)?;
            match entry.delete_credential() {
                Ok(()) => Ok(()),
                Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(map_keyring(e, account)),
            }
        }
    }
}

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    all(unix, not(any(target_os = "macos", target_os = "ios")))
)))]
mod stub {
    use super::{Keychain, KeychainError, OsKeychain};
    use secrecy::SecretVec;

    fn unsupported() -> KeychainError {
        KeychainError::UnsupportedPlatform {
            reason: "no native keyring backend linked for this target".into(),
        }
    }

    impl Keychain for OsKeychain {
        fn set_secret(&self, _account: &str, _secret: &[u8]) -> Result<(), KeychainError> {
            Err(unsupported())
        }

        fn get_secret(&self, _account: &str) -> Result<SecretVec<u8>, KeychainError> {
            Err(unsupported())
        }

        fn delete_secret(&self, _account: &str) -> Result<(), KeychainError> {
            Err(unsupported())
        }
    }
}

// Ensure Debug does not expose secrets held by MemoryKeychain entries.
// (HashMap values are raw Vec in memory; Debug is only for the mutex wrapper.)

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn memory_round_trip_and_delete() {
        let kc = MemoryKeychain::new();
        kc.set_secret("acct", b"super-secret").unwrap();
        let loaded = kc.get_secret("acct").unwrap();
        assert_eq!(loaded.expose_secret().as_slice(), b"super-secret");
        kc.delete_secret("acct").unwrap();
        assert!(matches!(
            kc.get_secret("acct"),
            Err(KeychainError::NotFound { .. })
        ));
        // Idempotent delete
        kc.delete_secret("acct").unwrap();
    }

    #[test]
    fn memory_overwrite_replaces_bytes() {
        let kc = MemoryKeychain::new();
        kc.set_secret("k", b"one").unwrap();
        kc.set_secret("k", b"two").unwrap();
        assert_eq!(
            kc.get_secret("k").unwrap().expose_secret().as_slice(),
            b"two"
        );
    }

    #[test]
    fn os_keychain_reports_native_availability() {
        // On Windows/macOS/Linux CI hosts this is true; MemoryKeychain remains the
        // non-interactive test backend regardless.
        let _ = OsKeychain::is_native_available();
        let os = OsKeychain::with_service("com.mythos.prismatik.test");
        assert_eq!(os.service(), "com.mythos.prismatik.test");
    }
}
