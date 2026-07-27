//! Backtest engine and metrics. Author: Aaron Stovall · Version 0.1.0 · 2026-07-07
//!
//! Lookahead free, cost aware. A weight decided from data through bar t is only
//! in force during bar t+1 (a one bar shift). Costs are charged on every unit of
//! turnover, so a round trip pays roughly twice. Degenerate metrics come back as
//! NaN, never as a fabricated number.

use crate::config::CostModel;
use crate::types::{Candle, EquityPoint, Metrics};

pub struct BacktestResult {
    pub equity: Vec<EquityPoint>,
    pub net_returns: Vec<f64>,
    pub n_trades: usize,
    pub total_costs: f64,
    pub metrics: Metrics,
    pub benchmark_equity: Vec<EquityPoint>,
    pub benchmark_metrics: Metrics,
}

/// Run a backtest given target weights aligned to candles (weights[i] decided
/// at the close of candles[i]).
pub fn backtest(
    candles: &[Candle],
    target_weights: &[f64],
    cost: &CostModel,
    max_weight: f64,
    initial_equity: f64,
    periods_per_year: f64,
) -> Result<BacktestResult, String> {
    if candles.len() < 2 {
        return Err("need at least two bars".into());
    }
    if candles.len() != target_weights.len() {
        return Err("weights must align to candles".into());
    }

    let n = candles.len();
    let per_turnover = cost.per_turnover();

    let mut equity_val = initial_equity;
    let mut bench_val = initial_equity;
    let mut prev_held = 0.0f64;
    let mut n_trades = 0usize;
    let mut total_costs = 0.0f64;

    let mut equity = Vec::with_capacity(n);
    let mut bench = Vec::with_capacity(n);
    let mut net_returns = Vec::with_capacity(n);

    equity.push(EquityPoint {
        time: candles[0].time,
        value: equity_val,
    });
    bench.push(EquityPoint {
        time: candles[0].time,
        value: bench_val,
    });

    for i in 1..n {
        let asset_ret = candles[i].close / candles[i - 1].close - 1.0;
        // The weight held during bar i was decided at the close of bar i-1.
        let held = target_weights[i - 1].clamp(-max_weight, max_weight);
        let turnover = (held - prev_held).abs();
        if turnover > 1e-12 {
            n_trades += 1;
        }
        let cost_paid = turnover * per_turnover;
        total_costs += cost_paid;
        let net = held * asset_ret - cost_paid;
        equity_val *= 1.0 + net;
        bench_val *= 1.0 + asset_ret;
        prev_held = held;
        net_returns.push(net);
        equity.push(EquityPoint {
            time: candles[i].time,
            value: equity_val,
        });
        bench.push(EquityPoint {
            time: candles[i].time,
            value: bench_val,
        });
    }

    let metrics = compute_metrics(&equity, periods_per_year);
    let benchmark_metrics = compute_metrics(&bench, periods_per_year);
    Ok(BacktestResult {
        equity,
        net_returns,
        n_trades,
        total_costs,
        metrics,
        benchmark_equity: bench,
        benchmark_metrics,
    })
}

pub fn max_drawdown(values: &[f64]) -> f64 {
    let mut peak = f64::NEG_INFINITY;
    let mut worst = 0.0f64;
    for &v in values {
        peak = peak.max(v);
        if peak > 0.0 {
            worst = worst.min(v / peak - 1.0);
        }
    }
    worst
}

pub fn compute_metrics(equity: &[EquityPoint], periods_per_year: f64) -> Metrics {
    let n = equity.len();
    if n < 2 {
        let nan = f64::NAN;
        return Metrics {
            total_return: nan,
            cagr: nan,
            ann_volatility: nan,
            sharpe: nan,
            sortino: nan,
            max_drawdown: nan,
            calmar: nan,
            n_periods: n,
        };
    }
    let vals: Vec<f64> = equity.iter().map(|p| p.value).collect();
    let rets: Vec<f64> = vals.windows(2).map(|w| w[1] / w[0] - 1.0).collect();
    let total_return = vals[n - 1] / vals[0] - 1.0;

    let years = n as f64 / periods_per_year;
    let growth = vals[n - 1] / vals[0];
    let cagr = if years > 0.0 && growth > 0.0 {
        let c = growth.powf(1.0 / years) - 1.0;
        if c.is_finite() {
            c
        } else {
            f64::NAN
        }
    } else {
        f64::NAN
    };

    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let var = if rets.len() > 1 {
        rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (rets.len() - 1) as f64
    } else {
        f64::NAN
    };
    let sd = var.sqrt();
    let ann_vol = sd * periods_per_year.sqrt();
    let sharpe = if sd > 0.0 {
        mean / sd * periods_per_year.sqrt()
    } else {
        f64::NAN
    };

    let downside: Vec<f64> = rets.iter().copied().filter(|r| *r < 0.0).collect();
    let sortino = if downside.len() > 1 {
        let dmean = downside.iter().sum::<f64>() / downside.len() as f64;
        let dvar =
            downside.iter().map(|r| (r - dmean).powi(2)).sum::<f64>() / (downside.len() - 1) as f64;
        let dsd = dvar.sqrt();
        if dsd > 0.0 {
            mean / dsd * periods_per_year.sqrt()
        } else {
            f64::NAN
        }
    } else {
        f64::NAN
    };

    let mdd = max_drawdown(&vals);
    let calmar = if mdd < 0.0 && cagr.is_finite() {
        cagr / mdd.abs()
    } else {
        f64::NAN
    };

    Metrics {
        total_return,
        cagr,
        ann_volatility: ann_vol,
        sharpe,
        sortino,
        max_drawdown: mdd,
        calmar,
        n_periods: n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn candles(prices: &[f64]) -> Vec<Candle> {
        prices
            .iter()
            .enumerate()
            .map(|(i, &p)| Candle {
                time: Utc
                    .timestamp_opt(1_700_000_000 + i as i64 * 3600, 0)
                    .unwrap(),
                open: p,
                high: p,
                low: p,
                close: p,
                volume: 1.0,
            })
            .collect()
    }

    fn cost(fee: f64, slip: f64) -> CostModel {
        CostModel {
            fee_rate: fee,
            slippage_rate: slip,
        }
    }

    #[test]
    fn buy_and_hold_matches_price_return_minus_entry_cost() {
        let c = candles(&[100.0, 100.0, 200.0]);
        let w = vec![1.0, 1.0, 1.0];
        let r = backtest(&c, &w, &cost(0.006, 0.0005), 1.0, 100.0, 8760.0).unwrap();
        let expected = 100.0 * (1.0 - 0.0065) * 2.0;
        assert!((r.equity.last().unwrap().value - expected).abs() < 1e-9);
        assert_eq!(r.n_trades, 1);
    }

    #[test]
    fn no_lookahead_signal_on_last_bar_captures_nothing() {
        let c = candles(&[100.0, 100.0, 130.0]);
        let w = vec![0.0, 0.0, 1.0]; // signal only on the final bar
        let r = backtest(&c, &w, &cost(0.0, 0.0), 1.0, 100.0, 8760.0).unwrap();
        assert!((r.equity.last().unwrap().value - 100.0).abs() < 1e-9);
    }

    #[test]
    fn churn_pays_costs() {
        let c = candles(&[100.0, 110.0, 100.0, 110.0, 100.0, 110.0]);
        let w = vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0];
        let free = backtest(&c, &w, &cost(0.0, 0.0), 1.0, 100.0, 8760.0).unwrap();
        let costly = backtest(&c, &w, &cost(0.01, 0.0), 1.0, 100.0, 8760.0).unwrap();
        assert!(costly.equity.last().unwrap().value < free.equity.last().unwrap().value);
        assert!(costly.total_costs > 0.0);
    }

    #[test]
    fn max_drawdown_known_value() {
        assert!((max_drawdown(&[100.0, 120.0, 60.0, 90.0]) + 0.5).abs() < 1e-12);
    }

    #[test]
    fn rising_equity_has_positive_sharpe() {
        let pts: Vec<EquityPoint> = (0..500)
            .map(|i| EquityPoint {
                time: Utc
                    .timestamp_opt(1_700_000_000 + i as i64 * 3600, 0)
                    .unwrap(),
                value: 100.0 * 1.001f64.powi(i),
            })
            .collect();
        let m = compute_metrics(&pts, 8760.0);
        assert!(m.sharpe > 0.0 && m.total_return > 0.0);
    }
}
