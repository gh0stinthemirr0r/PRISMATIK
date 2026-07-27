//! Audit entry types (redaction-by-type).

use prismatik_determinism::ContentHash;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Who performed the audited action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    /// Interactive human user.
    User {
        /// Stable user identifier.
        id: String,
    },
    /// Internal system component.
    System {
        /// Component name.
        component: String,
    },
    /// AI agent acting under policy.
    Ai {
        /// Agent identifier.
        agent_id: String,
    },
    /// WASM plugin.
    Plugin {
        /// Plugin identifier.
        plugin_id: String,
    },
    /// Research / pricing sidecar process.
    Sidecar {
        /// Sidecar name.
        name: String,
    },
}

/// High-level action classification. Payloads stay redacted by construction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// Ledger bootstrap / genesis.
    LedgerInit,
    /// Artifact hash mismatch (security event).
    ArtifactHashMismatch,
    /// Manifest signed.
    ManifestSigned,
    /// Plugin capability grant recorded.
    PluginCapabilityGranted,
    /// Generic operational event with a redacted detail blob.
    Operational {
        /// Short action code.
        code: String,
    },
    /// Portfolio mutation written through the ledger (`P6-DK-01`).
    ///
    /// The entry *is* the write; projections replay these variants.
    PortfolioWrite {
        /// Stable write-kind code (see [`crate::write_path::PortfolioWriteKind`]).
        kind: String,
        /// BLAKE3 of the structured (redacted) payload.
        payload_hash: ContentHash,
    },
}

/// What the action targeted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectRef {
    /// No subject (system-wide).
    None,
    /// Manifest identifier.
    Manifest {
        /// Manifest id.
        id: String,
    },
    /// Artifact identifier.
    Artifact {
        /// Artifact id.
        id: String,
    },
    /// Opaque subject key.
    Key {
        /// Subject key.
        key: String,
    },
}

/// Outcome of the audited action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Action permitted / succeeded.
    Allowed,
    /// Action denied by policy.
    Denied,
    /// Action attempted and failed.
    Failed,
}

/// JSON detail that never carries secrets (enforced by callers / typed actions).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedJson {
    /// Opaque key/value map of non-secret fields.
    #[serde(default)]
    pub fields: prismatik_determinism::DetMap<String, String>,
}

/// One append-only audit record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// When the action occurred (from `Clock`, never ambient).
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    /// Actor.
    pub actor: Actor,
    /// Action.
    pub action: AuditAction,
    /// Subject.
    pub subject: SubjectRef,
    /// Outcome.
    pub outcome: Outcome,
    /// Hash of the previous leaf (genesis uses all-zero).
    pub prev_hash: ContentHash,
    /// Redacted detail.
    pub detail: RedactedJson,
}

impl AuditEntry {
    /// Canonical bytes for hashing (stable JSON, no signature fields).
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        // Compact JSON is sufficient; field order is struct declaration order.
        serde_json::to_vec(self)
    }

    /// BLAKE3 leaf hash of this entry.
    pub fn leaf_hash(&self) -> Result<ContentHash, serde_json::Error> {
        Ok(ContentHash::from_bytes(&self.canonical_bytes()?))
    }
}

/// Receipt returned after a successful append.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditReceipt {
    /// Zero-based leaf position.
    pub position: u64,
    /// Leaf hash.
    pub leaf_hash: ContentHash,
    /// Tree head after the append.
    pub tree_head: crate::proof::TreeHead,
}
