//! Audit-as-write-path floor (`P6-DK-01`..`P6-DK-03`).
//!
//! The audit entry *is* the portfolio write. Projections are HashMap read
//! models rebuilt by replaying ledger entries; divergence between a cached
//! projection hash and a rebuild is a typed alert, not silent drift.

use crate::entry::{Actor, AuditAction, AuditEntry, Outcome, RedactedJson, SubjectRef};
use crate::error::AuditError;
use crate::ledger::{AuditLedger, InMemoryAuditLedger};
use crate::proof::hash_leaf;
use prismatik_determinism::ContentHash;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::OffsetDateTime;

/// Classification of a portfolio mutation recorded as the write itself.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioWriteKind {
    /// Open / increase a position.
    OpenPosition,
    /// Close / reduce a position.
    ClosePosition,
    /// Cash credit or debit.
    AdjustCash,
    /// Mark-to-market update.
    MarkToMarket,
    /// Opaque / test write code.
    Custom {
        /// Short kind code.
        code: String,
    },
}

impl PortfolioWriteKind {
    /// Stable string form stored on [`AuditAction::PortfolioWrite`].
    pub fn as_code(&self) -> String {
        match self {
            Self::OpenPosition => "open_position".into(),
            Self::ClosePosition => "close_position".into(),
            Self::AdjustCash => "adjust_cash".into(),
            Self::MarkToMarket => "mark_to_market".into(),
            Self::Custom { code } => code.clone(),
        }
    }

    /// Parse a kind code back into an enum (unknown codes become `Custom`).
    pub fn from_code(code: &str) -> Self {
        match code {
            "open_position" => Self::OpenPosition,
            "close_position" => Self::ClosePosition,
            "adjust_cash" => Self::AdjustCash,
            "mark_to_market" => Self::MarkToMarket,
            other => Self::Custom {
                code: other.to_string(),
            },
        }
    }
}

/// Structured portfolio write event — append this; do not mutate state first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioWriteEvent {
    /// Write kind.
    pub kind: PortfolioWriteKind,
    /// BLAKE3 of the structured (redacted) payload bytes.
    pub payload_hash: ContentHash,
    /// Projection key this write applies to (account / asset / lot id).
    pub projection_key: String,
}

impl PortfolioWriteEvent {
    /// Build an event from a kind and raw payload bytes (hashed, not stored).
    pub fn from_payload(
        kind: PortfolioWriteKind,
        projection_key: impl Into<String>,
        payload: &[u8],
    ) -> Self {
        Self {
            kind,
            payload_hash: ContentHash::from_bytes(payload),
            projection_key: projection_key.into(),
        }
    }

    /// Convert into a ledger [`AuditEntry`] ready for append.
    pub fn into_entry(
        self,
        occurred_at: OffsetDateTime,
        actor: Actor,
        outcome: Outcome,
        prev_hash: ContentHash,
    ) -> AuditEntry {
        AuditEntry {
            occurred_at,
            actor,
            action: AuditAction::PortfolioWrite {
                kind: self.kind.as_code(),
                payload_hash: self.payload_hash,
            },
            subject: SubjectRef::Key {
                key: self.projection_key,
            },
            outcome,
            prev_hash,
            detail: RedactedJson::default(),
        }
    }
}

/// Append a portfolio write through the ledger (the only write path).
pub async fn append_portfolio_write(
    ledger: &InMemoryAuditLedger,
    event: PortfolioWriteEvent,
    occurred_at: OffsetDateTime,
    actor: Actor,
    outcome: Outcome,
) -> Result<crate::entry::AuditReceipt, AuditError> {
    let entry = event.into_entry(occurred_at, actor, outcome, ledger.tip_hash());
    ledger.append(entry).await
}

/// In-memory portfolio projection rebuilt by ledger replay (read model).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioProjection {
    /// Latest payload hash per projection key (ordered for stable hashing).
    pub state: BTreeMap<String, ContentHash>,
}

impl PortfolioProjection {
    /// Empty projection.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Deterministic hash of the projection state.
    pub fn hash(&self) -> ContentHash {
        let mut h = blake3::Hasher::new();
        h.update(b"prismatik.audit.v1.projection");
        for (key, value) in &self.state {
            h.update(key.as_bytes());
            h.update(&[0xff]);
            h.update(value.as_slice());
            h.update(&[0xfe]);
        }
        ContentHash(h.finalize())
    }

    /// Apply one portfolio-write entry (non-write entries are ignored).
    pub fn apply_entry(&mut self, entry: &AuditEntry) {
        let AuditAction::PortfolioWrite {
            kind: _,
            payload_hash,
        } = &entry.action
        else {
            return;
        };
        let SubjectRef::Key { key } = &entry.subject else {
            return;
        };
        self.state.insert(key.clone(), *payload_hash);
    }
}

/// Result of replaying the ledger into a projection (`P6-DK-02`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionRebuild {
    /// Rebuilt read model.
    pub projection: PortfolioProjection,
    /// Hash of [`Self::projection`].
    pub projection_hash: ContentHash,
    /// Number of ledger entries considered.
    pub entry_count: u64,
    /// Number of portfolio-write entries applied.
    pub writes_applied: u64,
}

impl ProjectionRebuild {
    /// Replay `entries` into a fresh HashMap projection.
    pub fn replay(entries: &[AuditEntry]) -> Self {
        let mut projection = PortfolioProjection::empty();
        let mut writes_applied = 0u64;
        for entry in entries {
            if matches!(entry.action, AuditAction::PortfolioWrite { .. }) {
                projection.apply_entry(entry);
                writes_applied += 1;
            }
        }
        let projection_hash = projection.hash();
        Self {
            projection,
            projection_hash,
            entry_count: entries.len() as u64,
            writes_applied,
        }
    }

    /// Startup reconciliation stub: rebuild and require hash match (`P6-DK-02`/`03`).
    pub fn startup_reconcile(
        entries: &[AuditEntry],
        expected_projection_hash: &ContentHash,
    ) -> Result<Self, DivergenceAlert> {
        let rebuilt = Self::replay(entries);
        if rebuilt.projection_hash != *expected_projection_hash {
            return Err(DivergenceAlert {
                expected_hash: *expected_projection_hash,
                actual_hash: rebuilt.projection_hash,
                entry_count: rebuilt.entry_count,
                writes_applied: rebuilt.writes_applied,
            });
        }
        Ok(rebuilt)
    }
}

/// Divergence between a cached projection hash and a ledger rebuild (`P6-DK-03`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DivergenceAlert {
    /// Hash the caller believed was correct.
    pub expected_hash: ContentHash,
    /// Hash obtained by replaying the ledger.
    pub actual_hash: ContentHash,
    /// Ledger leaf count at reconcile time.
    pub entry_count: u64,
    /// Portfolio writes applied during rebuild.
    pub writes_applied: u64,
}

impl DivergenceAlert {
    /// Human-readable summary for logs / UI.
    pub fn message(&self) -> String {
        format!(
            "projection divergence: expected {}, actual {} (entries={}, writes={})",
            self.expected_hash, self.actual_hash, self.entry_count, self.writes_applied
        )
    }
}

/// Convenience: tip leaf hash after an append receipt (for chaining writes).
pub fn tip_after(receipt: &crate::entry::AuditReceipt) -> ContentHash {
    hash_leaf(&receipt.leaf_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::AuditLedger;
    use crate::proof::verify_inclusion;

    fn system_actor() -> Actor {
        Actor::System {
            component: "portfolio".into(),
        }
    }

    #[tokio::test]
    async fn append_rebuild_matches() {
        let ledger = InMemoryAuditLedger::new();
        let t0 = OffsetDateTime::UNIX_EPOCH;

        let e1 = PortfolioWriteEvent::from_payload(
            PortfolioWriteKind::OpenPosition,
            "AAPL",
            br#"{"qty":100}"#,
        );
        append_portfolio_write(&ledger, e1, t0, system_actor(), Outcome::Allowed)
            .await
            .unwrap();

        let e2 = PortfolioWriteEvent::from_payload(
            PortfolioWriteKind::AdjustCash,
            "CASH",
            br#"{"delta":-150000}"#,
        );
        append_portfolio_write(&ledger, e2, t0, system_actor(), Outcome::Allowed)
            .await
            .unwrap();

        let entries = ledger.entries();
        let rebuilt = ProjectionRebuild::replay(&entries);
        assert_eq!(rebuilt.writes_applied, 2);
        assert_eq!(rebuilt.entry_count, 2);
        assert_eq!(rebuilt.projection.state.len(), 2);

        let expected = rebuilt.projection_hash;
        let ok = ProjectionRebuild::startup_reconcile(&entries, &expected).unwrap();
        assert_eq!(ok.projection_hash, expected);

        // Existing Merkle inclusion path still works alongside write-path entries.
        let head = ledger.head();
        let proof = ledger.inclusion_proof(0, &head).await.unwrap();
        assert!(verify_inclusion(&proof));
        let report = ledger.verify_all().await.unwrap();
        assert!(report.ok());
    }

    #[tokio::test]
    async fn tamper_yields_divergence_alert() {
        let ledger = InMemoryAuditLedger::new();
        let t0 = OffsetDateTime::UNIX_EPOCH;
        let event = PortfolioWriteEvent::from_payload(
            PortfolioWriteKind::OpenPosition,
            "MSFT",
            br#"{"qty":10}"#,
        );
        append_portfolio_write(&ledger, event, t0, system_actor(), Outcome::Allowed)
            .await
            .unwrap();

        let entries = ledger.entries();
        let good = ProjectionRebuild::replay(&entries);
        let expected = good.projection_hash;

        // Simulate a corrupted cached projection hash (UI / table drift).
        let tampered_expected = ContentHash::from_bytes(b"tampered-projection");
        let alert = ProjectionRebuild::startup_reconcile(&entries, &tampered_expected)
            .expect_err("tampered expected hash must diverge");
        assert_eq!(alert.expected_hash, tampered_expected);
        assert_eq!(alert.actual_hash, expected);
        assert_eq!(alert.entry_count, 1);
        assert!(!alert.message().is_empty());

        // Mutating the rebuilt map in-process also diverges from the ledger hash.
        let mut dirty = good.projection.clone();
        dirty
            .state
            .insert("EVIL".into(), ContentHash::from_bytes(b"evil"));
        assert_ne!(dirty.hash(), expected);
    }

    #[tokio::test]
    async fn denied_writes_are_still_appended() {
        let ledger = InMemoryAuditLedger::new();
        let event = PortfolioWriteEvent::from_payload(
            PortfolioWriteKind::ClosePosition,
            "TSLA",
            br#"{"qty":1}"#,
        );
        append_portfolio_write(
            &ledger,
            event,
            OffsetDateTime::UNIX_EPOCH,
            system_actor(),
            Outcome::Denied,
        )
        .await
        .unwrap();
        let rebuilt = ProjectionRebuild::replay(&ledger.entries());
        assert_eq!(rebuilt.writes_applied, 1);
        assert!(rebuilt.projection.state.contains_key("TSLA"));
    }
}
