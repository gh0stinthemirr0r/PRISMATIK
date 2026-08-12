//! Deterministic behavioral diagnostics over completed trading records.
//!
//! Findings are descriptive review prompts, not psychological diagnoses. The
//! analyzer uses only supplied timestamps and fixed-point monetary values.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{Duration, OffsetDateTime};

/// A normalized completed trade for behavioral review.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeReview {
    /// Stable journal or execution identifier.
    pub id: String,
    /// Canonical instrument identifier.
    pub instrument_id: String,
    /// Position opening time.
    pub opened_at: OffsetDateTime,
    /// Position closing time.
    pub closed_at: OffsetDateTime,
    /// Signed realized profit or loss in currency micros.
    pub realized_pnl_micros: i64,
    /// Absolute opening notional in currency micros.
    pub notional_micros: u64,
    /// Account equity at entry in currency micros.
    pub account_equity_micros: u64,
    /// Whether a thesis existed before entry.
    pub thesis_recorded: bool,
}

/// Behavioral review thresholds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehaviorConfig {
    /// Maximum permitted notional/equity ratio in parts per million.
    pub maximum_leverage_ppm: u32,
    /// Same-instrument re-entry interval that warrants review.
    pub rapid_reentry_seconds: i64,
    /// Post-loss interval used to identify possible revenge trading.
    pub revenge_window_seconds: i64,
    /// Consecutive losses required for a loss-cluster finding.
    pub loss_cluster_count: usize,
}

/// Category of deterministic review finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorKind {
    /// Notional exceeded the configured leverage threshold.
    Overleverage,
    /// Same-instrument entry occurred soon after a close.
    RapidReentry,
    /// A larger same-instrument position followed a loss within the review window.
    PossibleRevengeTrade,
    /// Entry had no pre-recorded thesis.
    MissingThesis,
    /// Configured number of consecutive losses occurred.
    LossCluster,
}

/// Evidence-linked behavioral finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehaviorFinding {
    /// Finding category.
    pub kind: BehaviorKind,
    /// Trade primarily associated with the finding.
    pub trade_id: String,
    /// Related trade ids supporting the finding.
    pub related_trade_ids: Vec<String>,
    /// Deterministic human-readable explanation.
    pub explanation: String,
}

/// Invalid review input or configuration.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum BehaviorError {
    /// Configuration contained a zero or out-of-range threshold.
    #[error("behavior configuration is invalid")]
    InvalidConfiguration,
    /// Trade id or instrument id was empty.
    #[error("trade has an empty identifier: {0}")]
    EmptyIdentifier(String),
    /// Trade close preceded its open time.
    #[error("trade closes before it opens: {0}")]
    InvalidLifecycle(String),
    /// Account equity was zero, preventing leverage calculation.
    #[error("trade has zero account equity: {0}")]
    ZeroEquity(String),
    /// Duplicate trade id was supplied.
    #[error("duplicate trade id: {0}")]
    DuplicateTrade(String),
}

/// Analyze completed activity in deterministic open-time order.
pub fn analyze_behavior(
    trades: &[TradeReview],
    config: &BehaviorConfig,
) -> Result<Vec<BehaviorFinding>, BehaviorError> {
    validate_config(config)?;
    let mut ordered = trades.to_vec();
    ordered.sort_by(|left, right| {
        left.opened_at
            .cmp(&right.opened_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut seen = std::collections::BTreeSet::new();
    for trade in &ordered {
        validate_trade(trade)?;
        if !seen.insert(trade.id.clone()) {
            return Err(BehaviorError::DuplicateTrade(trade.id.clone()));
        }
    }

    let mut findings = Vec::new();
    let mut consecutive_losses = Vec::<String>::new();
    for (index, trade) in ordered.iter().enumerate() {
        let leverage_ppm =
            u128::from(trade.notional_micros) * 1_000_000 / u128::from(trade.account_equity_micros);
        if leverage_ppm > u128::from(config.maximum_leverage_ppm) {
            findings.push(BehaviorFinding {
                kind: BehaviorKind::Overleverage,
                trade_id: trade.id.clone(),
                related_trade_ids: vec![trade.id.clone()],
                explanation: format!(
                    "entry leverage was {} ppm; configured maximum is {} ppm",
                    leverage_ppm, config.maximum_leverage_ppm
                ),
            });
        }
        if !trade.thesis_recorded {
            findings.push(BehaviorFinding {
                kind: BehaviorKind::MissingThesis,
                trade_id: trade.id.clone(),
                related_trade_ids: vec![trade.id.clone()],
                explanation: "no thesis was recorded before entry".into(),
            });
        }

        if let Some(previous) = ordered[..index].iter().rev().find(|candidate| {
            candidate.instrument_id == trade.instrument_id && candidate.closed_at <= trade.opened_at
        }) {
            let elapsed = trade.opened_at - previous.closed_at;
            if elapsed <= Duration::seconds(config.rapid_reentry_seconds) {
                findings.push(BehaviorFinding {
                    kind: BehaviorKind::RapidReentry,
                    trade_id: trade.id.clone(),
                    related_trade_ids: vec![previous.id.clone(), trade.id.clone()],
                    explanation: format!(
                        "same-instrument re-entry occurred {} seconds after close",
                        elapsed.whole_seconds()
                    ),
                });
            }
            if previous.realized_pnl_micros < 0
                && trade.notional_micros > previous.notional_micros
                && elapsed <= Duration::seconds(config.revenge_window_seconds)
            {
                findings.push(BehaviorFinding {
                    kind: BehaviorKind::PossibleRevengeTrade,
                    trade_id: trade.id.clone(),
                    related_trade_ids: vec![previous.id.clone(), trade.id.clone()],
                    explanation:
                        "position size increased after a recent loss in the same instrument".into(),
                });
            }
        }

        if trade.realized_pnl_micros < 0 {
            consecutive_losses.push(trade.id.clone());
            if consecutive_losses.len() == config.loss_cluster_count {
                findings.push(BehaviorFinding {
                    kind: BehaviorKind::LossCluster,
                    trade_id: trade.id.clone(),
                    related_trade_ids: consecutive_losses.clone(),
                    explanation: format!(
                        "{} consecutive losing trades reached the review threshold",
                        config.loss_cluster_count
                    ),
                });
            } else if consecutive_losses.len() > config.loss_cluster_count {
                consecutive_losses.remove(0);
            }
        } else {
            consecutive_losses.clear();
        }
    }
    Ok(findings)
}

fn validate_config(config: &BehaviorConfig) -> Result<(), BehaviorError> {
    if config.maximum_leverage_ppm == 0
        || config.rapid_reentry_seconds < 0
        || config.revenge_window_seconds < 0
        || config.loss_cluster_count < 2
    {
        return Err(BehaviorError::InvalidConfiguration);
    }
    Ok(())
}

fn validate_trade(trade: &TradeReview) -> Result<(), BehaviorError> {
    if trade.id.trim().is_empty() || trade.instrument_id.trim().is_empty() {
        return Err(BehaviorError::EmptyIdentifier(trade.id.clone()));
    }
    if trade.closed_at < trade.opened_at {
        return Err(BehaviorError::InvalidLifecycle(trade.id.clone()));
    }
    if trade.account_equity_micros == 0 {
        return Err(BehaviorError::ZeroEquity(trade.id.clone()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trade(id: &str, opened_seconds: i64, pnl: i64, notional: u64, thesis: bool) -> TradeReview {
        let opened_at = OffsetDateTime::UNIX_EPOCH + Duration::seconds(opened_seconds);
        TradeReview {
            id: id.into(),
            instrument_id: "BTC-USD".into(),
            opened_at,
            closed_at: opened_at + Duration::minutes(10),
            realized_pnl_micros: pnl,
            notional_micros: notional,
            account_equity_micros: 10_000_000,
            thesis_recorded: thesis,
        }
    }

    fn config() -> BehaviorConfig {
        BehaviorConfig {
            maximum_leverage_ppm: 1_500_000,
            rapid_reentry_seconds: 1_800,
            revenge_window_seconds: 3_600,
            loss_cluster_count: 2,
        }
    }

    #[test]
    fn flags_reentry_revenge_leverage_thesis_and_loss_cluster() {
        let findings = analyze_behavior(
            &[
                trade("first", 0, -100, 10_000_000, true),
                trade("second", 900, -50, 20_000_000, false),
            ],
            &config(),
        )
        .unwrap();
        let kinds = findings
            .iter()
            .map(|finding| finding.kind)
            .collect::<Vec<_>>();
        assert!(kinds.contains(&BehaviorKind::RapidReentry));
        assert!(kinds.contains(&BehaviorKind::PossibleRevengeTrade));
        assert!(kinds.contains(&BehaviorKind::Overleverage));
        assert!(kinds.contains(&BehaviorKind::MissingThesis));
        assert!(kinds.contains(&BehaviorKind::LossCluster));
    }

    #[test]
    fn rejects_duplicate_trade_ids() {
        let row = trade("same", 0, 1, 1, true);
        assert_eq!(
            analyze_behavior(&[row.clone(), row], &config()),
            Err(BehaviorError::DuplicateTrade("same".into()))
        );
    }
}
