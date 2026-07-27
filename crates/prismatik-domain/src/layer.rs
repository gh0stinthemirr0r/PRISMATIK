//! Data layer taxonomy.

use serde::{Deserialize, Serialize};

/// Canonical storage/processing layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataLayer {
    /// Raw provider payloads.
    Raw,
    /// Canonical normalized records.
    Normalized,
    /// Curated analytical records.
    Curated,
    /// Feature-store values.
    Feature,
    /// Intelligence/conclusion artifacts.
    Intelligence,
    /// Presentation/UI projection layer.
    Presentation,
}
