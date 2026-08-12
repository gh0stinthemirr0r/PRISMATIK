use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OhlcvBar {
    pub(crate) timestamp: String,
    pub(crate) open: f64,
    pub(crate) high: f64,
    pub(crate) low: f64,
    pub(crate) close: f64,
    pub(crate) volume: f64,
    pub(crate) adjusted_close: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoricalDataResult {
    pub(crate) symbol: String,
    pub(crate) interval: String,
    pub(crate) bars: Vec<OhlcvBar>,
    pub(crate) source: String,
    pub(crate) count: usize,
    pub(crate) first_date: String,
    pub(crate) last_date: String,
}

/// Fetch historical OHLCV from Yahoo Finance (free, no API key).
/// Supports daily, weekly, monthly intervals going back decades.
#[tauri::command]
pub(crate) async fn get_historical_ohlcv(
    symbol: String,
    interval: Option<String>,
    period: Option<String>,
) -> Result<HistoricalDataResult, String> {
    let interval = interval.unwrap_or_else(|| "1d".into());
    let period = period.unwrap_or_else(|| "5y".into());

    let range = match period.as_str() {
        "1d" => "1d",
        "5d" => "5d",
        "1mo" => "1mo",
        "3mo" => "3mo",
        "6mo" => "6mo",
        "1y" => "1y",
        "2y" => "2y",
        "5y" => "5y",
        "10y" => "10y",
        "max" => "max",
        _ => "5y",
    };

    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?range={range}&interval={interval}"
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Yahoo Finance returned HTTP {}", resp.status()));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| format!("parse error: {e}"))?;

    let timestamps = val["chart"]["result"][0]["timestamp"]
        .as_array()
        .ok_or("no timestamp data")?;

    let quotes = &val["chart"]["result"][0]["indicators"]["quote"][0];
    let opens = quotes["open"].as_array().ok_or("no open data")?;
    let highs = quotes["high"].as_array().ok_or("no high data")?;
    let lows = quotes["low"].as_array().ok_or("no low data")?;
    let closes = quotes["close"].as_array().ok_or("no close data")?;
    let volumes = quotes["volume"].as_array().ok_or("no volume data")?;
    let adj = val["chart"]["result"][0]["indicators"]["adjclose"][0]["adjclose"].as_array();

    let mut bars = Vec::new();
    for i in 0..timestamps.len() {
        let ts = timestamps[i].as_i64().unwrap_or(0);
        let dt = chrono::DateTime::from_timestamp(ts, 0)
            .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_default();

        bars.push(OhlcvBar {
            timestamp: dt,
            open: opens[i].as_f64().unwrap_or(0.0),
            high: highs[i].as_f64().unwrap_or(0.0),
            low: lows[i].as_f64().unwrap_or(0.0),
            close: closes[i].as_f64().unwrap_or(0.0),
            volume: volumes[i].as_f64().unwrap_or(0.0),
            adjusted_close: adj.and_then(|a| a.get(i)?.as_f64()),
        });
    }

    let first = bars
        .first()
        .map(|b| b.timestamp.clone())
        .unwrap_or_default();
    let last = bars.last().map(|b| b.timestamp.clone()).unwrap_or_default();
    let count = bars.len();

    Ok(HistoricalDataResult {
        symbol,
        interval,
        bars,
        source: "Yahoo Finance".into(),
        count,
        first_date: first,
        last_date: last,
    })
}

/// Fetch historical OHLCV for crypto from CoinGecko (free tier).
#[tauri::command]
pub(crate) async fn get_crypto_historical(
    coin_id: String,
    vs_currency: Option<String>,
    days: Option<u32>,
) -> Result<HistoricalDataResult, String> {
    let vs = vs_currency.unwrap_or_else(|| "usd".into());
    let days = days.unwrap_or(365);

    let url = format!(
        "https://api.coingecko.com/api/v3/coins/{coin_id}/market_chart?vs_currency={vs}&days={days}&interval=daily"
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("CoinGecko returned HTTP {}", resp.status()));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| format!("parse error: {e}"))?;

    let prices = val["prices"].as_array().ok_or("no prices")?;
    let volumes = val["total_volumes"].as_array().ok_or("no volumes")?;

    let mut bars = Vec::new();
    for i in 0..prices.len() {
        let ts = prices[i][0].as_i64().unwrap_or(0);
        let price = prices[i][1].as_f64().unwrap_or(0.0);
        let vol = volumes.get(i).and_then(|v| v[1].as_f64()).unwrap_or(0.0);
        let dt = chrono::DateTime::from_timestamp_millis(ts)
            .map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_default();

        bars.push(OhlcvBar {
            timestamp: dt,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: vol,
            adjusted_close: None,
        });
    }

    let count = bars.len();
    let first = bars
        .first()
        .map(|b| b.timestamp.clone())
        .unwrap_or_default();
    let last = bars.last().map(|b| b.timestamp.clone()).unwrap_or_default();

    Ok(HistoricalDataResult {
        symbol: coin_id,
        interval: "1d".into(),
        bars,
        source: "CoinGecko".into(),
        count,
        first_date: first,
        last_date: last,
    })
}
