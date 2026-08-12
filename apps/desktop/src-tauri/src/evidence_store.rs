//! Desktop composition for policy-gated durable normalized provider evidence.

use std::path::Path;
use std::sync::OnceLock;

use prismatik_application::{
    DurableAppendOutcome, FileObservationStore, IntelligenceObservationKind,
};
use prismatik_determinism::{Clock, ContentHash, SystemClock};
use prismatik_market_data::{
    AcquisitionMethod, IngestManifestId, Observation, ObservationDraft, PayloadRetention,
    RetrievalAttemptId,
};
use serde::Serialize;
use time::OffsetDateTime;

static STORE: OnceLock<FileObservationStore> = OnceLock::new();

const MODEL_EVIDENCE_LIMIT: usize = 64;
const MODEL_EVIDENCE_PAYLOAD_LIMIT: usize = 16_384;
const MODEL_EVIDENCE_TOTAL_LIMIT: usize = 262_144;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModelEvidenceRecord {
    pub(crate) evidence_id: String,
    pub(crate) source_id: String,
    pub(crate) source_policy_version: u32,
    pub(crate) source_uri: String,
    pub(crate) publisher_record_id: Option<String>,
    pub(crate) event_time: Option<String>,
    pub(crate) publication_time: Option<String>,
    pub(crate) observed_at: String,
    pub(crate) media_type: String,
    pub(crate) payload_hash: String,
    pub(crate) normalized_payload: Option<serde_json::Value>,
    pub(crate) payload_disclosure: &'static str,
}

pub(crate) fn initialize(data_dir: &Path) -> Result<(), String> {
    STORE
        .set(
            FileObservationStore::open(data_dir.join("provider-observations"))
                .map_err(|error| error.to_string())?,
        )
        .map_err(|_| "provider observation store initialized twice".to_owned())
}

pub(crate) struct EvidenceRecord {
    pub(crate) source_uri: String,
    pub(crate) publisher_record_id: String,
    pub(crate) event_time: Option<OffsetDateTime>,
    pub(crate) publication_time: Option<OffsetDateTime>,
    pub(crate) normalized_payload: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersistenceReport {
    state: &'static str,
    inserted: usize,
    unchanged: usize,
    total_observations: usize,
    retained_payloads: bool,
    policy_version: Option<u32>,
    message: String,
}

impl PersistenceReport {
    pub(crate) fn failure(message: String) -> Self {
        Self {
            state: "failed",
            inserted: 0,
            unchanged: 0,
            total_observations: STORE.get().map_or(0, FileObservationStore::len),
            retained_payloads: false,
            policy_version: None,
            message,
        }
    }
}

pub(crate) fn persist(
    provider_id: &str,
    records: Vec<EvidenceRecord>,
    kind: IntelligenceObservationKind,
) -> Result<PersistenceReport, String> {
    let Some(policy) = crate::provider_policy::active_policy(provider_id)? else {
        return Ok(PersistenceReport {
            state: "policy_required",
            inserted: 0,
            unchanged: 0,
            total_observations: STORE.get().map_or(0, FileObservationStore::len),
            retained_payloads: false,
            policy_version: None,
            message: "Live data rendered, but durable retention is disabled until a reviewed provider policy is recorded in Integrations.".into(),
        });
    };
    let store = STORE
        .get()
        .ok_or("provider observation store is unavailable")?;
    let clock = SystemClock::new();
    let observed_at = clock.now();
    let mut inserted = 0;
    let mut unchanged = 0;
    for record in records {
        let payload_hash = ContentHash::from_bytes(&record.normalized_payload);
        if let Some(existing_id) = store
            .find_publisher_payload(
                policy.source_id().as_str(),
                &record.publisher_record_id,
                &payload_hash,
            )
            .map_err(|error| error.to_string())?
        {
            unchanged += 1;
            crate::intelligence::record_observation(kind, existing_id.to_string(), observed_at)?;
            continue;
        }
        let attempt_material = format!(
            "{}|{}|{}|{}",
            provider_id, record.publisher_record_id, payload_hash, observed_at
        );
        let mut draft = ObservationDraft::new(
            &clock,
            policy.source_id().clone(),
            policy.version(),
            AcquisitionMethod::Api,
            record.source_uri,
            "application/vnd.prismatik.normalized+json",
            payload_hash,
            RetrievalAttemptId::from_bytes(
                ContentHash::from_bytes(attempt_material.as_bytes()).as_bytes(),
            ),
            IngestManifestId::from_bytes(
                ContentHash::from_bytes(b"prismatik:desktop-normalized-provider:v1").as_bytes(),
            ),
        )
        .map_err(|error| error.to_string())?;
        draft.publisher_record_id = Some(record.publisher_record_id);
        draft.event_time = record.event_time;
        draft.publication_time = record.publication_time;
        let observation = Observation::seal(draft, &policy).map_err(|error| error.to_string())?;
        let raw = (policy.payload_retention() == PayloadRetention::RawPayloadPermitted)
            .then_some(record.normalized_payload.as_slice());
        let outcome = store
            .append(&observation, raw)
            .map_err(|error| error.to_string())?;
        match outcome {
            DurableAppendOutcome::Inserted { .. } => inserted += 1,
            DurableAppendOutcome::AlreadyPresent { .. } => unchanged += 1,
        }
        crate::intelligence::record_observation(kind, observation.id().to_string(), observed_at)?;
    }
    Ok(PersistenceReport {
        state: "persisted",
        inserted,
        unchanged,
        total_observations: store.len(),
        retained_payloads: policy.payload_retention() == PayloadRetention::RawPayloadPermitted,
        policy_version: Some(policy.version().get()),
        message: format!(
            "{inserted} new and {unchanged} unchanged normalized record(s); source policy {} v{}.",
            policy.source_id(),
            policy.version().get()
        ),
    })
}

/// Return the newest governed durable observations for an untrusted model
/// evidence packet. Payloads are disclosed only when the source policy caused
/// a content-addressed payload to be retained. Both per-record and aggregate
/// bounds are enforced before JSON decoding.
pub(crate) fn model_evidence() -> Result<Vec<ModelEvidenceRecord>, String> {
    let store = STORE
        .get()
        .ok_or("provider observation store is unavailable")?;
    let mut observations = store.iter().map_err(|error| error.to_string())?;
    let as_of = SystemClock::new().now();
    observations.retain(|observation| observation.observation_time() <= as_of);
    observations.sort_by_key(|observation| observation.observation_time());

    let mut total_payload_bytes = 0usize;
    let mut records = Vec::new();
    for observation in observations.into_iter().rev().take(MODEL_EVIDENCE_LIMIT) {
        let retained = observation.raw_object_ref().is_some();
        let payload = if retained {
            decode_model_payload(
                store
                    .read_payload(&observation.payload_hash())
                    .map_err(|error| error.to_string())?,
                &mut total_payload_bytes,
            )
        } else {
            None
        };
        let payload_disclosure = if payload.is_some() {
            "retained_normalized_payload_included"
        } else if retained {
            "retained_payload_omitted_by_model_packet_bounds"
        } else {
            "metadata_only_by_source_policy"
        };
        records.push(ModelEvidenceRecord {
            evidence_id: observation.id().to_string(),
            source_id: observation.source_id().to_string(),
            source_policy_version: observation.source_policy_version().get(),
            source_uri: observation.source_uri().to_owned(),
            publisher_record_id: observation.publisher_record_id().map(str::to_owned),
            event_time: observation.event_time().map(|value| value.to_string()),
            publication_time: observation
                .publication_time()
                .map(|value| value.to_string()),
            observed_at: observation.observation_time().to_string(),
            media_type: observation.media_type().to_owned(),
            payload_hash: observation.payload_hash().to_string(),
            normalized_payload: payload,
            payload_disclosure,
        });
    }
    Ok(records)
}

pub(crate) fn audit_events() -> Result<Vec<crate::audit_timeline::AuditEvent>, String> {
    let store = STORE
        .get()
        .ok_or("provider observation store is unavailable")?;
    let mut observations = store.iter().map_err(|error| error.to_string())?;
    observations.sort_by_key(|observation| observation.observation_time());
    Ok(observations
        .into_iter()
        .rev()
        .take(200)
        .map(|observation| {
            let source = observation.source_id().to_string();
            let route = if source.to_ascii_lowercase().contains("fred") {
                "/workspace/macro"
            } else if source.to_ascii_lowercase().contains("sec") {
                "/workspace/filings"
            } else {
                "/workspace/intelligence"
            };
            crate::audit_timeline::AuditEvent {
                id: format!("observation:{}", observation.id()),
                occurred_at: observation.observation_time().to_string(),
                domain: "evidence",
                severity: "info",
                state: "observed".into(),
                title: format!("Governed {source} observation"),
                summary: format!(
                    "Policy v{} · {} · publisher record {}",
                    observation.source_policy_version().get(),
                    observation.media_type(),
                    observation.publisher_record_id().unwrap_or("not supplied")
                ),
                evidence_id: Some(observation.id().to_string()),
                route,
                durable: true,
            }
        })
        .collect())
}

fn decode_model_payload(
    bytes: Option<Vec<u8>>,
    total_payload_bytes: &mut usize,
) -> Option<serde_json::Value> {
    let bytes = bytes?;
    if bytes.len() > MODEL_EVIDENCE_PAYLOAD_LIMIT
        || total_payload_bytes.saturating_add(bytes.len()) > MODEL_EVIDENCE_TOTAL_LIMIT
    {
        return None;
    }
    let decoded = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;
    *total_payload_bytes = total_payload_bytes.saturating_add(bytes.len());
    Some(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_payload_boundary_is_json_only_and_fail_closed() {
        let mut total = 0;
        assert_eq!(
            decode_model_payload(Some(br#"{"series":"GDP"}"#.to_vec()), &mut total),
            Some(serde_json::json!({"series": "GDP"}))
        );
        let accepted = total;
        assert!(decode_model_payload(Some(b"not-json".to_vec()), &mut total).is_none());
        assert_eq!(total, accepted);
        assert!(decode_model_payload(
            Some(vec![b'x'; MODEL_EVIDENCE_PAYLOAD_LIMIT + 1]),
            &mut total
        )
        .is_none());
        assert_eq!(total, accepted);
    }

    #[test]
    fn model_payload_boundary_enforces_aggregate_cap() {
        let mut total = MODEL_EVIDENCE_TOTAL_LIMIT;
        assert!(decode_model_payload(Some(b"{}".to_vec()), &mut total).is_none());
        assert_eq!(total, MODEL_EVIDENCE_TOTAL_LIMIT);
    }
}
