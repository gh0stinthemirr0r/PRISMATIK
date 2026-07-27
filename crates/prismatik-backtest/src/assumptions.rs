//! Execution assumptions for backtests.

use serde::{Deserialize, Serialize};

/// Fill model spec.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillModel {
    /// Fill at next bar open.
    NextOpen,
    /// Fill at bar close.
    Close,
}

/// Slippage model spec.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SlippageModel {
    /// Fixed basis points.
    FixedBps {
        /// Basis-point value.
        bps: String,
    },
}

/// Commission model spec.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommissionModel {
    /// Flat fee per order.
    Flat {
        /// Fee amount.
        amount: String,
    },
}

/// Composite execution assumptions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionAssumptions {
    /// Fill model.
    pub fill_model: FillModel,
    /// Slippage model.
    pub slippage_model: SlippageModel,
    /// Commission model.
    pub commission_model: CommissionModel,
}
