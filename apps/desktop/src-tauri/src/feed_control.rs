//! Append-only control plane for reviewed RSS/Atom source policies.

use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use prismatik_application::{fetch_reviewed_feed, reviewed_feed_host, FeedConditionalHeaders};
use prismatik_determinism::{Clock, SystemClock};
use prismatik_market_data::{
    AcquisitionMethod, PayloadRetention, PermittedMetadataField, RedistributionPolicy, SourceId,
    SourcePolicy, SourcePolicyVersion,
};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

static STORE: OnceLock<Mutex<FeedControlStore>> = OnceLock::new();

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeedSourceConfig {
    pub(crate) id: String,
    publisher: String,
    pub(crate) url: String,
    pub(crate) enabled: bool,
    pub(crate) minimum_poll_seconds: u64,
    terms_owner: String,
    review_reference: String,
    pub(crate) valid_until: String,
    retain_raw_payload: bool,
    allow_metadata_redistribution: bool,
    pub(crate) version: u32,
    recorded_at: String,
}

pub(crate) fn scheduler_sources() -> Result<Vec<FeedSourceConfig>, String> {
    list_feed_sources()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeedSourceDraft {
    id: String,
    publisher: String,
    url: String,
    enabled: bool,
    minimum_poll_seconds: u64,
    terms_owner: String,
    review_reference: String,
    valid_until: String,
    retain_raw_payload: bool,
    allow_metadata_redistribution: bool,
}

#[derive(Debug)]
struct FeedControlStore {
    path: PathBuf,
    latest: BTreeMap<String, FeedSourceConfig>,
}

impl FeedControlStore {
    fn open(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&path)
            .map_err(|error| error.to_string())?;
        let mut latest = BTreeMap::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line =
                line.map_err(|error| format!("feed control ledger line {}: {error}", index + 1))?;
            if line.trim().is_empty() {
                continue;
            }
            let record: FeedSourceConfig = serde_json::from_str(&line)
                .map_err(|error| format!("feed control ledger line {}: {error}", index + 1))?;
            latest.insert(record.id.clone(), record);
        }
        Ok(Self { path, latest })
    }

    fn append(&mut self, draft: FeedSourceDraft) -> Result<FeedSourceConfig, String> {
        validate(&draft)?;
        let version = self
            .latest
            .get(&draft.id)
            .map_or(1, |record| record.version.saturating_add(1));
        let record = FeedSourceConfig {
            id: draft.id,
            publisher: draft.publisher,
            url: draft.url,
            enabled: draft.enabled,
            minimum_poll_seconds: draft.minimum_poll_seconds,
            terms_owner: draft.terms_owner,
            review_reference: draft.review_reference,
            valid_until: draft.valid_until,
            retain_raw_payload: draft.retain_raw_payload,
            allow_metadata_redistribution: draft.allow_metadata_redistribution,
            version,
            recorded_at: SystemClock::new().now().to_string(),
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| error.to_string())?;
        serde_json::to_writer(&mut file, &record).map_err(|error| error.to_string())?;
        file.write_all(b"\n")
            .and_then(|_| file.flush())
            .and_then(|_| file.sync_data())
            .map_err(|error| error.to_string())?;
        self.latest.insert(record.id.clone(), record.clone());
        Ok(record)
    }
}

fn validate(draft: &FeedSourceDraft) -> Result<(), String> {
    if draft.id.trim().is_empty()
        || draft.publisher.trim().is_empty()
        || draft.terms_owner.trim().is_empty()
        || draft.review_reference.trim().is_empty()
    {
        return Err("id, publisher, terms owner, and review reference are required".into());
    }
    reviewed_feed_host(&draft.url)?;
    if !(60..=604_800).contains(&draft.minimum_poll_seconds) {
        return Err("poll cadence must be between 60 and 604800 seconds".into());
    }
    let now = SystemClock::new().now();
    let until = OffsetDateTime::parse(&draft.valid_until, &Rfc3339)
        .map_err(|_| "validUntil must be RFC3339".to_owned())?;
    let source_id = SourceId::new(draft.id.clone()).map_err(|error| error.to_string())?;
    SourcePolicy::new(
        source_id,
        SourcePolicyVersion::new(1).map_err(|error| error.to_string())?,
        [AcquisitionMethod::Rss],
        if draft.retain_raw_payload {
            PayloadRetention::RawPayloadPermitted
        } else {
            PayloadRetention::MetadataOnly
        },
        if draft.allow_metadata_redistribution {
            RedistributionPolicy::PermittedMetadataOnly
        } else {
            RedistributionPolicy::Prohibited
        },
        [
            PermittedMetadataField::PublisherRecordId,
            PermittedMetadataField::ResponseHeaders,
            PermittedMetadataField::Title,
            PermittedMetadataField::Entities,
        ],
        draft.terms_owner.clone(),
        now,
        now,
        until,
        draft.review_reference.clone(),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn initialize(data_dir: &Path) -> Result<(), String> {
    STORE
        .set(Mutex::new(FeedControlStore::open(
            data_dir.join("feed-source-policies.jsonl"),
        )?))
        .map_err(|_| "feed control store initialized twice".to_owned())
}

#[tauri::command]
pub(crate) fn list_feed_sources() -> Result<Vec<FeedSourceConfig>, String> {
    Ok(STORE
        .get()
        .ok_or("feed control store is unavailable")?
        .lock()
        .map_err(|_| "feed control store lock is unavailable".to_owned())?
        .latest
        .values()
        .cloned()
        .collect())
}

#[tauri::command]
pub(crate) fn upsert_feed_source(source: FeedSourceDraft) -> Result<FeedSourceConfig, String> {
    STORE
        .get()
        .ok_or("feed control store is unavailable")?
        .lock()
        .map_err(|_| "feed control store lock is unavailable".to_owned())?
        .append(source)
}

#[tauri::command]
pub(crate) fn import_feed_sources(
    sources: Vec<FeedSourceDraft>,
) -> Result<Vec<FeedSourceConfig>, String> {
    if sources.is_empty() || sources.len() > 10_000 {
        return Err("feed import requires 1–10,000 reviewed sources".into());
    }
    for source in &sources {
        validate(source)?;
    }
    let mut store = STORE
        .get()
        .ok_or("feed control store is unavailable")?
        .lock()
        .map_err(|_| "feed control store lock is unavailable".to_owned())?;
    sources
        .into_iter()
        .map(|source| store.append(source))
        .collect()
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeedProbeResult {
    source_id: String,
    status: u16,
    media_type: Option<String>,
    bytes_received: usize,
    etag: Option<String>,
    last_modified: Option<String>,
    checked_at: String,
    message: String,
}

#[tauri::command]
pub(crate) async fn test_feed_source(source_id: String) -> Result<FeedProbeResult, String> {
    let source = STORE
        .get()
        .ok_or("feed control store is unavailable")?
        .lock()
        .map_err(|_| "feed control store lock is unavailable".to_owned())?
        .latest
        .get(&source_id)
        .cloned()
        .ok_or_else(|| "feed source is not registered".to_owned())?;
    let response =
        fetch_reviewed_feed(&source.url, &FeedConditionalHeaders::default(), 2_000_000).await?;
    Ok(FeedProbeResult {
        source_id,
        status: response.status,
        media_type: response.media_type,
        bytes_received: response.bytes.len(),
        etag: response.etag,
        last_modified: response.last_modified,
        checked_at: SystemClock::new().now().to_string(),
        message: "Real feed response passed DNS pinning, SSRF, redirect, content-type, timeout, and size gates. XML parsing was not attempted.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(url: &str) -> FeedSourceDraft {
        FeedSourceDraft {
            id: "publisher:markets".into(),
            publisher: "Publisher".into(),
            url: url.into(),
            enabled: false,
            minimum_poll_seconds: 900,
            terms_owner: "Data governance".into(),
            review_reference: "review:2026-001".into(),
            valid_until: "2099-01-01T00:00:00Z".into(),
            retain_raw_payload: false,
            allow_metadata_redistribution: false,
        }
    }

    #[test]
    fn reviewed_https_feed_passes_native_source_policy_validation() {
        assert!(validate(&draft("https://example.com/markets.xml")).is_ok());
    }

    #[test]
    fn insecure_or_aggressive_feed_is_denied() {
        assert!(validate(&draft("http://example.com/markets.xml")).is_err());
        assert!(validate(&draft("https://user@example.com/markets.xml")).is_err());
        assert!(validate(&draft("https://127.0.0.1/markets.xml")).is_err());
        let mut aggressive = draft("https://example.com/markets.xml");
        aggressive.minimum_poll_seconds = 10;
        assert!(validate(&aggressive).is_err());
    }
}
