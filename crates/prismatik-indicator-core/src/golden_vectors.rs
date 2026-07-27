//! Golden vector contract.

use crate::Bar;
use serde::{Deserialize, Serialize};

/// Golden vector input/output set for one indicator.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoldenVector {
    /// Indicator id.
    pub indicator_id: String,
    /// Input bars.
    pub input: Vec<Bar>,
    /// Expected output values.
    pub expected: Vec<f64>,
}
