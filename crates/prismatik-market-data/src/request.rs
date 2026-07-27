//! Provider request/response types.

use prismatik_domain::ProviderId;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Stable endpoint identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EndpointId(pub String);

/// Provider cost units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostUnits(pub u32);

/// Normalized provider request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRequest {
    /// Target provider.
    pub provider: ProviderId,
    /// Endpoint key.
    pub endpoint: EndpointId,
    /// Canonical query key.
    pub query_key: String,
}

/// Normalized provider response metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderResponse {
    /// Provider that answered.
    pub provider: ProviderId,
    /// Retrieval time.
    pub retrieved_at: OffsetDateTime,
    /// Event time for payload.
    pub event_time: Option<OffsetDateTime>,
    /// Payload hash.
    pub payload_hash: String,
}
