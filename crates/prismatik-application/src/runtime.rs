//! Desktop application runtime composition.

use prismatik_determinism::{
    Clock, DeterminismContext, PinnedArtifactSet, RunId, SplitEntropy, SystemClock,
};
use prismatik_domain::ProviderId;
use prismatik_market_data::{
    GcraBudgetGovernor, IngestError, NormalizePipeline, NormalizedBatch, RawObservation, RawStore,
    RawStoreError,
};
use prismatik_storage::{
    AnalyticalBackend, DesktopProfile, ParquetRawWriter, RawAppendError, RawRecord,
    RepositoryError, SqliteBackend,
};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

/// Runtime construction and ingestion failures.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Profile directory creation failed.
    #[error("runtime I/O: {0}")]
    Io(#[from] std::io::Error),
    /// Operational storage failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// Market-data normalization or raw append failed.
    #[error(transparent)]
    Ingest(#[from] IngestError),
}

/// Wave 1 desktop runtime and its profile-scoped services.
#[derive(Debug)]
pub struct ApplicationRuntime {
    /// Profile paths.
    pub profile: DesktopProfile,
    /// SQLite operational backend.
    pub sqlite: SqliteBackend,
    /// Append-only raw Parquet writer.
    pub raw_writer: ParquetRawWriter,
    /// Curated analytical backend.
    pub analytical: AnalyticalBackend,
    /// Provider budget governor.
    pub governor: GcraBudgetGovernor,
    /// Wall-clock determinism boundary for the live application shell.
    pub determinism: DeterminismContext,
}

impl ApplicationRuntime {
    /// Open or create a complete desktop profile.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, RuntimeError> {
        let profile = DesktopProfile::new(root.into());
        profile.ensure_dirs()?;
        let sqlite = SqliteBackend::open(profile.sqlite_path())?;
        let raw_writer = ParquetRawWriter::new(profile.raw_parquet_root());
        let analytical = AnalyticalBackend::new(profile.curated_parquet_root());
        let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
        let mut run_hasher = blake3::Hasher::new();
        run_hasher.update(&clock.now().unix_timestamp_nanos().to_le_bytes());
        run_hasher.update(profile.root.to_string_lossy().as_bytes());
        let run_digest = run_hasher.finalize();
        let seed = u64::from_le_bytes(run_digest.as_bytes()[..8].try_into().expect("eight bytes"));
        let mut run_id = [0_u8; 16];
        run_id.copy_from_slice(&run_digest.as_bytes()[..16]);
        let determinism = DeterminismContext {
            clock,
            entropy: Box::new(SplitEntropy::from_seed(seed)),
            run_id: RunId::from_bytes(run_id),
            pinned: PinnedArtifactSet::default(),
        };
        Ok(Self {
            profile,
            sqlite,
            raw_writer,
            analytical,
            governor: GcraBudgetGovernor::coingecko_demo(),
            determinism,
        })
    }

    /// Normalize a provider JSON array through the market-data ingest pipeline
    /// and append its exact bytes to raw Parquet.
    pub fn ingest_raw_json(
        &self,
        provider: ProviderId,
        dataset: &str,
        payload: &[u8],
    ) -> Result<NormalizedBatch, RuntimeError> {
        let retrieved_at = self.determinism.clock.now();
        let store = RuntimeRawStore {
            writer: &self.raw_writer,
            dataset,
        };
        Ok(NormalizePipeline::new(&store).ingest_market_quotes(
            payload,
            provider,
            retrieved_at,
            format!("{provider}/{dataset}"),
        )?)
    }
}

#[derive(Debug)]
struct RuntimeRawStore<'a> {
    writer: &'a ParquetRawWriter,
    dataset: &'a str,
}

impl RawStore for RuntimeRawStore<'_> {
    fn append(&self, observation: RawObservation) -> Result<bool, RawStoreError> {
        let micros = observation.retrieved_at.unix_timestamp_nanos() / 1_000;
        let micros = i64::try_from(micros).unwrap_or_else(|_| {
            if micros.is_negative() {
                i64::MIN
            } else {
                i64::MAX
            }
        });
        let payload_json = String::from_utf8(observation.payload)
            .map_err(|error| RawStoreError::Append(error.to_string()))?;
        let record = RawRecord {
            provider: observation.provider.to_string(),
            dataset: self.dataset.into(),
            event_time_us: micros,
            retrieved_at_us: micros,
            payload_json,
            content_hash_hex: hex::encode(observation.content_hash),
        };
        match self.writer.append(&record) {
            Ok(_) => Ok(true),
            Err(RawAppendError::AlreadyExists(_)) => Ok(false),
            Err(error) => Err(RawStoreError::Append(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_domain::DataQualityScore;
    use prismatik_identity::ExternalIdentifier;
    use prismatik_market_data::CryptoMarketQuote;
    use time::OffsetDateTime;

    #[test]
    fn open_and_ingest_writes_raw_parquet() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = ApplicationRuntime::open(temp.path()).unwrap();
        let payload = serde_json::to_vec(&vec![CryptoMarketQuote {
            coingecko_id: "bitcoin".into(),
            symbol: "BTC".into(),
            name: "Bitcoin".into(),
            price: "100".into(),
            change_24h_pct: Some("1".into()),
            volume_24h: None,
            market_cap: None,
            provider: ProviderId::COINGECKO,
            external_id: ExternalIdentifier::CoinGeckoId("bitcoin".into()),
            event_time: OffsetDateTime::UNIX_EPOCH,
            retrieved_at: OffsetDateTime::UNIX_EPOCH,
            quality: DataQualityScore::PERFECT,
        }])
        .unwrap();
        let batch = runtime
            .ingest_raw_json(ProviderId::COINGECKO, "markets", &payload)
            .unwrap();
        assert_eq!(batch.quotes.len(), 1);
        assert!(batch.inserted);
        assert!(
            !runtime
                .ingest_raw_json(ProviderId::COINGECKO, "markets", &payload)
                .unwrap()
                .inserted
        );
    }
}
