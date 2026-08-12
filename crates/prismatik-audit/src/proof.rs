//! Merkle proof and tree-head types, plus the domain-separated hash helpers
//! used to build the append-only audit tree.
//!
//! Hash scheme: leaf hashes are BLAKE3 of the entry's canonical bytes, then
//! wrapped once more with a `leaf` domain tag so a leaf hash can never collide
//! with an interior node hash. Interior nodes hash their two children with an
//! `interior` domain tag.

use prismatik_determinism::{ContentHash, DualSignature};
use serde::{Deserialize, Serialize};

/// Domain-separation tag for audit leaf nodes.
const LEAF_TAG: &[u8] = b"prismatik.audit.v1.leaf";
/// Domain-separation tag for audit interior nodes.
const INTERIOR_TAG: &[u8] = b"prismatik.audit.v1.interior";

/// Current Merkle tree head.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeHead {
    /// Number of leaves in the tree.
    pub tree_size: u64,
    /// Root hash for the current tree (all-zero when empty).
    pub root_hash: ContentHash,
}

impl TreeHead {
    /// The canonical empty-tree head: zero leaves, all-zero root.
    pub fn empty() -> Self {
        Self {
            tree_size: 0,
            root_hash: ContentHash::from([0u8; 32]),
        }
    }
}

impl Default for TreeHead {
    fn default() -> Self {
        Self::empty()
    }
}

/// Signed tree head (published periodically for external verification).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedTreeHead {
    /// Tree head payload.
    pub head: TreeHead,
    /// Signature over canonical tree-head bytes.
    pub signature: DualSignature,
}

/// Merkle inclusion proof for one leaf under a given [`TreeHead`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InclusionProof {
    /// Zero-based leaf position this proof covers.
    pub position: u64,
    /// Tree head the proof is anchored to.
    pub head: TreeHead,
    /// Sibling hashes from the leaf up to the root.
    pub audit_path: Vec<ContentHash>,
    /// The leaf hash itself (so verifiers do not need to recompute it).
    pub leaf_hash: ContentHash,
}

/// Merkle consistency proof between two heads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsistencyProof {
    /// The smaller (`from`) head.
    pub from: TreeHead,
    /// The larger (`to`) head.
    pub to: TreeHead,
    /// Reveal nodes sufficient to bind `from` into `to`.
    pub nodes: Vec<ContentHash>,
}

/// Result of a full ledger verification pass.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Number of leaves checked.
    pub leaf_count: u64,
    /// Whether the `prev_hash` chain is intact.
    pub chain_ok: bool,
    /// Whether the recomputed Merkle root matches the stored head.
    pub merkle_ok: bool,
}

impl VerificationReport {
    /// Overall pass: chain intact and Merkle root matches.
    pub fn ok(&self) -> bool {
        self.chain_ok && self.merkle_ok
    }
}

/// Wrap a leaf entry hash with the leaf domain tag.
pub fn hash_leaf(entry_hash: &ContentHash) -> ContentHash {
    let mut h = blake3::Hasher::new();
    h.update(LEAF_TAG);
    h.update(entry_hash.as_slice());
    ContentHash(h.finalize())
}

/// Hash two child node hashes into an interior node.
pub fn hash_children(left: &ContentHash, right: &ContentHash) -> ContentHash {
    let mut h = blake3::Hasher::new();
    h.update(INTERIOR_TAG);
    h.update(left.as_slice());
    h.update(right.as_slice());
    ContentHash(h.finalize())
}

/// Verify an inclusion proof by recomputing the root from the leaf and path.
///
/// The ledger stores each leaf as `hash_leaf(entry_hash)` (domain-separated),
/// while the proof carries the raw `entry_hash` in [`InclusionProof::leaf_hash`].
/// This verifier therefore wraps the starting leaf once before walking the
/// sibling path. The walk mirrors the ledger's `inclusion_path` exactly: a
/// trailing odd child with no right sibling is promoted unchanged and consumes
/// no path entry, so the verifier and path-builder never desynchronize.
pub fn verify_inclusion(proof: &InclusionProof) -> bool {
    if proof.head.tree_size == 0 {
        return false;
    }
    let mut node = hash_leaf(&proof.leaf_hash);
    let mut idx = proof.position;
    let mut level_size = proof.head.tree_size;
    let mut path_iter = proof.audit_path.iter();
    while level_size > 1 {
        let even = idx % 2 == 0;
        let has_right = idx + 1 < level_size;
        if even && !has_right {
            // Trailing odd child: promoted unchanged, no path entry consumed.
        } else {
            let Some(sibling) = path_iter.next() else {
                break;
            };
            node = if even {
                hash_children(&node, sibling)
            } else {
                hash_children(sibling, &node)
            };
        }
        idx /= 2;
        level_size = level_size.div_ceil(2);
    }
    node == proof.head.root_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_head_is_all_zero_root() {
        let h = TreeHead::empty();
        assert_eq!(h.tree_size, 0);
        assert_eq!(h.root_hash, ContentHash::from([0u8; 32]));
    }

    #[test]
    fn leaf_and_interior_hashes_are_domain_separated() {
        let a = ContentHash::from_bytes(b"a");
        let b = ContentHash::from_bytes(b"b");
        let leaf_a = hash_leaf(&a);
        let interior_ab = hash_children(&a, &b);
        assert_ne!(leaf_a, interior_ab);
    }
}
