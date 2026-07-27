//! Idempotency key generation and collision detection (`P7-QM-02`).

use prismatik_determinism::DetMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Client-generated idempotency key (32-byte SHA-256 digest, hex in JSON).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IdempotencyKey(#[serde(with = "hex_bytes32")] pub [u8; 32]);

mod hex_bytes32 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let v = hex::decode(&s).map_err(serde::de::Error::custom)?;
        let arr: [u8; 32] = v
            .try_into()
            .map_err(|_| serde::de::Error::custom("idempotency key must be 32 bytes"))?;
        Ok(arr)
    }
}

impl IdempotencyKey {
    /// Construct from raw 32 bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Deterministic generation from opaque material (session + intent fields).
    ///
    /// Same inputs always yield the same key; callers must include enough
    /// entropy/domain separation so distinct intents do not collide.
    pub fn generate(material: &[u8]) -> Self {
        let digest = Sha256::digest(material);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&digest);
        Self(bytes)
    }

    /// Hex encoding for logs / broker client_order_id fields.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Idempotency store errors.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum IdempotencyError {
    /// Key already reserved for a different payload fingerprint.
    #[error("idempotency key collision: key reused with different payload")]
    Collision,
    /// Key already completed; replay should return the stored outcome, not resubmit.
    #[error("idempotency key already settled")]
    AlreadySettled,
}

/// Outcome fingerprint stored alongside a reserved key (opaque floor).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    /// Hash of the approved-order payload that first reserved the key.
    pub payload_fingerprint: [u8; 32],
    /// Optional opaque settled result token (e.g. broker order id hex).
    pub settled_token: Option<String>,
}

/// In-memory collision-detecting idempotency store.
#[derive(Clone, Debug, Default)]
pub struct IdempotencyStore {
    records: DetMap<IdempotencyKey, IdempotencyRecord>,
}

impl IdempotencyStore {
    /// Empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Fingerprint an arbitrary payload for collision checks.
    pub fn fingerprint(payload: &[u8]) -> [u8; 32] {
        let digest = Sha256::digest(payload);
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out
    }

    /// Reserve a key for `payload`. Same key + same payload is a no-op replay.
    /// Same key + different payload is a collision.
    pub fn reserve(
        &mut self,
        key: IdempotencyKey,
        payload: &[u8],
    ) -> Result<&IdempotencyRecord, IdempotencyError> {
        let fp = Self::fingerprint(payload);
        match self.records.get(&key) {
            Some(existing) if existing.payload_fingerprint != fp => {
                Err(IdempotencyError::Collision)
            },
            Some(_) => Ok(self.records.get(&key).expect("just matched")),
            None => {
                self.records.insert(
                    key,
                    IdempotencyRecord {
                        payload_fingerprint: fp,
                        settled_token: None,
                    },
                );
                Ok(self.records.get(&key).expect("just inserted"))
            },
        }
    }

    /// Mark a reserved key as settled with an opaque token.
    pub fn settle(
        &mut self,
        key: IdempotencyKey,
        token: impl Into<String>,
    ) -> Result<(), IdempotencyError> {
        let rec = self
            .records
            .get_mut(&key)
            .ok_or(IdempotencyError::AlreadySettled)?;
        if rec.settled_token.is_some() {
            return Err(IdempotencyError::AlreadySettled);
        }
        rec.settled_token = Some(token.into());
        Ok(())
    }

    /// Lookup a record.
    pub fn get(&self, key: &IdempotencyKey) -> Option<&IdempotencyRecord> {
        self.records.get(key)
    }

    /// True when the key is reserved.
    pub fn contains(&self, key: &IdempotencyKey) -> bool {
        self.records.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_is_deterministic() {
        let a = IdempotencyKey::generate(b"session|AAPL|buy|1");
        let b = IdempotencyKey::generate(b"session|AAPL|buy|1");
        let c = IdempotencyKey::generate(b"session|AAPL|buy|2");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn collision_on_different_payload() {
        let key = IdempotencyKey::generate(b"k");
        let mut store = IdempotencyStore::new();
        store.reserve(key, b"payload-a").unwrap();
        assert_eq!(
            store.reserve(key, b"payload-b").unwrap_err(),
            IdempotencyError::Collision
        );
    }

    #[test]
    fn same_payload_is_replay_safe() {
        let key = IdempotencyKey::generate(b"k");
        let mut store = IdempotencyStore::new();
        store.reserve(key, b"same").unwrap();
        store.reserve(key, b"same").unwrap();
        store.settle(key, "broker-1").unwrap();
        assert_eq!(
            store.get(&key).unwrap().settled_token.as_deref(),
            Some("broker-1")
        );
    }
}
