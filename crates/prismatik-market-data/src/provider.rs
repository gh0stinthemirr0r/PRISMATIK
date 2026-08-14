//! Provider trait, capabilities, entitlements, and health.

use crate::request::{CostUnits, ProviderRequest};
use async_trait::async_trait;
use prismatik_domain::ProviderId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use time::OffsetDateTime;

/// A discrete capability a provider advertises. Used to gate which endpoints a
/// provider is asked to serve (a provider without [`Capability::Ohlcv`] is
/// never asked for bars).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Asset / symbol search.
    AssetSearch,
    /// Crypto global market stats.
    CryptoGlobalStats,
    /// Crypto market quotes (top-of-book per asset).
    CryptoMarkets,
    /// Trending assets.
    Trending,
    /// OHLCV history.
    Ohlcv,
    /// OHLC history.
    Ohlc,
    /// Crypto categories.
    Categories,
    /// Crypto exchange catalog.
    Exchanges,
    /// Single-coin detail.
    CoinDetail,
    /// Equity historical bars.
    EquityBars,
    /// Generic historical bars (used by equity providers like Alpaca/Finnhub).
    Bars,
    /// SEC filings.
    Filings,
    /// SEC EDGAR filing index / submissions lookup.
    FilingIndex,
    /// SEC EDGAR company facts (XBRL).
    CompanyFacts,
    /// Insider transactions (Form 4).
    InsiderTransactions,
    /// Macro time series (FRED).
    MacroSeries,
    /// CFTC commitments-of-traders.
    CotReport,
    /// CFTC commitments-of-traders (plural alias used by adapters).
    CotReports,
    /// Options chain.
    OptionsChain,
    /// Options flow prints.
    OptionsFlow,
    /// Dealer positioning.
    DealerExposure,
}

/// A named entitlement granted to a provider account (API tier, dataset
/// permission). Requests against capabilities the account is not entitled to
/// fail closed without touching the network.
///
/// Modeled as an enum so the platform's entitlement surface is closed and
/// exhaustively checkable: a new provider tier is a new variant, reviewed at
/// compile time, never a free-form string invented at a call site.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Entitlement {
    /// CoinGecko demo (free public) tier.
    CoinGeckoDemo,
    /// CoinGecko paid (pro) tier.
    CoinGeckoPro,
    /// Alpaca market-data entitlement.
    AlpacaMarketData,
    /// CFTC public-data entitlement (no key required).
    CftcPublic,
    /// Finnhub entitlement.
    Finnhub,
    /// FRED entitlement.
    Fred,
    /// SEC EDGAR entitlement (public, with declared contact).
    SecEdgar,
    /// A public venue endpoint that needs no credential at all.
    ///
    /// Distinct from the keyed entitlements above: there is nothing to
    /// revoke, so "entitled" here means the operator turned the poll on.
    PublicVenue,
}

impl Entitlement {
    /// Stable string code for serialization contexts that want a flat label.
    pub fn as_code(&self) -> &'static str {
        match self {
            Self::CoinGeckoDemo => "coingecko:demo",
            Self::CoinGeckoPro => "coingecko:pro",
            Self::AlpacaMarketData => "alpaca:market_data",
            Self::CftcPublic => "cftc:public",
            Self::Finnhub => "finnhub:default",
            Self::Fred => "fred:default",
            Self::SecEdgar => "sec_edgar:public",
            Self::PublicVenue => "venue:public",
        }
    }
}

/// Provider capability matrix — which [`Capability`]s the provider serves.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderCapabilities(BTreeSet<Capability>);

impl ProviderCapabilities {
    /// Return whether a capability is advertised.
    pub fn supports(&self, capability: Capability) -> bool {
        self.0.contains(&capability)
    }

    /// Alias for capability membership accepting owned or borrowed values.
    pub fn contains(&self, capability: impl std::borrow::Borrow<Capability>) -> bool {
        self.0.contains(capability.borrow())
    }

    /// Insert an advertised capability.
    pub fn insert(&mut self, capability: Capability) -> bool {
        self.0.insert(capability)
    }
}

impl<const N: usize> From<[Capability; N]> for ProviderCapabilities {
    fn from(value: [Capability; N]) -> Self {
        Self(value.into_iter().collect())
    }
}

impl FromIterator<Capability> for ProviderCapabilities {
    fn from_iter<T: IntoIterator<Item = Capability>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// Entitlement set for a provider account.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntitlementSet(BTreeSet<Entitlement>);

impl EntitlementSet {
    /// Return whether an entitlement is active, accepting owned or borrowed values.
    pub fn contains(&self, entitlement: impl std::borrow::Borrow<Entitlement>) -> bool {
        self.0.contains(entitlement.borrow())
    }

    /// Insert an active entitlement.
    pub fn insert(&mut self, entitlement: Entitlement) -> bool {
        self.0.insert(entitlement)
    }
}

impl<const N: usize> From<[Entitlement; N]> for EntitlementSet {
    fn from(value: [Entitlement; N]) -> Self {
        Self(value.into_iter().collect())
    }
}

impl FromIterator<Entitlement> for EntitlementSet {
    fn from_iter<T: IntoIterator<Item = Entitlement>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// Coarse health classification reported by a provider probe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Provider answered the probe successfully.
    #[default]
    Healthy,
    /// Provider answered but signalled degradation (rate-limit pressure, partial outage).
    Degraded,
    /// Provider did not answer or returned a hard failure.
    Unhealthy,
}

/// Provider health snapshot returned by [`Provider::health`].
///
/// Every field is optional so a provider can report only what it actually
/// measured; absent telemetry is never fabricated. Float fields make this
/// struct `PartialEq`-only — health is telemetry, not an audited value.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProviderHealth {
    /// Coarse health classification.
    pub status: HealthStatus,
    /// Timestamp of the most recent successful response.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_success: Option<OffsetDateTime>,
    /// Timestamp of the most recent failure.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_failure: Option<OffsetDateTime>,
    /// Consecutive failure count (resets on success).
    #[serde(default)]
    pub consecutive_failures: u32,
    /// Fractional rate-limit headroom in `[0.0, 1.0]` (1.0 = full budget).
    #[serde(default)]
    pub rate_limit_headroom: Option<f64>,
    /// Observed p95 latency (seconds).
    #[serde(default)]
    pub observed_latency_p95: Option<f64>,
}

impl ProviderHealth {
    /// A clean "healthy, no telemetry yet" snapshot. Used by providers that do
    /// not run a live probe on every `health()` call.
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            last_success: None,
            last_failure: None,
            consecutive_failures: 0,
            rate_limit_headroom: Some(1.0),
            observed_latency_p95: None,
        }
    }
}

/// Provider contract.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Stable provider id.
    fn id(&self) -> ProviderId;
    /// Capability declaration.
    fn capabilities(&self) -> ProviderCapabilities;
    /// Active entitlements.
    fn entitlements(&self) -> &EntitlementSet;
    /// Predicted cost for request.
    fn cost_of(&self, request: &ProviderRequest) -> CostUnits;
    /// Provider health probe.
    async fn health(&self) -> ProviderHealth;
}
