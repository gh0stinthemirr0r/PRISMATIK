//! Bar-driven backtest simulation engine.
//!
//! This is the real market-data path for [`crate::BacktestEngine`]. It takes
//! an OHLCV bar series, a deterministic [`SignalStrategy`] that emits
//! long/flat/short signals per bar, applies execution assumptions (fill
//! model, slippage, commission), and produces a [`SimulationResult`] with
//! fully-computed statistics (Sharpe, Sortino, Calmar, drawdown, profit
//! factor, win rate, equity curve, trade list).
//!
//! Determinism (I3): single-threaded, fixed iteration order, compensated
//! summation. Same inputs always produce byte-identical outputs.

use crate::assumptions::{ExecutionAssumptions, FillModel};
use crate::stats::{
    annualized_return, calmar_ratio, max_drawdown, profit_factor, sharpe_ratio, sortino_ratio,
    win_rate, CalendarKind,
};
use serde::{Deserialize, Serialize};

/// OHLCV bar — intentionally compatible with [`prismatik_indicator_core::Bar`]
/// but kept local to avoid a cross-crate dependency in the backtest domain
/// layer.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OhlcBar {
    /// Open.
    pub open: f64,
    /// High.
    pub high: f64,
    /// Low.
    pub low: f64,
    /// Close.
    pub close: f64,
    /// Volume.
    pub volume: f64,
    /// Optional UNIX timestamp (seconds).
    pub timestamp_secs: Option<i64>,
}

impl OhlcBar {
    /// Construct from OHLCV.
    pub fn new(open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume,
            timestamp_secs: None,
        }
    }

    /// Typical (median) price.
    pub fn typical(&self) -> f64 {
        (self.high + self.low + self.close) / 3.0
    }
}

/// Trading signal emitted by a strategy on each bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    /// Go/Stay long (full target weight = 1.0).
    Long,
    /// Go/Stay flat (no position).
    Flat,
    /// Go/Stay short (full target weight = -1.0).
    Short,
}

/// Deterministic signal-producing strategy. Implementations must be pure
/// functions of their inputs (no ambient state, no time, no RNG outside the
/// supplied seed).
pub trait SignalStrategy: Send + Sync {
    /// Stable strategy id (matches the IR/backtest config).
    fn id(&self) -> &str;
    /// Compute the target signal for the bar at `index`, given the full
    /// history up to and including that bar.
    fn signal(&self, bars: &[OhlcBar], index: usize) -> Signal;
}

/// Realized fill in the simulation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimFill {
    /// Bar index of the fill.
    pub bar_index: usize,
    /// Fill price after slippage.
    pub price: f64,
    /// Traded quantity (signed: + buy, - sell).
    pub quantity: f64,
    /// Commission paid.
    pub commission: f64,
}

/// A round-trip trade (entry to exit).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimTrade {
    /// Entry bar index.
    pub entry_bar: usize,
    /// Exit bar index.
    pub exit_bar: usize,
    /// Direction: +1 long, -1 short.
    pub direction: i32,
    /// Entry price (post-cost).
    pub entry_price: f64,
    /// Exit price (post-cost).
    pub exit_price: f64,
    /// Quantity traded.
    pub quantity: f64,
    /// Net P&L in currency units (after commission + slippage).
    pub pnl: f64,
}

/// Per-bar equity snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EquityPoint {
    /// Bar index.
    pub bar_index: usize,
    /// Total equity (cash + position market value).
    pub equity: f64,
    /// Position direction at end of bar.
    pub position: f64,
}

/// Full simulation result with computed statistics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Strategy id.
    pub strategy_id: String,
    /// Bars processed.
    pub bars_processed: usize,
    /// Starting capital.
    pub starting_capital: f64,
    /// Ending equity.
    pub ending_equity: f64,
    /// Total return (fraction).
    pub total_return: f64,
    /// Annualized return (CAGR).
    pub annualized_return: f64,
    /// Annualized Sharpe ratio.
    pub sharpe: f64,
    /// Annualized Sortino ratio.
    pub sortino: f64,
    /// Calmar ratio.
    pub calmar: f64,
    /// Maximum drawdown (positive fraction).
    pub max_drawdown: f64,
    /// Profit factor.
    pub profit_factor: f64,
    /// Win rate (0..1).
    pub win_rate: f64,
    /// Number of closed trades.
    pub trade_count: usize,
    /// Number of fills.
    pub fill_count: usize,
    /// Per-bar equity curve.
    pub equity_curve: Vec<EquityPoint>,
    /// Per-trade P&L series.
    pub trade_pnl: Vec<f64>,
    /// Per-bar strategy returns.
    pub bar_returns: Vec<f64>,
    /// Periods per year used for annualization.
    pub periods_per_year: f64,
    /// Verdict: "positive_sharpe" if Sharpe > 0, else "non_positive_sharpe".
    pub verdict: String,
}

/// Run a bar-driven simulation with full-capital position sizing.
///
/// The strategy's target weight (`+1`/`0`/`-1`) is applied to current equity,
/// so the position quantity scales with the account. This is the correct
/// production behavior: a 100%-long signal means "deploy all available
/// equity into the asset," and returns reflect that.
///
/// - `bars`: OHLCV history in chronological order.
/// - `strategy`: produces a target signal per bar.
/// - `starting_capital`: initial cash.
/// - `execution`: fill model + costs.
/// - `calendar`: annualization convention.
/// - `bar_interval_seconds`: bar frequency (for annualization).
pub fn simulate(
    bars: &[OhlcBar],
    strategy: &dyn SignalStrategy,
    starting_capital: f64,
    execution: &ExecutionAssumptions,
    calendar: CalendarKind,
    bar_interval_seconds: u64,
) -> Result<SimulationResult, String> {
    simulate_weighted(
        bars,
        strategy,
        starting_capital,
        execution,
        calendar,
        bar_interval_seconds,
        1.0,
    )
}

/// Run a simulation with a custom target weight fraction (0.0..1.0). A weight
/// of 1.0 deploys all equity; 0.5 deploys half. Used for risk-budgeted sizing
/// when the caller has already computed the fraction (e.g. 1% risk / stop
/// distance).
pub fn simulate_weighted(
    bars: &[OhlcBar],
    strategy: &dyn SignalStrategy,
    starting_capital: f64,
    execution: &ExecutionAssumptions,
    calendar: CalendarKind,
    bar_interval_seconds: u64,
    target_weight: f64,
) -> Result<SimulationResult, String> {
    if bars.len() < 2 {
        return Err("need at least 2 bars to simulate".into());
    }
    if !starting_capital.is_finite() || starting_capital <= 0.0 {
        return Err("starting capital must be positive and finite".into());
    }
    let periods_per_year = calendar.periods_per_year(bar_interval_seconds);
    let slippage_bps = execution.slippage_bps as f64;
    let commission_per_share = execution.commission_per_share_micros as f64 / 1_000_000.0;

    let mut cash = starting_capital;
    let mut position: f64 = 0.0; // signed quantity
    let mut entry_price: f64 = 0.0;
    let mut entry_bar: usize = 0;
    let mut fills: Vec<SimFill> = Vec::new();
    let mut trades: Vec<SimTrade> = Vec::new();
    let mut equity_curve: Vec<EquityPoint> = Vec::with_capacity(bars.len());

    // Track the last signal so we only rebalance when the target *changes*.
    // A signal-based strategy declares a target weight; continuous
    // rebalancing to that weight every bar would bleed costs without adding
    // information. We trade on signal transitions only.
    let mut last_signal = if bars.is_empty() {
        Signal::Flat
    } else {
        strategy.signal(bars, 0)
    };

    for (i, bar) in bars.iter().enumerate() {
        let signal = if i == 0 {
            last_signal
        } else {
            strategy.signal(bars, i)
        };
        let signal_changed = signal != last_signal;
        last_signal = signal;

        // Target signed WEIGHT from the signal, scaled by target_weight.
        let target_weight_signed = match signal {
            Signal::Long => target_weight,
            Signal::Short => -target_weight,
            Signal::Flat => 0.0_f64,
        };

        // Current mark-to-market equity (for sizing).
        let current_equity = (cash + position * bar.close).max(1e-9);

        // Only trade when the signal changes. On the first bar where a new
        // signal is in force, we size to the target weight at current equity.
        if signal_changed || i == 0 {
            // Target quantity: signed fraction of equity at current price.
            let target_qty = if bar.close > 0.0 {
                target_weight_signed * current_equity / bar.close
            } else {
                0.0
            };

            // Determine execution price. Causal: Close fills at this bar's
            // close; NextOpen fills at the next bar's open.
            let (fill_price_base, fill_bar) = match execution.fill_model {
                FillModel::Close => (bar.close, i),
                FillModel::NextOpen => {
                    if i + 1 >= bars.len() {
                        equity_curve.push(EquityPoint {
                            bar_index: i,
                            equity: current_equity,
                            position,
                        });
                        continue;
                    }
                    (bars[i + 1].open, i + 1)
                },
            };

            let delta_qty = target_qty - position;
            let slip = fill_price_base * slippage_bps / 10_000.0;
            let fill_price = if delta_qty > 0.0 {
                fill_price_base + slip
            } else {
                fill_price_base - slip
            };

            let closing =
                position != 0.0 && (target_qty == 0.0 || target_qty.signum() != position.signum());
            if closing {
                let direction = position.signum() as i32;
                let qty = position.abs();
                let gross_pnl = direction as f64 * (fill_price - entry_price) * qty;
                let exit_commission = qty * commission_per_share;
                let net_pnl = gross_pnl - exit_commission;
                trades.push(SimTrade {
                    entry_bar,
                    exit_bar: fill_bar,
                    direction,
                    entry_price,
                    exit_price: fill_price,
                    quantity: qty,
                    pnl: net_pnl,
                });
            }

            let traded_qty = delta_qty.abs();
            cash += -delta_qty * fill_price;
            cash -= traded_qty * commission_per_share;

            let opening = target_qty != 0.0 && (position == 0.0 || closing);
            if opening {
                entry_price = fill_price;
                entry_bar = fill_bar;
            }

            position = target_qty;
            fills.push(SimFill {
                bar_index: fill_bar,
                price: fill_price,
                quantity: delta_qty,
                commission: traded_qty * commission_per_share,
            });
        }

        // Mark-to-market equity at this bar's close.
        let equity = cash + position * bar.close;
        equity_curve.push(EquityPoint {
            bar_index: i,
            equity,
            position,
        });
    }

    // Close any open position at the final bar's close for honest reporting.
    if position != 0.0 {
        let last = *bars.last().ok_or("bars empty")?;
        let direction = position.signum() as i32;
        let qty = position.abs();
        let gross_pnl = direction as f64 * (last.close - entry_price) * qty;
        let exit_commission = qty * commission_per_share;
        let net_pnl = gross_pnl - exit_commission;
        trades.push(SimTrade {
            entry_bar,
            exit_bar: bars.len() - 1,
            direction,
            entry_price,
            exit_price: last.close,
            quantity: qty,
            pnl: net_pnl,
        });
        // Closing = moving from `position` to 0; cash increases by
        // position * last.close (selling a long or buying back a short).
        cash += position * last.close;
        cash -= qty * commission_per_share;
        if let Some(last_eq) = equity_curve.last_mut() {
            last_eq.equity = cash;
            last_eq.position = 0.0;
        }
        // `position` is now implicitly 0 conceptually; we don't reassign
        // because it is never read again after this final-bar close.
    }

    // Compute per-bar returns from equity curve.
    let bar_returns: Vec<f64> = equity_curve
        .windows(2)
        .map(|w| {
            if w[0].equity != 0.0 {
                (w[1].equity - w[0].equity) / w[0].equity
            } else {
                0.0
            }
        })
        .collect();

    let trade_pnl: Vec<f64> = trades.iter().map(|t| t.pnl).collect();
    let ending_equity = equity_curve
        .last()
        .map(|p| p.equity)
        .unwrap_or(starting_capital);
    let total_return = (ending_equity - starting_capital) / starting_capital;
    let ann_return = annualized_return(total_return, bars.len(), periods_per_year);
    let sharpe = sharpe_ratio(&bar_returns, periods_per_year, 0.0);
    let sortino = sortino_ratio(&bar_returns, periods_per_year, 0.0);
    let mdd = max_drawdown(&equity_curve.iter().map(|p| p.equity).collect::<Vec<_>>());
    let calmar = calmar_ratio(ann_return, mdd);
    let pf = profit_factor(&trade_pnl);
    let wr = win_rate(&trade_pnl);
    let pf_display = if pf.is_infinite() { f64::MAX } else { pf };

    let verdict = if sharpe > 0.0 {
        "positive_sharpe"
    } else {
        "non_positive_sharpe"
    };

    Ok(SimulationResult {
        strategy_id: strategy.id().to_string(),
        bars_processed: bars.len(),
        starting_capital,
        ending_equity,
        total_return,
        annualized_return: ann_return,
        sharpe,
        sortino,
        calmar,
        max_drawdown: mdd,
        profit_factor: pf_display,
        win_rate: wr,
        trade_count: trades.len(),
        fill_count: fills.len(),
        equity_curve,
        trade_pnl,
        bar_returns,
        periods_per_year,
        verdict: verdict.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trending-up bar series — a long-only buy-and-hold should be positive
    /// Sharpe.
    fn uptrend_bars(n: usize) -> Vec<OhlcBar> {
        (0..n)
            .map(|i| {
                let close = 100.0 + i as f64;
                OhlcBar::new(close - 0.5, close + 0.5, close - 1.0, close, 1000.0)
            })
            .collect()
    }

    struct AlwaysLong;
    impl SignalStrategy for AlwaysLong {
        fn id(&self) -> &str {
            "test:always_long"
        }
        fn signal(&self, _bars: &[OhlcBar], _index: usize) -> Signal {
            Signal::Long
        }
    }

    struct AlwaysFlat;
    impl SignalStrategy for AlwaysFlat {
        fn id(&self) -> &str {
            "test:always_flat"
        }
        fn signal(&self, _bars: &[OhlcBar], _index: usize) -> Signal {
            Signal::Flat
        }
    }

    #[test]
    fn uptrend_buy_and_hold_is_positive_sharpe() {
        let bars = uptrend_bars(50);
        let exec = ExecutionAssumptions::default();
        let result = simulate(
            &bars,
            &AlwaysLong,
            100_000.0,
            &exec,
            CalendarKind::EquityUs,
            86_400,
        )
        .expect("simulation should succeed");
        assert!(
            result.sharpe > 0.0,
            "uptrend should be positive Sharpe, got {}",
            result.sharpe
        );
        assert!(result.total_return > 0.0);
        assert_eq!(result.verdict, "positive_sharpe");
    }

    #[test]
    fn flat_strategy_has_zero_drawdown() {
        let bars = uptrend_bars(20);
        let exec = ExecutionAssumptions::default();
        let result = simulate(
            &bars,
            &AlwaysFlat,
            100_000.0,
            &exec,
            CalendarKind::EquityUs,
            86_400,
        )
        .expect("simulation should succeed");
        assert_eq!(result.max_drawdown, 0.0);
        assert!((result.total_return).abs() < 1e-9);
        assert_eq!(result.trade_count, 0);
    }

    #[test]
    fn determinism_two_runs_identical_bits() {
        let bars = uptrend_bars(30);
        let exec = ExecutionAssumptions {
            slippage_bps: 5,
            commission_per_share_micros: 1_000,
            ..Default::default()
        };
        let r1 = simulate(
            &bars,
            &AlwaysLong,
            100_000.0,
            &exec,
            CalendarKind::EquityUs,
            86_400,
        )
        .unwrap();
        let r2 = simulate(
            &bars,
            &AlwaysLong,
            100_000.0,
            &exec,
            CalendarKind::EquityUs,
            86_400,
        )
        .unwrap();
        // Bit-identical check on the headline metrics.
        assert_eq!(r1.sharpe.to_bits(), r2.sharpe.to_bits());
        assert_eq!(r1.total_return.to_bits(), r2.total_return.to_bits());
        assert_eq!(r1.max_drawdown.to_bits(), r2.max_drawdown.to_bits());
    }

    #[test]
    fn costs_reduce_returns_vs_frictionless() {
        let bars = uptrend_bars(40);
        let frictionless = ExecutionAssumptions::default();
        let costly = ExecutionAssumptions {
            slippage_bps: 50,
            commission_per_share_micros: 10_000,
            ..Default::default()
        };
        let r1 = simulate(
            &bars,
            &AlwaysLong,
            100_000.0,
            &frictionless,
            CalendarKind::EquityUs,
            86_400,
        )
        .unwrap();
        let r2 = simulate(
            &bars,
            &AlwaysLong,
            100_000.0,
            &costly,
            CalendarKind::EquityUs,
            86_400,
        )
        .unwrap();
        assert!(
            r1.ending_equity > r2.ending_equity,
            "frictionless ({}) should beat costly ({})",
            r1.ending_equity,
            r2.ending_equity
        );
    }

    #[test]
    fn simulate_rejects_too_few_bars() {
        let bars = vec![OhlcBar::new(1.0, 1.0, 1.0, 1.0, 1.0)];
        let result = simulate(
            &bars,
            &AlwaysLong,
            100_000.0,
            &ExecutionAssumptions::default(),
            CalendarKind::EquityUs,
            86_400,
        );
        assert!(result.is_err());
    }
}
