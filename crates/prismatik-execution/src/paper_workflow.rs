//! Local paper-only execution ledger.
//!
//! The workflow accepts only risk-approved intent and a caller-supplied market
//! price observation. It has no network or live broker capability.

use prismatik_risk::RiskApprovedOrderIntent;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use time::OffsetDateTime;

/// Paper order inputs outside the risk-approved intent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperOrderRequest {
    /// Stable caller-provided idempotency key.
    pub idempotency_key: String,
    /// Execution price in currency micros from a real or replay observation.
    pub execution_price_micros: u64,
    /// Immutable evidence id for the price observation.
    pub price_evidence_id: String,
    /// Caller-supplied event time.
    pub occurred_at: OffsetDateTime,
}

/// Immutable paper fill appended to the local ledger.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperFill {
    /// Idempotency key that produced the fill.
    pub idempotency_key: String,
    /// Canonical asset identifier.
    pub asset_id: String,
    /// Normalized order side.
    pub side: String,
    /// Quantity scaled to eight decimal places.
    pub quantity_e8: i128,
    /// Execution price in currency micros.
    pub execution_price_micros: u64,
    /// Signed cash flow in currency micros; buys are negative.
    pub cash_flow_micros: i128,
    /// Evidence backing the execution price.
    pub price_evidence_id: String,
    /// Fill event time.
    pub occurred_at: OffsetDateTime,
}

/// Paper workflow validation failure.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PaperOrderError {
    /// Idempotency key or evidence id was empty.
    #[error("paper order requires idempotency and price-evidence identifiers")]
    EmptyIdentifier,
    /// Price was zero.
    #[error("paper execution price must be positive")]
    InvalidPrice,
    /// Side was neither buy nor sell.
    #[error("unsupported paper order side: {0}")]
    InvalidSide(String),
    /// Quantity was invalid, non-positive, or exceeded eight decimal places.
    #[error("invalid paper order quantity: {0}")]
    InvalidQuantity(String),
    /// Reused idempotency key described a different order.
    #[error("idempotency key was reused with different paper order inputs")]
    IdempotencyConflict,
    /// Fixed-point notional exceeded supported integer range.
    #[error("paper order notional overflow")]
    NotionalOverflow,
}

/// Append-only in-memory paper fill ledger.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PaperLedger {
    fills: Vec<PaperFill>,
    by_key: BTreeMap<String, usize>,
}

impl PaperLedger {
    /// Create an empty local paper ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild a ledger from previously verified durable fills.
    pub fn from_fills(fills: Vec<PaperFill>) -> Result<Self, PaperOrderError> {
        let mut by_key = BTreeMap::new();
        for (position, fill) in fills.iter().enumerate() {
            if fill.idempotency_key.trim().is_empty() || fill.price_evidence_id.trim().is_empty() {
                return Err(PaperOrderError::EmptyIdentifier);
            }
            if by_key
                .insert(fill.idempotency_key.clone(), position)
                .is_some()
            {
                return Err(PaperOrderError::IdempotencyConflict);
            }
        }
        Ok(Self { fills, by_key })
    }

    /// Fill a risk-approved intent at an evidence-backed supplied price.
    pub fn submit(
        &mut self,
        intent: &RiskApprovedOrderIntent,
        request: PaperOrderRequest,
    ) -> Result<&PaperFill, PaperOrderError> {
        if request.idempotency_key.trim().is_empty()
            || request.price_evidence_id.trim().is_empty()
            || intent.asset_id.trim().is_empty()
        {
            return Err(PaperOrderError::EmptyIdentifier);
        }
        if request.execution_price_micros == 0 {
            return Err(PaperOrderError::InvalidPrice);
        }
        let side_sign = match intent.side.trim().to_ascii_lowercase().as_str() {
            "buy" => -1_i128,
            "sell" => 1_i128,
            _ => return Err(PaperOrderError::InvalidSide(intent.side.clone())),
        };
        let quantity_e8 = parse_quantity_e8(&intent.quantity)?;
        let unsigned_cash_flow = quantity_e8
            .checked_mul(i128::from(request.execution_price_micros))
            .ok_or(PaperOrderError::NotionalOverflow)?
            / 100_000_000;
        let fill = PaperFill {
            idempotency_key: request.idempotency_key.clone(),
            asset_id: intent.asset_id.clone(),
            side: intent.side.trim().to_ascii_lowercase(),
            quantity_e8,
            execution_price_micros: request.execution_price_micros,
            cash_flow_micros: side_sign * unsigned_cash_flow,
            price_evidence_id: request.price_evidence_id,
            occurred_at: request.occurred_at,
        };
        if let Some(position) = self.by_key.get(&request.idempotency_key).copied() {
            return if self.fills[position] == fill {
                Ok(&self.fills[position])
            } else {
                Err(PaperOrderError::IdempotencyConflict)
            };
        }
        let position = self.fills.len();
        self.fills.push(fill);
        self.by_key.insert(request.idempotency_key, position);
        Ok(&self.fills[position])
    }

    /// Return fills in deterministic append order.
    pub fn fills(&self) -> &[PaperFill] {
        &self.fills
    }
}

fn parse_quantity_e8(raw: &str) -> Result<i128, PaperOrderError> {
    let raw = raw.trim();
    let mut parts = raw.split('.');
    let whole = parts.next().unwrap_or("");
    let fractional = parts.next().unwrap_or("");
    if raw.starts_with('-')
        || raw.starts_with('+')
        || parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fractional.bytes().all(|byte| byte.is_ascii_digit())
        || fractional.len() > 8
    {
        return Err(PaperOrderError::InvalidQuantity(raw.into()));
    }
    let whole = whole
        .parse::<i128>()
        .map_err(|_| PaperOrderError::InvalidQuantity(raw.into()))?;
    let fractional = if fractional.is_empty() {
        0
    } else {
        fractional
            .parse::<i128>()
            .map_err(|_| PaperOrderError::InvalidQuantity(raw.into()))?
            * 10_i128.pow(u32::try_from(8 - fractional.len()).expect("at most eight"))
    };
    let quantity = whole
        .checked_mul(100_000_000)
        .and_then(|value| value.checked_add(fractional))
        .ok_or_else(|| PaperOrderError::InvalidQuantity(raw.into()))?;
    if quantity == 0 {
        return Err(PaperOrderError::InvalidQuantity(raw.into()));
    }
    Ok(quantity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approved() -> RiskApprovedOrderIntent {
        RiskApprovedOrderIntent {
            asset_id: "BTC-USD".into(),
            side: "buy".into(),
            quantity: "0.125".into(),
        }
    }

    fn request(key: &str) -> PaperOrderRequest {
        PaperOrderRequest {
            idempotency_key: key.into(),
            execution_price_micros: 60_000_000_000,
            price_evidence_id: "observation:price:1".into(),
            occurred_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn paper_fill_is_exact_evidence_backed_and_idempotent() {
        let mut ledger = PaperLedger::new();
        let first = ledger
            .submit(&approved(), request("order:1"))
            .unwrap()
            .clone();
        let second = ledger.submit(&approved(), request("order:1")).unwrap();
        assert_eq!(&first, second);
        assert_eq!(ledger.fills().len(), 1);
        assert_eq!(first.quantity_e8, 12_500_000);
        assert_eq!(first.cash_flow_micros, -7_500_000_000);
    }

    #[test]
    fn reused_key_with_different_price_fails_closed() {
        let mut ledger = PaperLedger::new();
        ledger.submit(&approved(), request("order:1")).unwrap();
        let mut changed = request("order:1");
        changed.execution_price_micros += 1;
        assert_eq!(
            ledger.submit(&approved(), changed),
            Err(PaperOrderError::IdempotencyConflict)
        );
    }
}
