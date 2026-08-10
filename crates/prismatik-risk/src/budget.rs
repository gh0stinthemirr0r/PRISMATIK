//! Position sizing under risk budgets and the drawdown circuit breaker.
//!
//! Two load-bearing rules from the operator directive:
//!
//! 1. **1% max risk per trade** — no single trade may risk more than 1% of
//!    current equity, where "risk" is defined as `quantity * stop_distance`.
//!    This is the fractional-Kelly / R-multiple discipline that survives long
//!    losing streaks.
//! 2. **10% drawdown kill switch** — if equity falls 10% below its running
//!    peak, the system closes everything and halts until a human reviews and
//!    explicitly re-arms. This is invariant I6 (fail-closed on degradation)
//!    applied to capital, not just data staleness.
//!
//! Both are pure functions with no ambient state — the caller threads the
//! equity history and peak. Determinism (I3) is preserved by construction.

use serde::{Deserialize, Serialize};

/// Default max risk per trade (1% of equity).
pub const DEFAULT_MAX_RISK_PER_TRADE_PCT: f64 = 0.01;
/// Default drawdown halt threshold (10% from peak).
pub const DEFAULT_DRAWDOWN_HALT_PCT: f64 = 0.10;

/// Risk policy governing per-trade risk and portfolio drawdown limits.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RiskBudget {
    /// Maximum fraction of equity risked per trade (0.01 = 1%).
    pub max_risk_per_trade_pct: f64,
    /// Drawdown fraction from peak equity that triggers the kill switch
    /// (0.10 = 10%).
    pub drawdown_halt_pct: f64,
}

impl Default for RiskBudget {
    fn default() -> Self {
        Self {
            max_risk_per_trade_pct: DEFAULT_MAX_RISK_PER_TRADE_PCT,
            drawdown_halt_pct: DEFAULT_DRAWDOWN_HALT_PCT,
        }
    }
}

/// Result of a position-sizing calculation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PositionSize {
    /// Recommended quantity (in units of the asset).
    pub quantity: f64,
    /// Dollar risk this position represents (quantity * stop_distance).
    pub risk_amount: f64,
    /// Risk as a fraction of equity.
    pub risk_pct: f64,
    /// Whether this size is within the per-trade risk budget.
    pub within_budget: bool,
}

/// Compute position size so that the risk (quantity × stop distance) equals
/// at most `max_risk_per_trade_pct` of equity.
///
/// - `equity`: current account equity (cash + position market value).
/// - `entry_price`: expected fill price.
/// - `stop_price`: stop-loss price where the trade is exited at a loss.
/// - `budget`: the risk policy.
///
/// Returns the maximum quantity that respects the risk budget. If
/// `entry_price == stop_price` (no stop), returns quantity 0 — we never
/// size a trade without a defined risk.
pub fn size_by_risk(
    equity: f64,
    entry_price: f64,
    stop_price: f64,
    budget: &RiskBudget,
) -> PositionSize {
    if !equity.is_finite() || equity <= 0.0 || !entry_price.is_finite() || entry_price <= 0.0 {
        return PositionSize {
            quantity: 0.0,
            risk_amount: 0.0,
            risk_pct: 0.0,
            within_budget: false,
        };
    }
    let stop_distance = (entry_price - stop_price).abs();
    if stop_distance == 0.0 || !stop_distance.is_finite() {
        // No stop = undefined risk = no trade. This is a hard rule.
        return PositionSize {
            quantity: 0.0,
            risk_amount: 0.0,
            risk_pct: 0.0,
            within_budget: false,
        };
    }
    let max_risk_amount = equity * budget.max_risk_per_trade_pct;
    let quantity = max_risk_amount / stop_distance;
    let risk_amount = quantity * stop_distance;
    let risk_pct = risk_amount / equity;
    PositionSize {
        quantity,
        risk_amount,
        risk_pct,
        within_budget: risk_pct <= budget.max_risk_per_trade_pct + 1e-12,
    }
}

/// Circuit-breaker state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    /// Armed: trading is permitted.
    Armed,
    /// Tripped: drawdown exceeded the limit; all positions must be closed and
    /// no new trades permitted until a human re-arms.
    Tripped,
}

/// Circuit breaker evaluating current drawdown against the halt threshold.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CircuitBreaker {
    /// Running peak equity observed.
    pub peak_equity: f64,
    /// Current state.
    pub state: CircuitBreakerState,
    /// Drawdown fraction that triggered the trip (None if armed).
    pub tripped_drawdown: Option<f64>,
    /// Configured halt threshold.
    pub drawdown_halt_pct: f64,
}

impl CircuitBreaker {
    /// Construct an armed breaker at the given starting equity (the initial
    /// peak).
    pub fn armed(starting_equity: f64, budget: &RiskBudget) -> Self {
        Self {
            peak_equity: starting_equity.max(0.0),
            state: CircuitBreakerState::Armed,
            tripped_drawdown: None,
            drawdown_halt_pct: budget.drawdown_halt_pct,
        }
    }

    /// Update the breaker with a new equity reading. If armed and drawdown
    /// exceeds the threshold, trips. Once tripped, stays tripped regardless
    /// of subsequent equity (only [`rearm`] can clear it).
    pub fn update(&mut self, current_equity: f64) {
        if self.state == CircuitBreakerState::Tripped {
            return; // latched — stays tripped until human re-arm
        }
        if !current_equity.is_finite() {
            return;
        }
        if current_equity > self.peak_equity {
            self.peak_equity = current_equity;
            return;
        }
        if self.peak_equity > 0.0 {
            let drawdown = (self.peak_equity - current_equity) / self.peak_equity;
            if drawdown >= self.drawdown_halt_pct {
                self.state = CircuitBreakerState::Tripped;
                self.tripped_drawdown = Some(drawdown);
            }
        }
    }

    /// Current drawdown fraction from peak (0.0 if at or above peak).
    pub fn current_drawdown(&self, current_equity: f64) -> f64 {
        if self.peak_equity <= 0.0 {
            return 0.0;
        }
        ((self.peak_equity - current_equity) / self.peak_equity).max(0.0)
    }

    /// Whether new trades are permitted right now.
    pub fn trading_permitted(&self) -> bool {
        self.state == CircuitBreakerState::Armed
    }

    /// Human-initiated re-arm. Requires a new starting equity and resets the
    /// peak. This is the only way to clear a tripped state — there is no
    /// automatic recovery, by design.
    pub fn rearm(&mut self, current_equity: f64) {
        self.state = CircuitBreakerState::Armed;
        self.tripped_drawdown = None;
        self.peak_equity = current_equity.max(0.0);
    }
}

/// Evaluate a proposed trade against the full risk gate: circuit breaker +
/// per-trade risk budget. Returns the permitted quantity (which may be 0)
/// and an optional denial reason.
pub fn evaluate_trade(
    breaker: &CircuitBreaker,
    equity: f64,
    entry_price: f64,
    stop_price: f64,
    budget: &RiskBudget,
) -> (f64, Option<&'static str>) {
    if !breaker.trading_permitted() {
        return (0.0, Some("circuit_breaker_tripped"));
    }
    let size = size_by_risk(equity, entry_price, stop_price, budget);
    if size.quantity <= 0.0 {
        return (0.0, Some("no_defined_risk"));
    }
    if !size.within_budget {
        return (0.0, Some("exceeds_risk_budget"));
    }
    (size.quantity, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_percent_rule_sizes_correctly() {
        // $100k equity, entry $100, stop $95 → risk $5/share.
        // 1% of $100k = $1000 max risk → quantity = 1000/5 = 200 shares.
        let budget = RiskBudget::default();
        let size = size_by_risk(100_000.0, 100.0, 95.0, &budget);
        assert!(
            (size.quantity - 200.0).abs() < 1e-6,
            "expected 200, got {}",
            size.quantity
        );
        assert!((size.risk_amount - 1000.0).abs() < 1e-6);
        assert!((size.risk_pct - 0.01).abs() < 1e-9);
        assert!(size.within_budget);
    }

    #[test]
    fn tight_stop_allows_more_shares() {
        let budget = RiskBudget::default();
        // $100k, entry $100, stop $99 → $1 risk/share → 1000 shares.
        let size = size_by_risk(100_000.0, 100.0, 99.0, &budget);
        assert!((size.quantity - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn no_stop_means_no_trade() {
        let budget = RiskBudget::default();
        let size = size_by_risk(100_000.0, 100.0, 100.0, &budget);
        assert_eq!(size.quantity, 0.0);
        assert!(!size.within_budget);
    }

    #[test]
    fn short_stop_works() {
        // Short: entry $100, stop $105 → $5 risk/share.
        let budget = RiskBudget::default();
        let size = size_by_risk(100_000.0, 100.0, 105.0, &budget);
        assert!((size.quantity - 200.0).abs() < 1e-6);
    }

    #[test]
    fn breaker_trips_at_ten_percent_drawdown() {
        let budget = RiskBudget::default();
        let mut br = CircuitBreaker::armed(100_000.0, &budget);
        assert_eq!(br.state, CircuitBreakerState::Armed);
        assert!(br.trading_permitted());
        // Equity rises to 110k — new peak.
        br.update(110_000.0);
        assert_eq!(br.peak_equity, 110_000.0);
        // Drops to 100k — 9.09% drawdown, not tripped.
        br.update(100_000.0);
        assert_eq!(br.state, CircuitBreakerState::Armed);
        // Drops to 98k — 10.9% drawdown, trips.
        br.update(98_000.0);
        assert_eq!(br.state, CircuitBreakerState::Tripped);
        assert!(!br.trading_permitted());
        let dd = br.tripped_drawdown.unwrap();
        assert!(dd >= 0.10, "tripped drawdown {dd} should be >= 0.10");
    }

    #[test]
    fn breaker_latches_after_trip() {
        let budget = RiskBudget::default();
        let mut br = CircuitBreaker::armed(100_000.0, &budget);
        br.update(89_000.0); // 11% drawdown → trip
        assert_eq!(br.state, CircuitBreakerState::Tripped);
        // Even if equity recovers, stays tripped.
        br.update(150_000.0);
        assert_eq!(br.state, CircuitBreakerState::Tripped);
        assert!(!br.trading_permitted());
    }

    #[test]
    fn breaker_rearm_requires_human_action() {
        let budget = RiskBudget::default();
        let mut br = CircuitBreaker::armed(100_000.0, &budget);
        br.update(85_000.0); // 15% drawdown → trip
        assert_eq!(br.state, CircuitBreakerState::Tripped);
        br.rearm(90_000.0);
        assert_eq!(br.state, CircuitBreakerState::Armed);
        assert_eq!(br.peak_equity, 90_000.0);
        assert!(br.trading_permitted());
        assert!(br.tripped_drawdown.is_none());
    }

    #[test]
    fn breaker_tracks_peak_correctly() {
        let budget = RiskBudget::default();
        let mut br = CircuitBreaker::armed(100_000.0, &budget);
        br.update(105_000.0);
        br.update(102_000.0); // dip but new high-water mark is 105k
        br.update(108_000.0); // new peak
        assert_eq!(br.peak_equity, 108_000.0);
        assert_eq!(br.state, CircuitBreakerState::Armed);
        // 10.2% drawdown from 108k → trip
        br.update(97_000.0);
        assert_eq!(br.state, CircuitBreakerState::Tripped);
    }

    #[test]
    fn evaluate_trade_denies_when_breaker_tripped() {
        let budget = RiskBudget::default();
        let mut br = CircuitBreaker::armed(100_000.0, &budget);
        br.update(85_000.0); // trip
        let (qty, reason) = evaluate_trade(&br, 85_000.0, 100.0, 95.0, &budget);
        assert_eq!(qty, 0.0);
        assert_eq!(reason, Some("circuit_breaker_tripped"));
    }

    #[test]
    fn evaluate_trade_denies_without_stop() {
        let budget = RiskBudget::default();
        let br = CircuitBreaker::armed(100_000.0, &budget);
        let (qty, reason) = evaluate_trade(&br, 100_000.0, 100.0, 100.0, &budget);
        assert_eq!(qty, 0.0);
        assert_eq!(reason, Some("no_defined_risk"));
    }

    #[test]
    fn evaluate_trade_permits_sized_quantity() {
        let budget = RiskBudget::default();
        let br = CircuitBreaker::armed(100_000.0, &budget);
        let (qty, reason) = evaluate_trade(&br, 100_000.0, 100.0, 95.0, &budget);
        assert!((qty - 200.0).abs() < 1e-6);
        assert_eq!(reason, None);
    }

    #[test]
    fn default_budget_matches_directive() {
        let budget = RiskBudget::default();
        assert!((budget.max_risk_per_trade_pct - 0.01).abs() < 1e-12);
        assert!((budget.drawdown_halt_pct - 0.10).abs() < 1e-12);
    }

    #[test]
    fn custom_budget_respected() {
        // Aggressive: 2% risk, 20% drawdown halt.
        let budget = RiskBudget {
            max_risk_per_trade_pct: 0.02,
            drawdown_halt_pct: 0.20,
        };
        let size = size_by_risk(100_000.0, 100.0, 95.0, &budget);
        // 2% of $100k = $2000 / $5 = 400 shares
        assert!((size.quantity - 400.0).abs() < 1e-6);
        let mut br = CircuitBreaker::armed(100_000.0, &budget);
        br.update(85_000.0); // 15% drawdown — NOT tripped at 20% threshold
        assert_eq!(br.state, CircuitBreakerState::Armed);
        br.update(79_000.0); // 21% → trip
        assert_eq!(br.state, CircuitBreakerState::Tripped);
    }

    #[test]
    fn negative_or_zero_equity_denies() {
        let budget = RiskBudget::default();
        let size = size_by_risk(-100.0, 100.0, 95.0, &budget);
        assert_eq!(size.quantity, 0.0);
        let size = size_by_risk(0.0, 100.0, 95.0, &budget);
        assert_eq!(size.quantity, 0.0);
    }
}
