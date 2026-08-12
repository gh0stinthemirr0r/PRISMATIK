//! Raw-first normalization pipeline for crypto market observations.

use crate::types::CryptoMarketQuote;
use prismatik_domain::{DataQualityScore, ProviderId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Mutex;
use thiserror::Error;
use time::OffsetDateTime;

/// Exact provider payload and retrieval provenance presented to raw storage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawObservation {
    /// Provider that returned the bytes.
    pub provider: ProviderId,
    /// Exact response body.
    pub payload: Vec<u8>,
    /// Caller-supplied retrieval time.
    pub retrieved_at: OffsetDateTime,
    /// Content-addressed payload digest.
    pub content_hash: [u8; 32],
    /// Canonical source locator.
    pub source_uri: String,
}

/// Raw append failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RawStoreError {
    /// Store-specific append failure.
    #[error("raw append failed: {0}")]
    Append(String),
}

/// Append-only raw-observation port.
pub trait RawStore: Send + Sync {
    /// Append, returning false for an idempotent duplicate.
    fn append(&self, observation: RawObservation) -> Result<bool, RawStoreError>;
}

/// Thread-safe in-memory raw store for deterministic tests.
#[derive(Debug, Default)]
pub struct InMemoryRawStore {
    hashes: Mutex<BTreeSet<[u8; 32]>>,
}

impl InMemoryRawStore {
    /// Number of unique raw payloads retained.
    pub fn len(&self) -> usize {
        self.hashes.lock().expect("raw store lock").len()
    }

    /// Whether no raw payloads have been retained.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl RawStore for InMemoryRawStore {
    fn append(&self, observation: RawObservation) -> Result<bool, RawStoreError> {
        Ok(self
            .hashes
            .lock()
            .expect("raw store lock")
            .insert(observation.content_hash))
    }
}

/// Normalized result linked to its raw deduplication key.
#[derive(Clone, Debug, PartialEq)]
pub struct NormalizedBatch {
    /// Parsed normalized rows.
    pub quotes: Vec<CryptoMarketQuote>,
    /// Whether the exact raw payload was newly appended.
    pub inserted: bool,
    /// Hex content key shared by replay and normalized output.
    pub dedup_key: String,
    /// Mean source quality, or zero for an empty response.
    pub quality: DataQualityScore,
}

/// Raw append or response-contract failure.
#[derive(Debug, Error)]
pub enum NormalizeError {
    /// Raw persistence failed.
    #[error(transparent)]
    Store(#[from] RawStoreError),
    /// Payload did not match the normalized response contract.
    #[error("market quote response contract failed: {0}")]
    Decode(#[from] serde_json::Error),
    /// A row claimed a different provider than the request boundary.
    #[error("normalized row provider does not match retrieval provider")]
    ProviderMismatch,
    /// Source URI was empty.
    #[error("raw observation source URI must not be empty")]
    EmptySourceUri,
}

/// Raw-first normalizer over an injected append-only store.
pub struct NormalizePipeline<'a> {
    store: &'a dyn RawStore,
}

impl std::fmt::Debug for NormalizePipeline<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NormalizePipeline")
            .finish_non_exhaustive()
    }
}

impl<'a> NormalizePipeline<'a> {
    /// Construct a pipeline over raw storage.
    pub fn new(store: &'a dyn RawStore) -> Self {
        Self { store }
    }

    /// Append exact bytes before decoding normalized crypto market quotes.
    pub fn ingest_market_quotes(
        &self,
        payload: &[u8],
        provider: ProviderId,
        retrieved_at: OffsetDateTime,
        source_uri: impl Into<String>,
    ) -> Result<NormalizedBatch, NormalizeError> {
        let source_uri = source_uri.into();
        if source_uri.trim().is_empty() {
            return Err(NormalizeError::EmptySourceUri);
        }
        let content_hash = *blake3::hash(payload).as_bytes();
        let inserted = self.store.append(RawObservation {
            provider,
            payload: payload.to_vec(),
            retrieved_at,
            content_hash,
            source_uri,
        })?;
        let quotes: Vec<CryptoMarketQuote> = serde_json::from_slice(payload)?;
        if quotes.iter().any(|quote| quote.provider != provider) {
            return Err(NormalizeError::ProviderMismatch);
        }
        let quality = if quotes.is_empty() {
            DataQualityScore::new(0.0)
        } else {
            DataQualityScore::new(
                quotes.iter().map(|quote| quote.quality.get()).sum::<f32>() / quotes.len() as f32,
            )
        };
        Ok(NormalizedBatch {
            quotes,
            inserted,
            dedup_key: hex::encode(content_hash),
            quality,
        })
    }
}
