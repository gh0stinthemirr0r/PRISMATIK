//! Live provider contract tests.
//!
//! Every one of these is `#[ignore]`d, so `cargo test` stays hermetic and CI
//! stays green without network. Run them deliberately:
//!
//! ```text
//! cargo test --test live_providers -- --ignored --nocapture
//! ```
//!
//! **Why these exist.** The unit tests prove our code is internally
//! consistent; they cannot notice when a provider changes its terms, moves an
//! endpoint behind a paywall, or alters a response shape. That failure mode is
//! silent and it is the one that actually reaches users — an integration that
//! "was working" and now returns an opaque 401. Finnhub moving daily candles
//! to a paid plan is precisely this, and nothing in the suite would have
//! caught it.
//!
//! Tests needing a secret read it from the environment and **skip loudly**
//! rather than failing, because "you have no key" and "the provider broke" are
//! different findings and must not look alike.

use std::{collections::BTreeMap, sync::Arc};

use prismatik_application::ReqwestTransport;
use prismatik_market_data::{
    adapters::{
        alpaca::AlpacaAdapter, cftc::CftcAdapter, sec_edgar::SecEdgarAdapter, AlpacaCredentials,
        BinanceAdapter, CoinbaseAdapter, GdeltAdapter, KalshiAdapter, KrakenAdapter,
        PolymarketAdapter,
    },
    http::{HttpMethod, HttpRequest, HttpTransport},
};

/// Read a credential, or report the skip and return `None`.
fn credential(variable: &str) -> Option<String> {
    match std::env::var(variable) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => {
            println!("SKIPPED — set {variable} to exercise this provider");
            None
        },
    }
}

/// SEC EDGAR needs only a contact address, so this one always runs.
///
/// It is the canary for the whole outbound path: if this passes, the
/// transport, the adapter and the response contract are all sound, and any
/// other provider's failure is about that provider's credential or terms
/// rather than about PRISMATIK.
#[tokio::test]
#[ignore = "network"]
async fn sec_edgar_submissions_still_parse() {
    let transport = ReqwestTransport::new("https://data.sec.gov").expect("transport");
    let adapter = SecEdgarAdapter::new(
        Arc::new(transport),
        "Mythos-PRISMATIK/0.1 live-test@example.com",
    )
    .expect("user agent accepted");

    // Apple's CIK. A stable, always-populated issuer.
    let filings = adapter.submissions(320_193).await.expect("submissions");
    assert!(
        !filings.is_empty(),
        "EDGAR returned no filings for CIK 320193 — the response contract has changed",
    );
    println!("SEC EDGAR ok — {} filings normalised", filings.len());
}

/// The three frontier catalog endpoints `test_model_provider` validates against.
///
/// Asserts the *shape* our parser depends on, not just a 200: a catalog that
/// stopped carrying `data`/`models` would break connection validation while
/// still returning success at the HTTP layer.
#[tokio::test]
#[ignore = "network"]
async fn frontier_model_catalogs_answer_in_the_expected_shape() {
    struct Case {
        name: &'static str,
        variable: &'static str,
        base: &'static str,
        header: Option<&'static str>,
        bearer: bool,
        query_key: bool,
        array_field: &'static str,
    }

    let cases = [
        Case {
            name: "openai",
            variable: "OPENAI_API_KEY",
            base: "https://api.openai.com/v1/",
            header: None,
            bearer: true,
            query_key: false,
            array_field: "data",
        },
        Case {
            name: "anthropic",
            variable: "ANTHROPIC_API_KEY",
            base: "https://api.anthropic.com/v1/",
            header: Some("x-api-key"),
            bearer: false,
            query_key: false,
            array_field: "data",
        },
        Case {
            name: "google",
            variable: "GOOGLE_API_KEY",
            base: "https://generativelanguage.googleapis.com/v1beta/",
            header: None,
            bearer: false,
            query_key: true,
            array_field: "models",
        },
    ];

    for case in cases {
        let Some(key) = credential(case.variable) else {
            continue;
        };
        let mut headers = BTreeMap::new();
        let mut query = BTreeMap::new();
        if case.bearer {
            headers.insert("authorization".to_owned(), format!("Bearer {key}"));
        }
        if let Some(name) = case.header {
            headers.insert(name.to_owned(), key.clone());
        }
        if case.query_key {
            query.insert("key".to_owned(), key.clone());
        }
        if case.name == "anthropic" {
            headers.insert("anthropic-version".to_owned(), "2023-06-01".to_owned());
        }

        let transport = ReqwestTransport::new(case.base).expect("transport");
        let response = transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: "models".into(),
                query,
                headers,
                body: None,
            })
            .await
            .unwrap_or_else(|error| panic!("{} transport failed: {error}", case.name));

        assert!(
            (200..300).contains(&response.status),
            "{} returned HTTP {} — credential rejected or endpoint moved",
            case.name,
            response.status,
        );
        let value: serde_json::Value =
            serde_json::from_str(&response.body).expect("catalog is JSON");
        let count = value
            .get(case.array_field)
            .and_then(|value| value.as_array())
            .map(Vec::len)
            .unwrap_or_else(|| {
                panic!(
                    "{} catalog no longer carries a `{}` array — connection validation will \
                     report a false failure",
                    case.name, case.array_field
                )
            });
        assert!(count > 0, "{} returned an empty catalog", case.name);
        println!("{} ok — {count} models", case.name);
    }
}

/// Finnhub's daily-candle endpoint, which connection validation depends on.
///
/// Broken out on its own because it is the known-fragile one: `/stock/candle`
/// moved to a paid plan, so a perfectly valid free key now yields a 403 and
/// the desk reports "Finnhub validation failed" for what is really an
/// entitlement change. The assertion message says so, because the next person
/// to see this fail should not spend an afternoon on their key.
#[tokio::test]
#[ignore = "network"]
async fn finnhub_daily_candles_are_still_reachable() {
    let Some(token) = credential("FINNHUB_API_KEY") else {
        return;
    };
    let transport = ReqwestTransport::new("https://finnhub.io/api/v1").expect("transport");
    let response = transport
        .execute(&HttpRequest {
            method: HttpMethod::Get,
            path: "stock/candle".into(),
            query: BTreeMap::from([
                ("symbol".to_owned(), "AAPL".to_owned()),
                ("resolution".to_owned(), "D".to_owned()),
                ("from".to_owned(), "1750000000".to_owned()),
                ("to".to_owned(), "1755000000".to_owned()),
                ("token".to_owned(), token),
            ]),
            headers: BTreeMap::new(),
            body: None,
        })
        .await
        .expect("transport");

    assert_ne!(
        response.status, 403,
        "Finnhub answered 403: the key is probably fine and /stock/candle is a paid endpoint \
         now. Connection validation cannot succeed on a free plan until the adapter moves to an \
         endpoint the plan includes.",
    );
    assert!(
        (200..300).contains(&response.status),
        "Finnhub returned HTTP {}",
        response.status,
    );
    println!("Finnhub ok — candles reachable");
}

/// Alpaca market data, which the desk needs before it can ever place an order.
///
/// Alpaca authenticates with two headers rather than a bearer token, and free
/// accounts are entitled to the IEX feed only — asking for the default SIP
/// feed returns a subscription error that reads exactly like a rejected key.
/// Both of those were wrong in the adapter until this test existed.
#[tokio::test]
#[ignore = "network"]
async fn alpaca_bars_authenticate_and_parse() {
    let (Some(key_id), Some(secret_key)) = (
        credential("ALPACA_API_KEY_ID"),
        credential("ALPACA_API_SECRET_KEY"),
    ) else {
        return;
    };
    let transport = ReqwestTransport::new("https://data.alpaca.markets").expect("transport");
    let adapter = AlpacaAdapter::with_credentials(
        Arc::new(transport),
        AlpacaCredentials { key_id, secret_key },
    );
    let bars = adapter
        .bars("AAPL", "1Day", time::OffsetDateTime::now_utc())
        .await
        .expect("bars");
    assert!(
        !bars.is_empty(),
        "Alpaca authenticated but returned no bars — the account most likely lacks a \
         market-data entitlement for the IEX feed",
    );
    println!("Alpaca ok — {} AAPL daily bars", bars.len());
}

/// CFTC Commitments of Traders. Public data, no credential.
///
/// The adapter previously called `/api/v1/commitments`, which the CFTC does
/// not operate, so this integration had never worked. It now reads the public
/// Socrata dataset.
#[tokio::test]
#[ignore = "network"]
async fn cftc_commitments_parse_from_socrata() {
    let transport = ReqwestTransport::new("https://publicreporting.cftc.gov").expect("transport");
    let adapter = CftcAdapter::new(Arc::new(transport));

    // CBOT wheat: a contract with continuous weekly history.
    let report = adapter.commitments("001602").await.expect("commitments");
    assert!(
        report.open_interest > 0,
        "open interest should be positive, got {}",
        report.open_interest,
    );
    assert!(
        report.market_name.to_uppercase().contains("WHEAT"),
        "{}",
        report.market_name
    );
    println!(
        "CFTC ok — {} as of {}: {} long / {} short, OI {}",
        report.market_name,
        report.as_of,
        report.long_positions,
        report.short_positions,
        report.open_interest,
    );
}

/// Kraken daily candles. Public, no credential.
#[tokio::test]
#[ignore = "network"]
async fn kraken_candles_parse() {
    let transport = ReqwestTransport::new("https://api.kraken.com").expect("transport");
    let adapter = KrakenAdapter::new(Arc::new(transport));
    let candles = adapter
        .candles("XBTUSD", 1_440, time::OffsetDateTime::now_utc())
        .await
        .expect("candles");
    assert!(!candles.is_empty(), "Kraken returned no candles");
    let last = candles.last().expect("candle");
    // Sanity on the column mapping rather than merely on the count: a high
    // below the low would mean the positional read is wrong.
    let high: f64 = last.high.parse().expect("high");
    let low: f64 = last.low.parse().expect("low");
    assert!(
        high >= low,
        "high {high} below low {low} — column order is wrong"
    );
    println!(
        "Kraken ok — {} candles, last close {}",
        candles.len(),
        last.close
    );
}

/// Coinbase Exchange daily candles. Public, no credential.
///
/// Coinbase orders its columns `[time, low, high, open, close, volume]`, so
/// this also guards the one mapping most likely to be "corrected" into a bug.
#[tokio::test]
#[ignore = "network"]
async fn coinbase_candles_parse_in_the_right_column_order() {
    let transport = ReqwestTransport::new("https://api.exchange.coinbase.com").expect("transport");
    let adapter = CoinbaseAdapter::new(Arc::new(transport));
    let candles = adapter
        .candles("BTC-USD", 86_400, time::OffsetDateTime::now_utc())
        .await
        .expect("candles");
    assert!(!candles.is_empty(), "Coinbase returned no candles");
    for candle in candles.iter().take(20) {
        let (high, low, open, close): (f64, f64, f64, f64) = (
            candle.high.parse().expect("high"),
            candle.low.parse().expect("low"),
            candle.open.parse().expect("open"),
            candle.close.parse().expect("close"),
        );
        assert!(high >= low, "high {high} below low {low}");
        assert!(
            open >= low && open <= high,
            "open {open} outside [{low}, {high}]"
        );
        assert!(
            close >= low && close <= high,
            "close {close} outside [{low}, {high}]"
        );
    }
    println!("Coinbase ok — {} candles, columns in range", candles.len());
}

/// Both public prediction venues. No credential.
#[tokio::test]
#[ignore = "network"]
async fn prediction_venues_list_open_markets() {
    let now = time::OffsetDateTime::now_utc();

    let poly = PolymarketAdapter::new(Arc::new(
        ReqwestTransport::new("https://gamma-api.polymarket.com").expect("transport"),
    ));
    let markets = poly.open_markets(20, now).await.expect("polymarket");
    assert!(!markets.is_empty(), "Polymarket returned no open markets");
    for market in &markets {
        if let Some(price) = &market.yes_price {
            let parsed: f64 = price.parse().expect("price parses");
            assert!(
                parsed > 0.0 && parsed <= 1.0,
                "{price} is not a probability"
            );
        }
    }
    println!(
        "Polymarket ok — {} markets, {} quoted",
        markets.len(),
        markets.iter().filter(|m| m.yes_price.is_some()).count(),
    );

    // Kalshi is queried by series, not through the open listing. That listing
    // is dominated by auto-generated sports parlays with no bid on either
    // side — a full thousand-row sweep returned not one quoted market — so it
    // cannot demonstrate a working connection. A named series can.
    let kalshi = KalshiAdapter::new(Arc::new(
        ReqwestTransport::new("https://api.elections.kalshi.com").expect("transport"),
    ));
    let markets = kalshi
        .markets_in_series("KXFEDDECISION", 20, now)
        .await
        .expect("kalshi");
    assert!(
        !markets.is_empty(),
        "Kalshi returned no markets for KXFEDDECISION"
    );
    let quoted = markets.iter().filter(|m| m.yes_price.is_some()).count();
    assert!(quoted > 0, "no market in KXFEDDECISION carried a quote");
    for market in &markets {
        if let Some(price) = &market.yes_price {
            let parsed: f64 = price.parse().expect("price parses");
            assert!(
                parsed > 0.0 && parsed <= 1.0,
                "{price} is not a probability"
            );
        }
    }
    println!(
        "Kalshi ok — {} markets in KXFEDDECISION, {quoted} quoted",
        markets.len()
    );
}

/// Binance and GDELT. Public, no credential.
///
/// Binance is pointed at binance.us because the global host answers 451 to US
/// addresses. GDELT throttles hard, so a 429 here is the service saying "not
/// now" rather than a broken adapter — the assertion says so.
#[tokio::test]
#[ignore = "network"]
async fn binance_and_gdelt_are_reachable() {
    let now = time::OffsetDateTime::now_utc();

    let binance = BinanceAdapter::new(Arc::new(
        ReqwestTransport::new("https://api.binance.us").expect("transport"),
    ));
    let candles = binance
        .candles("BTCUSDT", "1d", 5, now)
        .await
        .expect("klines");
    assert!(!candles.is_empty());
    for candle in &candles {
        // Millisecond timestamps read as seconds would land ~50,000 years out.
        let year = candle.bar_start.year();
        assert!((2000..2100).contains(&year), "implausible year {year}");
    }
    println!(
        "Binance ok — {} klines, last close {}",
        candles.len(),
        candles.last().unwrap().close
    );

    let gdelt = GdeltAdapter::new(Arc::new(
        ReqwestTransport::new("https://api.gdeltproject.org").expect("transport"),
    ));
    match gdelt.search("stock market", 5).await {
        Ok(articles) => println!("GDELT ok — {} articles", articles.len()),
        Err(error) => {
            let text = format!("{error}");
            assert!(
                text.contains("rate limited"),
                "GDELT failed for a reason other than throttling: {text}",
            );
            println!("GDELT throttled — reported as such, not as a decode failure");
        },
    }
}
