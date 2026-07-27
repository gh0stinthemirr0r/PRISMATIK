//! Merkle proof and tree-head types.

use prismatik_determinism::{ContentHash, DualSignature};
use serde::{Deserialize, Serialize};

/// Current Merkle tree head.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeHead {
    /// Number of entries in tree.
    pub tree_size: u64,
    /// Root hash for current tree.
    pub root_hash: ContentHash,
}

/// Signed tree head.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedTreeHead {
    /// Tree head payload.
    pub head: TreeHead,
    /// Signature over canonical tree-head bytes.
    pub signature: DualSignature,
}

/// Merkle inclusion proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InclusionProof {
    /// Entry position.
    pub position: u64,
    /// Tree size used to produce this proof.
    pub tree_size: u64,
    /// Hash path nodes.
    pub path: Vec<ContentHash>,
}

/// Merkle consistency proof between two heads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsistencyProof {
    /// Start head.
    pub from: TreeHead,
    /// End head.
    pub to: TreeHead,
    /// Hash path nodes.
    pub path: Vec<ContentHash>,
}

/// Result of full ledger verification pass.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Whether ledger verification succeeded.
    pub ok: bool,
    /// Number of checked entries.
    pub checked_entries: u64,
    /// Optional error details when `ok=false`.
    pub errors: Vec<String>,
}
