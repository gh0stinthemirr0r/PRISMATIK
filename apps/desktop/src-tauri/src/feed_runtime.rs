//! Durable, parser-independent scheduler for reviewed RSS/Atom acquisition.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use prismatik_application::{
    fetch_reviewed_feed, reviewed_feed_host, FeedConditionalHeaders, FileStateJournal,
};
use prismatik_determinism::{Clock, SystemClock};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeedRuntimeStatus {
    pub(crate) source_id: String,
    pub(crate) policy_version: u32,
    pub(crate) state: String,
    pub(crate) last_attempt_at: Option<String>,
    pub(crate) last_success_at: Option<String>,
    pub(crate) next_attempt_at: String,
    pub(crate) consecutive_failures: u16,
    pub(crate) status_code: Option<u16>,
    pub(crate) bytes_received: usize,
    pub(crate) etag: Option<String>,
    pub(crate) last_modified: Option<String>,
    pub(crate) message: String,
}

#[derive(Debug)]
struct Runtime {
    latest: BTreeMap<String, FeedRuntimeStatus>,
    journal: FileStateJournal<BTreeMap<String, FeedRuntimeStatus>>,
}

static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();

pub(crate) fn initialize(data_dir: &Path) -> Result<(), String> {
    let mut journal: FileStateJournal<BTreeMap<String, FeedRuntimeStatus>> =
        FileStateJournal::open(
            data_dir.join("feed-runtime.jsonl"),
            "prismatik.feed-runtime.v1",
        )
        .map_err(|error| error.to_string())?;
    let latest = journal.latest().cloned().unwrap_or_default();
    if journal.latest().is_none() {
        journal
            .append("initialized", latest.clone(), SystemClock::new().now())
            .map_err(|error| error.to_string())?;
    }
    RUNTIME
        .set(Mutex::new(Runtime { latest, journal }))
        .map_err(|_| "feed runtime initialized twice".to_owned())
}

#[tauri::command]
pub(crate) fn feed_runtime_status() -> Result<Vec<FeedRuntimeStatus>, String> {
    Ok(RUNTIME
        .get()
        .ok_or("feed runtime is unavailable")?
        .lock()
        .map_err(|_| "feed runtime lock is unavailable".to_owned())?
        .latest
        .values()
        .cloned()
        .collect())
}

pub(crate) fn audit_events() -> Result<Vec<crate::audit_timeline::AuditEvent>, String> {
    Ok(feed_runtime_status()?
        .into_iter()
        .filter_map(|status| {
            Some(crate::audit_timeline::AuditEvent {
                id: format!(
                    "feed:{}:{}",
                    status.source_id,
                    status.last_attempt_at.as_deref()?
                ),
                occurred_at: status.last_attempt_at?,
                domain: "feeds",
                severity: if status.state == "backoff" {
                    "warning"
                } else {
                    "info"
                },
                state: status.state,
                title: format!("Feed acquisition · {}", status.source_id),
                summary: format!(
                    "HTTP {} · {} bytes · policy v{} · {}",
                    status
                        .status_code
                        .map_or_else(|| "unavailable".into(), |value| value.to_string()),
                    status.bytes_received,
                    status.policy_version,
                    status.message
                ),
                evidence_id: None,
                route: "/workspace/feeds",
                durable: true,
            })
        })
        .collect())
}

#[tauri::command]
pub(crate) async fn run_feed_scheduler_once() -> Result<Vec<FeedRuntimeStatus>, String> {
    run_due(8).await
}

pub(crate) fn start_scheduler() {
    std::thread::Builder::new()
        .name("prismatik-feed-scheduler".into())
        .spawn(|| loop {
            let _ = tauri::async_runtime::block_on(run_due(8));
            std::thread::sleep(Duration::from_secs(30));
        })
        .expect("spawn feed scheduler");
}

async fn run_due(limit: usize) -> Result<Vec<FeedRuntimeStatus>, String> {
    let now = SystemClock::new().now();
    let sources = crate::feed_control::scheduler_sources()?;
    let existing = RUNTIME
        .get()
        .ok_or("feed runtime is unavailable")?
        .lock()
        .map_err(|_| "feed runtime lock is unavailable".to_owned())?
        .latest
        .clone();
    let mut due = sources
        .into_iter()
        .filter(|source| source.enabled && source_is_due(source, existing.get(&source.id), now))
        .collect::<Vec<_>>();
    due.sort_by(|a, b| {
        let scheduled_at = |source: &crate::feed_control::FeedSourceConfig| {
            existing
                .get(&source.id)
                .and_then(|row| OffsetDateTime::parse(&row.next_attempt_at, &Rfc3339).ok())
                .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        };
        scheduled_at(a)
            .cmp(&scheduled_at(b))
            .then_with(|| a.id.cmp(&b.id))
    });
    retain_host_fair(&mut due, |source| &source.url);
    due.truncate(limit);
    let mut updates = Vec::new();
    for source in due {
        let prior = existing.get(&source.id).cloned().unwrap_or_default();
        let conditional = FeedConditionalHeaders {
            etag: prior.etag.clone(),
            last_modified: prior.last_modified.clone(),
        };
        let attempted = SystemClock::new().now();
        let result = fetch_reviewed_feed(&source.url, &conditional, 2_000_000).await;
        let status = match result {
            Ok(response) => FeedRuntimeStatus {
                source_id: source.id,
                policy_version: source.version,
                state: if response.status == 304 { "not_modified" } else { "fetched_unparsed" }.into(),
                last_attempt_at: Some(attempted.to_string()),
                last_success_at: Some(attempted.to_string()),
                next_attempt_at: (attempted + time::Duration::seconds(source.minimum_poll_seconds as i64)).to_string(),
                consecutive_failures: 0,
                status_code: Some(response.status),
                bytes_received: response.bytes.len(),
                etag: response.etag.or(prior.etag),
                last_modified: response.last_modified.or(prior.last_modified),
                message: if response.status == 304 { "Publisher reported no change; conditional checkpoint advanced." } else { "Real bounded feed fetched. Body is not persisted or analyzed until an approved parser is activated." }.into(),
            },
            Err(error) => {
                let failures = prior.consecutive_failures.saturating_add(1);
                let delay = backoff_seconds(source.minimum_poll_seconds, failures);
                FeedRuntimeStatus { source_id: source.id, policy_version: source.version, state: "backoff".into(), last_attempt_at: Some(attempted.to_string()), last_success_at: prior.last_success_at, next_attempt_at: (attempted + time::Duration::seconds(delay as i64)).to_string(), consecutive_failures: failures, status_code: None, bytes_received: 0, etag: prior.etag, last_modified: prior.last_modified, message: error }
            }
        };
        updates.push(status);
    }
    if !updates.is_empty() {
        let mut runtime = RUNTIME
            .get()
            .ok_or("feed runtime is unavailable")?
            .lock()
            .map_err(|_| "feed runtime lock is unavailable".to_owned())?;
        let mut next = runtime.latest.clone();
        for row in &updates {
            next.insert(row.source_id.clone(), row.clone());
        }
        runtime
            .journal
            .append("scheduler_cycle", next.clone(), SystemClock::new().now())
            .map_err(|error| error.to_string())?;
        runtime.latest = next;
    }
    Ok(updates)
}

fn retain_host_fair<T>(rows: &mut Vec<T>, url: impl Fn(&T) -> &str) {
    let mut scheduled_hosts = BTreeSet::new();
    rows.retain(|row| {
        reviewed_feed_host(url(row))
            .map(|host| scheduled_hosts.insert(host))
            // Legacy invalid records still run once and produce an explicit
            // failed status instead of disappearing from operator visibility.
            .unwrap_or(true)
    });
}

fn source_is_due(
    source: &crate::feed_control::FeedSourceConfig,
    prior: Option<&FeedRuntimeStatus>,
    now: OffsetDateTime,
) -> bool {
    if !OffsetDateTime::parse(&source.valid_until, &Rfc3339).is_ok_and(|until| until > now) {
        return false;
    }
    prior.is_none_or(|row| {
        row.policy_version != source.version
            || OffsetDateTime::parse(&row.next_attempt_at, &Rfc3339).is_ok_and(|next| next <= now)
    })
}

fn backoff_seconds(minimum_poll_seconds: u64, failures: u16) -> u64 {
    minimum_poll_seconds
        .saturating_mul(2u64.saturating_pow(u32::from(failures.min(10))))
        .min(86_400)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_exponential_and_bounded() {
        assert_eq!(backoff_seconds(60, 1), 120);
        assert_eq!(backoff_seconds(900, 3), 7_200);
        assert_eq!(backoff_seconds(3_600, 20), 86_400);
    }

    #[test]
    fn scheduler_selects_at_most_one_source_per_host() {
        let mut urls = vec![
            "https://publisher.example/markets.xml",
            "https://publisher.example/company.xml",
            "https://other.example/feed.xml",
        ];
        retain_host_fair(&mut urls, |url| url);
        assert_eq!(
            urls,
            vec![
                "https://publisher.example/markets.xml",
                "https://other.example/feed.xml"
            ]
        );
    }
}
