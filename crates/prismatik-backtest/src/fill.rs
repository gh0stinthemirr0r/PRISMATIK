//! Fill contracts.

use serde::{Deserialize, Serialize};

/// Fill type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillKind {
    /// Full fill.
    Full,
    /// Partial fill.
    Partial,
}

/// Fill event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fill {
    /// Fill kind.
    pub kind: FillKind,
    /// Fill price.
    pub price: String,
    /// Fill quantity.
    pub quantity: String,
}
