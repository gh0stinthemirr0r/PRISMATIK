use async_trait::async_trait;
use prismatik_domain::{DataQualityScore, ProviderId};
use prismatik_identity::ExternalIdentifier;
use prismatik_market_data::{
    FetchMarkets, FetchMarketsError, InMemoryRawStore, Lineage, NormalizePipeline,
    ProviderChainExecutor, TransformId,
};
use std::collections::VecDeque;
use std::sync::Mutex;
use time::OffsetDateTime;

#[test]
fn lineage_invalidation_changes_with_pinned_digest() {
    let upstream = vec![[1; 32], [2; 32]];
    let first = Lineage::new(
        [9; 32],
        upstream.clone(),
        TransformId::new("normalize"),
        "1",
        [3; 32],
    );
    let second = Lineage::new(
        [9; 32],
        upstream,
        TransformId::new("normalize"),
        "1",
        [4; 32],
    );

    assert_ne!(first.invalidation_hash, second.invalidation_hash);
}

#[test]
fn normalization_deduplicates_identical_payloads() {
    let store = InMemoryRawStore::default();
    let pipeline = NormalizePipeline::new(&store);
    let now = OffsetDateTime::UNIX_EPOCH;
    let payload = serde_json::to_vec(&vec![prismatik_market_data::CryptoMarketQuote {
        coingecko_id: "bitcoin".into(),
        symbol: "BTC".into(),
        name: "Bitcoin".into(),
        price: "100".into(),
        change_24h_pct: Some("1.0".into()),
        volume_24h: Some("10".into()),
        market_cap: Some("1000".into()),
        provider: ProviderId::COINGECKO,
        external_id: ExternalIdentifier::CoinGeckoId("bitcoin".into()),
        event_time: now,
        retrieved_at: now,
        quality: DataQualityScore::PERFECT,
    }])
    .unwrap();

    let first = pipeline
        .ingest_market_quotes(&payload, ProviderId::COINGECKO, now, "raw/bitcoin.json")
        .unwrap();
    let second = pipeline
        .ingest_market_quotes(&payload, ProviderId::COINGECKO, now, "raw/bitcoin.json")
        .unwrap();

    assert!(first.inserted);
    assert!(!second.inserted);
    assert_eq!(first.dedup_key, second.dedup_key);
    assert_eq!(first.quality, DataQualityScore::PERFECT);
    assert_eq!(store.len(), 1);
}

#[derive(Debug)]
struct StubProvider {
    id: ProviderId,
    responses:
        Mutex<VecDeque<Result<Vec<prismatik_market_data::CryptoMarketQuote>, FetchMarketsError>>>,
}

#[async_trait]
impl FetchMarkets for StubProvider {
    fn provider_id(&self) -> ProviderId {
        self.id
    }

    async fn fetch_markets(
        &self,
    ) -> Result<Vec<prismatik_market_data::CryptoMarketQuote>, FetchMarketsError> {
        self.responses.lock().unwrap().pop_front().unwrap()
    }
}

#[tokio::test]
async fn provider_chain_fails_over_on_empty() {
    let first = StubProvider {
        id: ProviderId::COINGECKO,
        responses: Mutex::new(VecDeque::from([Err(FetchMarketsError::Empty)])),
    };
    let second = StubProvider {
        id: ProviderId::CCXT,
        responses: Mutex::new(VecDeque::from([Ok(Vec::new())])),
    };
    let executor = ProviderChainExecutor::new(vec![Box::new(first), Box::new(second)]);

    let result = executor.fetch_markets().await.unwrap();
    assert_eq!(result.provider, ProviderId::CCXT);
}
