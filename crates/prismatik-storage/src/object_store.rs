//! S3-compatible object store port (`P8-DP-04` floor).
//!
//! Trait + in-memory backend only — no AWS SDK. Real S3/MinIO adapters land later.

use prismatik_determinism::ContentHash;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;
use thiserror::Error;

/// Validated object key (S3-style relative path).
///
/// Rules: non-empty; `/`-separated; no leading `/`; no empty, `.`, or `..`
/// segments; no NUL; max 1024 bytes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PathKey(String);

/// Errors constructing a [`PathKey`].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PathKeyError {
    /// Empty key.
    #[error("object path key must be non-empty")]
    Empty,
    /// Key longer than 1024 bytes.
    #[error("object path key exceeds 1024 bytes")]
    TooLong,
    /// Leading `/` or empty / `.` / `..` segment.
    #[error("invalid object path key segment or leading slash")]
    InvalidSegment,
    /// Embedded NUL byte.
    #[error("object path key must not contain NUL")]
    Nul,
}

impl PathKey {
    /// Parse and validate a relative object key.
    pub fn new(key: impl AsRef<str>) -> Result<Self, PathKeyError> {
        let key = key.as_ref();
        if key.is_empty() {
            return Err(PathKeyError::Empty);
        }
        if key.len() > 1024 {
            return Err(PathKeyError::TooLong);
        }
        if key.contains('\0') {
            return Err(PathKeyError::Nul);
        }
        if key.starts_with('/') {
            return Err(PathKeyError::InvalidSegment);
        }
        for segment in key.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return Err(PathKeyError::InvalidSegment);
            }
        }
        Ok(Self(key.to_string()))
    }

    /// Content-addressed key: `cas/blake3/<hex>`.
    pub fn content_addressed(hash: &ContentHash) -> Self {
        Self(format!("cas/blake3/{}", hex::encode(hash.as_bytes())))
    }

    /// Borrow the key string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PathKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for PathKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Object store operation errors.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ObjectStoreError {
    /// Key is not present.
    #[error("object not found: {0}")]
    NotFound(PathKey),
    /// Backend failure (mutex poison, adapter IO, etc.).
    #[error("object store error: {0}")]
    Backend(String),
}

/// S3-compatible object storage port: put / get / delete by [`PathKey`].
pub trait ObjectStore: Send + Sync {
    /// Store `bytes` at `key` (overwrite semantics, like S3 PUT).
    fn put(&self, key: &PathKey, bytes: &[u8]) -> Result<(), ObjectStoreError>;

    /// Load bytes for `key`.
    fn get(&self, key: &PathKey) -> Result<Vec<u8>, ObjectStoreError>;

    /// Delete `key`. Missing keys succeed (S3 DeleteObject idempotency).
    fn delete(&self, key: &PathKey) -> Result<(), ObjectStoreError>;
}

/// Content-addressed put helper: hashes with BLAKE3, stores under
/// [`PathKey::content_addressed`], returns key + digest.
pub fn put_blake3<S: ObjectStore + ?Sized>(
    store: &S,
    bytes: &[u8],
) -> Result<(PathKey, ContentHash), ObjectStoreError> {
    let hash = ContentHash::from_bytes(bytes);
    let key = PathKey::content_addressed(&hash);
    store.put(&key, bytes)?;
    Ok((key, hash))
}

/// In-memory object store for tests and local floors.
#[derive(Debug, Default)]
pub struct InMemoryObjectStore {
    objects: Mutex<BTreeMap<PathKey, Vec<u8>>>,
}

impl InMemoryObjectStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of distinct keys.
    pub fn len(&self) -> usize {
        self.objects
            .lock()
            .expect("in-memory object store mutex poisoned")
            .len()
    }

    /// Whether the store holds no objects.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl ObjectStore for InMemoryObjectStore {
    fn put(&self, key: &PathKey, bytes: &[u8]) -> Result<(), ObjectStoreError> {
        let mut objects = self
            .objects
            .lock()
            .map_err(|_| ObjectStoreError::Backend("mutex poisoned".into()))?;
        objects.insert(key.clone(), bytes.to_vec());
        Ok(())
    }

    fn get(&self, key: &PathKey) -> Result<Vec<u8>, ObjectStoreError> {
        let objects = self
            .objects
            .lock()
            .map_err(|_| ObjectStoreError::Backend("mutex poisoned".into()))?;
        objects
            .get(key)
            .cloned()
            .ok_or_else(|| ObjectStoreError::NotFound(key.clone()))
    }

    fn delete(&self, key: &PathKey) -> Result<(), ObjectStoreError> {
        let mut objects = self
            .objects
            .lock()
            .map_err(|_| ObjectStoreError::Backend("mutex poisoned".into()))?;
        objects.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_key_rejects_invalid() {
        assert_eq!(PathKey::new(""), Err(PathKeyError::Empty));
        assert_eq!(PathKey::new("/a"), Err(PathKeyError::InvalidSegment));
        assert_eq!(PathKey::new("a//b"), Err(PathKeyError::InvalidSegment));
        assert_eq!(PathKey::new("a/../b"), Err(PathKeyError::InvalidSegment));
        assert_eq!(PathKey::new("a/./b"), Err(PathKeyError::InvalidSegment));
        assert_eq!(PathKey::new("a\0b"), Err(PathKeyError::Nul));
        assert!(PathKey::new("a".repeat(1025)).is_err());
    }

    #[test]
    fn put_get_delete_roundtrip() {
        let store = InMemoryObjectStore::new();
        let key = PathKey::new("artifacts/bundle.bin").unwrap();
        store.put(&key, b"hello").unwrap();
        assert_eq!(store.get(&key).unwrap(), b"hello");
        store.put(&key, b"world").unwrap();
        assert_eq!(store.get(&key).unwrap(), b"world");
        store.delete(&key).unwrap();
        assert!(matches!(
            store.get(&key),
            Err(ObjectStoreError::NotFound(_))
        ));
        // Idempotent delete.
        store.delete(&key).unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn put_blake3_is_content_addressed() {
        let store = InMemoryObjectStore::new();
        let payload = b"prismatik-cas-payload";
        let (key, hash) = put_blake3(&store, payload).unwrap();
        assert_eq!(key.as_str(), PathKey::content_addressed(&hash).as_str());
        assert!(key.as_str().starts_with("cas/blake3/"));
        assert_eq!(store.get(&key).unwrap(), payload);
        assert_eq!(hash, ContentHash::from_bytes(payload));
        // Same bytes → same key; overwrite is fine.
        let (key2, hash2) = put_blake3(&store, payload).unwrap();
        assert_eq!(key, key2);
        assert_eq!(hash, hash2);
        assert_eq!(store.len(), 1);
    }
}
