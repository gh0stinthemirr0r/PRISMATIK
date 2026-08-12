//! CoinGecko provider adapter.
//!
//! Spec: `DOCS/spec/PROVIDER_ADAPTERS.md` §2, Wave 1 `P1-DP-04`.

use crate::http::{HttpMethod, HttpRequest, HttpTransport, TransportError};
use crate::provider::{
    Capability, Entitlement, EntitlementSet, HealthStatus, Provider, ProviderCapabilities,
    ProviderHealth,
};
use crate::request::{CostUnits, ProviderRequest};
use crate::types::{
    CoinDetail, CryptoCategory, CryptoExchange, CryptoGlobalStats, CryptoMarketQuote,
    MarketChartPoint, MarketChartSeries, OhlcBar, TrendingCoin,
};
use async_trait::async_trait;
use prismatik_domain::{DataQualityScore, ProviderId};
use prismatik_identity::ExternalIdentifier;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;
use time::OffsetDateTime;

/// CoinGecko API errors mapped to local taxonomy.
#[derive(Debug, Error)]
pub enum CoinGeckoError {
    /// Missing entitlement — no request made.
    #[error("not entitled: {0:?}")]
    NotEntitled(Entitlement),
    /// Transport failure.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// Auth failure (401).
    #[error("coingecko auth failure")]
    Auth,
    /// Rate limited (429).
    #[error("coingecko rate limited; retry after {retry_after_secs:?}s")]
    RateLimited {
        /// Retry-After seconds if present.
        retry_after_secs: Option<u64>,
    },
    /// Empty 200 — failover trigger.
    #[error("coingecko empty response")]
    Empty,
    /// Transient upstream error.
    #[error("coingecko upstream status {status}")]
    Upstream {
        /// HTTP status.
        status: u16,
    },
    /// Body parse failure.
    #[error("coingecko decode: {0}")]
    Decode(String),
}

/// Auth mode for CoinGecko.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoinGeckoAuth {
    /// Demo key as query param.
    Demo {
        /// API key.
        api_key: String,
    },
    /// Pro key as header.
    Pro {
        /// API key.
        api_key: String,
    },
}

impl CoinGeckoAuth {
    fn entitlements(&self) -> EntitlementSet {
        match self {
            Self::Demo { .. } => [Entitlement::CoinGeckoDemo].into_iter().collect(),
            Self::Pro { .. } => [Entitlement::CoinGeckoDemo, Entitlement::CoinGeckoPro]
                .into_iter()
                .collect(),
        }
    }
}

/// CoinGecko adapter.
pub struct CoinGeckoAdapter {
    transport: Arc<dyn HttpTransport>,
    auth: CoinGeckoAuth,
    entitlements: EntitlementSet,
    base_host_hint: String,
}

impl std::fmt::Debug for CoinGeckoAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoinGeckoAdapter")
            .field("auth", &self.auth)
            .field("entitlements", &self.entitlements)
            .field("base_host_hint", &self.base_host_hint)
            .finish_non_exhaustive()
    }
}

impl CoinGeckoAdapter {
    /// Construct against any transport (cassette or live).
    pub fn new(transport: Arc<dyn HttpTransport>, auth: CoinGeckoAuth) -> Self {
        let entitlements = auth.entitlements();
        Self {
            transport,
            auth,
            entitlements,
            base_host_hint: "api.coingecko.com".into(),
        }
    }

    fn require(&self, need: Entitlement) -> Result<(), CoinGeckoError> {
        if self.entitlements.contains(&need) {
            Ok(())
        } else {
            Err(CoinGeckoError::NotEntitled(need))
        }
    }

    fn apply_auth(
        &self,
        headers: &mut BTreeMap<String, String>,
        query: &mut BTreeMap<String, String>,
    ) {
        match &self.auth {
            CoinGeckoAuth::Demo { api_key } => {
                query.insert("x_cg_demo_api_key".into(), api_key.clone());
            },
            CoinGeckoAuth::Pro { api_key } => {
                headers.insert("x-cg-pro-api-key".into(), api_key.clone());
            },
        }
    }

    async fn get(
        &self,
        path: &str,
        mut query: BTreeMap<String, String>,
    ) -> Result<(u16, BTreeMap<String, String>, String), CoinGeckoError> {
        let mut headers = BTreeMap::new();
        self.apply_auth(&mut headers, &mut query);
        let res = self
            .transport
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                path: path.to_string(),
                query,
                headers,
                body: None,
            })
            .await?;
        Ok((res.status, res.headers, res.body))
    }

    fn map_status(
        status: u16,
        headers: &BTreeMap<String, String>,
        body: &str,
    ) -> Result<(), CoinGeckoError> {
        match status {
            200 => {
                if body.trim().is_empty()
                    || body.trim() == "null"
                    || body.trim() == "{}"
                    || body.trim() == "[]"
                {
                    return Err(CoinGeckoError::Empty);
                }
                Ok(())
            },
            401 => Err(CoinGeckoError::Auth),
            429 => {
                let retry = headers
                    .get("retry-after")
                    .or_else(|| headers.get("Retry-After"))
                    .and_then(|v| v.parse().ok());
                Err(CoinGeckoError::RateLimited {
                    retry_after_secs: retry,
                })
            },
            403 | 500 | 502 | 503 | 504 => Err(CoinGeckoError::Upstream { status }),
            other => Err(CoinGeckoError::Upstream { status: other }),
        }
    }

    /// `GET /coins/markets`
    pub async fn markets(
        &self,
        vs_currency: &str,
        ids: &[&str],
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<CryptoMarketQuote>, CoinGeckoError> {
        self.require(Entitlement::CoinGeckoDemo)?;
        let mut query = BTreeMap::new();
        query.insert("vs_currency".into(), vs_currency.into());
        query.insert("order".into(), "market_cap_desc".into());
        query.insert("per_page".into(), "50".into());
        query.insert("page".into(), "1".into());
        query.insert("sparkline".into(), "false".into());
        query.insert("price_change_percentage".into(), "24h".into());
        if !ids.is_empty() {
            query.insert("ids".into(), ids.join(","));
        }
        let (status, headers, body) = self.get("/coins/markets", query).await?;
        Self::map_status(status, &headers, &body)?;
        let rows: Vec<CgMarketRow> =
            serde_json::from_str(&body).map_err(|e| CoinGeckoError::Decode(e.to_string()))?;
        if rows.is_empty() {
            return Err(CoinGeckoError::Empty);
        }
        Ok(rows
            .into_iter()
            .map(|r| r.into_normalized(retrieved_at))
            .collect())
    }

    /// `GET /global`
    pub async fn global(
        &self,
        retrieved_at: OffsetDateTime,
    ) -> Result<CryptoGlobalStats, CoinGeckoError> {
        self.require(Entitlement::CoinGeckoDemo)?;
        let (status, headers, body) = self.get("/global", BTreeMap::new()).await?;
        Self::map_status(status, &headers, &body)?;
        let parsed: CgGlobalEnvelope =
            serde_json::from_str(&body).map_err(|e| CoinGeckoError::Decode(e.to_string()))?;
        let data = parsed.data.ok_or(CoinGeckoError::Empty)?;
        Ok(CryptoGlobalStats {
            btc_dominance: data.market_cap_percentage.get("btc").copied().map(fmt_f64),
            eth_dominance: data.market_cap_percentage.get("eth").copied().map(fmt_f64),
            total_market_cap_usd: data.total_market_cap.get("usd").copied().map(fmt_f64),
            total_volume_usd: data.total_volume.get("usd").copied().map(fmt_f64),
            provider: ProviderId::COINGECKO,
            retrieved_at,
        })
    }

    /// `GET /search/trending`
    pub async fn trending(&self) -> Result<Vec<TrendingCoin>, CoinGeckoError> {
        self.require(Entitlement::CoinGeckoDemo)?;
        let (status, headers, body) = self.get("/search/trending", BTreeMap::new()).await?;
        Self::map_status(status, &headers, &body)?;
        let parsed: CgTrendingEnvelope =
            serde_json::from_str(&body).map_err(|e| CoinGeckoError::Decode(e.to_string()))?;
        if parsed.coins.is_empty() {
            return Err(CoinGeckoError::Empty);
        }
        Ok(parsed
            .coins
            .into_iter()
            .filter_map(|c| c.item)
            .map(|item| TrendingCoin {
                coingecko_id: item.id,
                symbol: item.symbol.to_uppercase(),
                name: item.name,
                market_cap_rank: item.market_cap_rank,
            })
            .collect())
    }

    /// `GET /search?query=`
    pub async fn search(&self, query_text: &str) -> Result<Vec<TrendingCoin>, CoinGeckoError> {
        self.require(Entitlement::CoinGeckoDemo)?;
        let mut query = BTreeMap::new();
        query.insert("query".into(), query_text.into());
        let (status, headers, body) = self.get("/search", query).await?;
        Self::map_status(status, &headers, &body)?;
        let parsed: Value =
            serde_json::from_str(&body).map_err(|e| CoinGeckoError::Decode(e.to_string()))?;
        let coins = parsed
            .get("coins")
            .and_then(|c| c.as_array())
            .ok_or(CoinGeckoError::Empty)?;
        if coins.is_empty() {
            return Err(CoinGeckoError::Empty);
        }
        Ok(coins
            .iter()
            .filter_map(|c| {
                Some(TrendingCoin {
                    coingecko_id: c.get("id")?.as_str()?.to_string(),
                    symbol: c.get("symbol")?.as_str()?.to_uppercase(),
                    name: c.get("name")?.as_str()?.to_string(),
                    market_cap_rank: c
                        .get("market_cap_rank")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u32),
                })
            })
            .collect())
    }

    /// `GET /coins/{id}/ohlc` (Pro).
    pub async fn ohlc(
        &self,
        id: &str,
        vs_currency: &str,
        days: u32,
        retrieved_at: OffsetDateTime,
    ) -> Result<Vec<OhlcBar>, CoinGeckoError> {
        self.require(Entitlement::CoinGeckoPro)?;
        let mut query = BTreeMap::new();
        query.insert("vs_currency".into(), vs_currency.into());
        query.insert("days".into(), days.to_string());
        let path = format!("/coins/{id}/ohlc");
        let (status, headers, body) = self.get(&path, query).await?;
        Self::map_status(status, &headers, &body)?;
        let rows: Vec<Vec<f64>> =
            serde_json::from_str(&body).map_err(|e| CoinGeckoError::Decode(e.to_string()))?;
        if rows.is_empty() {
            return Err(CoinGeckoError::Empty);
        }
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            if row.len() < 5 {
                return Err(CoinGeckoError::Decode(
                    "ohlc row expected [ts, o, h, l, c]".into(),
                ));
            }
            let ms = row[0] as i64;
            let bar_start = OffsetDateTime::from_unix_timestamp_nanos(ms as i128 * 1_000_000)
                .map_err(|e| CoinGeckoError::Decode(e.to_string()))?;
            out.push(OhlcBar {
                coingecko_id: id.to_string(),
                bar_start,
                open: fmt_f64(row[1]),
                high: fmt_f64(row[2]),
                low: fmt_f64(row[3]),
                close: fmt_f64(row[4]),
                provider: ProviderId::COINGECKO,
                retrieved_at,
                quality: DataQualityScore::PERFECT,
            });
        }
        Ok(out)
    }

    /// `GET /coins/{id}/market_chart` (Pro).
    pub async fn market_chart(
        &self,
        id: &str,
        vs_currency: &str,
        days: u32,
        retrieved_at: OffsetDateTime,
    ) -> Result<MarketChartSeries, CoinGeckoError> {
        self.require(Entitlement::CoinGeckoPro)?;
        let mut query = BTreeMap::new();
        query.insert("vs_currency".into(), vs_currency.into());
        query.insert("days".into(), days.to_string());
        let path = format!("/coins/{id}/market_chart");
        let (status, headers, body) = self.get(&path, query).await?;
        Self::map_status(status, &headers, &body)?;
        let parsed: CgMarketChart =
            serde_json::from_str(&body).map_err(|e| CoinGeckoError::Decode(e.to_string()))?;
        if parsed.prices.is_empty() {
            return Err(CoinGeckoError::Empty);
        }
        Ok(MarketChartSeries {
            coingecko_id: id.to_string(),
            prices: map_chart_points(&parsed.prices)?,
            market_caps: map_chart_points(&parsed.market_caps)?,
            total_volumes: map_chart_points(&parsed.total_volumes)?,
            provider: ProviderId::COINGECKO,
            retrieved_at,
        })
    }

    /// `GET /coins/{id}` (Pro) — profile fields only.
    pub async fn coin_detail(
        &self,
        id: &str,
        retrieved_at: OffsetDateTime,
    ) -> Result<CoinDetail, CoinGeckoError> {
        self.require(Entitlement::CoinGeckoPro)?;
        let mut query = BTreeMap::new();
        query.insert("localization".into(), "false".into());
        query.insert("tickers".into(), "false".into());
        query.insert("market_data".into(), "false".into());
        query.insert("community_data".into(), "false".into());
        query.insert("developer_data".into(), "false".into());
        let path = format!("/coins/{id}");
        let (status, headers, body) = self.get(&path, query).await?;
        Self::map_status(status, &headers, &body)?;
        let parsed: CgCoinDetail =
            serde_json::from_str(&body).map_err(|e| CoinGeckoError::Decode(e.to_string()))?;
        let homepage = parsed
            .links
            .as_ref()
            .and_then(|l| l.homepage.first())
            .cloned()
            .filter(|s| !s.is_empty());
        let description = parsed
            .description
            .as_ref()
            .and_then(|d| d.en.clone())
            .map(|s| {
                let s = s.trim();
                if s.len() > 480 {
                    format!("{}…", &s[..480])
                } else {
                    s.to_string()
                }
            })
            .filter(|s| !s.is_empty());
        Ok(CoinDetail {
            coingecko_id: parsed.id.unwrap_or_else(|| id.to_string()),
            symbol: parsed.symbol.unwrap_or_default().to_uppercase(),
            name: parsed.name.unwrap_or_default(),
            market_cap_rank: parsed.market_cap_rank,
            homepage,
            description,
            categories: parsed.categories.unwrap_or_default(),
            provider: ProviderId::COINGECKO,
            retrieved_at,
        })
    }

    /// `GET /coins/categories` (Pro).
    pub async fn categories(&self) -> Result<Vec<CryptoCategory>, CoinGeckoError> {
        self.require(Entitlement::CoinGeckoPro)?;
        let (status, headers, body) = self.get("/coins/categories", BTreeMap::new()).await?;
        Self::map_status(status, &headers, &body)?;
        let rows: Vec<CgCategory> =
            serde_json::from_str(&body).map_err(|e| CoinGeckoError::Decode(e.to_string()))?;
        if rows.is_empty() {
            return Err(CoinGeckoError::Empty);
        }
        Ok(rows
            .into_iter()
            .map(|r| CryptoCategory {
                id: r.id.unwrap_or_default(),
                name: r.name.unwrap_or_default(),
                market_cap: r.market_cap.map(fmt_f64),
                change_24h_pct: r.market_cap_change_24h.map(fmt_f64),
                top_3_coins: r.top_3_coins.unwrap_or_default(),
            })
            .collect())
    }

    /// `GET /exchanges` (Pro).
    pub async fn exchanges(&self, per_page: u32) -> Result<Vec<CryptoExchange>, CoinGeckoError> {
        self.require(Entitlement::CoinGeckoPro)?;
        let mut query = BTreeMap::new();
        query.insert("per_page".into(), per_page.to_string());
        query.insert("page".into(), "1".into());
        let (status, headers, body) = self.get("/exchanges", query).await?;
        Self::map_status(status, &headers, &body)?;
        let rows: Vec<CgExchange> =
            serde_json::from_str(&body).map_err(|e| CoinGeckoError::Decode(e.to_string()))?;
        if rows.is_empty() {
            return Err(CoinGeckoError::Empty);
        }
        Ok(rows
            .into_iter()
            .map(|r| CryptoExchange {
                id: r.id.unwrap_or_default(),
                name: r.name.unwrap_or_default(),
                trust_score: r.trust_score,
                trade_volume_24h_btc: r.trade_volume_24h_btc.map(fmt_f64),
                country: r.country.filter(|s| !s.is_empty()),
            })
            .collect())
    }
}

fn map_chart_points(rows: &[Vec<f64>]) -> Result<Vec<MarketChartPoint>, CoinGeckoError> {
    rows.iter()
        .map(|pair| {
            if pair.len() < 2 {
                return Err(CoinGeckoError::Decode(
                    "market_chart point expected [ts, value]".into(),
                ));
            }
            let ms = pair[0] as i64;
            let time = OffsetDateTime::from_unix_timestamp_nanos(ms as i128 * 1_000_000)
                .map_err(|e| CoinGeckoError::Decode(e.to_string()))?;
            Ok(MarketChartPoint {
                time,
                value: fmt_f64(pair[1]),
            })
        })
        .collect()
}

#[async_trait]
impl Provider for CoinGeckoAdapter {
    fn id(&self) -> ProviderId {
        ProviderId::COINGECKO
    }

    fn capabilities(&self) -> ProviderCapabilities {
        [
            Capability::AssetSearch,
            Capability::CryptoGlobalStats,
            Capability::CryptoMarkets,
            Capability::Trending,
            Capability::Ohlcv,
            Capability::Ohlc,
            Capability::Categories,
            Capability::Exchanges,
            Capability::CoinDetail,
        ]
        .into_iter()
        .collect()
    }

    fn entitlements(&self) -> &EntitlementSet {
        &self.entitlements
    }

    fn cost_of(&self, _request: &ProviderRequest) -> CostUnits {
        CostUnits::new(1)
    }

    async fn health(&self) -> ProviderHealth {
        let _ = &self.base_host_hint;
        ProviderHealth {
            status: HealthStatus::Healthy,
            last_success: None,
            last_failure: None,
            consecutive_failures: 0,
            rate_limit_headroom: Some(1.0),
            observed_latency_p95: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct CgMarketChart {
    #[serde(default)]
    prices: Vec<Vec<f64>>,
    #[serde(default)]
    market_caps: Vec<Vec<f64>>,
    #[serde(default)]
    total_volumes: Vec<Vec<f64>>,
}

#[derive(Debug, Deserialize)]
struct CgCoinDetail {
    id: Option<String>,
    symbol: Option<String>,
    name: Option<String>,
    market_cap_rank: Option<u32>,
    categories: Option<Vec<String>>,
    description: Option<CgDescription>,
    links: Option<CgLinks>,
}

#[derive(Debug, Deserialize)]
struct CgDescription {
    en: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CgLinks {
    #[serde(default)]
    homepage: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CgCategory {
    id: Option<String>,
    name: Option<String>,
    market_cap: Option<f64>,
    market_cap_change_24h: Option<f64>,
    top_3_coins: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CgExchange {
    id: Option<String>,
    name: Option<String>,
    trust_score: Option<u32>,
    trade_volume_24h_btc: Option<f64>,
    country: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CgMarketRow {
    id: String,
    symbol: String,
    name: String,
    current_price: Option<f64>,
    price_change_percentage_24h: Option<f64>,
    total_volume: Option<f64>,
    market_cap: Option<f64>,
}

impl CgMarketRow {
    fn into_normalized(self, retrieved_at: OffsetDateTime) -> CryptoMarketQuote {
        let id = self.id.clone();
        CryptoMarketQuote {
            coingecko_id: self.id,
            symbol: self.symbol.to_uppercase(),
            name: self.name,
            price: self
                .current_price
                .map(fmt_f64)
                .unwrap_or_else(|| "0".into()),
            change_24h_pct: self.price_change_percentage_24h.map(fmt_f64),
            volume_24h: self.total_volume.map(fmt_f64),
            market_cap: self.market_cap.map(fmt_f64),
            provider: ProviderId::COINGECKO,
            external_id: ExternalIdentifier::CoinGeckoId(id),
            event_time: retrieved_at,
            retrieved_at,
            quality: DataQualityScore::new(0.95),
        }
    }
}

#[derive(Debug, Deserialize)]
struct CgGlobalEnvelope {
    data: Option<CgGlobalData>,
}

#[derive(Debug, Deserialize)]
struct CgGlobalData {
    #[serde(default)]
    market_cap_percentage: BTreeMap<String, f64>,
    #[serde(default)]
    total_market_cap: BTreeMap<String, f64>,
    #[serde(default)]
    total_volume: BTreeMap<String, f64>,
}

#[derive(Debug, Deserialize)]
struct CgTrendingEnvelope {
    #[serde(default)]
    coins: Vec<CgTrendingCoin>,
}

#[derive(Debug, Deserialize)]
struct CgTrendingCoin {
    item: Option<CgTrendingItem>,
}

#[derive(Debug, Deserialize)]
struct CgTrendingItem {
    id: String,
    symbol: String,
    name: String,
    market_cap_rank: Option<u32>,
}

fn fmt_f64(v: f64) -> String {
    // Stable display string — UI formats further.
    if v.abs() >= 1000.0 {
        format!("{v:.2}")
    } else if v.abs() >= 1.0 {
        format!("{v:.4}")
    } else {
        format!("{v:.6}")
    }
}

/// Built-in Wave 1 demo cassette (offline, no network).
pub fn demo_cassette_json() -> &'static str {
    include_str!("../../cassettes/coingecko/demo_markets.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cassette::CassetteTransport;
    use time::OffsetDateTime;

    fn adapter() -> CoinGeckoAdapter {
        let transport = CassetteTransport::from_json(demo_cassette_json()).unwrap();
        CoinGeckoAdapter::new(
            Arc::new(transport),
            CoinGeckoAuth::Demo {
                api_key: "demo".into(),
            },
        )
    }

    fn pro_adapter() -> CoinGeckoAdapter {
        let transport = CassetteTransport::from_json(demo_cassette_json()).unwrap();
        CoinGeckoAdapter::new(
            Arc::new(transport),
            CoinGeckoAuth::Pro {
                api_key: "pro-demo".into(),
            },
        )
    }

    #[tokio::test]
    async fn markets_from_cassette_normalize() {
        let a = adapter();
        let rows = a
            .markets("usd", &[], OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap();
        assert!(rows.len() >= 5);
        assert_eq!(rows[0].symbol, "BTC");
        assert_eq!(rows[0].provider, ProviderId::COINGECKO);
    }

    #[tokio::test]
    async fn pro_only_endpoint_denied_on_demo() {
        let a = adapter();
        let err = a
            .ohlc("bitcoin", "usd", 30, OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap_err();
        assert!(matches!(err, CoinGeckoError::NotEntitled(_)));
    }

    #[tokio::test]
    async fn ohlc_and_market_chart_replay() {
        let a = pro_adapter();
        let bars = a
            .ohlc("bitcoin", "usd", 30, OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap();
        assert_eq!(bars.len(), 30);
        assert_eq!(bars[0].coingecko_id, "bitcoin");
        let eth = a
            .ohlc("ethereum", "usd", 30, OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap();
        assert_eq!(eth.len(), 30);
        let series = a
            .market_chart("bitcoin", "usd", 30, OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap();
        assert_eq!(series.prices.len(), 30);
    }

    #[tokio::test]
    async fn detail_categories_exchanges_replay() {
        let a = pro_adapter();
        let d = a
            .coin_detail("bitcoin", OffsetDateTime::UNIX_EPOCH)
            .await
            .unwrap();
        assert_eq!(d.symbol, "BTC");
        assert!(d.categories.iter().any(|c| c.contains("Layer 1")));
        let cats = a.categories().await.unwrap();
        assert!(cats.len() >= 3);
        let xs = a.exchanges(20).await.unwrap();
        assert_eq!(xs[0].id, "binance");
    }

    #[tokio::test]
    async fn global_and_trending_replay() {
        let a = adapter();
        let g = a.global(OffsetDateTime::UNIX_EPOCH).await.unwrap();
        assert_eq!(g.btc_dominance.as_deref(), Some("54.2000"));
        let t = a.trending().await.unwrap();
        assert!(!t.is_empty());
    }

    #[tokio::test]
    async fn search_bitcoin() {
        let a = adapter();
        let hits = a.search("bitcoin").await.unwrap();
        assert_eq!(hits[0].coingecko_id, "bitcoin");
    }
}
