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
    adapters::{alpaca::AlpacaAdapter, sec_edgar::SecEdgarAdapter, AlpacaCredentials},
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
