//! Trading Constitution — immutable risk controls (Fabric §1) and the
//! expanded action vocabulary (§14).
//!
//! The canon: *"Above every strategy, model, agent, optimization process, and
//! trading decision should exist a deterministic Trading Constitution. It is
//! the supreme authority of the system. The Constitution must not be editable
//! by an inference-time LLM, RL policy, strategy agent, research agent, or
//! execution algorithm."*
//!
//! This module defines the full constitution as a typed, validated struct
//! that the risk gate checks against before any order reaches execution.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The complete trading constitution — all configurable risk limits the
/// system enforces deterministically. Values are operator policy, not
/// universal constants (§1: "Values should be configuration and capital-
/// policy decisions").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TradingConstitution {
    // --- Capital limits ---
    /// Maximum total capital the system may deploy (USD micros).
    pub max_portfolio_capital_micros: i64,
    /// Maximum gross exposure (long + short absolute) as fraction of capital.
    pub max_gross_exposure_pct: f64,
    /// Maximum net exposure (long - short) as fraction of capital.
    pub max_net_exposure_pct: f64,
    /// Maximum leverage ratio.
    pub max_leverage: f64,
    /// Maximum single-position size as fraction of capital.
    pub max_single_position_pct: f64,

    // --- Concentration limits ---
    /// Maximum sector exposure as fraction of capital.
    pub max_sector_exposure_pct: f64,
    /// Maximum correlated-exposure cluster as fraction of capital.
    pub max_correlated_exposure_pct: f64,
    /// Maximum instrument concentration (single name) as fraction of capital.
    pub max_instrument_concentration_pct: f64,

    // --- Loss limits ---
    /// Maximum intraday loss as fraction of starting equity.
    pub max_intraday_loss_pct: f64,
    /// Maximum rolling drawdown as fraction of peak (the lockdown threshold).
    pub max_drawdown_pct: f64,
    /// Maximum per-trade risk as fraction of equity.
    pub max_risk_per_trade_pct: f64,

    // --- Execution limits ---
    /// Maximum execution slippage in basis points before an order is rejected.
    pub max_slippage_bps: f64,
    /// Maximum orders per second.
    pub max_orders_per_second: u32,
    /// Maximum cancel/replace rate per second.
    pub max_cancel_replace_rate: u32,

    // --- Eligibility ---
    /// Approved instrument symbols (empty = all allowed).
    pub approved_instruments: BTreeSet<String>,
    /// Approved venue identifiers (empty = all allowed).
    pub approved_venues: BTreeSet<String>,
    /// Approved trading sessions (e.g. "US_EQUITY_RTH", "CRYPTO_247").
    pub approved_sessions: BTreeSet<String>,
    /// Whether short selling is permitted.
    pub short_sale_eligible: bool,
    /// Restricted symbols that may not be traded.
    pub restricted_symbols: BTreeSet<String>,

    // --- Staleness / halt ---
    /// Maximum acceptable data staleness in seconds before trading halts.
    pub max_data_staleness_seconds: u64,
    /// Whether to halt on market-wide circuit breakers.
    pub halt_on_market_breaker: bool,
    /// Minutes before a high-impact scheduled event to halt new positions.
    pub pre_event_halt_minutes: u32,
}

impl Default for TradingConstitution {
    fn default() -> Self {
        Self {
            max_portfolio_capital_micros: 100_000_000_000, // $100k
            max_gross_exposure_pct: 1.0,
            max_net_exposure_pct: 1.0,
            max_leverage: 1.0,
            max_single_position_pct: 0.25,
            max_sector_exposure_pct: 0.40,
            max_correlated_exposure_pct: 0.50,
            max_instrument_concentration_pct: 0.20,
            max_intraday_loss_pct: 0.03,
            max_drawdown_pct: 0.10,
            max_risk_per_trade_pct: 0.01,
            max_slippage_bps: 20.0,
            max_orders_per_second: 5,
            max_cancel_replace_rate: 10,
            approved_instruments: BTreeSet::new(),
            approved_venues: BTreeSet::new(),
            approved_sessions: BTreeSet::new(),
            short_sale_eligible: false,
            restricted_symbols: BTreeSet::new(),
            max_data_staleness_seconds: 300,
            halt_on_market_breaker: true,
            pre_event_halt_minutes: 15,
        }
    }
}

impl TradingConstitution {
    /// Validate the constitution for internal consistency.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_gross_exposure_pct <= 0.0 || self.max_gross_exposure_pct > 10.0 {
            return Err("max_gross_exposure must be in (0, 10]".into());
        }
        if self.max_leverage < 1.0 {
            return Err("max_leverage must be >= 1.0".into());
        }
        if self.max_risk_per_trade_pct <= 0.0 || self.max_risk_per_trade_pct > 0.05 {
            return Err("max_risk_per_trade must be in (0, 0.05]".into());
        }
        if self.max_drawdown_pct <= 0.0 || self.max_drawdown_pct > 0.50 {
            return Err("max_drawdown must be in (0, 0.50]".into());
        }
        if self.max_single_position_pct > self.max_gross_exposure_pct {
            return Err("single position cannot exceed gross exposure".into());
        }
        Ok(())
    }

    /// Whether an instrument is approved for trading.
    pub fn instrument_approved(&self, symbol: &str) -> bool {
        if self.restricted_symbols.contains(symbol) {
            return false;
        }
        if self.approved_instruments.is_empty() {
            return true; // empty = all allowed
        }
        self.approved_instruments.contains(symbol)
    }

    /// Whether a venue is approved.
    pub fn venue_approved(&self, venue: &str) -> bool {
        if self.approved_venues.is_empty() {
            return true;
        }
        self.approved_venues.contains(venue)
    }
}

/// Expanded action vocabulary (Fabric §14). The system must be good at doing
/// nothing; abstention is a first-class action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeAction {
    /// Open or increase a long position.
    Long,
    /// Open or increase a short position.
    Short,
    /// Reduce an existing position (partial close).
    Reduce,
    /// Close a position entirely.
    Exit,
    /// Add a hedge against an existing position.
    Hedge,
    /// Take no action but remain eligible to act.
    Wait,
    /// Actively decline to trade — expected edge too low, uncertainty too
    /// high, or regime incompatible (§14 abstention conditions).
    Abstain,
    /// Disable a specific strategy (scoped kill).
    DisableStrategy,
    /// Disable trading for a specific symbol (scoped kill).
    DisableSymbol,
    /// Disable trading for an entire market/asset class (scoped kill).
    DisableMarket,
}

impl TradeAction {
    /// Whether this action opens or increases risk.
    pub fn opens_risk(self) -> bool {
        matches!(self, TradeAction::Long | TradeAction::Short)
    }

    /// Whether this action reduces or closes risk.
    pub fn reduces_risk(self) -> bool {
        matches!(
            self,
            TradeAction::Reduce | TradeAction::Exit | TradeAction::Hedge
        )
    }

    /// Whether this action is a scoped kill (disables a strategy/symbol/market).
    pub fn is_scoped_kill(self) -> bool {
        matches!(
            self,
            TradeAction::DisableStrategy | TradeAction::DisableSymbol | TradeAction::DisableMarket
        )
    }

    /// Whether this action abstains from trading.
    pub fn is_abstention(self) -> bool {
        matches!(
            self,
            TradeAction::Wait
                | TradeAction::Abstain
                | TradeAction::DisableStrategy
                | TradeAction::DisableSymbol
                | TradeAction::DisableMarket
        )
    }

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            TradeAction::Long => "LONG",
            TradeAction::Short => "SHORT",
            TradeAction::Reduce => "REDUCE",
            TradeAction::Exit => "EXIT",
            TradeAction::Hedge => "HEDGE",
            TradeAction::Wait => "WAIT",
            TradeAction::Abstain => "ABSTAIN",
            TradeAction::DisableStrategy => "DISABLE_STRATEGY",
            TradeAction::DisableSymbol => "DISABLE_SYMBOL",
            TradeAction::DisableMarket => "DISABLE_MARKET",
        }
    }
}

/// Kill-switch scope (Fabric §83) — multiple scopes so a single broken
/// strategy doesn't require destroying the entire session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KillSwitchScope {
    /// Kill a specific strategy.
    Strategy,
    /// Kill a specific symbol.
    Symbol,
    /// Kill a specific venue.
    Venue,
    /// Kill an entire asset class.
    AssetClass,
    /// Kill a broker connection.
    Broker,
    /// Kill all execution.
    Execution,
    /// Flatten all positions.
    FlattenPortfolio,
    /// Global lockdown — halt everything.
    GlobalLockdown,
}

/// Operational mode (Fabric §79) — explicit system modes with auditable
/// transitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalMode {
    /// Offline — no data, no trading, research only.
    OfflineResearch,
    /// Historical replay — replaying past data through the system.
    HistoricalReplay,
    /// Simulation — synthetic data generation.
    Simulation,
    /// Shadow — live data, decisions recorded but no orders placed.
    Shadow,
    /// Paper — live data, simulated execution.
    Paper,
    /// Canary live — small real capital, strictly bounded.
    CanaryLive,
    /// Full live execution.
    Live,
    /// Restricted — reduced limits after a risk event.
    Restricted,
    /// Flatten-only — only position-closing orders permitted.
    FlattenOnly,
    /// Emergency stop — everything halted.
    EmergencyStop,
}

impl OperationalMode {
    /// Whether this mode permits any order submission.
    pub fn permits_orders(self) -> bool {
        matches!(
            self,
            OperationalMode::Paper
                | OperationalMode::CanaryLive
                | OperationalMode::Live
                | OperationalMode::Restricted
                | OperationalMode::FlattenOnly
        )
    }

    /// Whether this mode uses real (non-simulated) capital.
    pub fn is_live_capital(self) -> bool {
        matches!(self, OperationalMode::CanaryLive | OperationalMode::Live)
    }

    /// Whether this mode processes live market data.
    pub fn uses_live_data(self) -> bool {
        matches!(
            self,
            OperationalMode::Shadow
                | OperationalMode::Paper
                | OperationalMode::CanaryLive
                | OperationalMode::Live
                | OperationalMode::Restricted
                | OperationalMode::FlattenOnly
        )
    }

    /// Whether only flatten (close) orders are permitted.
    pub fn is_flatten_only(self) -> bool {
        matches!(self, OperationalMode::FlattenOnly)
    }

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            OperationalMode::OfflineResearch => "OFFLINE RESEARCH",
            OperationalMode::HistoricalReplay => "HISTORICAL REPLAY",
            OperationalMode::Simulation => "SIMULATION",
            OperationalMode::Shadow => "SHADOW",
            OperationalMode::Paper => "PAPER",
            OperationalMode::CanaryLive => "CANARY LIVE",
            OperationalMode::Live => "LIVE",
            OperationalMode::Restricted => "RESTRICTED",
            OperationalMode::FlattenOnly => "FLATTEN-ONLY",
            OperationalMode::EmergencyStop => "EMERGENCY STOP",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_constitution_validates() {
        assert!(TradingConstitution::default().validate().is_ok());
    }

    #[test]
    fn constitution_rejects_excessive_risk_per_trade() {
        let mut c = TradingConstitution::default();
        c.max_risk_per_trade_pct = 0.10; // 10% — too high
        assert!(c.validate().is_err());
    }

    #[test]
    fn instrument_approved_respects_allowlist() {
        let mut c = TradingConstitution::default();
        // Empty = all allowed
        assert!(c.instrument_approved("BTC"));
        assert!(c.instrument_approved("AAPL"));
        // Restricted
        c.restricted_symbols.insert("SCAM".into());
        assert!(!c.instrument_approved("SCAM"));
        // Allowlist
        c.approved_instruments.insert("BTC".into());
        c.approved_instruments.insert("ETH".into());
        assert!(c.instrument_approved("BTC"));
        assert!(!c.instrument_approved("AAPL")); // not in allowlist
    }

    #[test]
    fn action_opens_risk_classification() {
        assert!(TradeAction::Long.opens_risk());
        assert!(TradeAction::Short.opens_risk());
        assert!(!TradeAction::Exit.opens_risk());
        assert!(!TradeAction::Abstain.opens_risk());
    }

    #[test]
    fn action_abstention_classification() {
        assert!(TradeAction::Abstain.is_abstention());
        assert!(TradeAction::Wait.is_abstention());
        assert!(TradeAction::DisableStrategy.is_abstention());
        assert!(!TradeAction::Long.is_abstention());
    }

    #[test]
    fn action_scoped_kill_classification() {
        assert!(TradeAction::DisableStrategy.is_scoped_kill());
        assert!(TradeAction::DisableSymbol.is_scoped_kill());
        assert!(TradeAction::DisableMarket.is_scoped_kill());
        assert!(!TradeAction::Exit.is_scoped_kill());
    }

    #[test]
    fn operational_mode_paper_permits_orders_not_live_capital() {
        assert!(OperationalMode::Paper.permits_orders());
        assert!(!OperationalMode::Paper.is_live_capital());
        assert!(OperationalMode::Paper.uses_live_data());
    }

    #[test]
    fn operational_mode_live_is_live_capital() {
        assert!(OperationalMode::Live.is_live_capital());
        assert!(OperationalMode::Live.permits_orders());
    }

    #[test]
    fn operational_mode_emergency_stops_everything() {
        assert!(!OperationalMode::EmergencyStop.permits_orders());
        assert!(!OperationalMode::EmergencyStop.is_live_capital());
    }

    #[test]
    fn operational_mode_flatten_only_restricts_to_closing() {
        assert!(OperationalMode::FlattenOnly.permits_orders());
        assert!(OperationalMode::FlattenOnly.is_flatten_only());
        assert!(!OperationalMode::FlattenOnly.is_live_capital());
    }

    #[test]
    fn operational_mode_shadow_no_orders() {
        assert!(!OperationalMode::Shadow.permits_orders());
        assert!(OperationalMode::Shadow.uses_live_data());
    }

    #[test]
    fn kill_switch_scope_enum_is_exhaustive() {
        // All variants should be constructible.
        let scopes = [
            KillSwitchScope::Strategy,
            KillSwitchScope::Symbol,
            KillSwitchScope::Venue,
            KillSwitchScope::AssetClass,
            KillSwitchScope::Broker,
            KillSwitchScope::Execution,
            KillSwitchScope::FlattenPortfolio,
            KillSwitchScope::GlobalLockdown,
        ];
        assert_eq!(scopes.len(), 8);
    }
}
