//! OpenLineage-shaped lineage event types and emit sinks (P8-DP-05 floor).
//!
//! Schema-aligned stubs only — Mode D projection, no OpenLineage crate
//! dependency. Wire field names stay OpenLineage camelCase per ADR-0032.
//! Live HTTP/Marquez wiring remains author-ops; [`HttpOpenLineageSink`] fails
//! closed with [`EmitError::NotLinked`].

use serde::{Deserialize, Serialize};

/// OpenLineage job identity (name only at this floor).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    /// Job name (OpenLineage `job.name`).
    pub name: String,
}

/// OpenLineage run identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    /// Run id (OpenLineage `run.runId`).
    pub run_id: String,
}

/// Minimal OpenLineage [`RunEvent`](https://openlineage.io) stub.
///
/// Wire fields use OpenLineage camelCase (`eventType`, nested `run` /
/// `job`, `producer`). Full facets / inputs / outputs deferred.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEvent {
    /// OpenLineage event type (`START`, `COMPLETE`, `FAIL`, …).
    pub event_type: String,
    /// Nested run identity.
    pub run: Run,
    /// Nested job identity.
    pub job: Job,
    /// Producer URI / identifier (e.g. `https://github.com/mythos/prismatik`).
    pub producer: String,
}

impl RunEvent {
    /// Construct a minimal RunEvent stub.
    #[must_use]
    pub fn new(
        event_type: impl Into<String>,
        run_id: impl Into<String>,
        job_name: impl Into<String>,
        producer: impl Into<String>,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            run: Run {
                run_id: run_id.into(),
            },
            job: Job {
                name: job_name.into(),
            },
            producer: producer.into(),
        }
    }
}

/// Errors from an OpenLineage emission sink.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EmitError {
    /// No remote / HTTP sink is linked (author-ops residual).
    #[error("openlineage emit sink not linked: {0}")]
    NotLinked(String),
}

/// Trait for projecting [`RunEvent`] stubs to an OpenLineage backend.
pub trait OpenLineageEmitter: Send + Sync + std::fmt::Debug {
    /// Emit one run event.
    fn emit(&self, event: &RunEvent) -> Result<(), EmitError>;
}

/// In-memory / recording sink — accepts events without network I/O.
///
/// Prefer this alias in tests and local floors; [`InMemoryOpenLineageSink`] is
/// the concrete type.
pub type RecordingOpenLineageSink = InMemoryOpenLineageSink;

/// In-memory sink for tests — records events without network I/O.
#[derive(Debug, Default)]
pub struct InMemoryOpenLineageSink {
    events: std::sync::Mutex<Vec<RunEvent>>,
}

impl InMemoryOpenLineageSink {
    /// Construct an empty sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of emitted events (order preserved).
    #[must_use]
    pub fn snapshot(&self) -> Vec<RunEvent> {
        self.events.lock().expect("lock").clone()
    }

    /// Number of recorded events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.lock().expect("lock").len()
    }

    /// Whether no events have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drop all recorded events.
    pub fn clear(&self) {
        self.events.lock().expect("lock").clear();
    }
}

impl OpenLineageEmitter for InMemoryOpenLineageSink {
    fn emit(&self, event: &RunEvent) -> Result<(), EmitError> {
        self.events.lock().expect("lock").push(event.clone());
        Ok(())
    }
}

/// HTTP / Marquez-style sink — fails closed until author-ops wiring.
#[derive(Clone, Debug)]
pub struct HttpOpenLineageSink {
    /// Target endpoint (unused at floor; echoed in [`EmitError::NotLinked`]).
    pub endpoint: String,
}

impl HttpOpenLineageSink {
    /// Construct with an endpoint URL.
    #[must_use]
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }
}

impl OpenLineageEmitter for HttpOpenLineageSink {
    fn emit(&self, event: &RunEvent) -> Result<(), EmitError> {
        Err(EmitError::NotLinked(format!(
            "endpoint={} event_type={} — live OpenLineage emit is author-ops",
            self.endpoint, event.event_type
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_event_serde_uses_openlineage_camel_case() {
        let event = RunEvent::new(
            "COMPLETE",
            "run-abc-123",
            "prismatik.ingest.normalize",
            "https://github.com/mythos/prismatik",
        );
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json["eventType"], "COMPLETE");
        assert_eq!(json["run"]["runId"], "run-abc-123");
        assert_eq!(json["job"]["name"], "prismatik.ingest.normalize");
        assert_eq!(json["producer"], "https://github.com/mythos/prismatik");
        assert!(json.get("event_type").is_none());
        assert!(json["run"].get("run_id").is_none());

        let back: RunEvent = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, event);
    }

    #[test]
    fn recording_sink_accepts_and_preserves_events() {
        let sink = RecordingOpenLineageSink::new();
        assert!(sink.is_empty());

        let start = RunEvent::new("START", "r1", "prismatik.job", "producer");
        let complete = RunEvent::new("COMPLETE", "r1", "prismatik.job", "producer");
        sink.emit(&start).expect("recording sink accepts START");
        sink.emit(&complete)
            .expect("recording sink accepts COMPLETE");

        assert_eq!(sink.len(), 2);
        let recorded = sink.snapshot();
        assert_eq!(recorded[0], start);
        assert_eq!(recorded[1], complete);

        // Recorded payloads stay wire-compatible (camelCase).
        let wire = serde_json::to_value(&recorded[0]).expect("serialize recorded");
        assert_eq!(wire["eventType"], "START");
        assert_eq!(wire["run"]["runId"], "r1");

        sink.clear();
        assert!(sink.is_empty());
    }

    #[test]
    fn http_sink_fails_closed_with_not_linked() {
        let endpoint = "http://127.0.0.1:5000/api/v1/lineage";
        let sink = HttpOpenLineageSink::new(endpoint);
        let event = RunEvent::new("FAIL", "r2", "prismatik.job", "producer");
        let err = sink.emit(&event).expect_err("HTTP sink must fail closed");
        match err {
            EmitError::NotLinked(msg) => {
                assert!(msg.contains(endpoint), "error should echo endpoint: {msg}");
                assert!(msg.contains("FAIL"), "error should echo event_type: {msg}");
            },
        }
        // Fail-closed path must not mutate any recording surface.
        let recorder = RecordingOpenLineageSink::new();
        assert!(recorder.is_empty());
    }
}
