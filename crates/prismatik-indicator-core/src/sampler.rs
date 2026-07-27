//! Bar sampling contracts.

use serde::{Deserialize, Serialize};

/// Time bar interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarInterval {
    /// 1-minute bars.
    OneMinute,
    /// 5-minute bars.
    FiveMinute,
    /// 1-hour bars.
    OneHour,
    /// 1-day bars.
    OneDay,
}

/// Bar sampling configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BarSampler {
    /// Time bars.
    Time(BarInterval),
    /// Tick bars.
    Tick {
        /// Tick count threshold.
        count: usize,
    },
    /// Volume bars.
    Volume {
        /// Volume threshold.
        threshold: f64,
    },
    /// Dollar bars.
    Dollar {
        /// Dollar-notional threshold.
        threshold: f64,
    },
}
