//! Content-addressed, signed artifact references.
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md` §1.1, §1.2,
//! `DOCS/spec/MANIFEST_SCHEMA.md` §3.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A BLAKE3 hash of artifact bytes, used as the content address.
///
/// Stored as a 32-byte array; serialized as a hex string (`"blake3:..."`)
/// in JSON for readability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(#[serde(with = "blake3_serde")] pub blake3::Hash);

impl ContentHash {
    /// Hash the given bytes and return a `ContentHash`.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(blake3::hash(bytes))
    }

    /// Return the underlying 32-byte digest by value.
    pub fn as_bytes(&self) -> [u8; 32] {
        *self.0.as_bytes()
    }

    /// Return the underlying digest as a byte slice. Use this for hashing.
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "blake3:{}", hex::encode(self.as_bytes()))
    }
}

/// Stable identifier for a pinned artifact. Constructed from a stable name
/// (e.g. `"cal-nyse-2026-07-26"`); uniqueness is enforced at registration.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactId(pub String);

impl ArtifactId {
    /// Construct from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ArtifactId {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

/// What kind of artifact is referenced. Affects how the runtime loads and
/// validates it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// Trading-session calendar (NYSE, NASDAQ, etc.).
    Calendar,
    /// Bitemporal symbology snapshot.
    SymbologySnapshot,
    /// Corporate-action event ledger.
    CorporateActionLedger,
    /// TSFM tokenizer codebook.
    TokenizerCodebook,
    /// Model weights (TSFM or other).
    ModelWeights,
    /// Computed calibration table (conformal).
    CalibrationTable,
    /// Indicator golden vectors.
    IndicatorGoldenVectors,
    /// Reproducibility manifest bundle.
    ManifestBundle,
    /// WASM plugin module.
    PluginModule,
}

/// A semantic version (MAJOR.MINOR.PATCH). Stored as a string in JSON
/// because the `semver` crate's `Version` is more permissive than we need
/// and adds serialization surface we don't want at the kernel boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SemanticVersion(pub [u16; 3]);

impl SemanticVersion {
    /// Construct a `SemanticVersion`.
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self([major, minor, patch])
    }
}

impl fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0[0], self.0[1], self.0[2])
    }
}

/// A content-addressed, signed artifact reference.
///
/// This is the canonical way to refer to any external artifact whose change
/// would alter a result. If it is not in `PinnedArtifactSet`, it cannot
/// influence a run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// Stable artifact identifier.
    pub artifact_id: ArtifactId,
    /// What kind of artifact it is.
    pub kind: ArtifactKind,
    /// Semantic version.
    pub version: SemanticVersion,
    /// BLAKE3 of the artifact bytes. Verified on load, every load.
    pub content_hash: ContentHash,
    /// Detached signature over `content_hash`. Dual scheme per spec.
    #[serde(default)]
    pub signature: crate::DualSignature,
}

mod blake3_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(h: &blake3::Hash, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(h.as_bytes()))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<blake3::Hash, D::Error> {
        let s = String::deserialize(d)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        let arr: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 32 bytes"))?;
        Ok(arr.into())
    }
}

impl From<[u8; 32]> for ContentHash {
    fn from(arr: [u8; 32]) -> Self {
        Self(arr.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_round_trips_through_json() {
        let h = ContentHash::from_bytes(b"hello world");
        let json = serde_json::to_string(&h).unwrap();
        let back: ContentHash = serde_json::from_str(&json).unwrap();
        assert_eq!(h, back);
    }

    #[test]
    fn content_hash_display_is_prefixed_hex() {
        let h = ContentHash::from_bytes(b"hello world");
        let s = h.to_string();
        assert!(s.starts_with("blake3:"));
        assert_eq!(s.len(), "blake3:".len() + 64);
    }

    #[test]
    fn artifact_id_round_trips() {
        let id = ArtifactId::new("cal-nyse-2026-07-26");
        let parsed: ArtifactId = "cal-nyse-2026-07-26".parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn semantic_version_display() {
        let v = SemanticVersion::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }
}
