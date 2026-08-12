//! In-process durable-shape job lifecycle with explicit wait and cursor streaming.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Condvar, Mutex};
use std::time::Duration;
use time::OffsetDateTime;

/// Job state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Awaiting execution.
    Queued,
    /// Actively running.
    Running,
    /// Completed successfully.
    Succeeded,
    /// Completed with failure.
    Failed,
    /// Cancelled before completion.
    Cancelled,
}

impl JobStatus {
    fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

/// One immutable job transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobEvent {
    /// Monotonic per-registry cursor.
    pub cursor: u64,
    /// Job identifier.
    pub job_id: String,
    /// New state.
    pub status: JobStatus,
    /// Caller-clock timestamp.
    pub observed_at: OffsetDateTime,
    /// Optional message.
    pub message: Option<String>,
}

/// Latest materialized job state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRecord {
    /// Job identifier.
    pub id: String,
    /// Job kind.
    pub kind: String,
    /// Current state.
    pub status: JobStatus,
    /// Last transition cursor.
    pub cursor: u64,
}

#[derive(Debug, Default)]
struct State {
    jobs: BTreeMap<String, JobRecord>,
    events: Vec<JobEvent>,
    next_cursor: u64,
}

/// Thread-safe lifecycle registry used behind job APIs.
#[derive(Debug, Default)]
pub struct JobRegistry {
    state: Mutex<State>,
    changed: Condvar,
}

impl JobRegistry {
    /// Register a queued job.
    pub fn create(
        &self,
        id: &str,
        kind: &str,
        at: OffsetDateTime,
    ) -> Result<JobRecord, &'static str> {
        if id.trim().is_empty() || kind.trim().is_empty() {
            return Err("job id and kind are required");
        }
        let mut state = self.state.lock().map_err(|_| "job registry poisoned")?;
        if state.jobs.contains_key(id) {
            return Err("job already exists");
        }
        let cursor = append(&mut state, id, JobStatus::Queued, at, None);
        let record = JobRecord {
            id: id.into(),
            kind: kind.into(),
            status: JobStatus::Queued,
            cursor,
        };
        state.jobs.insert(id.into(), record.clone());
        self.changed.notify_all();
        Ok(record)
    }
    /// Transition a job, rejecting transitions from terminal states.
    pub fn transition(
        &self,
        id: &str,
        status: JobStatus,
        at: OffsetDateTime,
        message: Option<String>,
    ) -> Result<JobRecord, &'static str> {
        let mut state = self.state.lock().map_err(|_| "job registry poisoned")?;
        let existing = state.jobs.get(id).cloned().ok_or("job not found")?;
        if existing.status.terminal() {
            return Err("job is terminal");
        }
        let cursor = append(&mut state, id, status, at, message);
        let record = JobRecord {
            id: existing.id,
            kind: existing.kind,
            status,
            cursor,
        };
        state.jobs.insert(id.into(), record.clone());
        self.changed.notify_all();
        Ok(record)
    }
    /// Wait until terminal or timeout without agent-side polling.
    pub fn wait_for_terminal(
        &self,
        id: &str,
        timeout: Duration,
    ) -> Result<Option<JobRecord>, &'static str> {
        let state = self.state.lock().map_err(|_| "job registry poisoned")?;
        let (state, _) = self
            .changed
            .wait_timeout_while(state, timeout, |state| {
                state.jobs.get(id).is_some_and(|job| !job.status.terminal())
            })
            .map_err(|_| "job registry poisoned")?;
        Ok(state
            .jobs
            .get(id)
            .filter(|job| job.status.terminal())
            .cloned())
    }
    /// Return immutable transition events after a cursor.
    pub fn stream_since(&self, cursor: u64) -> Result<Vec<JobEvent>, &'static str> {
        let state = self.state.lock().map_err(|_| "job registry poisoned")?;
        Ok(state
            .events
            .iter()
            .filter(|event| event.cursor > cursor)
            .cloned()
            .collect())
    }
}

fn append(
    state: &mut State,
    id: &str,
    status: JobStatus,
    at: OffsetDateTime,
    message: Option<String>,
) -> u64 {
    state.next_cursor = state.next_cursor.saturating_add(1);
    let cursor = state.next_cursor;
    state.events.push(JobEvent {
        cursor,
        job_id: id.into(),
        status,
        observed_at: at,
        message,
    });
    cursor
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streams_and_waits_for_terminal() {
        let registry = JobRegistry::default();
        registry
            .create("j", "backtest", OffsetDateTime::UNIX_EPOCH)
            .unwrap();
        registry
            .transition("j", JobStatus::Succeeded, OffsetDateTime::UNIX_EPOCH, None)
            .unwrap();
        assert_eq!(
            registry
                .wait_for_terminal("j", Duration::ZERO)
                .unwrap()
                .unwrap()
                .status,
            JobStatus::Succeeded
        );
        assert_eq!(registry.stream_since(0).unwrap().len(), 2);
    }
}
