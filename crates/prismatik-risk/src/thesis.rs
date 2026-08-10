//! Trade Thesis Contract — the typed intermediary between analysis and
//! execution (Trading Intelligence Fabric §11).
//!
//! The canon is explicit: *"Do not allow an analyst agent to communicate with
//! execution using prose. Create a typed intermediary."*
//!
//! A `TradeThesis` is what the Portfolio Engine consumes. The broker never
//! sees it. It carries: instrument, direction, strategy, regime, confidence,
//! uncertainty, expected edge/cost, horizon, entry zone, invalidation level,
//! target logic, max adverse excursion, evidence, counter-evidence, preferred
//! execution style, and expiry. This makes every proposed trade inspectable,
//! testable, and reproducible.
//!
//! Combined with the signal TTL (§9), a thesis that has expired or whose
//! invalidation condition has been hit is structurally barred from becoming
//! an order intent.

use serde::{Deserialize, Serialize};

/// Trade direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThesisDirection {
    /// Long.
    Long,
    /// Short.
    Short,
}

/// Execution style preference (§25 execution strategies).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStyle {
    /// Marketable limit — take liquidity with a price cap.
    MarketableLimit,
    /// Passive limit — provide liquidity at the bid/ask.
    PassiveLimit,
    /// Time-weighted average price — slice over time.
    Twap,
    /// Volume-weighted average price — slice following volume profile.
    Vwap,
}

/// Evidence reference — resolves to a governed observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Evidence identifier (resolves through the evidence store).
    pub evidence_id: String,
    /// Human-readable summary of what this evidence supports.
    pub summary: String,
    /// Whether this evidence supports or contradicts the thesis.
    pub stance: EvidenceStance,
}

/// Whether a piece of evidence supports or contradicts the thesis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStance {
    /// Supports the thesis.
    Supporting,
    /// Contradicts the thesis (adversarial evidence must be carried).
    Contradicting,
}

/// A typed trade thesis — the only thing the Portfolio Engine accepts from
/// the analysis layer. No prose-only communication reaches execution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TradeThesis {
    /// Unique thesis identifier.
    pub thesis_id: String,
    /// Canonical instrument identifier.
    pub instrument: String,
    /// Direction.
    pub direction: ThesisDirection,
    /// Source strategy identifier.
    pub strategy: String,
    /// Market regime under which this thesis was formed.
    pub regime: String,
    /// Confidence 0.0..1.0.
    pub confidence: f64,
    /// Uncertainty 0.0..1.0 (higher = less certain).
    pub uncertainty: f64,
    /// Expected edge (return fraction) net of estimated costs.
    pub expected_edge: f64,
    /// Expected transaction cost as a fraction.
    pub expected_cost: f64,
    /// Holding horizon in minutes.
    pub horizon_minutes: u32,
    /// Entry zone lower bound.
    pub entry_zone_low: f64,
    /// Entry zone upper bound.
    pub entry_zone_high: f64,
    /// Stop-loss / invalidation price level.
    pub invalidation_level: f64,
    /// Target price (exit on profit).
    pub target_level: f64,
    /// Maximum adverse excursion the thesis tolerates (price units).
    pub max_adverse_excursion: f64,
    /// Evidence supporting and contradicting the thesis.
    pub evidence: Vec<EvidenceRef>,
    /// Preferred execution style.
    pub preferred_execution: ExecutionStyle,
    /// Creation timestamp (RFC 3339).
    pub created_at: String,
    /// Expiry timestamp (RFC 3339) — a thesis that has expired is not
    /// executable (§9 signal TTL).
    pub expires_at: String,
}

impl TradeThesis {
    /// Whether the thesis has been invalidated by the given current price.
    /// A long is invalidated when price falls to/below the invalidation
    /// level; a short when price rises to/above it.
    pub fn is_invalidated_by(&self, current_price: f64) -> bool {
        match self.direction {
            ThesisDirection::Long => current_price <= self.invalidation_level,
            ThesisDirection::Short => current_price >= self.invalidation_level,
        }
    }

    /// Whether the thesis has expired (past its expiry timestamp). The caller
    /// provides the current timestamp string for comparison.
    pub fn is_expired(&self, now_rfc3339: &str) -> bool {
        self.expires_at.as_str() < now_rfc3339
    }

    /// Whether the current price is within the entry zone.
    pub fn price_in_entry_zone(&self, current_price: f64) -> bool {
        current_price >= self.entry_zone_low && current_price <= self.entry_zone_high
    }

    /// Whether the thesis has hit its profit target.
    pub fn target_hit(&self, current_price: f64) -> bool {
        match self.direction {
            ThesisDirection::Long => current_price >= self.target_level,
            ThesisDirection::Short => current_price <= self.target_level,
        }
    }

    /// Split evidence into supporting and contradicting.
    pub fn evidence_split(&self) -> (Vec<&EvidenceRef>, Vec<&EvidenceRef>) {
        self.evidence
            .iter()
            .partition(|e| e.stance == EvidenceStance::Supporting)
    }

    /// Net expected edge after cost.
    pub fn net_expected_edge(&self) -> f64 {
        self.expected_edge - self.expected_cost
    }

    /// Whether the thesis is executable: not expired, not invalidated, and
    /// has positive net expected edge.
    pub fn is_executable(&self, current_price: f64, now_rfc3339: &str) -> bool {
        !self.is_expired(now_rfc3339)
            && !self.is_invalidated_by(current_price)
            && self.net_expected_edge() > 0.0
            && self.confidence > 0.0
    }
}

/// Validate a thesis for internal consistency. Returns an error message if
/// the thesis is malformed.
pub fn validate_thesis(thesis: &TradeThesis) -> Result<(), String> {
    if thesis.thesis_id.trim().is_empty() {
        return Err("thesis_id is empty".into());
    }
    if thesis.instrument.trim().is_empty() {
        return Err("instrument is empty".into());
    }
    if !(0.0..=1.0).contains(&thesis.confidence) {
        return Err(format!("confidence {} must be in [0,1]", thesis.confidence));
    }
    if !(0.0..=1.0).contains(&thesis.uncertainty) {
        return Err(format!(
            "uncertainty {} must be in [0,1]",
            thesis.uncertainty
        ));
    }
    if thesis.entry_zone_low > thesis.entry_zone_high {
        return Err(format!(
            "entry zone low {} > high {}",
            thesis.entry_zone_low, thesis.entry_zone_high
        ));
    }
    if thesis.horizon_minutes == 0 {
        return Err("horizon must be positive".into());
    }
    if thesis.max_adverse_excursion < 0.0 {
        return Err("max_adverse_excursion must be non-negative".into());
    }
    // Invalidation must be on the correct side of the entry zone.
    match thesis.direction {
        ThesisDirection::Long => {
            if thesis.invalidation_level >= thesis.entry_zone_low {
                return Err(format!(
                    "long invalidation {} must be below entry zone low {}",
                    thesis.invalidation_level, thesis.entry_zone_low
                ));
            }
            if thesis.target_level <= thesis.entry_zone_high {
                return Err(format!(
                    "long target {} must be above entry zone high {}",
                    thesis.target_level, thesis.entry_zone_high
                ));
            }
        },
        ThesisDirection::Short => {
            if thesis.invalidation_level <= thesis.entry_zone_high {
                return Err(format!(
                    "short invalidation {} must be above entry zone high {}",
                    thesis.invalidation_level, thesis.entry_zone_high
                ));
            }
            if thesis.target_level >= thesis.entry_zone_low {
                return Err(format!(
                    "short target {} must be below entry zone low {}",
                    thesis.target_level, thesis.entry_zone_low
                ));
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_long() -> TradeThesis {
        TradeThesis {
            thesis_id: "thesis-001".into(),
            instrument: "BTC".into(),
            direction: ThesisDirection::Long,
            strategy: "seed:momentum".into(),
            regime: "trending".into(),
            confidence: 0.72,
            uncertainty: 0.28,
            expected_edge: 0.03,
            expected_cost: 0.005,
            horizon_minutes: 1440,
            entry_zone_low: 118_000.0,
            entry_zone_high: 119_000.0,
            invalidation_level: 116_000.0,
            target_level: 124_000.0,
            max_adverse_excursion: 2_000.0,
            evidence: vec![
                EvidenceRef {
                    evidence_id: "market:coingecko:BTC:t1".into(),
                    summary: "BTC breaking above 200-SMA with volume".into(),
                    stance: EvidenceStance::Supporting,
                },
                EvidenceRef {
                    evidence_id: "news:reuters:fud:t2".into(),
                    summary: "Regulatory FUD may cap upside".into(),
                    stance: EvidenceStance::Contradicting,
                },
            ],
            preferred_execution: ExecutionStyle::MarketableLimit,
            created_at: "2026-08-09T10:00:00Z".into(),
            expires_at: "2026-08-10T10:00:00Z".into(),
        }
    }

    #[test]
    fn long_invalidated_below_stop() {
        let t = sample_long();
        assert!(t.is_invalidated_by(115_000.0));
        assert!(t.is_invalidated_by(116_000.0)); // at stop
        assert!(!t.is_invalidated_by(118_500.0));
    }

    #[test]
    fn target_hit_above_target_for_long() {
        let t = sample_long();
        assert!(t.target_hit(124_500.0));
        assert!(!t.target_hit(123_000.0));
    }

    #[test]
    fn entry_zone_check() {
        let t = sample_long();
        assert!(t.price_in_entry_zone(118_500.0));
        assert!(!t.price_in_entry_zone(117_000.0));
        assert!(!t.price_in_entry_zone(120_000.0));
    }

    #[test]
    fn evidence_split_separates_supporting_and_contradicting() {
        let t = sample_long();
        let (sup, con) = t.evidence_split();
        assert_eq!(sup.len(), 1);
        assert_eq!(con.len(), 1);
        assert_eq!(sup[0].stance, EvidenceStance::Supporting);
        assert_eq!(con[0].stance, EvidenceStance::Contradicting);
    }

    #[test]
    fn net_edge_subtracts_cost() {
        let t = sample_long();
        assert!((t.net_expected_edge() - 0.025).abs() < 1e-9);
    }

    #[test]
    fn executable_requires_not_expired_not_invalidated_positive_edge() {
        let t = sample_long();
        // Valid state: price in entry zone, not expired, positive edge.
        assert!(t.is_executable(118_500.0, "2026-08-09T12:00:00Z"));
        // Expired.
        assert!(!t.is_executable(118_500.0, "2026-08-11T00:00:00Z"));
        // Invalidated.
        assert!(!t.is_executable(115_000.0, "2026-08-09T12:00:00Z"));
    }

    #[test]
    fn validate_accepts_correct_thesis() {
        assert!(validate_thesis(&sample_long()).is_ok());
    }

    #[test]
    fn validate_rejects_inconsistent_long_levels() {
        let mut t = sample_long();
        t.invalidation_level = 120_000.0; // above entry zone low — wrong for long
        assert!(validate_thesis(&t).is_err());
    }

    #[test]
    fn validate_rejects_bad_confidence() {
        let mut t = sample_long();
        t.confidence = 1.5;
        assert!(validate_thesis(&t).is_err());
    }

    #[test]
    fn validate_rejects_inverted_entry_zone() {
        let mut t = sample_long();
        t.entry_zone_low = 120_000.0;
        t.entry_zone_high = 119_000.0;
        assert!(validate_thesis(&t).is_err());
    }

    #[test]
    fn short_thesis_invalidation_above_stop() {
        let mut t = sample_long();
        t.direction = ThesisDirection::Short;
        t.invalidation_level = 122_000.0;
        t.target_level = 114_000.0;
        t.entry_zone_low = 118_000.0;
        t.entry_zone_high = 120_000.0;
        assert!(validate_thesis(&t).is_ok());
        assert!(t.is_invalidated_by(123_000.0)); // above stop
        assert!(!t.is_invalidated_by(119_000.0));
        assert!(t.target_hit(113_000.0)); // below target
    }
}
