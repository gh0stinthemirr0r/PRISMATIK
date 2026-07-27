//! Offline multi-asset experience fixtures (equity, filings, macro, options).
//!
//! These commands deliberately use embedded, deterministic demo data. They expose
//! the final IPC shapes without introducing network access in the desktop shell.

use serde::Serialize;

const AS_OF: &str = "2026-07-25T20:00:00Z";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityBar {
    time: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityBars {
    symbol: String,
    currency: String,
    provider: String,
    retrieved_at: String,
    bars: Vec<EquityBar>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilingSummary {
    cik: String,
    issuer: String,
    form: String,
    filed_at: String,
    period: String,
    headline: String,
    value_usd: u64,
    provider: String,
    retrieved_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsiderTransaction {
    cik: String,
    issuer: String,
    insider: String,
    title: String,
    transaction_code: String,
    shares: u64,
    price: f64,
    filed_at: String,
    provider: String,
    retrieved_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroPoint {
    date: String,
    value: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroSeries {
    series_id: String,
    title: String,
    unit: String,
    provider: String,
    retrieved_at: String,
    points: Vec<MacroPoint>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CotReport {
    market_code: String,
    market_name: String,
    report_date: String,
    long: u64,
    short: u64,
    net: i64,
    provider: String,
    retrieved_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionContract {
    occ_symbol: String,
    expiration: String,
    strike: f64,
    option_type: String,
    bid: f64,
    ask: f64,
    implied_volatility: f64,
    open_interest: u64,
    delta: f64,
    gamma: f64,
    theta: f64,
    vega: f64,
    liquidity_score: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsChain {
    underlying: String,
    spot: f64,
    provider: String,
    retrieved_at: String,
    contracts: Vec<OptionContract>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowPrint {
    id: String,
    occ_symbol: String,
    side: String,
    contracts: u64,
    premium: f64,
    classification: String,
    confidence: f64,
    trade_quality_version: String,
    evidence: String,
    occurred_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealerExposure {
    underlying: String,
    net_gex: f64,
    net_dex: f64,
    by_strike: Vec<DealerStrike>,
    provider: String,
    retrieved_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealerStrike {
    strike: f64,
    gex: f64,
    dex: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalystEvent {
    id: String,
    kind: String,
    title: String,
    occurs_at: String,
    related_symbol: String,
    provider: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionsFlow {
    underlying: String,
    provider: String,
    retrieved_at: String,
    prints: Vec<FlowPrint>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextSession {
    venue: String,
    opens_at: String,
    closes_at: String,
    state: String,
    provider: String,
    retrieved_at: String,
}

fn equity_bars(symbol: &str) -> Result<EquityBars, String> {
    let symbol = symbol.trim().to_uppercase();
    if symbol != "AAPL" {
        return Err(format!("offline equity fixture unavailable for {symbol}"));
    }
    Ok(EquityBars {
        symbol,
        currency: "USD".into(),
        provider: "embedded-equity-cassette".into(),
        retrieved_at: AS_OF.into(),
        bars: vec![
            EquityBar {
                time: 1_753_449_600,
                open: 213.88,
                high: 215.42,
                low: 212.61,
                close: 214.95,
                volume: 48_921_300,
            },
            EquityBar {
                time: 1_753_536_000,
                open: 215.12,
                high: 216.75,
                low: 214.30,
                close: 216.22,
                volume: 51_304_100,
            },
            EquityBar {
                time: 1_753_622_400,
                open: 216.02,
                high: 217.21,
                low: 214.92,
                close: 215.68,
                volume: 46_872_900,
            },
            EquityBar {
                time: 1_753_708_800,
                open: 215.94,
                high: 218.04,
                low: 215.41,
                close: 217.73,
                volume: 54_180_600,
            },
        ],
    })
}

fn filing_summaries(cik: Option<&str>) -> Vec<FilingSummary> {
    let rows = vec![
        FilingSummary {
            cik: "0000320193".into(),
            issuer: "Apple Inc.".into(),
            form: "13F-HR".into(),
            filed_at: "2026-07-18".into(),
            period: "2026-Q2".into(),
            headline: "Demo institutional holdings snapshot".into(),
            value_usd: 1_284_000_000,
            provider: "sec-embedded".into(),
            retrieved_at: AS_OF.into(),
        },
        FilingSummary {
            cik: "0000789019".into(),
            issuer: "Microsoft Corp.".into(),
            form: "4".into(),
            filed_at: "2026-07-22".into(),
            period: "2026-Q3".into(),
            headline: "Officer equity award disposition".into(),
            value_usd: 2_740_500,
            provider: "sec-embedded".into(),
            retrieved_at: AS_OF.into(),
        },
    ];
    rows.into_iter()
        .filter(|row| cik.is_none_or(|value| row.cik == value))
        .collect()
}

fn insider_transactions(cik: Option<&str>) -> Vec<InsiderTransaction> {
    let rows = vec![
        InsiderTransaction {
            cik: "0000320193".into(),
            issuer: "Apple Inc.".into(),
            insider: "Alex Morgan".into(),
            title: "SVP, Operations".into(),
            transaction_code: "S".into(),
            shares: 12_500,
            price: 216.18,
            filed_at: "2026-07-23".into(),
            provider: "sec-embedded".into(),
            retrieved_at: AS_OF.into(),
        },
        InsiderTransaction {
            cik: "0000789019".into(),
            issuer: "Microsoft Corp.".into(),
            insider: "Jordan Lee".into(),
            title: "EVP".into(),
            transaction_code: "A".into(),
            shares: 4_000,
            price: 0.0,
            filed_at: "2026-07-22".into(),
            provider: "sec-embedded".into(),
            retrieved_at: AS_OF.into(),
        },
    ];
    rows.into_iter()
        .filter(|row| cik.is_none_or(|value| row.cik == value))
        .collect()
}

fn macro_series(series_id: &str) -> Result<MacroSeries, String> {
    let id = series_id.trim().to_uppercase();
    let (title, unit, values) = match id.as_str() {
        "DGS10" => (
            "10-Year Treasury Constant Maturity Rate",
            "percent",
            [4.31, 4.28, 4.34, 4.37],
        ),
        "CPIAUCSL" => (
            "Consumer Price Index for All Urban Consumers",
            "index",
            [321.2, 321.9, 322.4, 323.1],
        ),
        _ => return Err(format!("offline FRED fixture unavailable for {id}")),
    };
    Ok(MacroSeries {
        series_id: id,
        title: title.into(),
        unit: unit.into(),
        provider: "fred-embedded".into(),
        retrieved_at: AS_OF.into(),
        points: ["2026-04-01", "2026-05-01", "2026-06-01", "2026-07-01"]
            .into_iter()
            .zip(values)
            .map(|(date, value)| MacroPoint {
                date: date.into(),
                value,
            })
            .collect(),
    })
}

fn cot_report(market_code: &str) -> Result<CotReport, String> {
    if market_code.trim() != "067651" {
        return Err(format!("offline COT fixture unavailable for {market_code}"));
    }
    Ok(CotReport {
        market_code: "067651".into(),
        market_name: "WTI Crude Oil".into(),
        report_date: "2026-07-21".into(),
        long: 322_410,
        short: 201_884,
        net: 120_526,
        provider: "cftc-embedded".into(),
        retrieved_at: AS_OF.into(),
    })
}

fn options_chain(underlying: &str) -> Result<OptionsChain, String> {
    let underlying = underlying.trim().to_uppercase();
    if underlying != "AAPL" {
        return Err(format!(
            "offline options fixture unavailable for {underlying}"
        ));
    }
    Ok(OptionsChain {
        underlying,
        spot: 217.73,
        provider: "occ-embedded".into(),
        retrieved_at: AS_OF.into(),
        // Dense-enough term×strike grid for the IV surface mesh (offline fixture).
        contracts: {
            let mut rows = Vec::new();
            let terms = [
                ("2026-08-21", "260821"),
                ("2026-09-18", "260918"),
                ("2026-10-16", "261016"),
            ];
            let strikes = [205.0, 210.0, 215.0, 220.0, 225.0, 230.0];
            for (ti, (expiration, yymmdd)) in terms.iter().enumerate() {
                for (si, strike) in strikes.iter().enumerate() {
                    let moneyness: f64 = (strike - 217.73) / 217.73;
                    let term_bump = 0.004 * ti as f64;
                    let call_iv = 0.255 + moneyness.abs() * 0.22 + term_bump;
                    let put_iv = call_iv + 0.008;
                    let strike_code = format!("{:08}", (*strike as i32) * 1000);
                    let liq = (0.95 - (si as f64 - 2.5).abs() * 0.06).clamp(0.55, 0.97);
                    rows.push(OptionContract {
                        occ_symbol: format!("AAPL  {yymmdd}C{strike_code}"),
                        expiration: (*expiration).into(),
                        strike: *strike,
                        option_type: "call".into(),
                        bid: (8.5 - moneyness.abs() * 12.0 - ti as f64).max(0.35),
                        ask: (8.75 - moneyness.abs() * 12.0 - ti as f64).max(0.45),
                        implied_volatility: call_iv,
                        open_interest: 8_000 + (si * 1_200 + ti * 900) as u64,
                        delta: (0.55 - moneyness * 2.2).clamp(0.08, 0.92),
                        gamma: 0.016 + (0.01 - moneyness.abs() * 0.008).max(0.0),
                        theta: -0.13 + ti as f64 * 0.01,
                        vega: 0.30 + ti as f64 * 0.04,
                        liquidity_score: liq,
                    });
                    rows.push(OptionContract {
                        occ_symbol: format!("AAPL  {yymmdd}P{strike_code}"),
                        expiration: (*expiration).into(),
                        strike: *strike,
                        option_type: "put".into(),
                        bid: (5.4 + moneyness.abs() * 9.0).max(0.3),
                        ask: (5.65 + moneyness.abs() * 9.0).max(0.4),
                        implied_volatility: put_iv,
                        open_interest: 6_500 + (si * 900 + ti * 700) as u64,
                        delta: (-0.45 - moneyness * 1.8).clamp(-0.92, -0.08),
                        gamma: 0.015 + (0.01 - moneyness.abs() * 0.008).max(0.0),
                        theta: -0.11 + ti as f64 * 0.008,
                        vega: 0.29 + ti as f64 * 0.035,
                        liquidity_score: (liq - 0.05).clamp(0.5, 0.95),
                    });
                }
            }
            rows
        },
    })
}

fn options_flow(underlying: &str) -> Result<OptionsFlow, String> {
    let underlying = underlying.trim().to_uppercase();
    if underlying != "AAPL" {
        return Err(format!(
            "offline options-flow fixture unavailable for {underlying}"
        ));
    }
    Ok(OptionsFlow {
        underlying,
        provider: "opra-embedded".into(),
        retrieved_at: AS_OF.into(),
        prints: vec![
            FlowPrint {
                id: "flow-001".into(),
                occ_symbol: "AAPL  260821C00220000".into(),
                side: "ask".into(),
                contracts: 640,
                premium: 355_200.0,
                classification: "Directional".into(),
                confidence: 0.91,
                trade_quality_version: "tq.v1".into(),
                evidence: "opening ask · size > OI · sweep tape".into(),
                occurred_at: "2026-07-25T19:42:11Z".into(),
            },
            FlowPrint {
                id: "flow-002".into(),
                occ_symbol: "AAPL  260821P00215000".into(),
                side: "mid".into(),
                contracts: 225,
                premium: 120_150.0,
                classification: "SpreadLeg".into(),
                confidence: 0.68,
                trade_quality_version: "tq.v1".into(),
                evidence: "mid print · paired legs suspected".into(),
                occurred_at: "2026-07-25T19:48:03Z".into(),
            },
            FlowPrint {
                id: "flow-003".into(),
                occ_symbol: "AAPL  260918C00225000".into(),
                side: "bid".into(),
                contracts: 180,
                premium: 98_400.0,
                classification: "Hedging".into(),
                confidence: 0.62,
                trade_quality_version: "tq.v1".into(),
                evidence: "opening bid · size ≤ OI".into(),
                occurred_at: "2026-07-25T19:55:40Z".into(),
            },
        ],
    })
}

fn dealer_exposure(underlying: &str) -> Result<DealerExposure, String> {
    let underlying = underlying.trim().to_uppercase();
    if underlying != "AAPL" {
        return Err(format!(
            "offline dealer-exposure fixture unavailable for {underlying}"
        ));
    }
    Ok(DealerExposure {
        underlying,
        net_gex: -1.84e8,
        net_dex: 4.2e6,
        by_strike: vec![
            DealerStrike {
                strike: 210.0,
                gex: 4.1e7,
                dex: -1.2e6,
            },
            DealerStrike {
                strike: 215.0,
                gex: -9.5e7,
                dex: 2.4e6,
            },
            DealerStrike {
                strike: 220.0,
                gex: -8.8e7,
                dex: 2.1e6,
            },
            DealerStrike {
                strike: 225.0,
                gex: 2.2e7,
                dex: 0.9e6,
            },
        ],
        provider: "flow-aggregate-embedded".into(),
        retrieved_at: AS_OF.into(),
    })
}

fn catalyst_calendar() -> Vec<CatalystEvent> {
    vec![
        CatalystEvent {
            id: "cat-001".into(),
            kind: "earnings".into(),
            title: "AAPL fiscal Q3 earnings".into(),
            occurs_at: "2026-07-31T20:00:00Z".into(),
            related_symbol: "AAPL".into(),
            provider: "catalyst-embedded".into(),
        },
        CatalystEvent {
            id: "cat-002".into(),
            kind: "macro".into(),
            title: "FOMC rate decision".into(),
            occurs_at: "2026-07-29T18:00:00Z".into(),
            related_symbol: "SPX".into(),
            provider: "catalyst-embedded".into(),
        },
        CatalystEvent {
            id: "cat-003".into(),
            kind: "macro".into(),
            title: "CPI release".into(),
            occurs_at: "2026-08-12T12:30:00Z".into(),
            related_symbol: "USD".into(),
            provider: "catalyst-embedded".into(),
        },
    ]
}

fn next_session(venue: &str) -> Result<NextSession, String> {
    let venue = venue.trim().to_uppercase();
    if venue != "XNYS" && venue != "XNAS" {
        return Err(format!("offline calendar fixture unavailable for {venue}"));
    }
    Ok(NextSession {
        venue,
        opens_at: "2026-07-27T13:30:00Z".into(),
        closes_at: "2026-07-27T20:00:00Z".into(),
        state: "scheduled".into(),
        provider: "calendar-embedded".into(),
        retrieved_at: AS_OF.into(),
    })
}

#[tauri::command]
pub fn get_equity_bars(symbol: String) -> Result<EquityBars, String> {
    equity_bars(&symbol)
}

#[tauri::command]
pub fn get_filing_summaries(cik: Option<String>) -> Vec<FilingSummary> {
    filing_summaries(cik.as_deref())
}

#[tauri::command]
pub fn get_insider_transactions(cik: Option<String>) -> Vec<InsiderTransaction> {
    insider_transactions(cik.as_deref())
}

#[tauri::command]
pub fn get_macro_series(series_id: String) -> Result<MacroSeries, String> {
    macro_series(&series_id)
}

#[tauri::command]
pub fn get_cot_report(market_code: String) -> Result<CotReport, String> {
    cot_report(&market_code)
}

#[tauri::command]
pub fn get_options_chain(underlying: String) -> Result<OptionsChain, String> {
    options_chain(&underlying)
}

#[tauri::command]
pub fn get_options_flow(underlying: String) -> Result<OptionsFlow, String> {
    options_flow(&underlying)
}

#[tauri::command]
pub fn get_dealer_exposure(underlying: String) -> Result<DealerExposure, String> {
    dealer_exposure(&underlying)
}

#[tauri::command]
pub fn get_catalyst_calendar() -> Vec<CatalystEvent> {
    catalyst_calendar()
}

#[tauri::command]
pub fn get_next_session(venue: String) -> Result<NextSession, String> {
    next_session(&venue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equity_fixtures_cover_every_experience() {
        assert_eq!(equity_bars("aapl").unwrap().symbol, "AAPL");
        assert!(!filing_summaries(None).is_empty());
        assert!(!insider_transactions(Some("0000320193")).is_empty());
        assert_eq!(macro_series("DGS10").unwrap().series_id, "DGS10");
        assert_eq!(cot_report("067651").unwrap().market_code, "067651");
        assert_eq!(options_chain("aapl").unwrap().underlying, "AAPL");
        assert!(!options_flow("AAPL").unwrap().prints.is_empty());
        assert_eq!(dealer_exposure("AAPL").unwrap().underlying, "AAPL");
        assert!(!catalyst_calendar().is_empty());
        assert_eq!(next_session("XNYS").unwrap().venue, "XNYS");
    }

    #[test]
    fn optional_cik_filters_embedded_filings() {
        assert!(filing_summaries(Some("missing")).is_empty());
        assert!(insider_transactions(Some("missing")).is_empty());
    }
}
