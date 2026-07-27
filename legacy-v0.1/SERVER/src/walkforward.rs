//! Walk forward validation with parameter stability, search accounting, and a
//! block bootstrap. Author: Aaron Stovall · Version 0.1.0 · 2026-07-07
//!
//! Tune on an in sample window, score on the unseen window after it, stitch the
//! out of sample segments into one honest curve. Report how many configurations
//! were searched, because every one raises the odds the winner is luck, and how
//! much chosen parameters wander between folds, because unstable parameters
//! signal a fit to noise.

use crate::config::CostModel;
use crate::engine::{backtest, compute_metrics};
use crate::strategy::{apply_vol_target, Strategy, StrategySpec};
use crate::types::{Candle, EquityPoint, Metrics};
use prismatik_determinism::{Entropy, SplitEntropy};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct BootstrapReport {
    pub n_sims: usize,
    pub block_bars: usize,
    pub total_return_p05: f64,
    pub total_return_p50: f64,
    pub total_return_p95: f64,
    pub prob_loss: f64,
}

#[derive(Serialize)]
pub struct WalkForwardReport {
    pub oos_equity: Vec<EquityPoint>,
    pub oos_metrics: Metrics,
    pub chosen_params: Vec<serde_json::Value>,
    pub n_folds: usize,
    pub configs_searched: usize,
    pub param_stability: BTreeMap<String, f64>,
    pub stable: bool,
    pub caution: String,
    pub bootstrap: Option<BootstrapReport>,
}

pub fn grid_for(kind: &str) -> Result<Vec<serde_json::Value>, String> {
    let grid = match kind {
        "ma" => {
            let mut g = Vec::new();
            for fast in [12u32, 24, 48] {
                for slow in [72u32, 96, 168] {
                    if fast < slow {
                        g.push(serde_json::json!({"fast": fast, "slow": slow}));
                    }
                }
            }
            g
        },
        "donchian" => [24u32, 48, 96, 168]
            .iter()
            .map(|lb| serde_json::json!({"lookback": lb}))
            .collect(),
        "rsi" => {
            let mut g = Vec::new();
            for period in [7u32, 14, 21] {
                for entry in [25.0f64, 30.0, 35.0] {
                    g.push(serde_json::json!({"period": period, "entry": entry, "exit": 55.0}));
                }
            }
            g
        },
        "hold" => vec![serde_json::json!({})],
        other => return Err(format!("no walk forward grid for {other:?}")),
    };
    Ok(grid)
}

fn run_once(
    candles: &[Candle],
    spec: &StrategySpec,
    params: &serde_json::Value,
    cost: &CostModel,
    max_weight: f64,
    initial_equity: f64,
    ppy: f64,
) -> Result<(Vec<EquityPoint>, Vec<f64>, Metrics), String> {
    let concrete = StrategySpec {
        kind: spec.kind.clone(),
        params: params.as_object().cloned().unwrap_or_default(),
        vol_target: spec.vol_target,
    };
    let strat = Strategy::from_spec(&concrete)?;
    let mut weights = strat.target_weights(candles);
    if let Some(vt) = spec.vol_target {
        apply_vol_target(&mut weights, candles, vt, 48, ppy, max_weight);
    }
    let r = backtest(candles, &weights, cost, max_weight, initial_equity, ppy)?;
    Ok((r.equity, r.net_returns, r.metrics))
}

#[allow(clippy::too_many_arguments)]
pub fn walk_forward(
    candles: &[Candle],
    spec: &StrategySpec,
    grid: &[serde_json::Value],
    n_folds: usize,
    train_ratio: f64,
    cost: &CostModel,
    max_weight: f64,
    initial_equity: f64,
    ppy: f64,
) -> Result<WalkForwardReport, String> {
    if grid.is_empty() {
        return Err("empty parameter grid".into());
    }
    if n_folds < 2 {
        return Err("need at least two folds".into());
    }
    if !(0.0 < train_ratio && train_ratio < 1.0) {
        return Err("train_ratio in (0,1)".into());
    }
    let n = candles.len();
    if n < n_folds * 20 {
        return Err(format!("not enough bars ({n}) for {n_folds} folds"));
    }

    let fold_size = n / n_folds;
    let mut oos_returns: Vec<f64> = Vec::new();
    let mut oos_times: Vec<chrono::DateTime<chrono::Utc>> = Vec::new();
    let mut chosen: Vec<serde_json::Value> = Vec::new();

    for k in 0..n_folds {
        let lo = k * fold_size;
        let hi = if k == n_folds - 1 {
            n
        } else {
            (k + 1) * fold_size
        };
        let fold = &candles[lo..hi];
        let split = (fold.len() as f64 * train_ratio) as usize;
        if split < 10 || fold.len() - split < 5 {
            continue;
        }
        let (train, test) = (&fold[..split], &fold[split..]);

        let mut best: Option<(f64, &serde_json::Value)> = None;
        for params in grid {
            if let Ok((_, _, m)) =
                run_once(train, spec, params, cost, max_weight, initial_equity, ppy)
            {
                let score = if m.sharpe.is_finite() {
                    m.sharpe
                } else {
                    f64::NEG_INFINITY
                };
                if best.map_or(true, |(b, _)| score > b) {
                    best = Some((score, params));
                }
            }
        }
        let Some((_, best_params)) = best else {
            continue;
        };
        let (_, rets, m) = run_once(
            test,
            spec,
            best_params,
            cost,
            max_weight,
            initial_equity,
            ppy,
        )?;
        tracing::info!(fold = k, oos_return = m.total_return, params = %best_params, "fold_done");
        for (i, r) in rets.iter().enumerate() {
            oos_returns.push(*r);
            oos_times.push(test[i + 1].time);
        }
        chosen.push(best_params.clone());
    }

    if oos_returns.is_empty() {
        return Err("no valid folds produced out of sample results".into());
    }

    let mut equity_val = initial_equity;
    let mut oos_equity = Vec::with_capacity(oos_returns.len());
    for (i, r) in oos_returns.iter().enumerate() {
        equity_val *= 1.0 + r;
        oos_equity.push(EquityPoint {
            time: oos_times[i],
            value: equity_val,
        });
    }
    let oos_metrics = compute_metrics(&oos_equity, ppy);

    // Parameter stability: coefficient of variation per numeric param across folds.
    let mut stability: BTreeMap<String, f64> = BTreeMap::new();
    let mut keys: Vec<String> = Vec::new();
    for p in &chosen {
        if let Some(obj) = p.as_object() {
            for k in obj.keys() {
                if !keys.contains(k) {
                    keys.push(k.clone());
                }
            }
        }
    }
    for key in keys {
        let vals: Vec<f64> = chosen
            .iter()
            .filter_map(|p| p.get(&key).and_then(|v| v.as_f64()))
            .collect();
        if vals.len() >= 2 {
            let mean = vals.iter().sum::<f64>() / vals.len() as f64;
            let var =
                vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (vals.len() - 1) as f64;
            stability.insert(
                key,
                if mean.abs() > 0.0 {
                    var.sqrt() / mean.abs()
                } else {
                    f64::INFINITY
                },
            );
        }
    }
    let stable = stability.values().all(|v| *v <= 0.5);
    let configs_searched = grid.len() * chosen.len();
    let caution = format!(
        "{} configurations were searched per fold ({} total evaluations). Every \
         additional configuration searched raises the odds the winner is luck. \
         Treat marginal out of sample results as noise, and treat this entire \
         report as necessary, not sufficient.",
        grid.len(),
        configs_searched,
    );

    let bootstrap = block_bootstrap(&oos_returns, 2000, 24).ok();

    Ok(WalkForwardReport {
        oos_equity,
        oos_metrics,
        chosen_params: chosen,
        n_folds: 0, // set by caller-visible field below
        configs_searched,
        param_stability: stability,
        stable,
        caution,
        bootstrap,
    }
    .with_folds())
}

impl WalkForwardReport {
    fn with_folds(mut self) -> Self {
        self.n_folds = self.chosen_params.len();
        self
    }
}

pub fn block_bootstrap(
    returns: &[f64],
    n_sims: usize,
    block: usize,
) -> Result<BootstrapReport, String> {
    if returns.len() < block * 3 {
        return Err(format!(
            "need at least {} bars for a block bootstrap",
            block * 3
        ));
    }
    if n_sims < 100 {
        return Err("n_sims must be >= 100".into());
    }
    let n = returns.len();
    let mut rng = SplitEntropy::from_seed(returns.len() as u64 ^ 0x9e3779b97f4a7c15);
    let mut totals: Vec<f64> = Vec::with_capacity(n_sims);
    for _ in 0..n_sims {
        let mut sample: Vec<f64> = Vec::with_capacity(n);
        while sample.len() < n {
            let start = rng.next_u64() as usize % (n - block + 1);
            sample.extend_from_slice(&returns[start..start + block]);
        }
        sample.truncate(n);
        totals.push(sample.iter().fold(1.0, |acc, r| acc * (1.0 + r)) - 1.0);
    }
    totals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pct = |p: f64| totals[((totals.len() as f64 - 1.0) * p) as usize];
    Ok(BootstrapReport {
        n_sims,
        block_bars: block,
        total_return_p05: pct(0.05),
        total_return_p50: pct(0.50),
        total_return_p95: pct(0.95),
        prob_loss: totals.iter().filter(|t| **t < 0.0).count() as f64 / totals.len() as f64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn drift_candles(n: usize, seed: u64) -> Vec<Candle> {
        // Deterministic pseudo-random walk (LCG), test fixture only.
        let mut state = seed;
        let mut price = 100.0f64;
        (0..n)
            .map(|i| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let u = ((state >> 33) as f64) / (u32::MAX as f64) - 0.5;
                price *= 1.0 + 0.0004 + u * 0.016;
                Candle {
                    time: Utc
                        .timestamp_opt(1_700_000_000 + i as i64 * 3600, 0)
                        .unwrap(),
                    open: price,
                    high: price,
                    low: price,
                    close: price,
                    volume: 1.0,
                }
            })
            .collect()
    }

    #[test]
    fn walkforward_reports_shape_and_accounting() {
        let candles = drift_candles(1600, 7);
        let spec = StrategySpec {
            kind: "ma".into(),
            params: Default::default(),
            vol_target: None,
        };
        let grid = grid_for("ma").unwrap();
        let cost = CostModel {
            fee_rate: 0.006,
            slippage_rate: 0.0005,
        };
        let rep = walk_forward(&candles, &spec, &grid, 4, 0.6, &cost, 1.0, 100.0, 8760.0).unwrap();
        assert_eq!(rep.configs_searched, grid.len() * rep.n_folds);
        assert!(rep.n_folds >= 2);
        assert!(rep.caution.len() > 40);
        assert!(!rep.oos_equity.is_empty());
        assert!(rep.param_stability.contains_key("fast"));
    }

    #[test]
    fn bootstrap_percentiles_ordered() {
        let rets: Vec<f64> = (0..800).map(|i| ((i as f64 * 0.37).sin()) * 0.01).collect();
        let b = block_bootstrap(&rets, 300, 24).unwrap();
        assert!(b.total_return_p05 <= b.total_return_p50);
        assert!(b.total_return_p50 <= b.total_return_p95);
        assert!((0.0..=1.0).contains(&b.prob_loss));
    }

    #[test]
    fn bootstrap_rejects_tiny_samples() {
        assert!(block_bootstrap(&[0.01; 10], 300, 24).is_err());
    }
}
