//! Order intent draft and risk-gated paper ticket floor (`P7-EX-01` / `P7-QM-01`).
//!
//! Production OMS will accept only `RiskApprovedOrderIntent` from `prismatik-risk`.
//! This module carries the same submission-critical fields for Wave 4/5 floors:
//! draft [`OrderIntent`] → [`RiskPolicy::evaluate`] → optional [`ApprovedOrder`].

use crate::idempotency::IdempotencyKey;
use prismatik_risk::{CheckId, RiskEvalInput, RiskLimits, RiskPolicy};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Buy / sell direction for a paper or live intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderSide {
    /// Open / increase long (or cover short).
    Buy,
    /// Open / increase short (or reduce long).
    Sell,
}

impl OrderSide {
    /// Parse a UI / IPC side string (`buy` / `sell`). Fail closed on unknown.
    pub fn parse(raw: &str) -> Result<Self, OrderDraftError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "buy" => Ok(Self::Buy),
            "sell" => Ok(Self::Sell),
            other => Err(OrderDraftError::InvalidSide(other.to_string())),
        }
    }

    /// Stable snake_case label for IPC / evidence chips.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
        }
    }
}

/// Order type for the ticket floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    /// Market order (no limit).
    Market,
    /// Limit order (requires [`OrderIntent::limit_price_micros`]).
    Limit,
}

impl OrderType {
    /// Stable snake_case label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Market => "market",
            Self::Limit => "limit",
        }
    }
}

/// Time-in-force for the ticket floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeInForce {
    /// Good for the trading day.
    Day,
    /// Good till cancelled.
    Gtc,
    /// Immediate-or-cancel.
    Ioc,
}

impl TimeInForce {
    /// Stable snake_case label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Gtc => "gtc",
            Self::Ioc => "ioc",
        }
    }
}

/// Unsigned draft intent emitted before risk approval (never submitted raw).
///
/// Strategies / EX surfaces produce intents; the risk kernel gates them into
/// [`ApprovedOrder`] for the broker gateway.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderIntent {
    /// Opaque instrument / symbol id.
    pub instrument_id: String,
    /// Buy or sell.
    pub side: OrderSide,
    /// Absolute quantity in integer lots (`> 0`).
    pub quantity: u32,
    /// Market or limit.
    pub order_type: OrderType,
    /// Limit price in currency micros when [`OrderType::Limit`].
    pub limit_price_micros: Option<i64>,
    /// Time in force.
    pub time_in_force: TimeInForce,
    /// Client-generated idempotency key — retries must reuse this value.
    pub idempotency_key: IdempotencyKey,
}

impl OrderIntent {
    /// Signed quantity for OMS / paper broker (`+` buy, `-` sell).
    pub fn signed_quantity(&self) -> i64 {
        let q = i64::from(self.quantity);
        match self.side {
            OrderSide::Buy => q,
            OrderSide::Sell => -q,
        }
    }

    /// Notional in currency micros using limit (or zero for market without mark).
    pub fn notional_micros(&self) -> i64 {
        let px = self.limit_price_micros.unwrap_or(0).abs();
        px.saturating_mul(i64::from(self.quantity))
    }

    /// Convert a risk-approved intent into the gateway submission stand-in.
    pub fn to_approved(&self) -> ApprovedOrder {
        ApprovedOrder::new(
            self.instrument_id.clone(),
            self.signed_quantity(),
            self.idempotency_key,
        )
    }
}

/// Floor order accepted by [`crate::BrokerGateway::submit`].
///
/// Production OMS will accept only `RiskApprovedOrderIntent` from
/// `prismatik-risk`. This type carries the same submission-critical fields
/// for Wave 4/5 floors.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedOrder {
    /// Opaque instrument / symbol id.
    pub instrument_id: String,
    /// Signed quantity in integer lots (positive = buy / open long).
    pub quantity: i64,
    /// Client-generated idempotency key — retries must reuse this value.
    pub idempotency_key: IdempotencyKey,
}

impl ApprovedOrder {
    /// Construct an approved order.
    pub fn new(
        instrument_id: impl Into<String>,
        quantity: i64,
        idempotency_key: IdempotencyKey,
    ) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            quantity,
            idempotency_key,
        }
    }
}

/// Outcome of the pre-trade risk gate for a paper ticket preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskGateOutcome {
    /// All enabled checks passed; [`PaperOrderTicket::approved`] is populated.
    Approved,
    /// First failing check (HardDeny). No approved order is produced.
    Denied {
        /// Failing [`CheckId`] as snake_case string.
        check: String,
    },
}

/// Offline paper ticket: typed intent + risk decision (no broker submit).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperOrderTicket {
    /// Draft intent (never live-submitted from this type alone).
    pub intent: OrderIntent,
    /// Risk gate result.
    pub risk_outcome: RiskGateOutcome,
    /// Human / machine label for the policy used (e.g. `max_position`).
    pub policy_label: String,
    /// Present only when risk approved — submission-shaped floor type.
    pub approved: Option<ApprovedOrder>,
}

/// Fail-closed draft errors (missing / invalid ticket fields).
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum OrderDraftError {
    /// Empty instrument id.
    #[error("instrument_id required")]
    MissingInstrument,
    /// Quantity must be strictly positive.
    #[error("quantity must be > 0")]
    InvalidQuantity,
    /// Unrecognized side string.
    #[error("invalid side: {0}")]
    InvalidSide(String),
    /// Limit order without a limit price.
    #[error("limit order requires limit_price_micros")]
    MissingLimitPrice,
}

/// Demo limit used by the desktop paper ticket when callers omit a price.
pub const PAPER_DEMO_LIMIT_MICROS: i64 = 214_500_000; // $214.50

/// Default MaxPosition ceiling for the paper ticket floor ($1M notional).
pub const PAPER_MAX_POSITION_MICROS: i64 = 1_000_000_000_000;

/// Build a deterministic paper order ticket and run [`RiskPolicy::evaluate`].
///
/// Offline / demo only — does **not** call a broker. Fail closed on invalid
/// fields; risk deny yields [`RiskGateOutcome::Denied`] without an
/// [`ApprovedOrder`] (still `Ok` so EX can display the decision).
pub fn draft_paper_order_ticket(
    instrument_id: impl Into<String>,
    side: OrderSide,
    quantity: u32,
    limit_price_micros: Option<i64>,
    policy: &RiskPolicy,
    policy_label: impl Into<String>,
) -> Result<PaperOrderTicket, OrderDraftError> {
    let instrument_id = instrument_id.into();
    if instrument_id.trim().is_empty() {
        return Err(OrderDraftError::MissingInstrument);
    }
    if quantity == 0 {
        return Err(OrderDraftError::InvalidQuantity);
    }

    let order_type = OrderType::Limit;
    // Paper tickets are always limit orders; a missing price defaults to the
    // demo ceiling rather than rejecting, so the idempotency material is stable.
    let limit_price_micros = limit_price_micros.unwrap_or(PAPER_DEMO_LIMIT_MICROS);

    let mut material = Vec::new();
    material.extend_from_slice(b"prismatik-paper-ticket|");
    material.extend_from_slice(instrument_id.as_bytes());
    material.push(b'|');
    material.extend_from_slice(side.as_str().as_bytes());
    material.push(b'|');
    material.extend_from_slice(&u32::to_le_bytes(quantity));
    material.push(b'|');
    material.extend_from_slice(&limit_price_micros.to_le_bytes());

    let intent = OrderIntent {
        instrument_id,
        side,
        quantity,
        order_type,
        limit_price_micros: Some(limit_price_micros),
        time_in_force: TimeInForce::Day,
        idempotency_key: IdempotencyKey::generate(&material),
    };

    let mut input = RiskEvalInput::permissive();
    input.position_notional_micros = intent.notional_micros();

    let (risk_outcome, approved) = match policy.evaluate(&input) {
        Ok(()) => (RiskGateOutcome::Approved, Some(intent.to_approved())),
        Err(failure) => (
            RiskGateOutcome::Denied {
                check: check_id_snake(failure.0),
            },
            None,
        ),
    };

    Ok(PaperOrderTicket {
        intent,
        risk_outcome,
        policy_label: policy_label.into(),
        approved,
    })
}

fn check_id_snake(id: CheckId) -> String {
    match id {
        CheckId::MaxPosition => "max_position",
        CheckId::MaxPremium => "max_premium",
        CheckId::MaxAccountLoss => "max_account_loss",
        CheckId::SectorConcentration => "sector_concentration",
        CheckId::AccountPermissions => "account_permissions",
        CheckId::MarketStatus => "market_status",
        CheckId::StaleData => "stale_data",
        CheckId::DuplicateOrder => "duplicate_order",
        CheckId::AdjustedContract => "adjusted_contract",
        CheckId::CalendarMismatch => "calendar_mismatch",
        CheckId::ModelSuppressed => "model_suppressed",
        CheckId::CorrelationCluster => "correlation_cluster",
        CheckId::EventProximity => "event_proximity",
        CheckId::SpreadWidth => "spread_width",
        CheckId::LiquidityVolume => "liquidity_volume",
        CheckId::OpenInterest => "open_interest",
    }
    .to_string()
}

/// Paper-ticket policy with [`CheckId::MaxPosition`] enabled at
/// [`PAPER_MAX_POSITION_MICROS`].
pub fn paper_max_position_policy() -> RiskPolicy {
    RiskPolicy::with_checks([CheckId::MaxPosition]).with_limits(RiskLimits {
        max_position_micros: PAPER_MAX_POSITION_MICROS,
        ..RiskLimits::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_approves_under_max_position() {
        let ticket = draft_paper_order_ticket(
            "AAPL",
            OrderSide::Buy,
            10,
            None,
            &paper_max_position_policy(),
            "max_position",
        )
        .expect("draft");
        assert_eq!(ticket.risk_outcome, RiskGateOutcome::Approved);
        assert!(ticket.approved.is_some());
        assert_eq!(ticket.intent.side, OrderSide::Buy);
        assert_eq!(ticket.intent.quantity, 10);
        assert_eq!(ticket.intent.order_type, OrderType::Limit);
        assert_eq!(
            ticket.intent.limit_price_micros,
            Some(PAPER_DEMO_LIMIT_MICROS)
        );
        let approved = ticket.approved.unwrap();
        assert_eq!(approved.quantity, 10);
        assert_eq!(approved.instrument_id, "AAPL");
    }

    #[test]
    fn draft_denies_over_max_position() {
        // 10_000_000 shares * $214.50 ≫ $1M max.
        let ticket = draft_paper_order_ticket(
            "AAPL",
            OrderSide::Buy,
            10_000_000,
            None,
            &paper_max_position_policy(),
            "max_position",
        )
        .expect("draft builds even when denied");
        assert_eq!(
            ticket.risk_outcome,
            RiskGateOutcome::Denied {
                check: "max_position".into()
            }
        );
        assert!(ticket.approved.is_none());
    }

    #[test]
    fn empty_policy_always_approves() {
        let ticket = draft_paper_order_ticket(
            "SPY",
            OrderSide::Sell,
            1,
            Some(100_000_000),
            &RiskPolicy::empty(),
            "empty",
        )
        .expect("draft");
        assert_eq!(ticket.risk_outcome, RiskGateOutcome::Approved);
        assert_eq!(ticket.approved.as_ref().unwrap().quantity, -1);
    }

    #[test]
    fn fail_closed_on_empty_symbol_and_zero_qty() {
        assert_eq!(
            draft_paper_order_ticket("  ", OrderSide::Buy, 1, None, &RiskPolicy::empty(), "empty"),
            Err(OrderDraftError::MissingInstrument)
        );
        assert_eq!(
            draft_paper_order_ticket(
                "AAPL",
                OrderSide::Buy,
                0,
                None,
                &RiskPolicy::empty(),
                "empty"
            ),
            Err(OrderDraftError::InvalidQuantity)
        );
    }

    #[test]
    fn side_parse_fail_closed() {
        assert!(OrderSide::parse("buy").is_ok());
        assert_eq!(
            OrderSide::parse("hold"),
            Err(OrderDraftError::InvalidSide("hold".into()))
        );
    }

    #[test]
    fn idempotency_key_deterministic() {
        let a = draft_paper_order_ticket(
            "AAPL",
            OrderSide::Buy,
            10,
            None,
            &paper_max_position_policy(),
            "max_position",
        )
        .unwrap();
        let b = draft_paper_order_ticket(
            "AAPL",
            OrderSide::Buy,
            10,
            None,
            &paper_max_position_policy(),
            "max_position",
        )
        .unwrap();
        assert_eq!(a.intent.idempotency_key, b.intent.idempotency_key);
    }
}
