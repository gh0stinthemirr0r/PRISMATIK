//! Lawful, conditional-fetch registry for large public feed corpora.

use crate::{AcquisitionMethod, SourcePolicy};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use time::OffsetDateTime;

/// One reviewed RSS or Atom source and its scheduler state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedRegistration {
    /// Stable source key.
    pub id: String,
    /// HTTPS feed URL.
    pub url: String,
    /// Reviewed source policy authorizing RSS acquisition.
    pub policy: SourcePolicy,
    /// Minimum polling cadence.
    pub minimum_poll_seconds: u64,
    /// Publisher/domain concurrency partition.
    pub host_partition: String,
    /// Last entity tag returned by the publisher.
    pub etag: Option<String>,
    /// Last-Modified value returned by the publisher.
    pub last_modified: Option<String>,
    /// Earliest lawful next attempt after cadence/backoff/jitter calculation.
    pub next_attempt_at: OffsetDateTime,
    /// Consecutive transient failure count.
    pub consecutive_failures: u16,
    /// Whether an operator has enabled this reviewed source.
    pub enabled: bool,
}

/// Registry validation error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeedRegistryError {
    /// Stable id is empty or duplicated.
    InvalidId,
    /// Only HTTPS feeds are accepted.
    InsecureUrl,
    /// Source policy does not authorize RSS.
    RssNotAuthorized,
    /// Poll cadence is too aggressive.
    UnsafeCadence,
}

/// Deterministic registry and fair scheduler for thousands of reviewed feeds.
#[derive(Clone, Debug, Default)]
pub struct FeedRegistry {
    feeds: BTreeMap<String, FeedRegistration>,
}

impl FeedRegistry {
    /// Register a reviewed feed. Catalog presence never enables acquisition by itself.
    pub fn register(&mut self, feed: FeedRegistration) -> Result<(), FeedRegistryError> {
        if feed.id.trim().is_empty() || self.feeds.contains_key(&feed.id) {
            return Err(FeedRegistryError::InvalidId);
        }
        if !feed.url.starts_with("https://") {
            return Err(FeedRegistryError::InsecureUrl);
        }
        if !feed
            .policy
            .allowed_methods()
            .contains(&AcquisitionMethod::Rss)
        {
            return Err(FeedRegistryError::RssNotAuthorized);
        }
        if feed.minimum_poll_seconds < 60 {
            return Err(FeedRegistryError::UnsafeCadence);
        }
        self.feeds.insert(feed.id.clone(), feed);
        Ok(())
    }

    /// Return due sources in stable time/id order, capped globally and once per host partition.
    pub fn due(&self, now: OffsetDateTime, limit: usize) -> Vec<FeedRegistration> {
        let mut rows: Vec<_> = self
            .feeds
            .values()
            .filter(|feed| feed.enabled && feed.next_attempt_at <= now)
            .cloned()
            .collect();
        rows.sort_by_key(|feed| (feed.next_attempt_at, feed.id.clone()));
        let mut hosts = BTreeSet::new();
        rows.into_iter()
            .filter(|feed| hosts.insert(feed.host_partition.clone()))
            .take(limit)
            .collect()
    }

    /// Update conditional-request metadata and schedule the next normal poll.
    pub fn record_success(
        &mut self,
        id: &str,
        now: OffsetDateTime,
        etag: Option<String>,
        last_modified: Option<String>,
    ) {
        if let Some(feed) = self.feeds.get_mut(id) {
            feed.etag = etag;
            feed.last_modified = last_modified;
            feed.consecutive_failures = 0;
            feed.next_attempt_at = now + time::Duration::seconds(feed.minimum_poll_seconds as i64);
        }
    }

    /// Apply bounded exponential backoff after a transient failure.
    pub fn record_failure(&mut self, id: &str, now: OffsetDateTime) {
        if let Some(feed) = self.feeds.get_mut(id) {
            feed.consecutive_failures = feed.consecutive_failures.saturating_add(1);
            let exponent = u32::from(feed.consecutive_failures.min(10));
            let seconds = feed
                .minimum_poll_seconds
                .saturating_mul(2u64.saturating_pow(exponent))
                .min(86_400);
            feed.next_attempt_at = now + time::Duration::seconds(seconds as i64);
        }
    }

    /// Number of registered sources.
    pub fn len(&self) -> usize {
        self.feeds.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.feeds.is_empty()
    }
}
