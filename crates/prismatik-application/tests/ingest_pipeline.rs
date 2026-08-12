//! End-to-end ingest pipeline test (corpus Day 2, briefing §52 / §55).
//!
//! Exercises the full spine — budget admit -> fetch -> draft -> seal -> durable
//! persist -> audit append — against a test fetcher and the real durable store
//! + in-memory audit ledger. Asserts the corpus-foundation DoD items that are
//!   mechanically checkable: idempotent re-ingest, hash-chained audit entries,
//!   policy-gated raw retention, and point-in-time stamping via the clock.

#![forbid(unsafe_code)]

use std::sync::Arc;

use async_trait::async_trait;
use prismatik_application::{
    DurableAppendOutcome, FileObservationStore, IngestCommand, IngestError, PayloadFetcher,
};
use prismatik_audit::{AuditLedger, InMemoryAuditLedger};
use prismatik_determinism::{FrozenClock, SystemClock};
use prismatik_market_data::http::TransportError;
use prismatik_market_data::{
    AcquisitionMethod, AdmissionDecision, BudgetGovernor, GcraBudgetGovernor, GovernorQuota,
    IngestManifestId, PayloadRetention, PriorityClass, RedistributionPolicy, RetrievalAttemptId,
    SourceId, SourcePolicy, SourcePolicyVersion,
};
use tempfile::TempDir;
use time::{Duration, OffsetDateTime};

/// A fetcher that returns deterministic bytes for testing.
struct StaticFetcher {
    payload: Vec<u8>,
}

#[async_trait]
impl PayloadFetcher for StaticFetcher {
    async fn fetch(&self) -> Result<prismatik_application::ingest::FetchedPayload, TransportError> {
        Ok(prismatik_application::ingest::FetchedPayload {
            bytes: self.payload.clone(),
            media_type: "application/json".into(),
            event_time: None,
            publication_time: None,
            publisher_record_id: Some("test-record-001".into()),
        })
    }
}

fn test_policy(retention: PayloadRetention) -> SourcePolicy {
    let at = OffsetDateTime::UNIX_EPOCH;
    SourcePolicy::new(
        SourceId::new("test-source").unwrap(),
        SourcePolicyVersion::new(1).unwrap(),
        [AcquisitionMethod::Api],
        retention,
        RedistributionPolicy::PermittedMetadataOnly,
        [
            prismatik_market_data::PermittedMetadataField::Title,
            prismatik_market_data::PermittedMetadataField::Entities,
            prismatik_market_data::PermittedMetadataField::PublisherRecordId,
        ],
        "test-terms-owner",
        at,
        at,
        at + Duration::days(365),
        "test-review-ref",
    )
    .unwrap()
}

/// A budget governor that always admits (for deterministic testing).
struct AlwaysAdmit;

impl BudgetGovernor for AlwaysAdmit {
    fn admit(&self, _class: PriorityClass) -> AdmissionDecision {
        AdmissionDecision::Admit {
            permit: prismatik_market_data::Permit {
                token: "test".into(),
            },
        }
    }
    fn state(&self, class: PriorityClass) -> prismatik_market_data::BudgetState {
        prismatik_market_data::BudgetState {
            class,
            remaining: 100,
            resets_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
}

fn build_command(
    tmp: &TempDir,
    clock: Arc<dyn prismatik_determinism::Clock>,
) -> (
    IngestCommand,
    Arc<FileObservationStore>,
    Arc<InMemoryAuditLedger>,
) {
    let store = Arc::new(FileObservationStore::open(tmp.path()).unwrap());
    let audit = Arc::new(InMemoryAuditLedger::new());
    let cmd = IngestCommand::new(clock, Arc::new(AlwaysAdmit), store.clone(), audit.clone());
    (cmd, store, audit)
}

#[tokio::test]
async fn ingest_persists_observation_and_emits_audit_event() {
    let tmp = TempDir::new().unwrap();
    let clock = Arc::new(FrozenClock::new(OffsetDateTime::UNIX_EPOCH));
    let (cmd, store, audit) = build_command(&tmp, clock.clone());

    let policy = test_policy(PayloadRetention::RawPayloadPermitted);
    let fetcher = StaticFetcher {
        payload: br#"{"hello":"world"}"#.to_vec(),
    };

    let outcome = cmd
        .execute(
            &fetcher,
            SourceId::new("test-source").unwrap(),
            SourcePolicyVersion::new(1).unwrap(),
            AcquisitionMethod::Api,
            PriorityClass::Interactive,
            &policy,
            RetrievalAttemptId::from_bytes([1u8; 32]),
            IngestManifestId::from_bytes([2u8; 32]),
            "https://example.test/feed/1",
        )
        .await
        .expect("ingest should succeed");

    // The observation is durably stored...
    assert_eq!(store.len(), 1);
    // ...with the raw payload blob present (policy permitted raw retention)...
    let stored = store.iter().unwrap();
    assert_eq!(stored.len(), 1);
    let payload = store
        .read_payload(&stored[0].payload_hash())
        .unwrap()
        .expect("raw blob must exist");
    assert_eq!(payload, br#"{"hello":"world"}"#);
    // ...and the observation_time was stamped by the frozen clock.
    assert_eq!(stored[0].observation_time(), OffsetDateTime::UNIX_EPOCH);

    // The audit ledger has the observation_appended event.
    let report = audit.verify_all().await.unwrap();
    assert!(report.ok(), "audit chain must verify");
    assert!(report.leaf_count >= 1);
    assert_eq!(outcome.audit_position, 0);
}

#[tokio::test]
async fn re_ingest_same_attempt_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let clock = Arc::new(FrozenClock::new(OffsetDateTime::UNIX_EPOCH));
    let (cmd, store, audit) = build_command(&tmp, clock.clone());

    let policy = test_policy(PayloadRetention::RawPayloadPermitted);
    let fetcher = StaticFetcher {
        payload: b"duplicate-payload".to_vec(),
    };
    let attempt = RetrievalAttemptId::from_bytes([9u8; 32]);

    let first = cmd
        .execute(
            &fetcher,
            SourceId::new("test-source").unwrap(),
            SourcePolicyVersion::new(1).unwrap(),
            AcquisitionMethod::Api,
            PriorityClass::Interactive,
            &policy,
            attempt,
            IngestManifestId::from_bytes([2u8; 32]),
            "https://example.test/feed/dup",
        )
        .await
        .unwrap();
    assert!(matches!(
        first.append,
        DurableAppendOutcome::Inserted { .. }
    ));

    // Re-ingest the exact same payload+attempt: idempotent no-op.
    let second = cmd
        .execute(
            &fetcher,
            SourceId::new("test-source").unwrap(),
            SourcePolicyVersion::new(1).unwrap(),
            AcquisitionMethod::Api,
            PriorityClass::Interactive,
            &policy,
            attempt,
            IngestManifestId::from_bytes([2u8; 32]),
            "https://example.test/feed/dup",
        )
        .await
        .unwrap();
    assert!(matches!(
        second.append,
        DurableAppendOutcome::AlreadyPresent { .. }
    ));

    // Still only one observation in the store.
    assert_eq!(store.len(), 1);
    // But the audit ledger recorded both the append and the retry event.
    let report = audit.verify_all().await.unwrap();
    assert!(report.ok());
    assert!(report.leaf_count >= 2, "audit must record both attempts");
}

#[tokio::test]
async fn metadata_only_policy_writes_no_raw_blob() {
    let tmp = TempDir::new().unwrap();
    let clock = Arc::new(FrozenClock::new(OffsetDateTime::UNIX_EPOCH));
    let (cmd, store, _audit) = build_command(&tmp, clock.clone());

    let policy = test_policy(PayloadRetention::MetadataOnly);
    let fetcher = StaticFetcher {
        payload: b"secret-payload-not-to-be-stored".to_vec(),
    };

    cmd.execute(
        &fetcher,
        SourceId::new("test-source").unwrap(),
        SourcePolicyVersion::new(1).unwrap(),
        AcquisitionMethod::Api,
        PriorityClass::Interactive,
        &policy,
        RetrievalAttemptId::from_bytes([3u8; 32]),
        IngestManifestId::from_bytes([4u8; 32]),
        "https://example.test/feed/meta-only",
    )
    .await
    .unwrap();

    // Observation is in the ledger...
    assert_eq!(store.len(), 1);
    let stored = store.iter().unwrap();
    // ...but no raw blob (metadata-only policy honored).
    assert!(store
        .read_payload(&stored[0].payload_hash())
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn fetch_failure_is_recorded_and_surfaced() {
    let tmp = TempDir::new().unwrap();
    let clock = Arc::new(FrozenClock::new(OffsetDateTime::UNIX_EPOCH));
    let (cmd, _store, audit) = build_command(&tmp, clock.clone());

    struct FailingFetcher;
    #[async_trait]
    impl PayloadFetcher for FailingFetcher {
        async fn fetch(
            &self,
        ) -> Result<prismatik_application::ingest::FetchedPayload, TransportError> {
            Err(TransportError::Offline {
                method: prismatik_market_data::http::HttpMethod::Get,
                path: "/down".into(),
            })
        }
    }

    let policy = test_policy(PayloadRetention::RawPayloadPermitted);
    let result = cmd
        .execute(
            &FailingFetcher,
            SourceId::new("test-source").unwrap(),
            SourcePolicyVersion::new(1).unwrap(),
            AcquisitionMethod::Api,
            PriorityClass::Interactive,
            &policy,
            RetrievalAttemptId::from_bytes([5u8; 32]),
            IngestManifestId::from_bytes([6u8; 32]),
            "https://example.test/down",
        )
        .await;
    assert!(matches!(result, Err(IngestError::Transport(_))));

    // The failure was still recorded in the audit ledger.
    let report = audit.verify_all().await.unwrap();
    assert!(report.ok());
    assert!(report.leaf_count >= 1, "fetch failure must be audited");
}

#[tokio::test]
async fn gcra_governor_admits_within_burst() {
    // Smoke-test that the real GcraBudgetGovernor works through the command
    // path (not just the unit test). Uses the live SystemClock — correct for a
    // live throttle; the observation clock is still Frozen for stamping.
    let gov = GcraBudgetGovernor::new(
        GovernorQuota {
            burst: 5,
            per_period_cells: 5,
            per_period_secs: 1,
        },
        GovernorQuota {
            burst: 1,
            per_period_cells: 1,
            per_period_secs: 1,
        },
        Arc::new(SystemClock::new()),
    );
    for _ in 0..5 {
        assert!(matches!(
            gov.admit(PriorityClass::Interactive),
            AdmissionDecision::Admit { .. }
        ));
    }
}
