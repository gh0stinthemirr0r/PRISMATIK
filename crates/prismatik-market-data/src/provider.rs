//! Provider trait and capabilities.

use crate::request::{CostUnits, ProviderRequest};
use async_trait::async_trait;
use prismatik_domain::ProviderId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use time::OffsetDateTime;

/// Provider capability matrix.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    /// Supports historical bar reads.
    pub supports_historical_bars: bool,
    /// Supports real-time quotes.
    pub supports_realtime_quotes: bool,
    /// Supports search APIs.
    pub supports_search: bool,
}

/// Entitlement set for a provider account.
pub type EntitlementSet = BTreeSet<String>;

/// Provider health status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderHealth {
    /// Whether provider is reachable.
    pub reachable: bool,
    /// Optional retry hint.
    pub retry_at: Option<OffsetDateTime>,
    /// Optional health note.
    pub note: Option<String>,
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
