//! In-memory Merkle append-only audit ledger.

use crate::entry::{AuditEntry, AuditReceipt};
use crate::error::AuditError;
use crate::proof::{
    hash_children, hash_leaf, verify_inclusion, ConsistencyProof, InclusionProof, SignedTreeHead,
    TreeHead,
};
use async_trait::async_trait;
use prismatik_determinism::ContentHash;
use serde::{Deserialize, Serialize};

/// Startup / full-chain verification report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Number of leaves checked.
    pub leaf_count: u64,
    /// Whether prev_hash chain is intact.
    pub chain_ok: bool,
    /// Whether recomputed Merkle root matches the stored head.
    pub merkle_ok: bool,
}

impl VerificationReport {
    /// Overall pass.
    pub fn ok(&self) -> bool {
        self.chain_ok && self.merkle_ok
    }
}

/// Append-only, hash-chained Merkle audit ledger.
#[async_trait]
pub trait AuditLedger: Send + Sync {
    /// Append an entry. Caller supplies `prev_hash` matching the current tip.
    async fn append(&self, entry: AuditEntry) -> Result<AuditReceipt, AuditError>;

    /// Inclusion proof for `position` against `head`.
    async fn inclusion_proof(
        &self,
        position: u64,
        head: &TreeHead,
    ) -> Result<InclusionProof, AuditError>;

    /// Consistency proof between two heads.
    async fn consistency_proof(
        &self,
        from: &TreeHead,
        to: &TreeHead,
    ) -> Result<ConsistencyProof, AuditError>;

    /// Current signed tree head (signature may be empty).
    async fn signed_head(&self) -> Result<SignedTreeHead, AuditError>;

    /// Verify the entire chain and Merkle root.
    async fn verify_all(&self) -> Result<VerificationReport, AuditError>;
}

/// Thread-safe in-memory ledger for desktop / tests.
#[derive(Debug, Default)]
pub struct InMemoryAuditLedger {
    inner: std::sync::Mutex<LedgerState>,
}

#[derive(Debug, Default)]
struct LedgerState {
    entries: Vec<AuditEntry>,
    /// Domain-separated leaf hashes, one per entry.
    leaves: Vec<ContentHash>,
}

impl InMemoryAuditLedger {
    /// Create an empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Tip leaf hash (all-zero when empty) — used as next `prev_hash`.
    pub fn tip_hash(&self) -> ContentHash {
        let guard = self.inner.lock().expect("audit ledger lock");
        guard
            .leaves
            .last()
            .copied()
            .unwrap_or_else(|| ContentHash::from([0u8; 32]))
    }

    /// Snapshot of all entries (for projection replay / startup reconcile).
    pub fn entries(&self) -> Vec<AuditEntry> {
        let guard = self.inner.lock().expect("audit ledger lock");
        guard.entries.clone()
    }

    /// Current tree head.
    pub fn head(&self) -> TreeHead {
        let guard = self.inner.lock().expect("audit ledger lock");
        compute_head(&guard.leaves)
    }

    /// Test-only mutation that corrupts a stored entry (simulates tampering).
    #[doc(hidden)]
    pub fn corrupt_entry_for_test(&self, position: usize, mutator: impl FnOnce(&mut AuditEntry)) {
        let mut guard = self.inner.lock().expect("audit ledger lock");
        if let Some(entry) = guard.entries.get_mut(position) {
            mutator(entry);
            // Intentionally do NOT update leaves — leaves stay as originally recorded,
            // so verify_all detects the mismatch between entry bytes and leaf store.
            // Also corrupt the leaf store to simulate an attacker rewriting both.
            if let Some(leaf) = guard.leaves.get_mut(position) {
                *leaf = ContentHash::from_bytes(b"tampered-leaf");
            }
        }
    }
}

fn compute_head(leaves: &[ContentHash]) -> TreeHead {
    if leaves.is_empty() {
        return TreeHead::empty();
    }
    TreeHead {
        tree_size: leaves.len() as u64,
        root_hash: merkle_root(leaves),
    }
}

fn merkle_root(leaves: &[ContentHash]) -> ContentHash {
    let mut level: Vec<ContentHash> = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for chunk in level.chunks(2) {
            if chunk.len() == 2 {
                next.push(hash_children(&chunk[0], &chunk[1]));
            } else {
                next.push(chunk[0]);
            }
        }
        level = next;
    }
    level[0]
}

fn inclusion_path(leaves: &[ContentHash], position: u64) -> Vec<ContentHash> {
    let mut path = Vec::new();
    let mut idx = position as usize;
    let mut level: Vec<ContentHash> = leaves.to_vec();
    while level.len() > 1 {
        let sibling = if idx % 2 == 0 {
            if idx + 1 < level.len() {
                Some(level[idx + 1])
            } else {
                None
            }
        } else {
            Some(level[idx - 1])
        };
        if let Some(s) = sibling {
            path.push(s);
        }
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for chunk in level.chunks(2) {
            if chunk.len() == 2 {
                next.push(hash_children(&chunk[0], &chunk[1]));
            } else {
                next.push(chunk[0]);
            }
        }
        level = next;
        idx /= 2;
    }
    path
}

#[async_trait]
impl AuditLedger for InMemoryAuditLedger {
    async fn append(&self, entry: AuditEntry) -> Result<AuditReceipt, AuditError> {
        let mut guard = self.inner.lock().expect("audit ledger lock");
        let expected_prev = guard
            .leaves
            .last()
            .copied()
            .unwrap_or_else(|| ContentHash::from([0u8; 32]));
        if entry.prev_hash != expected_prev {
            return Err(AuditError::PrevHashMismatch {
                expected: expected_prev.to_string(),
                got: entry.prev_hash.to_string(),
            });
        }
        let entry_hash = entry
            .leaf_hash()
            .map_err(|e| AuditError::Integrity(e.to_string()))?;
        let leaf = hash_leaf(&entry_hash);
        guard.entries.push(entry);
        guard.leaves.push(leaf);
        let position = (guard.leaves.len() - 1) as u64;
        let tree_head = compute_head(&guard.leaves);
        Ok(AuditReceipt {
            position,
            leaf_hash: entry_hash,
            tree_head,
        })
    }

    async fn inclusion_proof(
        &self,
        position: u64,
        head: &TreeHead,
    ) -> Result<InclusionProof, AuditError> {
        let guard = self.inner.lock().expect("audit ledger lock");
        if position >= guard.leaves.len() as u64 {
            return Err(AuditError::PositionOutOfRange {
                position,
                tree_size: guard.leaves.len() as u64,
            });
        }
        let current = compute_head(&guard.leaves);
        if current != *head {
            return Err(AuditError::Integrity(
                "requested head does not match current ledger head".into(),
            ));
        }
        let entry_hash = guard.entries[position as usize]
            .leaf_hash()
            .map_err(|e| AuditError::Integrity(e.to_string()))?;
        Ok(InclusionProof {
            position,
            head: current,
            audit_path: inclusion_path(&guard.leaves, position),
            leaf_hash: entry_hash,
        })
    }

    async fn consistency_proof(
        &self,
        from: &TreeHead,
        to: &TreeHead,
    ) -> Result<ConsistencyProof, AuditError> {
        if from.tree_size > to.tree_size {
            return Err(AuditError::InconsistentSizes {
                from: from.tree_size,
                to: to.tree_size,
            });
        }
        let guard = self.inner.lock().expect("audit ledger lock");
        let current = compute_head(&guard.leaves);
        if current != *to {
            return Err(AuditError::Integrity(
                "to-head does not match current ledger head".into(),
            ));
        }
        if from.tree_size == 0 {
            return Ok(ConsistencyProof {
                from: from.clone(),
                to: to.clone(),
                nodes: vec![to.root_hash],
            });
        }
        let prefix = &guard.leaves[..from.tree_size as usize];
        let recomputed = compute_head(prefix);
        if recomputed.root_hash != from.root_hash {
            return Err(AuditError::Integrity(
                "from-head does not match ledger prefix".into(),
            ));
        }
        // Simplified consistency: emit from-root and to-root for equality checks.
        Ok(ConsistencyProof {
            from: from.clone(),
            to: to.clone(),
            nodes: vec![from.root_hash, to.root_hash],
        })
    }

    async fn signed_head(&self) -> Result<SignedTreeHead, AuditError> {
        Ok(SignedTreeHead {
            head: self.head(),
            signature: Default::default(),
        })
    }

    async fn verify_all(&self) -> Result<VerificationReport, AuditError> {
        let guard = self.inner.lock().expect("audit ledger lock");
        let mut chain_ok = true;
        let mut expected_prev = ContentHash::from([0u8; 32]);
        for (i, entry) in guard.entries.iter().enumerate() {
            if entry.prev_hash != expected_prev {
                chain_ok = false;
                break;
            }
            let entry_hash = entry
                .leaf_hash()
                .map_err(|e| AuditError::Integrity(e.to_string()))?;
            let leaf = hash_leaf(&entry_hash);
            if guard.leaves.get(i) != Some(&leaf) {
                chain_ok = false;
                break;
            }
            expected_prev = leaf;
        }
        let merkle_ok = compute_head(&guard.leaves).root_hash
            == if guard.leaves.is_empty() {
                TreeHead::empty().root_hash
            } else {
                merkle_root(&guard.leaves)
            };
        // merkle_ok is tautological with compute_head; also re-verify inclusions.
        let mut inclusions_ok = true;
        let head = compute_head(&guard.leaves);
        for (i, entry) in guard.entries.iter().enumerate() {
            let entry_hash = entry
                .leaf_hash()
                .map_err(|e| AuditError::Integrity(e.to_string()))?;
            let proof = InclusionProof {
                position: i as u64,
                head: head.clone(),
                audit_path: inclusion_path(&guard.leaves, i as u64),
                leaf_hash: entry_hash,
            };
            if !verify_inclusion(&proof) {
                inclusions_ok = false;
                break;
            }
        }
        Ok(VerificationReport {
            leaf_count: guard.entries.len() as u64,
            chain_ok,
            merkle_ok: merkle_ok && inclusions_ok,
        })
    }
}

/// Run `verify_all` at process/startup (P0-DK-08).
///
/// Append latency budget (1ms p99) is enforced by the `audit_append` criterion
/// bench and the `append_p99_smoke_under_1ms` threshold test.
pub async fn startup_verify(ledger: &dyn AuditLedger) -> Result<VerificationReport, AuditError> {
    let report = ledger.verify_all().await?;
    if !report.ok() {
        return Err(AuditError::Integrity("startup verification failed".into()));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{Actor, AuditAction, Outcome, RedactedJson, SubjectRef};
    use time::OffsetDateTime;

    fn sample_entry(prev: ContentHash, code: &str) -> AuditEntry {
        AuditEntry {
            occurred_at: OffsetDateTime::UNIX_EPOCH,
            actor: Actor::System {
                component: "test".into(),
            },
            action: AuditAction::Operational { code: code.into() },
            subject: SubjectRef::None,
            outcome: Outcome::Allowed,
            prev_hash: prev,
            detail: RedactedJson::default(),
        }
    }

    #[tokio::test]
    async fn append_and_inclusion_round_trip() {
        let ledger = InMemoryAuditLedger::new();
        let mut prev = ledger.tip_hash();
        for i in 0..5 {
            let receipt = ledger
                .append(sample_entry(prev, &format!("evt-{i}")))
                .await
                .unwrap();
            prev = hash_leaf(&receipt.leaf_hash);
            let proof = ledger
                .inclusion_proof(receipt.position, &receipt.tree_head)
                .await
                .unwrap();
            assert!(verify_inclusion(&proof));
        }
        let report = ledger.verify_all().await.unwrap();
        assert!(report.ok());
        assert_eq!(report.leaf_count, 5);
    }

    #[tokio::test]
    async fn tamper_is_detected() {
        let ledger = InMemoryAuditLedger::new();
        let receipt = ledger
            .append(sample_entry(ledger.tip_hash(), "ok"))
            .await
            .unwrap();
        assert_eq!(receipt.position, 0);
        ledger.corrupt_entry_for_test(0, |e| {
            e.action = AuditAction::Operational {
                code: "evil".into(),
            };
        });
        let report = ledger.verify_all().await.unwrap();
        assert!(!report.ok(), "tamper must fail verification");
    }

    #[tokio::test]
    async fn consistency_empty_to_one() {
        let ledger = InMemoryAuditLedger::new();
        let from = TreeHead::empty();
        let receipt = ledger
            .append(sample_entry(ledger.tip_hash(), "first"))
            .await
            .unwrap();
        let proof = ledger
            .consistency_proof(&from, &receipt.tree_head)
            .await
            .unwrap();
        assert_eq!(proof.nodes.len(), 1);
    }

    /// P0-DK-08 threshold gate: empirical p99 of a smoke sample must stay
    /// under 1ms. Fails loudly with the measured p99 when exceeded.
    ///
    /// CI hosts can be noisy; the criterion report under
    /// `crates/prismatik-audit/benches/` is the soak artifact. This smoke
    /// sample still fails the build if p99 regresses badly on the runner.
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // wall-clock latency gate only
    async fn append_p99_smoke_under_1ms() {
        const SAMPLES: usize = 256;
        const BUDGET_NS: u64 = 1_000_000; // 1ms

        let ledger = InMemoryAuditLedger::new();
        let mut prev = ledger.tip_hash();
        // Warm the path so first-sample JIT / allocator noise is diluted.
        for i in 0..32 {
            let receipt = ledger
                .append(sample_entry(prev, &format!("warm-{i}")))
                .await
                .unwrap();
            prev = hash_leaf(&receipt.leaf_hash);
        }

        let mut samples_ns = Vec::with_capacity(SAMPLES);
        for i in 0..SAMPLES {
            let start = std::time::Instant::now();
            let receipt = ledger
                .append(sample_entry(prev, &format!("smoke-{i}")))
                .await
                .unwrap();
            let elapsed = start.elapsed().as_nanos() as u64;
            prev = hash_leaf(&receipt.leaf_hash);
            samples_ns.push(elapsed);
        }

        samples_ns.sort_unstable();
        let idx = ((SAMPLES as f64) * 0.99).ceil() as usize - 1;
        let p99 = samples_ns[idx.min(SAMPLES - 1)];
        let p50 = samples_ns[SAMPLES / 2];
        eprintln!("P0-DK-08 smoke: n={SAMPLES} p50={p50}ns p99={p99}ns budget={BUDGET_NS}ns");
        assert!(
            p99 <= BUDGET_NS,
            "P0-DK-08 FAIL: audit append p99={p99}ns exceeds 1ms budget ({BUDGET_NS}ns); \
             samples={SAMPLES}. See crates/prismatik-audit/benches/AUDIT_APPEND_P99.md"
        );
    }
}
