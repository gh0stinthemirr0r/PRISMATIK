//! Provider chain contracts.

use prismatik_domain::ProviderId;
use serde::{Deserialize, Serialize};

/// How provider results are compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgreementPolicy {
    /// First successful provider wins.
    FirstSuccess,
    /// Majority agreement required.
    Majority,
}

/// Action when provider outputs diverge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivergenceAction {
    /// Mark result as divergent but return.
    MarkAndReturn,
    /// Fail request.
    HardFail,
}

/// Trigger for provider failover.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailoverTrigger {
    /// HTTP/API error.
    Error,
    /// Empty-success response.
    Empty,
    /// Stale data returned.
    Stale,
}

/// Ordered provider chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderChain {
    /// Capability served by this chain.
    pub capability: crate::Capability,
    /// Preferred provider.
    pub primary: ProviderId,
    /// Ordered fallback providers.
    pub fallbacks: Vec<ProviderId>,
    /// Agreement policy.
    pub agreement: AgreementPolicy,
    /// Divergence behavior.
    pub divergence_action: DivergenceAction,
}

impl ProviderChain {
    /// Default equity chains with Alpaca primary and Finnhub fallback.
    pub fn equity_default_chains() -> Vec<Self> {
        [crate::Capability::Bars, crate::Capability::Ohlcv]
            .into_iter()
            .map(|capability| Self {
                capability,
                primary: ProviderId::ALPACA,
                fallbacks: vec![ProviderId::FINNHUB],
                agreement: AgreementPolicy::FirstSuccess,
                divergence_action: DivergenceAction::MarkAndReturn,
            })
            .collect()
    }
}
