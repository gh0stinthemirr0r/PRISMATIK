use prismatik_domain::ProviderId;
use prismatik_market_data::{
    adapters::{
        alpaca::{demo_cassette_json as alpaca_cassette, AlpacaAdapter},
        cftc::{demo_cassette_json as cftc_cassette, CftcAdapter},
        finnhub::{demo_cassette_json as finnhub_cassette, FinnhubAdapter},
        fred::{demo_cassette_json as fred_cassette, FredAdapter},
        sec_edgar::{demo_cassette_json as sec_cassette, SecEdgarAdapter},
    },
    Capability, CassetteTransport, CostUnits, Entitlement, Provider, ProviderChain,
    ProviderRequest,
};
use std::sync::Arc;
use time::{Date, Month, OffsetDateTime};

#[tokio::test]
async fn sec_edgar_replays_all_wave_2a_endpoints() {
    let adapter = SecEdgarAdapter::new(
        Arc::new(CassetteTransport::from_json(sec_cassette()).unwrap()),
        "Mythos-PRISMATIK/0.1 contact:dev@mythos.systems",
    )
    .unwrap();
    assert_eq!(adapter.id(), ProviderId::SEC_EDGAR);
    assert!(adapter.entitlements().contains(Entitlement::SecEdgar));
    assert!(adapter.capabilities().supports(Capability::CompanyFacts));
    assert_eq!(
        adapter.cost_of(&ProviderRequest::new("submissions")),
        CostUnits::new(1)
    );

    let filings = adapter.submissions(320193).await.unwrap();
    assert_eq!(filings[0].form, "10-K");
    let facts = adapter.company_facts(320193).await.unwrap();
    assert!(facts.iter().any(|fact| fact.concept == "Revenues"));
    let hits = adapter.filing_index("AAPL", "4").await.unwrap();
    assert_eq!(hits[0].form, "4");
}

#[test]
fn sec_edgar_rejects_an_undeclared_user_agent() {
    let transport = Arc::new(CassetteTransport::from_json(sec_cassette()).unwrap());
    assert!(SecEdgarAdapter::new(transport, "generic-client").is_err());
}

#[tokio::test]
async fn cftc_fred_and_alpaca_cassettes_normalize() {
    let cftc = CftcAdapter::new(Arc::new(
        CassetteTransport::from_json(cftc_cassette()).unwrap(),
    ));
    let cot = cftc.commitments("067651").await.unwrap();
    assert_eq!(cot.market_code, "067651");
    assert!(cot.published_at.date() > cot.as_of);

    let fred = FredAdapter::new(
        Arc::new(CassetteTransport::from_json(fred_cassette()).unwrap()),
        "demo",
    );
    let points = fred.observations("GDP").await.unwrap();
    assert_eq!(
        points[0].available_at,
        Date::from_calendar_date(2024, Month::April, 25).unwrap()
    );

    let alpaca = AlpacaAdapter::new(Arc::new(
        CassetteTransport::from_json(alpaca_cassette()).unwrap(),
    ));
    let bars = alpaca
        .bars("AAPL", "1Day", OffsetDateTime::UNIX_EPOCH)
        .await
        .unwrap();
    assert_eq!(bars[0].symbol, "AAPL");
    assert_eq!(bars[0].provider, ProviderId::ALPACA);
    assert!(bars[0].volume > 0);
}

#[tokio::test]
async fn finnhub_cassette_normalizes_equity_bars() {
    let adapter = FinnhubAdapter::new(
        Arc::new(CassetteTransport::from_json(finnhub_cassette()).unwrap()),
        "demo",
    );
    assert_eq!(adapter.id(), ProviderId::FINNHUB);
    assert!(adapter.entitlements().contains(Entitlement::Finnhub));
    assert!(adapter.capabilities().supports(Capability::Bars));
    assert_eq!(
        adapter.cost_of(&ProviderRequest::new("stock_candle")),
        CostUnits::new(1)
    );

    let bars = adapter
        .bars(
            "aapl",
            "D",
            1_721_606_400,
            1_721_692_800,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .unwrap();
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].symbol, "AAPL");
    assert_eq!(bars[0].provider, ProviderId::FINNHUB);
    assert_eq!(bars[0].close, "223.96");
    assert!(bars[0].volume > 0);
    assert!(bars[0].trade_count.is_none());
    assert!(bars[0].vwap.is_none());
}

#[test]
fn equity_default_chains_prefer_alpaca_with_finnhub_fallback() {
    let chains = ProviderChain::equity_default_chains();
    assert!(chains.iter().any(|c| {
        c.capability == Capability::Bars
            && c.primary == ProviderId::ALPACA
            && c.fallbacks == vec![ProviderId::FINNHUB]
    }));
    assert!(chains.iter().any(|c| {
        c.capability == Capability::Ohlcv
            && c.primary == ProviderId::ALPACA
            && c.fallbacks == vec![ProviderId::FINNHUB]
    }));
}
