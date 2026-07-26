//! Dual-signature envelope (Ed25519 + ML-DSA).
//!
//! Spec: `DOCS/spec/CRATE_ARCHITECTURE.md` §1.1, `DOCS/spec/MANIFEST_SCHEMA.md` §9.
//!
//! Per Wave 0 risk: if ML-DSA Rust implementation maturity is uneven at Wave 0
//! close, ship Ed25519-only with the dual-signature *format* in place and the
//! `ml_dsa` field present but `signature: null`. Retrofit is field population,
//! not schema migration.

use serde::{Deserialize, Serialize};

/// A signature over an artifact or manifest digest. Dual scheme by default.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DualSignature {
    /// Name of the signing scheme (`"dual-ed25519-ml-dsa-65"` when complete).
    #[serde(default)]
    pub scheme: SignatureScheme,
    /// Ed25519 signature (always present).
    #[serde(default)]
    pub ed25519: Option<Ed25519Signature>,
    /// ML-DSA-65 signature (post-quantum). `None` until Wave 0 conditional
    /// shipping is complete.
    #[serde(default)]
    pub ml_dsa: Option<Bytes64>,
}

/// Signature-scheme identifier recorded in the manifest.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureScheme {
    /// No signature present yet (placeholder).
    #[default]
    None,
    /// Ed25519 only (Wave 0 conditional shipping).
    Ed25519Only,
    /// Dual Ed25519 + ML-DSA-65 (target end state).
    DualEd25519MlDsa65,
}

/// An Ed25519 signature with the public key used to verify it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ed25519Signature {
    /// 32-byte public key, base64 in JSON.
    #[serde(with = "bytes_b64_32")]
    pub public_key: [u8; 32],
    /// 64-byte signature, base64 in JSON.
    #[serde(with = "bytes_b64_64")]
    pub signature: [u8; 64],
    /// Who signed it.
    pub identity: SigningIdentity,
}

/// Who signed an artifact or manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningIdentity {
    /// Kind of signer.
    pub kind: SigningIdentityKind,
    /// Stable identifier (email, CI run id, service name).
    pub id: String,
    /// When the signature was applied.
    ///
    /// Stored as an RFC 3339 string in JSON.
    #[serde(default)]
    pub signed_at: Option<String>,
}

/// Kind of signer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SigningIdentityKind {
    /// Human operator.
    Operator,
    /// CI/CD system.
    Ci,
    /// Long-running service.
    Service,
}

/// A 64-byte buffer (used for ML-DSA signatures and Ed25519 signature payloads).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bytes64(#[serde(with = "bytes_b64_64_inner")] pub [u8; 64]);

impl Default for Bytes64 {
    fn default() -> Self {
        Self([0u8; 64])
    }
}

mod bytes_b64_32 {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(b: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(b))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let s = String::deserialize(d)?;
        let v = base64::engine::general_purpose::STANDARD
            .decode(s.as_bytes())
            .map_err(serde::de::Error::custom)?;
        v.as_slice()
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 32 bytes"))
    }
}

mod bytes_b64_64 {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(b: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(b))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let s = String::deserialize(d)?;
        let v = base64::engine::general_purpose::STANDARD
            .decode(s.as_bytes())
            .map_err(serde::de::Error::custom)?;
        v.as_slice()
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 64 bytes"))
    }
}

mod bytes_b64_64_inner {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(b: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(b))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let s = String::deserialize(d)?;
        let v = base64::engine::general_purpose::STANDARD
            .decode(s.as_bytes())
            .map_err(serde::de::Error::custom)?;
        v.as_slice()
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 64 bytes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_unsigned() {
        let s = DualSignature::default();
        assert_eq!(s.scheme, SignatureScheme::None);
        assert!(s.ed25519.is_none());
        assert!(s.ml_dsa.is_none());
    }

    #[test]
    fn ed25519_signature_round_trips() {
        let sig = Ed25519Signature {
            public_key: [7u8; 32],
            signature: [11u8; 64],
            identity: SigningIdentity {
                kind: SigningIdentityKind::Ci,
                id: "github-actions/run/123".into(),
                signed_at: Some("2026-07-26T14:32:01Z".into()),
            },
        };
        let json = serde_json::to_string(&sig).unwrap();
        let back: Ed25519Signature = serde_json::from_str(&json).unwrap();
        assert_eq!(sig, back);
    }
}
