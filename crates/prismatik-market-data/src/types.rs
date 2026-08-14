//! Normalized market-data records (Layer 2 view of provider payloads).

use prismatik_domain::{DataQualityScore, ProviderId};
use prismatik_identity::ExternalIdentifier;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

/// A normalized crypto market row (CoinGecko `/coins/markets`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CryptoMarketQuote {
    /// CoinGecko id (canonical crypto identity).
    pub coingecko_id: String,
    /// Display symbol (e.g. BTC).
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Last price in quote currency.
    pub price: String,
    /// 24h change percent as decimal string (e.g. `"2.14"`).
    pub change_24h_pct: Option<String>,
    /// 24h volume in quote currency.
    pub volume_24h: Option<String>,
    /// Market cap if present.
    pub market_cap: Option<String>,
    /// Provider actually used.
    pub provider: ProviderId,
    /// External id attribute.
    pub external_id: ExternalIdentifier,
    /// Event time from provider when available; else retrieval time.
    pub event_time: OffsetDateTime,
    /// When we retrieved it.
    pub retrieved_at: OffsetDateTime,
    /// Quality score.
    pub quality: DataQualityScore,
}

/// Global crypto stats snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CryptoGlobalStats {
    /// BTC dominance percent.
    pub btc_dominance: Option<String>,
    /// ETH dominance percent.
    pub eth_dominance: Option<String>,
    /// Total market cap USD.
    pub total_market_cap_usd: Option<String>,
    /// Total 24h volume USD.
    pub total_volume_usd: Option<String>,
    /// Provider.
    pub provider: ProviderId,
    /// Retrieved at.
    pub retrieved_at: OffsetDateTime,
}

/// Trending coin entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrendingCoin {
    /// CoinGecko id.
    pub coingecko_id: String,
    /// Symbol.
    pub symbol: String,
    /// Name.
    pub name: String,
    /// Rank on trending list.
    pub market_cap_rank: Option<u32>,
}

/// Normalized OHLC candle (CoinGecko `/coins/{id}/ohlc`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OhlcBar {
    /// CoinGecko id.
    pub coingecko_id: String,
    /// Bar open time (UTC).
    pub bar_start: OffsetDateTime,
    /// Open.
    pub open: String,
    /// High.
    pub high: String,
    /// Low.
    pub low: String,
    /// Close.
    pub close: String,
    /// Provider actually used.
    pub provider: ProviderId,
    /// Retrieved at.
    pub retrieved_at: OffsetDateTime,
    /// Quality score.
    pub quality: DataQualityScore,
}

/// One OHLCV candle from a public trading venue.
///
/// Separate from `OhlcBar`, which is keyed on a CoinGecko id and carries no
/// volume, and from `EquityBar`, whose name would be a lie on a crypto pair.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VenueCandle {
    /// The venue's own pair symbol, as requested.
    pub pair: String,
    /// Candle open time (UTC).
    pub bar_start: OffsetDateTime,
    /// Open, as a decimal string to avoid binary-float drift.
    pub open: String,
    /// High.
    pub high: String,
    /// Low.
    pub low: String,
    /// Close.
    pub close: String,
    /// Base-asset volume.
    pub volume: String,
    /// Venue this came from.
    pub provider: ProviderId,
    /// Retrieved at.
    pub retrieved_at: OffsetDateTime,
}

/// A `[timestamp_ms, value]` series point from market_chart.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarketChartPoint {
    /// Event time.
    pub time: OffsetDateTime,
    /// Value as decimal string.
    pub value: String,
}

/// Normalized market chart series (prices / caps / volumes).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarketChartSeries {
    /// CoinGecko id.
    pub coingecko_id: String,
    /// Price series.
    pub prices: Vec<MarketChartPoint>,
    /// Market cap series (may be empty).
    pub market_caps: Vec<MarketChartPoint>,
    /// Volume series (may be empty).
    pub total_volumes: Vec<MarketChartPoint>,
    /// Provider.
    pub provider: ProviderId,
    /// Retrieved at.
    pub retrieved_at: OffsetDateTime,
}

/// Coin detail snapshot (Pro `/coins/{id}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoinDetail {
    /// CoinGecko id.
    pub coingecko_id: String,
    /// Symbol.
    pub symbol: String,
    /// Name.
    pub name: String,
    /// Market cap rank.
    pub market_cap_rank: Option<u32>,
    /// Homepage URL if present.
    pub homepage: Option<String>,
    /// Short description (may be truncated upstream).
    pub description: Option<String>,
    /// Category labels.
    pub categories: Vec<String>,
    /// Provider.
    pub provider: ProviderId,
    /// Retrieved at.
    pub retrieved_at: OffsetDateTime,
}

/// Market category row (`/coins/categories`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CryptoCategory {
    /// Category id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Market cap USD.
    pub market_cap: Option<String>,
    /// 24h change percent.
    pub change_24h_pct: Option<String>,
    /// Top coins in the category (ids).
    pub top_3_coins: Vec<String>,
}

/// Exchange row (`/exchanges`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CryptoExchange {
    /// Exchange id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Trust score (0–10) when present.
    pub trust_score: Option<u32>,
    /// 24h BTC volume.
    pub trade_volume_24h_btc: Option<String>,
    /// Country if known.
    pub country: Option<String>,
}

/// Filing metadata normalized from SEC submissions or filing search.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilingSummary {
    /// SEC central index key.
    pub cik: u64,
    /// Accession number.
    pub accession_number: String,
    /// SEC form type.
    pub form: String,
    /// Date the filing became public.
    pub filing_date: Date,
    /// Reporting period, when supplied.
    pub report_date: Option<Date>,
    /// Primary filing document.
    pub primary_document: Option<String>,
}

/// A normalized subset of an SEC XBRL company fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyFact {
    /// SEC central index key.
    pub cik: u64,
    /// XBRL concept name.
    pub concept: String,
    /// Unit of measure.
    pub unit: String,
    /// Value represented without floating-point loss.
    pub value: String,
    /// Period start for duration facts.
    pub period_start: Option<Date>,
    /// Period end.
    pub period_end: Date,
    /// Filing date and observability hinge.
    pub filing_date: Date,
    /// SEC form type.
    pub form: String,
}

/// A weekly CFTC Commitments of Traders report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CotReport {
    /// CFTC market code.
    pub market_code: String,
    /// Market display name.
    pub market_name: String,
    /// Tuesday position date.
    pub as_of: Date,
    /// Public release timestamp.
    pub published_at: OffsetDateTime,
    /// Reportable long positions.
    pub long_positions: i64,
    /// Reportable short positions.
    pub short_positions: i64,
    /// Total open interest.
    pub open_interest: i64,
}

/// A point-in-time-correct FRED observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroSeriesPoint {
    /// FRED series identifier.
    pub series_id: String,
    /// Observation date.
    pub date: Date,
    /// Value; `None` represents FRED's `.` missing marker.
    pub value: Option<String>,
    /// First date of this vintage.
    pub realtime_start: Date,
    /// Last date of this vintage, when bounded.
    pub realtime_end: Option<Date>,
    /// Earliest date this value was available.
    pub available_at: Date,
}

/// A normalized daily or intraday equity OHLCV bar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquityBar {
    /// Equity symbol.
    pub symbol: String,
    /// Bar start timestamp.
    pub bar_start: OffsetDateTime,
    /// Open price.
    pub open: String,
    /// High price.
    pub high: String,
    /// Low price.
    pub low: String,
    /// Close price.
    pub close: String,
    /// Traded volume.
    pub volume: u64,
    /// Number of trades, if supplied.
    pub trade_count: Option<u64>,
    /// Volume-weighted average price, if supplied.
    pub vwap: Option<String>,
    /// Provider actually used.
    pub provider: ProviderId,
    /// Retrieval time.
    pub retrieved_at: OffsetDateTime,
}

/// Normalized equity symbol-search hit (Finnhub `/search`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EquitySearchHit {
    /// Tradable symbol, e.g. `AAPL`.
    pub symbol: String,
    /// Human-readable security description.
    pub description: String,
    /// Listing venue or instrument type as reported by the provider.
    pub exchange: String,
    /// Provider actually used.
    pub provider: ProviderId,
    /// Retrieval time.
    pub retrieved_at: OffsetDateTime,
}
