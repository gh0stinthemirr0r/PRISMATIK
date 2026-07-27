//! Audit ledger trait and entry types.

use crate::error::AuditError;
use crate::proof::{
    ConsistencyProof, InclusionProof, SignedTreeHead, TreeHead, VerificationReport,
};
use crate::RedactedJson;
use async_trait::async_trait;
use prismatik_determinism::ContentHash;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Principal that performed an action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    /// Human user.
    User,
    /// Trusted core subsystem.
    System,
    /// AI subsystem.
    Ai,
    /// Plugin process/module.
    Plugin,
    /// Isolated sidecar service.
    Sidecar,
}

/// Action recorded in audit log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditAction {
    /// Credential mutation event.
    CredentialAdded {
        /// Credential type.
        credential_kind: String,
        /// Non-secret fingerprint.
        fingerprint: String,
    },
    /// Live execution gate enabled.
    LiveExecutionEnabled {
        /// Session id.
        session_id: String,
    },
    /// Generic action when no stricter typed variant exists yet.
    Custom {
        /// Action name.
        name: String,
    },
}

/// Subject reference affected by audit action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectRef {
    /// Subject kind.
    pub kind: String,
    /// Subject identifier.
    pub id: String,
}

/// Audit action outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Operation allowed.
    Allowed,
    /// Operation denied.
    Denied,
    /// Operation failed.
    Failed,
}

/// Single audit log entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Event timestamp.
    pub occurred_at: OffsetDateTime,
    /// Event actor.
    pub actor: Actor,
    /// Action details.
    pub action: AuditAction,
    /// Subject reference.
    pub subject: SubjectRef,
    /// Outcome status.
    pub outcome: Outcome,
    /// Previous entry hash.
    pub prev_hash: ContentHash,
    /// Redacted detail payload.
    pub detail: RedactedJson,
}

/// Receipt returned after append.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditReceipt {
    /// New entry position.
    pub position: u64,
    /// New tree head.
    pub head: TreeHead,
    /// Entry hash.
    pub entry_hash: ContentHash,
}

/// Audit ledger contract.
#[async_trait]
pub trait AuditLedger: Send + Sync {
    /// Append an entry and return receipt.
    async fn append(&self, entry: AuditEntry) -> Result<AuditReceipt, AuditError>;
    /// Build inclusion proof for a position under a specific head.
    async fn inclusion_proof(
        &self,
        position: u64,
        head: &TreeHead,
    ) -> Result<InclusionProof, AuditError>;
    /// Build consistency proof between two heads.
    async fn consistency_proof(
        &self,
        from: &TreeHead,
        to: &TreeHead,
    ) -> Result<ConsistencyProof, AuditError>;
    /// Return signed tree head.
    async fn signed_head(&self) -> Result<SignedTreeHead, AuditError>;
    /// Verify entire ledger consistency.
    async fn verify_all(&self) -> Result<VerificationReport, AuditError>;
}
