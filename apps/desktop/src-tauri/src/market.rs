//! Cassette-backed crypto market IPC (Wave 1 P1-DP-04/05 → shell).
//!
//! Live HTTP stays out of Layer 2; the shell constructs a `CassetteTransport`
//! (offline demo) until a reqwest transport is added at the application layer.

use std::sync::Arc;

use prismatik_market_data::{
    demo_cassette_json, CassetteTransport, CoinDetail, CoinGeckoAdapter, CoinGeckoAuth,
    CryptoCategory, CryptoExchange, CryptoGlobalStats, CryptoMarketQuote, OhlcBar, TrendingCoin,
};
use serde::Serialize;
use time::OffsetDateTime;

/// UI-facing market quote (ISO timestamps, display-ready provider label).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiCryptoQuote {
    pub coingecko_id: String,
    pub symbol: String,
    pub name: String,
    pub price: String,
    pub change_24h_pct: Option<String>,
    pub volume_24h: Option<String>,
    pub market_cap: Option<String>,
    pub provider: String,
    pub event_time: String,
    pub retrieved_at: String,
    pub quality: f64,
}

/// UI-facing global stats.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiCryptoGlobal {
    pub btc_dominance: Option<String>,
    pub eth_dominance: Option<String>,
    pub total_market_cap_usd: Option<String>,
    pub total_volume_usd: Option<String>,
    pub provider: String,
    pub retrieved_at: String,
}

/// UI-facing trending coin.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiTrendingCoin {
    pub coingecko_id: String,
    pub symbol: String,
    pub name: String,
    pub market_cap_rank: Option<u32>,
}

/// UI-facing OHLC candle (unix seconds for Lightweight Charts).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiOhlcBar {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

/// Chart payload for a focused asset.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiCryptoChart {
    pub coingecko_id: String,
    pub provider: String,
    pub retrieved_at: String,
    pub interval: String,
    pub candles: Vec<UiOhlcBar>,
}

/// Coin profile for the focused asset.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiCoinDetail {
    pub coingecko_id: String,
    pub symbol: String,
    pub name: String,
    pub market_cap_rank: Option<u32>,
    pub homepage: Option<String>,
    pub description: Option<String>,
    pub categories: Vec<String>,
    pub provider: String,
    pub retrieved_at: String,
}

/// Category map row.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiCryptoCategory {
    pub id: String,
    pub name: String,
    pub market_cap: Option<String>,
    pub change_24h_pct: Option<String>,
    pub top_3_coins: Vec<String>,
}

/// Exchange breadth row.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiCryptoExchange {
    pub id: String,
    pub name: String,
    pub trust_score: Option<u32>,
    pub trade_volume_24h_btc: Option<String>,
    pub country: Option<String>,
}

fn adapter() -> Result<CoinGeckoAdapter, String> {
    let transport = CassetteTransport::from_json(demo_cassette_json())
        .map_err(|e| format!("cassette load failed: {e}"))?;
    // Pro auth unlocks OHLC/market_chart while retaining Demo endpoints.
    Ok(CoinGeckoAdapter::new(
        Arc::new(transport),
        CoinGeckoAuth::Pro {
            api_key: "pro-demo".into(),
        },
    ))
}

fn fmt_rfc3339(t: OffsetDateTime) -> String {
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| t.to_string())
}

fn map_quote(q: CryptoMarketQuote) -> UiCryptoQuote {
    UiCryptoQuote {
        coingecko_id: q.coingecko_id,
        symbol: q.symbol,
        name: q.name,
        price: q.price,
        change_24h_pct: q.change_24h_pct,
        volume_24h: q.volume_24h,
        market_cap: q.market_cap,
        provider: q.provider.to_string(),
        event_time: fmt_rfc3339(q.event_time),
        retrieved_at: fmt_rfc3339(q.retrieved_at),
        quality: q.quality.0,
    }
}

fn map_global(g: CryptoGlobalStats) -> UiCryptoGlobal {
    UiCryptoGlobal {
        btc_dominance: g.btc_dominance,
        eth_dominance: g.eth_dominance,
        total_market_cap_usd: g.total_market_cap_usd,
        total_volume_usd: g.total_volume_usd,
        provider: g.provider.to_string(),
        retrieved_at: fmt_rfc3339(g.retrieved_at),
    }
}

fn map_trending(t: TrendingCoin) -> UiTrendingCoin {
    UiTrendingCoin {
        coingecko_id: t.coingecko_id,
        symbol: t.symbol,
        name: t.name,
        market_cap_rank: t.market_cap_rank,
    }
}

fn map_ohlc(bar: OhlcBar) -> Result<UiOhlcBar, String> {
    Ok(UiOhlcBar {
        time: bar.bar_start.unix_timestamp(),
        open: bar.open.parse().map_err(|e| format!("open: {e}"))?,
        high: bar.high.parse().map_err(|e| format!("high: {e}"))?,
        low: bar.low.parse().map_err(|e| format!("low: {e}"))?,
        close: bar.close.parse().map_err(|e| format!("close: {e}"))?,
    })
}

/// Top crypto markets from the demo cassette (offline, no network).
#[tauri::command]
pub async fn get_crypto_markets() -> Result<Vec<UiCryptoQuote>, String> {
    let a = adapter()?;
    let now = OffsetDateTime::now_utc();
    let rows = a
        .markets("usd", &[], now)
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(map_quote).collect())
}

/// Global crypto stats from the demo cassette.
#[tauri::command]
pub async fn get_crypto_global() -> Result<UiCryptoGlobal, String> {
    let a = adapter()?;
    let now = OffsetDateTime::now_utc();
    let g = a.global(now).await.map_err(|e| e.to_string())?;
    Ok(map_global(g))
}

/// Trending coins from the demo cassette.
#[tauri::command]
pub async fn get_crypto_trending() -> Result<Vec<UiTrendingCoin>, String> {
    let a = adapter()?;
    let rows = a.trending().await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(map_trending).collect())
}

/// OHLC candles for a CoinGecko id (Pro cassette).
#[tauri::command]
pub async fn get_crypto_ohlc(
    coingecko_id: String,
    days: Option<u32>,
) -> Result<UiCryptoChart, String> {
    let days = days.unwrap_or(30);
    let a = adapter()?;
    let now = OffsetDateTime::now_utc();
    let bars = a
        .ohlc(&coingecko_id, "usd", days, now)
        .await
        .map_err(|e| e.to_string())?;
    let candles = bars
        .into_iter()
        .map(map_ohlc)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(UiCryptoChart {
        coingecko_id,
        provider: "coingecko".into(),
        retrieved_at: fmt_rfc3339(now),
        interval: format!("{days}d"),
        candles,
    })
}

/// Coin detail profile (Pro cassette).
#[tauri::command]
pub async fn get_coin_detail(coingecko_id: String) -> Result<UiCoinDetail, String> {
    let a = adapter()?;
    let now = OffsetDateTime::now_utc();
    let d = a
        .coin_detail(&coingecko_id, now)
        .await
        .map_err(|e| e.to_string())?;
    Ok(map_detail(d))
}

/// Category map (Pro cassette).
#[tauri::command]
pub async fn get_crypto_categories() -> Result<Vec<UiCryptoCategory>, String> {
    let a = adapter()?;
    let rows = a.categories().await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(map_category).collect())
}

/// Exchange list (Pro cassette).
#[tauri::command]
pub async fn get_crypto_exchanges() -> Result<Vec<UiCryptoExchange>, String> {
    let a = adapter()?;
    let rows = a.exchanges(20).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(map_exchange).collect())
}

fn map_detail(d: CoinDetail) -> UiCoinDetail {
    UiCoinDetail {
        coingecko_id: d.coingecko_id,
        symbol: d.symbol,
        name: d.name,
        market_cap_rank: d.market_cap_rank,
        homepage: d.homepage,
        description: d.description,
        categories: d.categories,
        provider: d.provider.to_string(),
        retrieved_at: fmt_rfc3339(d.retrieved_at),
    }
}

fn map_category(c: CryptoCategory) -> UiCryptoCategory {
    UiCryptoCategory {
        id: c.id,
        name: c.name,
        market_cap: c.market_cap,
        change_24h_pct: c.change_24h_pct,
        top_3_coins: c.top_3_coins,
    }
}

fn map_exchange(x: CryptoExchange) -> UiCryptoExchange {
    UiCryptoExchange {
        id: x.id,
        name: x.name,
        trust_score: x.trust_score,
        trade_volume_24h_btc: x.trade_volume_24h_btc,
        country: x.country,
    }
}

/// Asset search against the CoinGecko cassette (`/search`).
#[tauri::command]
pub async fn search_crypto(query: String) -> Result<Vec<UiTrendingCoin>, String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let a = adapter()?;
    match a.search(&q).await {
        Ok(rows) => Ok(rows.into_iter().map(map_trending).collect()),
        Err(e) => {
            // Offline cassette miss → empty results, not a hard failure.
            let msg = e.to_string();
            if msg.contains("offline") || msg.contains("empty") || msg.contains("Offline") {
                Ok(vec![])
            } else {
                Err(msg)
            }
        },
    }
}
