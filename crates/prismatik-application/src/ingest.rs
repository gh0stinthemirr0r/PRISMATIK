//! Application-layer ingest command — corpus Day 2 (briefing §52 / §44.1).
//!
//! Stitches the live ingestion spine together:
//!
//! ```text
//! budget.admit()  →  transport.fetch()  →  ObservationDraft::new(&clock)
//!   →  Observation::seal(draft, policy)  →  store.append(obs, raw)
//!   →  audit_ledger.append(AuditEntry)
//! ```
//!
//! Every step that touches time uses the injected `Clock` (no ambient time).
//! Rate-limit admission uses the `GcraBudgetGovernor` (live wall-clock inside
//! the governor crate, which is correct for throttling real provider APIs).
//! The audit bridge (briefing §52 Day 2) writes a hash-chained entry for each
//! retrieval attempt, success, rejection, and supersession.

use std::sync::Arc;

use prismatik_audit::{
    Actor, AuditAction, AuditEntry, AuditError as AuditLedgerError, AuditLedger, Outcome,
    RedactedJson, SubjectRef,
};
use prismatik_determinism::{Clock, ContentHash};
use prismatik_market_data::http::TransportError;
use prismatik_market_data::{
    AcquisitionMethod, BudgetGovernor, Observation, ObservationDraft, PayloadRetention,
    PriorityClass, RetrievalAttemptId, SourceId, SourcePolicy, SourcePolicyVersion,
};
use thiserror::Error;
use time::OffsetDateTime;

use crate::observation_store::{DurableAppendOutcome, FileObservationStore};

// Re-export the store error so callers can import the full error surface from
// the application crate. (`ObservationStoreError as _` above keeps the explicit
// re-export below as the canonical path.)
pub use crate::observation_store::ObservationStoreError;
/// The in-memory audit ledger reference used by the bridge.
pub use prismatik_audit::InMemoryAuditLedger;

/// Errors raised by the ingest command.
#[derive(Debug, Error)]
pub enum IngestError {
    /// The budget governor deferred or denied the request.
    #[error("ingest deferred by budget governor until {retry_at}")]
    Deferred {
        /// Earliest retry time (from the injected clock).
        retry_at: OffsetDateTime,
    },
    /// The provider transport failed (network, decode, offline).
    #[error("ingest transport error: {0}")]
    Transport(#[from] TransportError),
    /// The observation could not be sealed against the source policy.
    #[error("ingest policy error: {0}")]
    Policy(String),
    /// The durable observation store failed.
    #[error("ingest store error: {0}")]
    Store(#[from] ObservationStoreError),
    /// The audit ledger rejected the event.
    #[error("ingest audit error: {0}")]
    Audit(#[from] AuditLedgerError),
}

/// Outcome of a successful ingest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestOutcome {
    /// The sealed observation id.
    pub observation_id: prismatik_market_data::ObservationId,
    /// Whether this was a fresh insert or an idempotent duplicate.
    pub append: DurableAppendOutcome,
    /// Audit ledger leaf position of the `observation_appended` event.
    pub audit_position: u64,
}

/// A single ingestion attempt's fetched payload.
#[derive(Clone, Debug)]
pub struct FetchedPayload {
    /// Raw bytes retrieved from the provider.
    pub bytes: Vec<u8>,
    /// MIME / content type declared by the provider.
    pub media_type: String,
    /// Provider-declared event time, if any.
    pub event_time: Option<OffsetDateTime>,
    /// Provider-declared publication time, if any.
    pub publication_time: Option<OffsetDateTime>,
    /// Provider record id (e.g. filing accession, feed entry id), if any.
    pub publisher_record_id: Option<String>,
}

/// Fetches a payload from a provider. Implementations wrap an adapter + transport.
#[async_trait::async_trait]
pub trait PayloadFetcher: Send + Sync {
    /// Fetch the payload for this ingest attempt.
    async fn fetch(&self) -> Result<FetchedPayload, TransportError>;
}

/// The corpus Day 2 ingest command. Owns the durable store and audit ledger
/// references; takes the clock, governor, policy, and a per-attempt fetcher.
pub struct IngestCommand {
    clock: Arc<dyn Clock>,
    governor: Arc<dyn BudgetGovernor>,
    store: Arc<FileObservationStore>,
    audit: Arc<InMemoryAuditLedger>,
}

impl std::fmt::Debug for IngestCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IngestCommand")
            .field("clock_kind", &self.clock.kind())
            .finish_non_exhaustive()
    }
}

impl IngestCommand {
    /// Construct an ingest command with its durable dependencies.
    pub fn new(
        clock: Arc<dyn Clock>,
        governor: Arc<dyn BudgetGovernor>,
        store: Arc<FileObservationStore>,
        audit: Arc<InMemoryAuditLedger>,
    ) -> Self {
        Self {
            clock,
            governor,
            store,
            audit,
        }
    }

    /// Execute one ingestion attempt end-to-end.
    ///
    /// `source_id` / `policy_version` identify the registered source policy.
    /// `method` is how the payload was acquired. `priority` governs rate-limit
    /// admission. `attempt_id` is a content-derived retrieval-attempt id.
    #[allow(clippy::too_many_arguments)] // each arg is a distinct per-call input
    pub async fn execute(
        &self,
        fetcher: &dyn PayloadFetcher,
        source_id: SourceId,
        policy_version: SourcePolicyVersion,
        method: AcquisitionMethod,
        priority: PriorityClass,
        policy: &SourcePolicy,
        attempt_id: RetrievalAttemptId,
        ingest_manifest_id: prismatik_market_data::IngestManifestId,
        source_uri: &str,
    ) -> Result<IngestOutcome, IngestError> {
        // 1. Admit through the budget governor.
        match self.governor.admit(priority) {
            prismatik_market_data::AdmissionDecision::Admit { .. } => {},
            prismatik_market_data::AdmissionDecision::Defer { retry_at, .. } => {
                // Record the deferral as an audit event before surfacing.
                self.emit_audit("ingest_deferred", Outcome::Denied, attempt_id, retry_at)
                    .await?;
                return Err(IngestError::Deferred { retry_at });
            },
            prismatik_market_data::AdmissionDecision::BudgetExhausted { resets_at, .. } => {
                self.emit_audit(
                    "ingest_budget_exhausted",
                    Outcome::Denied,
                    attempt_id,
                    resets_at,
                )
                .await?;
                return Err(IngestError::Deferred {
                    retry_at: resets_at,
                });
            },
            prismatik_market_data::AdmissionDecision::NotEntitled { .. } => {
                self.emit_audit(
                    "ingest_not_entitled",
                    Outcome::Denied,
                    attempt_id,
                    self.clock.now(),
                )
                .await?;
                return Err(IngestError::Policy("not entitled".into()));
            },
        }

        // 2. Fetch the payload (the only network step).
        let fetched = match fetcher.fetch().await {
            Ok(payload) => payload,
            Err(error) => {
                self.emit_audit(
                    "ingest_fetch_failed",
                    Outcome::Failed,
                    attempt_id,
                    self.clock.now(),
                )
                .await?;
                return Err(IngestError::Transport(error));
            },
        };

        // 3. Build the observation draft, stamping observation_time via the clock.
        let payload_hash = ContentHash::from_bytes(&fetched.bytes);
        let mut draft = ObservationDraft::new(
            &*self.clock,
            source_id,
            policy_version,
            method,
            source_uri,
            &fetched.media_type,
            payload_hash,
            attempt_id,
            ingest_manifest_id,
        )
        .map_err(|e| IngestError::Policy(e.to_string()))?;
        draft.event_time = fetched.event_time;
        draft.publication_time = fetched.publication_time;
        draft.publisher_record_id = fetched.publisher_record_id;

        // 4. Seal against the policy (hard-denies policy violations).
        let observation =
            Observation::seal(draft, policy).map_err(|e| IngestError::Policy(e.to_string()))?;

        // 5. Persist: raw bytes only when the policy permitted raw retention.
        let raw = if policy.payload_retention() == PayloadRetention::RawPayloadPermitted {
            Some(fetched.bytes.as_slice())
        } else {
            None
        };
        let append = self.store.append(&observation, raw)?;

        // 6. Emit the audit event (the bridge — briefing §52 Day 2). Both
        //    fresh inserts and idempotent duplicates are `Allowed` outcomes —
        //    a duplicate is not a denial, it is a correctly-recognized retry.
        let receipt = self
            .emit_audit(
                "observation_appended",
                Outcome::Allowed,
                attempt_id,
                self.clock.now(),
            )
            .await?;

        Ok(IngestOutcome {
            observation_id: observation.id(),
            append,
            audit_position: receipt.position,
        })
    }

    /// Append one audit event for an ingest lifecycle transition. Returns the
    /// receipt so callers can record the leaf position.
    async fn emit_audit(
        &self,
        code: &str,
        outcome: Outcome,
        attempt_id: RetrievalAttemptId,
        occurred_at: OffsetDateTime,
    ) -> Result<prismatik_audit::AuditReceipt, AuditLedgerError> {
        let mut detail = RedactedJson::default();
        detail
            .fields
            .insert("attempt_id".into(), attempt_id.to_string());
        let entry = AuditEntry {
            occurred_at,
            actor: Actor::System {
                component: "corpus-ingest".into(),
            },
            action: AuditAction::Operational {
                code: code.to_string(),
            },
            subject: SubjectRef::Key {
                key: attempt_id.to_string(),
            },
            outcome,
            prev_hash: self.audit.tip_hash(),
            detail,
        };
        self.audit.append(entry).await
    }
}
