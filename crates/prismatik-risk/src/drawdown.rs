//! Hierarchical drawdown ladder — graded de-risking per the Trading
//! Intelligence Fabric canon §20.
//!
//! The canon explicitly warns: *"Do not implement only a binary daily-loss
//! switch. Build hierarchical drawdown states: NORMAL → CAUTION → DE-RISK →
//! RESTRICT → FLATTEN → LOCKDOWN."*
//!
//! This replaces the binary `CircuitBreaker` with a graded system. Each level
//! triggers at a progressively deeper drawdown and imposes progressively
//! stricter constraints on trading authority. The binary `Tripped` state of
//! the legacy breaker maps to `Lockdown` (the final, most severe level).
//!
//! ## Design
//!
//! - **Normal** (< caution threshold): full trading authority.
//! - **Caution**: new positions require higher conviction; sizing reduced.
//! - **De-risk**: position sizing halved; no new strategies activated.
//! - **Restrict**: no new positions; existing positions may only be reduced.
//! - **Flatten**: all positions closed to zero; no new orders.
//! - **Lockdown**: all automation halted; human re-arm required (latched).
//!
//! Transitions are monotonic downward (drawdown deepens) until the operator
//! re-arms. Recovery does NOT auto-de-escalate — each level above the current
//! requires explicit operator confirmation (§41: "Promotion is based on
//! evidence, not elapsed time").

use serde::{Deserialize, Serialize};

/// Drawdown ladder level, ordered from least to most severe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DrawdownLevel {
    /// Full trading authority. Drawdown is within normal range.
    Normal,
    /// Elevated drawdown — new positions require higher conviction; sizing
    /// reduced (e.g. to 50% of risk budget).
    Caution,
    /// Significant drawdown — sizing halved; no new strategies activated.
    DeRisk,
    /// Severe drawdown — no new positions; existing positions may only be
    /// reduced (sells to close permitted, no buys to open).
    Restrict,
    /// Critical drawdown — all positions flattened to zero; no new orders.
    Flatten,
    /// Maximum drawdown — all automation halted; requires human re-arm.
    /// Latched: stays in Lockdown regardless of equity recovery.
    Lockdown,
}

impl DrawdownLevel {
    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            DrawdownLevel::Normal => "NORMAL",
            DrawdownLevel::Caution => "CAUTION",
            DrawdownLevel::DeRisk => "DE-RISK",
            DrawdownLevel::Restrict => "RESTRICT",
            DrawdownLevel::Flatten => "FLATTEN",
            DrawdownLevel::Lockdown => "LOCKDOWN",
        }
    }

    /// Whether new positions (buys-to-open) are permitted at this level.
    pub fn permits_new_positions(self) -> bool {
        matches!(
            self,
            DrawdownLevel::Normal | DrawdownLevel::Caution | DrawdownLevel::DeRisk
        )
    }

    /// Whether existing positions may be reduced (sells-to-close) at this
    /// level. Always true except in Lockdown where ALL automation halts.
    pub fn permits_position_reduction(self) -> bool {
        !matches!(self, DrawdownLevel::Lockdown)
    }

    /// Sizing multiplier applied to the risk budget at this level. Normal =
    /// 1.0 (full risk budget); deeper levels reduce it.
    pub fn sizing_multiplier(self) -> f64 {
        match self {
            DrawdownLevel::Normal => 1.0,
            DrawdownLevel::Caution => 0.5,
            DrawdownLevel::DeRisk => 0.25,
            DrawdownLevel::Restrict => 0.0, // no new sizing
            DrawdownLevel::Flatten => 0.0,
            DrawdownLevel::Lockdown => 0.0,
        }
    }

    /// Whether automation (strategy execution, agent loops) is permitted.
    pub fn permits_automation(self) -> bool {
        !matches!(self, DrawdownLevel::Lockdown)
    }

    /// Whether this level is latched (requires human re-arm to clear).
    pub fn is_latched(self) -> bool {
        matches!(self, DrawdownLevel::Lockdown)
    }

    /// Map from the legacy binary `CircuitBreakerState` for backward
    /// compatibility. `Armed → Normal`, `Tripped → Lockdown`.
    pub fn from_binary(tripped: bool) -> Self {
        if tripped {
            DrawdownLevel::Lockdown
        } else {
            DrawdownLevel::Normal
        }
    }
}

/// Thresholds for each drawdown level, expressed as fractions of peak equity
/// lost. All thresholds must be in (0, 1] and strictly increasing.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawdownThresholds {
    /// Caution threshold (e.g. 0.03 = 3% drawdown).
    pub caution: f64,
    /// De-risk threshold (e.g. 0.05).
    pub de_risk: f64,
    /// Restrict threshold (e.g. 0.07).
    pub restrict: f64,
    /// Flatten threshold (e.g. 0.09).
    pub flatten: f64,
    /// Lockdown threshold (e.g. 0.10 = 10%).
    pub lockdown: f64,
}

impl Default for DrawdownThresholds {
    fn default() -> Self {
        // Conservative defaults that escalate before the 10% hard halt.
        Self {
            caution: 0.03,
            de_risk: 0.05,
            restrict: 0.07,
            flatten: 0.09,
            lockdown: 0.10,
        }
    }
}

impl DrawdownThresholds {
    /// Validate that thresholds are strictly increasing and in valid range.
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.caution)
            || !(0.0..=1.0).contains(&self.lockdown)
            || self.caution >= self.de_risk
            || self.de_risk >= self.restrict
            || self.restrict >= self.flatten
            || self.flatten >= self.lockdown
        {
            return Err(format!(
                "drawdown thresholds must be strictly increasing in (0,1]: \
                 caution={}, de_risk={}, restrict={}, flatten={}, lockdown={}",
                self.caution, self.de_risk, self.restrict, self.flatten, self.lockdown
            ));
        }
        Ok(())
    }

    /// Determine the drawdown level for a given drawdown fraction.
    pub fn level_for_drawdown(&self, drawdown: f64) -> DrawdownLevel {
        if drawdown >= self.lockdown {
            DrawdownLevel::Lockdown
        } else if drawdown >= self.flatten {
            DrawdownLevel::Flatten
        } else if drawdown >= self.restrict {
            DrawdownLevel::Restrict
        } else if drawdown >= self.de_risk {
            DrawdownLevel::DeRisk
        } else if drawdown >= self.caution {
            DrawdownLevel::Caution
        } else {
            DrawdownLevel::Normal
        }
    }
}

/// Hierarchical drawdown ladder with peak tracking and latched lockdown.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawdownLadder {
    /// Running peak equity.
    pub peak_equity: f64,
    /// Current level.
    pub level: DrawdownLevel,
    /// Drawdown fraction at which the current level was triggered.
    pub triggered_drawdown: Option<f64>,
    /// Configured thresholds.
    pub thresholds: DrawdownThresholds,
}

impl DrawdownLadder {
    /// Construct a ladder at Normal level with the given starting equity.
    pub fn new(starting_equity: f64, thresholds: DrawdownThresholds) -> Self {
        Self {
            peak_equity: starting_equity.max(0.0),
            level: DrawdownLevel::Normal,
            triggered_drawdown: None,
            thresholds,
        }
    }

    /// Construct with default thresholds.
    pub fn with_defaults(starting_equity: f64) -> Self {
        Self::new(starting_equity, DrawdownThresholds::default())
    }

    /// Update the ladder with a new equity reading. Escalates the level
    /// if drawdown deepens past the next threshold. Once in Lockdown, stays
    /// latched regardless of equity.
    pub fn update(&mut self, current_equity: f64) {
        // Lockdown is latched — never auto-de-escalate.
        if self.level == DrawdownLevel::Lockdown {
            return;
        }
        if !current_equity.is_finite() {
            return;
        }
        if current_equity > self.peak_equity {
            self.peak_equity = current_equity;
            // We do NOT auto-de-escalate even on new highs. The canon §41:
            // "Promotion is based on evidence, not elapsed time." Re-arming
            // to a less-restrictive level requires explicit operator action.
            return;
        }
        if self.peak_equity > 0.0 {
            let drawdown = (self.peak_equity - current_equity) / self.peak_equity;
            let target = self.thresholds.level_for_drawdown(drawdown);
            // Only escalate (move to more severe levels), never de-escalate.
            if target > self.level {
                self.level = target;
                self.triggered_drawdown = Some(drawdown);
            }
        }
    }

    /// Current drawdown fraction from peak.
    pub fn current_drawdown(&self, current_equity: f64) -> f64 {
        if self.peak_equity <= 0.0 {
            return 0.0;
        }
        ((self.peak_equity - current_equity) / self.peak_equity).max(0.0)
    }

    /// Whether new positions are permitted.
    pub fn permits_new_positions(&self) -> bool {
        self.level.permits_new_positions()
    }

    /// Whether automation (strategy/agent loops) is permitted.
    pub fn permits_automation(&self) -> bool {
        self.level.permits_automation()
    }

    /// Effective risk-budget multiplier given the current level.
    pub fn sizing_multiplier(&self) -> f64 {
        self.level.sizing_multiplier()
    }

    /// Human re-arm: clears Lockdown and resets to Normal at the current
    /// equity. This is the ONLY way to exit Lockdown. Also resets the peak
    /// so the operator's re-entry point is the new high-water mark.
    pub fn rearm(&mut self, current_equity: f64) {
        self.level = DrawdownLevel::Normal;
        self.triggered_drawdown = None;
        self.peak_equity = current_equity.max(0.0);
    }

    /// Operator-initiated de-escalation by one level (e.g. Lockdown → Flatten,
    /// or Restrict → DeRisk). Requires evidence that the drawdown has
    /// stabilized. Does NOT clear Lockdown's latch unless the operator
    /// explicitly calls [`rearm`].
    pub fn de_escalate(&mut self) {
        self.level = match self.level {
            DrawdownLevel::Lockdown => DrawdownLevel::Lockdown, // must rearm
            DrawdownLevel::Flatten => DrawdownLevel::Restrict,
            DrawdownLevel::Restrict => DrawdownLevel::DeRisk,
            DrawdownLevel::DeRisk => DrawdownLevel::Caution,
            DrawdownLevel::Caution => DrawdownLevel::Normal,
            DrawdownLevel::Normal => DrawdownLevel::Normal,
        };
        self.triggered_drawdown = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ordering_is_monotonic() {
        assert!(DrawdownLevel::Normal < DrawdownLevel::Caution);
        assert!(DrawdownLevel::Caution < DrawdownLevel::DeRisk);
        assert!(DrawdownLevel::DeRisk < DrawdownLevel::Restrict);
        assert!(DrawdownLevel::Restrict < DrawdownLevel::Flatten);
        assert!(DrawdownLevel::Flatten < DrawdownLevel::Lockdown);
    }

    #[test]
    fn thresholds_default_validate() {
        assert!(DrawdownThresholds::default().validate().is_ok());
    }

    #[test]
    fn thresholds_reject_non_increasing() {
        let bad = DrawdownThresholds {
            caution: 0.05,
            de_risk: 0.03, // < caution
            restrict: 0.07,
            flatten: 0.09,
            lockdown: 0.10,
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn level_for_drawdown_maps_correctly() {
        let t = DrawdownThresholds::default();
        assert_eq!(t.level_for_drawdown(0.01), DrawdownLevel::Normal);
        assert_eq!(t.level_for_drawdown(0.03), DrawdownLevel::Caution);
        assert_eq!(t.level_for_drawdown(0.05), DrawdownLevel::DeRisk);
        assert_eq!(t.level_for_drawdown(0.07), DrawdownLevel::Restrict);
        assert_eq!(t.level_for_drawdown(0.09), DrawdownLevel::Flatten);
        assert_eq!(t.level_for_drawdown(0.10), DrawdownLevel::Lockdown);
        assert_eq!(t.level_for_drawdown(0.25), DrawdownLevel::Lockdown);
    }

    #[test]
    fn ladder_escalates_through_all_levels() {
        let mut ladder = DrawdownLadder::with_defaults(100_000.0);
        ladder.update(96_000.0); // 4% → Caution
        assert_eq!(ladder.level, DrawdownLevel::Caution);
        ladder.update(94_000.0); // 6% → DeRisk
        assert_eq!(ladder.level, DrawdownLevel::DeRisk);
        ladder.update(92_000.0); // 8% → Restrict
        assert_eq!(ladder.level, DrawdownLevel::Restrict);
        ladder.update(90_500.0); // 9.5% → Flatten
        assert_eq!(ladder.level, DrawdownLevel::Flatten);
        ladder.update(89_000.0); // 11% → Lockdown
        assert_eq!(ladder.level, DrawdownLevel::Lockdown);
    }

    #[test]
    fn lockdown_is_latched_through_recovery() {
        let mut ladder = DrawdownLadder::with_defaults(100_000.0);
        ladder.update(85_000.0); // 15% → Lockdown
        assert_eq!(ladder.level, DrawdownLevel::Lockdown);
        ladder.update(200_000.0); // equity doubles
        assert_eq!(
            ladder.level,
            DrawdownLevel::Lockdown,
            "Lockdown must latch through recovery"
        );
    }

    #[test]
    fn rearm_clears_lockdown() {
        let mut ladder = DrawdownLadder::with_defaults(100_000.0);
        ladder.update(80_000.0); // 20% → Lockdown
        assert_eq!(ladder.level, DrawdownLevel::Lockdown);
        ladder.rearm(90_000.0);
        assert_eq!(ladder.level, DrawdownLevel::Normal);
        assert_eq!(ladder.peak_equity, 90_000.0);
    }

    #[test]
    fn de_escalate_drops_one_level() {
        let mut ladder = DrawdownLadder::with_defaults(100_000.0);
        ladder.update(92_000.0); // 8% → Restrict
        assert_eq!(ladder.level, DrawdownLevel::Restrict);
        ladder.de_escalate();
        assert_eq!(ladder.level, DrawdownLevel::DeRisk);
        ladder.de_escalate();
        assert_eq!(ladder.level, DrawdownLevel::Caution);
        ladder.de_escalate();
        assert_eq!(ladder.level, DrawdownLevel::Normal);
        // Can't go below Normal
        ladder.de_escalate();
        assert_eq!(ladder.level, DrawdownLevel::Normal);
    }

    #[test]
    fn de_escalate_does_not_clear_lockdown() {
        let mut ladder = DrawdownLadder::with_defaults(100_000.0);
        ladder.update(80_000.0); // Lockdown
        ladder.de_escalate();
        assert_eq!(
            ladder.level,
            DrawdownLevel::Lockdown,
            "must rearm from Lockdown"
        );
    }

    #[test]
    fn no_auto_de_escalation_on_new_high() {
        let mut ladder = DrawdownLadder::with_defaults(100_000.0);
        ladder.update(96_000.0); // 4% → Caution
        assert_eq!(ladder.level, DrawdownLevel::Caution);
        ladder.update(110_000.0); // new peak
        assert_eq!(
            ladder.level,
            DrawdownLevel::Caution,
            "no auto de-escalation even on new high — requires operator action"
        );
    }

    #[test]
    fn sizing_multiplier_decreases_with_level() {
        assert_eq!(DrawdownLevel::Normal.sizing_multiplier(), 1.0);
        assert_eq!(DrawdownLevel::Caution.sizing_multiplier(), 0.5);
        assert_eq!(DrawdownLevel::DeRisk.sizing_multiplier(), 0.25);
        assert_eq!(DrawdownLevel::Restrict.sizing_multiplier(), 0.0);
        assert_eq!(DrawdownLevel::Flatten.sizing_multiplier(), 0.0);
        assert_eq!(DrawdownLevel::Lockdown.sizing_multiplier(), 0.0);
    }

    #[test]
    fn permits_new_positions_only_at_safe_levels() {
        assert!(DrawdownLevel::Normal.permits_new_positions());
        assert!(DrawdownLevel::Caution.permits_new_positions());
        assert!(DrawdownLevel::DeRisk.permits_new_positions());
        assert!(!DrawdownLevel::Restrict.permits_new_positions());
        assert!(!DrawdownLevel::Flatten.permits_new_positions());
        assert!(!DrawdownLevel::Lockdown.permits_new_positions());
    }

    #[test]
    fn from_binary_maps_legacy_states() {
        assert_eq!(DrawdownLevel::from_binary(false), DrawdownLevel::Normal);
        assert_eq!(DrawdownLevel::from_binary(true), DrawdownLevel::Lockdown);
    }

    #[test]
    fn permits_automation_except_lockdown() {
        assert!(DrawdownLevel::Normal.permits_automation());
        assert!(DrawdownLevel::Caution.permits_automation());
        assert!(DrawdownLevel::DeRisk.permits_automation());
        assert!(DrawdownLevel::Restrict.permits_automation());
        assert!(DrawdownLevel::Flatten.permits_automation());
        assert!(!DrawdownLevel::Lockdown.permits_automation());
    }

    #[test]
    fn peak_tracks_correctly() {
        let mut ladder = DrawdownLadder::with_defaults(100_000.0);
        ladder.update(105_000.0);
        ladder.update(102_000.0);
        ladder.update(108_000.0); // new peak
        assert_eq!(ladder.peak_equity, 108_000.0);
        assert_eq!(ladder.level, DrawdownLevel::Normal);
    }
}
