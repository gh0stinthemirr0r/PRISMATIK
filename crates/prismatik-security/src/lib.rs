//! # prismatik-prismatik-security
//!
//! Layer 3 — Extension and AI
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — key and envelope contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use envelope::{Envelope, EnvelopeError};
pub use keychain::{KeychainError, OsKeychain};
pub use keys::{DerivedKey, DeviceRootKey, KeyPurpose, SecretRef};
pub use zeroize_secret::ZeroizeSecret;

/// OS keychain contracts.
pub mod keychain {
    /// Keychain interaction error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct KeychainError {
        /// Error message.
        pub message: String,
    }

    /// OS keychain contract.
    pub trait OsKeychain: Send + Sync {
        /// Store a secret value.
        fn store(&self, key: &str, value: &[u8]) -> Result<(), KeychainError>;
        /// Read a secret value.
        fn read(&self, key: &str) -> Result<Vec<u8>, KeychainError>;
    }
}

/// Key hierarchy contracts.
pub mod keys {
    /// Purpose-specific key derivation.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum KeyPurpose {
        /// Audit-ledger key.
        AuditLedger,
        /// Broker-credential key.
        BrokerCredential,
        /// Local-cache key.
        LocalCache,
    }

    /// Device root key reference.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct DeviceRootKey {
        /// Opaque key handle id.
        pub handle_id: String,
    }

    /// Derived key reference.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct DerivedKey {
        /// Opaque key id.
        pub key_id: String,
        /// Derivation purpose.
        pub purpose: KeyPurpose,
    }

    /// Stable secret reference.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SecretRef {
        /// Secret id.
        pub id: String,
    }
}

/// Envelope encryption contracts.
pub mod envelope {
    /// Envelope encryption output.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Envelope {
        /// Ciphertext bytes.
        pub ciphertext: Vec<u8>,
        /// Nonce bytes.
        pub nonce: Vec<u8>,
    }

    /// Envelope error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct EnvelopeError {
        /// Error message.
        pub message: String,
    }
}

/// Secret wrapper contracts.
pub mod zeroize_secret {
    /// Secret wrapper zeroized on drop by implementation.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ZeroizeSecret(pub Vec<u8>);
}
