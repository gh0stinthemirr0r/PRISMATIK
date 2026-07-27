//! Feature-view contracts.

use crate::ObservationDelay;
use serde::{Deserialize, Serialize};

/// Stable feature-view id.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FeatureViewId(pub String);

/// Feature entity kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    /// Asset entity.
    Asset,
    /// Venue entity.
    Venue,
}

/// Feature spec descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureSpec {
    /// Feature key.
    pub key: String,
    /// Observation delay.
    pub observation_delay: ObservationDelay,
}

/// Feature view metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureView {
    /// View id.
    pub id: FeatureViewId,
    /// Entity kind.
    pub entity_kind: EntityKind,
    /// Feature list.
    pub features: Vec<FeatureSpec>,
}

/// Online-store marker trait.
pub trait OnlineStore: Send + Sync {}

/// Offline-store marker trait.
pub trait OfflineStore: Send + Sync {}
