//! # prismatik-prismatik-portfolio
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — portfolio model contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use lot::{Lot, LotId, RealizedLot};
pub use pnl::{PnlError, RealizedPnl, UnrealizedPnl};
pub use position::{AvgCostSource, Multiplier, Position, PositionRisk, PositionSide};
pub use projection::{PortfolioProjection, ProjectionError, ReconciliationStatus};

/// Position contracts.
pub mod position {
    use prismatik_identity::AssetId;

    /// Position side.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum PositionSide {
        /// Long exposure.
        Long,
        /// Short exposure.
        Short,
    }

    /// Average cost source visibility marker.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum AvgCostSource {
        /// Broker-reported basis.
        Broker,
        /// Wallet-derived basis.
        Wallet,
    }

    /// Contract multiplier wrapper.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Multiplier(pub String);

    /// Nested risk details.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PositionRisk {
        /// Maximum loss as decimal string.
        pub max_loss: String,
    }

    /// Portfolio position.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Position {
        /// Canonical asset id.
        pub asset_id: AssetId,
        /// Currency code.
        pub currency: String,
        /// Position side.
        pub side: PositionSide,
        /// Position quantity.
        pub quantity: String,
        /// Average cost.
        pub avg_cost: String,
        /// Current market price.
        pub market_price: String,
        /// Current market value.
        pub market_value: String,
        /// Unrealized pnl.
        pub unrealized_pnl: String,
        /// Realized pnl.
        pub realized_pnl: String,
        /// Required contract multiplier.
        pub multiplier: Multiplier,
        /// Source of average cost.
        pub avg_cost_source: AvgCostSource,
        /// Optional nested risk block.
        pub risk: Option<PositionRisk>,
    }
}

/// Lot contracts.
pub mod lot {
    /// Stable lot identifier.
    pub type LotId = String;

    /// Open lot.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Lot {
        /// Lot id.
        pub id: LotId,
        /// Quantity as decimal string.
        pub quantity: String,
        /// Unit cost as decimal string.
        pub cost: String,
    }

    /// Realized lot summary.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RealizedLot {
        /// Lot id.
        pub id: LotId,
        /// Realized pnl.
        pub realized_pnl: String,
    }
}

/// PnL contracts.
pub mod pnl {
    /// PnL error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PnlError {
        /// Error message.
        pub message: String,
    }

    impl PnlError {
        /// Construct a new error.
        pub fn new(message: impl Into<String>) -> Self {
            Self {
                message: message.into(),
            }
        }
    }

    /// Realized pnl amount.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RealizedPnl {
        /// Amount as decimal string.
        pub amount: String,
    }

    /// Unrealized pnl amount.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct UnrealizedPnl {
        /// Amount as decimal string.
        pub amount: String,
    }
}

/// Projection contracts.
pub mod projection {
    use crate::position::Position;

    /// Reconciliation status.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ReconciliationStatus {
        /// Projection and source are aligned.
        InSync,
        /// Projection diverges from source.
        Diverged,
        /// Reconciliation is pending.
        Pending,
    }

    /// Projection error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ProjectionError {
        /// Error message.
        pub message: String,
    }

    /// Portfolio projection snapshot.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PortfolioProjection {
        /// Positions in the projection.
        pub positions: Vec<Position>,
        /// Reconciliation status.
        pub status: ReconciliationStatus,
    }
}
