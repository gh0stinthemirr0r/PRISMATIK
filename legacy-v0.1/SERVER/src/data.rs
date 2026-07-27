//! Market data: Coinbase Exchange public REST, no key required.
//! Author: Aaron Stovall · Version 0.1.0 · 2026-07-07
//!
//! Real network integration with timeouts, bounded retries, and loud failures.
//! Candles arrive as [time, low, high, open, close, volume], max 300 rows per
//! request, so we page backward through time. We never fabricate a price: if
//! the venue is unreachable, this returns an error.

use crate::types::Candle;
use chrono::{DateTime, Duration, Utc};
use std::time::Duration as StdDuration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataError {
    #[error("http error from venue: {0}")]
    Http(String),
    #[error("venue returned an unexpected payload: {0}")]
    BadPayload(String),
    #[error("no candles returned for {0}; check the symbol and granularity")]
    Empty(String),
}

const MAX_ROWS_PER_REQ: i64 = 300;
const MAX_RETRIES: u32 = 4;

pub struct CoinbaseData {
    client: reqwest::Client,
    base: String,
}

impl CoinbaseData {
    pub fn new(base: &str, timeout_secs: u64) -> Result<Self, DataError> {
        let client = reqwest::Client::builder()
            .timeout(StdDuration::from_secs(timeout_secs))
            .user_agent(format!("prismatik/{}", crate::config::VERSION))
            .build()
            .map_err(|e| DataError::Http(e.to_string()))?;
        Ok(Self {
            client,
            base: base.trim_end_matches('/').to_string(),
        })
    }

    async fn get_json(
        &self,
        url: &str,
        params: &[(&str, String)],
    ) -> Result<serde_json::Value, DataError> {
        let mut last: Option<String> = None;
        for attempt in 1..=MAX_RETRIES {
            let resp = self.client.get(url).query(params).send().await;
            match resp {
                Ok(r) if r.status().as_u16() == 429 => {
                    tracing::warn!(attempt, "rate_limited");
                    tokio::time::sleep(StdDuration::from_millis(700 * attempt as u64)).await;
                },
                Ok(r) if r.status().is_success() => {
                    return r
                        .json()
                        .await
                        .map_err(|e| DataError::BadPayload(e.to_string()));
                },
                Ok(r) => return Err(DataError::Http(format!("{} from {}", r.status(), url))),
                Err(e) if e.is_timeout() || e.is_connect() => {
                    last = Some(e.to_string());
                    tracing::warn!(attempt, error = %e, "transient_error");
                    tokio::time::sleep(StdDuration::from_millis(700 * attempt as u64)).await;
                },
                Err(e) => return Err(DataError::Http(e.to_string())),
            }
        }
        Err(DataError::Http(format!(
            "exhausted retries: {}",
            last.unwrap_or_default()
        )))
    }

    pub async fn fetch_candles(
        &self,
        symbol: &str,
        granularity_s: u32,
        days: u32,
    ) -> Result<Vec<Candle>, DataError> {
        let end: DateTime<Utc> = Utc::now();
        let start = end - Duration::days(days as i64);
        let window = Duration::seconds(granularity_s as i64 * MAX_ROWS_PER_REQ);
        let url = format!("{}/products/{}/candles", self.base, symbol);

        let mut rows: Vec<Candle> = Vec::new();
        let mut cursor_end = end;
        let mut pages = 0u32;
        while cursor_end > start {
            let cursor_start = std::cmp::max(start, cursor_end - window);
            let payload = self
                .get_json(
                    &url,
                    &[
                        ("granularity", granularity_s.to_string()),
                        ("start", cursor_start.to_rfc3339()),
                        ("end", cursor_end.to_rfc3339()),
                    ],
                )
                .await?;
            let batch = payload
                .as_array()
                .ok_or_else(|| DataError::BadPayload("expected array of candles".into()))?;
            for item in batch {
                rows.push(parse_candle(item)?);
            }
            pages += 1;
            tracing::info!(pages, batch = batch.len(), "page_fetched");
            cursor_end = cursor_start;
            tokio::time::sleep(StdDuration::from_millis(340)).await;
        }

        rows.sort_by_key(|c| c.time);
        rows.dedup_by_key(|c| c.time);
        if rows.is_empty() {
            return Err(DataError::Empty(symbol.to_string()));
        }
        tracing::info!(symbol, bars = rows.len(), "candles_ready");
        Ok(rows)
    }
}

pub fn parse_candle(item: &serde_json::Value) -> Result<Candle, DataError> {
    let arr = item
        .as_array()
        .ok_or_else(|| DataError::BadPayload("candle row is not an array".into()))?;
    if arr.len() < 6 {
        return Err(DataError::BadPayload(format!(
            "candle row has {} fields",
            arr.len()
        )));
    }
    let num = |i: usize| -> Result<f64, DataError> {
        arr[i]
            .as_f64()
            .ok_or_else(|| DataError::BadPayload(format!("field {i} not numeric")))
    };
    let ts = num(0)? as i64;
    let time = DateTime::<Utc>::from_timestamp(ts, 0)
        .ok_or_else(|| DataError::BadPayload(format!("bad timestamp {ts}")))?;
    // Coinbase order: [time, low, high, open, close, volume]
    Ok(Candle {
        time,
        low: num(1)?,
        high: num(2)?,
        open: num(3)?,
        close: num(4)?,
        volume: num(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_coinbase_row_order() {
        let row = serde_json::json!([1720000000, 99.0, 101.0, 100.0, 100.5, 12.0]);
        let c = parse_candle(&row).unwrap();
        assert_eq!(c.low, 99.0);
        assert_eq!(c.high, 101.0);
        assert_eq!(c.open, 100.0);
        assert_eq!(c.close, 100.5);
    }

    #[test]
    fn rejects_malformed_rows() {
        assert!(parse_candle(&serde_json::json!([1, 2, 3])).is_err());
        assert!(parse_candle(&serde_json::json!({"time": 1})).is_err());
        assert!(parse_candle(&serde_json::json!([1720000000, "x", 1, 1, 1, 1])).is_err());
    }
}
