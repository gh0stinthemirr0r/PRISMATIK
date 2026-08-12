//! Canonical identity key types.

use crate::resolver::CanonicalIdentityRecord;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Canonical internal identifier for an asset.
///
/// The value is a BLAKE3 digest over a canonical identity record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(pub [u8; 32]);

impl AssetId {
    /// Derive an `AssetId` from a canonical identity record.
    pub fn from_canonical_record(rec: &CanonicalIdentityRecord) -> Self {
        let bytes = serde_json::to_vec(rec).expect("canonical identity record must serialize");
        Self(*blake3::hash(&bytes).as_bytes())
    }

    /// Derive an `AssetId` directly from canonical key bytes (BLAKE3 of the
    /// key). Used by cassette loaders that carry an explicit canonical key
    /// string: the same key always yields the same id (invariant I3), so a
    /// pinned cassette resolves identically on any machine.
    pub fn from_canonical_bytes(key: &[u8]) -> Self {
        Self(*blake3::hash(key).as_bytes())
    }

    /// Wrap a content hash directly into an `AssetId`. Used by the entropy
    /// factory for synthetic ids; the hash is already a 32-byte digest.
    pub fn from_hash(hash: prismatik_determinism::ContentHash) -> Self {
        Self(hash.as_bytes())
    }
}

/// Stable venue identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VenueId(pub u16);

/// ISO 10383 Market Identifier Code.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MicCode(String);

impl MicCode {
    /// Create a validated MIC code.
    ///
    /// MIC codes are upper-case ASCII and exactly 4 characters.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, MicCodeError> {
        let value = value.as_ref().trim().to_ascii_uppercase();
        if value.len() != 4 {
            return Err(MicCodeError::InvalidLength(value.len()));
        }
        if !value.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(MicCodeError::InvalidCharacters);
        }
        Ok(Self(value))
    }

    /// Construct a MIC from a four-byte literal.
    ///
    /// Intended for fixed MIC literals (e.g. `MicCode::new(*b"XNAS")`). The
    /// bytes are upper-cased; callers are responsible for supplying valid
    /// ISO 10383 codes.
    pub fn new(bytes: [u8; 4]) -> Self {
        let upper = [
            upper_ascii(bytes[0]),
            upper_ascii(bytes[1]),
            upper_ascii(bytes[2]),
            upper_ascii(bytes[3]),
        ];
        Self(String::from_utf8(upper.to_vec()).expect("ASCII bytes are valid UTF-8"))
    }

    /// Construct a MIC from a string slice without validation.
    ///
    /// Used by cassette loaders that have already validated the field length
    /// and character set; avoids a second allocation-generating parse pass.
    pub fn from_str_unchecked(value: &str) -> Self {
        Self(value.trim().to_ascii_uppercase())
    }

    /// Return the MIC string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Map an ASCII byte to its upper-case form.
fn upper_ascii(b: u8) -> u8 {
    if b.is_ascii_lowercase() {
        b - 32
    } else {
        b
    }
}

impl fmt::Display for MicCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for MicCode {
    type Err = MicCodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// MIC parsing errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MicCodeError {
    /// Code was not exactly 4 characters.
    #[error("MIC code must be 4 characters, got {0}")]
    InvalidLength(usize),
    /// Code had non-alphanumeric characters.
    #[error("MIC code must contain only ASCII alphanumeric characters")]
    InvalidCharacters,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_id::ExternalIdentifier;
    use crate::resolver::CanonicalIdentityRecord;
    use time::OffsetDateTime;

    #[test]
    fn asset_id_is_stable_for_same_record() {
        let rec = CanonicalIdentityRecord {
            primary_identifier: ExternalIdentifier::CoinGeckoId("bitcoin".into()),
            venue: Some(VenueId(1)),
            valid_from: Some(OffsetDateTime::UNIX_EPOCH),
            valid_to: None,
        };
        let a = AssetId::from_canonical_record(&rec);
        let b = AssetId::from_canonical_record(&rec);
        assert_eq!(a, b);
    }

    #[test]
    fn mic_code_must_be_four_alnum_chars() {
        assert!(MicCode::parse("XNYS").is_ok());
        assert!(MicCode::parse("XN").is_err());
        assert!(MicCode::parse("AB$%").is_err());
    }
}
