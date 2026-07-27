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
    /// Ordered providers.
    pub providers: Vec<ProviderId>,
    /// Agreement policy.
    pub agreement: AgreementPolicy,
    /// Divergence behavior.
    pub divergence_action: DivergenceAction,
}
