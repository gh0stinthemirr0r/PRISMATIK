//! Device root key and HKDF-derived purpose keys.
//!
//! Hierarchy (threat model §6.1):
//!
//! ```text
//! Device Root Key (OS keychain, never leaves)
//!    ├─ HKDF-SHA256(info="audit_ledger")  → Audit Ledger Key
//!    ├─ HKDF-SHA256(info="broker_cred")   → Broker Credential Sealing Key
//!    ├─ HKDF-SHA256(info="local_cache")   → Local Cache Sealing Key
//!    └─ HKDF-SHA256(info="manifest_sign") → Manifest Signing Key seed
//! ```

use hkdf::Hkdf;
use rand::rngs::SysRng;
use rand::TryRng;
use secrecy::{ExposeSecret, Secret, SecretVec};
use sha2::Sha256;
use thiserror::Error;

use crate::keychain::{Keychain, KeychainError};

/// Keychain account used for the device root key.
pub const DEVICE_ROOT_ACCOUNT: &str = "device_root_key";

/// Length of the device root and each derived key (AES-256 / Ed25519 seed).
pub const KEY_LEN: usize = 32;

/// Errors from key generation or derivation.
#[derive(Debug, Error)]
pub enum KeyError {
    /// OS CSPRNG failed to provide bytes.
    #[error("OS CSPRNG failed: {0}")]
    Csprng(String),
    /// Stored root key has an unexpected length.
    #[error("device root key has invalid length {got} (expected {KEY_LEN})")]
    InvalidRootLength {
        /// Observed byte length.
        got: usize,
    },
    /// HKDF expand failed (should not happen for fixed 32-byte OKM).
    #[error("HKDF expand failed: {0}")]
    Hkdf(String),
    /// Keychain backend failure while loading or storing the root.
    #[error(transparent)]
    Keychain(#[from] KeychainError),
}

/// Purpose labels for HKDF `info` (stable; changing breaks sealed data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyPurpose {
    /// Audit ledger sealing / MAC material.
    AuditLedger,
    /// Broker credential envelope sealing key.
    BrokerCredential,
    /// Local cache sealing key.
    LocalCache,
    /// Manifest signing key seed (Ed25519).
    ManifestSign,
}

impl KeyPurpose {
    /// HKDF info string bytes — must stay stable across releases.
    pub const fn info(self) -> &'static [u8] {
        match self {
            Self::AuditLedger => b"audit_ledger",
            Self::BrokerCredential => b"broker_cred",
            Self::LocalCache => b"local_cache",
            Self::ManifestSign => b"manifest_sign",
        }
    }

    /// All defined purposes (for rotation / inventory).
    pub const ALL: [KeyPurpose; 4] = [
        Self::AuditLedger,
        Self::BrokerCredential,
        Self::LocalCache,
        Self::ManifestSign,
    ];
}

/// Opaque reference to a secret held in the keychain (never the value).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretRef {
    account: String,
}

impl SecretRef {
    /// Build a reference for `account` within the active keychain service.
    pub fn new(account: impl Into<String>) -> Self {
        Self {
            account: account.into(),
        }
    }

    /// Keychain account name.
    pub fn account(&self) -> &str {
        &self.account
    }

    /// Reference for the device root key account.
    pub fn device_root() -> Self {
        Self::new(DEVICE_ROOT_ACCOUNT)
    }
}

/// Device root key — generated once, stored in the OS keychain, never logged.
///
/// Material is held in a [`Secret`] and zeroized on drop via `secrecy`/`zeroize`.
pub struct DeviceRootKey {
    material: Secret<[u8; KEY_LEN]>,
}

impl std::fmt::Debug for DeviceRootKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeviceRootKey([REDACTED])")
    }
}

impl DeviceRootKey {
    /// Generate a new root key from the OS CSPRNG.
    pub fn generate() -> Result<Self, KeyError> {
        let mut bytes = [0u8; KEY_LEN];
        SysRng
            .try_fill_bytes(&mut bytes)
            .map_err(|e| KeyError::Csprng(e.to_string()))?;
        Ok(Self {
            material: Secret::new(bytes),
        })
    }

    /// Wrap already-known root material (tests / restore). Prefer [`generate`].
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self {
            material: Secret::new(bytes),
        }
    }

    /// Persist this root under [`DEVICE_ROOT_ACCOUNT`].
    pub fn store(&self, keychain: &impl Keychain) -> Result<(), KeyError> {
        keychain.set_secret(DEVICE_ROOT_ACCOUNT, self.material.expose_secret())?;
        Ok(())
    }

    /// Load the root from the keychain.
    pub fn load(keychain: &impl Keychain) -> Result<Self, KeyError> {
        let secret = keychain.get_secret(DEVICE_ROOT_ACCOUNT)?;
        let slice = secret.expose_secret();
        if slice.len() != KEY_LEN {
            return Err(KeyError::InvalidRootLength { got: slice.len() });
        }
        let mut bytes = [0u8; KEY_LEN];
        bytes.copy_from_slice(slice);
        Ok(Self::from_bytes(bytes))
    }

    /// Load an existing root or generate and store a new one.
    pub fn load_or_create(keychain: &impl Keychain) -> Result<Self, KeyError> {
        match Self::load(keychain) {
            Ok(existing) => Ok(existing),
            Err(KeyError::Keychain(KeychainError::NotFound { .. })) => {
                let key = Self::generate()?;
                key.store(keychain)?;
                Ok(key)
            },
            Err(other) => Err(other),
        }
    }

    /// Derive a purpose-bound key via HKDF-SHA256 (salt empty; info = purpose).
    pub fn derive(&self, purpose: KeyPurpose) -> Result<DerivedKey, KeyError> {
        let hk = Hkdf::<Sha256>::new(None, self.material.expose_secret());
        let mut okm = [0u8; KEY_LEN];
        hk.expand(purpose.info(), &mut okm)
            .map_err(|e| KeyError::Hkdf(e.to_string()))?;
        Ok(DerivedKey {
            purpose,
            material: Secret::new(okm),
        })
    }

    /// Borrow raw root bytes for cryptographic use; prefer [`derive`].
    pub fn expose_bytes(&self) -> &[u8; KEY_LEN] {
        self.material.expose_secret()
    }

    /// [`SecretRef`] pointing at the root keychain account.
    pub fn secret_ref() -> SecretRef {
        SecretRef::device_root()
    }
}

/// Key derived from [`DeviceRootKey`] for a single [`KeyPurpose`].
pub struct DerivedKey {
    purpose: KeyPurpose,
    material: Secret<[u8; KEY_LEN]>,
}

impl std::fmt::Debug for DerivedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DerivedKey")
            .field("purpose", &self.purpose)
            .field("material", &"[REDACTED]")
            .finish()
    }
}

impl DerivedKey {
    /// Purpose this key was derived for.
    pub fn purpose(&self) -> KeyPurpose {
        self.purpose
    }

    /// Borrow key bytes for cryptographic use (AES-GCM, Ed25519 seed, etc.).
    pub fn expose_bytes(&self) -> &[u8; KEY_LEN] {
        self.material.expose_secret()
    }

    /// Clone material into a [`SecretVec`] for APIs that need owned secret buffers.
    pub fn to_secret_vec(&self) -> SecretVec<u8> {
        SecretVec::new(self.material.expose_secret().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keychain::MemoryKeychain;

    #[test]
    fn generate_is_non_zero_and_unique() {
        let a = DeviceRootKey::generate().unwrap();
        let b = DeviceRootKey::generate().unwrap();
        assert_ne!(a.expose_bytes(), b.expose_bytes());
        assert!(a.expose_bytes().iter().any(|&b| b != 0));
    }

    #[test]
    fn derive_is_deterministic_per_purpose() {
        let root = DeviceRootKey::from_bytes([7u8; KEY_LEN]);
        let a1 = root.derive(KeyPurpose::AuditLedger).unwrap();
        let a2 = root.derive(KeyPurpose::AuditLedger).unwrap();
        let broker = root.derive(KeyPurpose::BrokerCredential).unwrap();
        assert_eq!(a1.expose_bytes(), a2.expose_bytes());
        assert_ne!(a1.expose_bytes(), broker.expose_bytes());
        assert_eq!(a1.purpose(), KeyPurpose::AuditLedger);
    }

    #[test]
    fn purposes_have_distinct_info_labels() {
        let mut infos: Vec<&[u8]> = KeyPurpose::ALL.iter().map(|p| p.info()).collect();
        infos.sort();
        infos.dedup();
        assert_eq!(infos.len(), KeyPurpose::ALL.len());
    }

    #[test]
    fn load_or_create_round_trips_via_memory_keychain() {
        let kc = MemoryKeychain::new();
        let created = DeviceRootKey::load_or_create(&kc).unwrap();
        let loaded = DeviceRootKey::load(&kc).unwrap();
        assert_eq!(created.expose_bytes(), loaded.expose_bytes());
        // Second load_or_create must not rotate.
        let again = DeviceRootKey::load_or_create(&kc).unwrap();
        assert_eq!(again.expose_bytes(), created.expose_bytes());
    }

    #[test]
    fn invalid_root_length_is_rejected() {
        let kc = MemoryKeychain::new();
        kc.set_secret(DEVICE_ROOT_ACCOUNT, b"too-short").unwrap();
        let err = DeviceRootKey::load(&kc).unwrap_err();
        assert!(matches!(err, KeyError::InvalidRootLength { got: 9 }));
    }

    #[test]
    fn debug_redacts_material() {
        let root = DeviceRootKey::from_bytes([9u8; KEY_LEN]);
        let dbg = format!("{root:?}");
        assert!(dbg.contains("REDACTED"));
        assert!(!dbg.contains("9, 9, 9"));
        let derived = root.derive(KeyPurpose::LocalCache).unwrap();
        let ddbg = format!("{derived:?}");
        assert!(ddbg.contains("REDACTED"));
    }

    #[test]
    fn secret_ref_device_root() {
        assert_eq!(SecretRef::device_root().account(), DEVICE_ROOT_ACCOUNT);
    }
}
