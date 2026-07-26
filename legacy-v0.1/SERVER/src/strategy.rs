//! Strategies. Author: Aaron Stovall · Version 0.1.0 · 2026-07-07
//!
//! A strategy maps candle history to a target weight per bar using only data at
//! or before that bar; the engine enforces execution timing. Baselines are a
//! measuring stick, not an edge; expect most to lose to buy and hold after
//! costs, which is the framework working. Volatility targeting is honest
//! position sizing layered over any signal; it reshapes risk and cannot rescue
//! a signal with no edge.

use crate::types::Candle;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct StrategySpec {
    pub kind: String,
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub vol_target: Option<f64>,
}

#[derive(Clone, Debug)]
pub enum Strategy {
    Hold,
    MaCross { fast: usize, slow: usize },
    Donchian { lookback: usize },
    Rsi { period: usize, entry: f64, exit: f64 },
}

impl Strategy {
    pub fn name(&self) -> String {
        match self {
            Strategy::Hold => "buy_and_hold".into(),
            Strategy::MaCross { fast, slow } => format!("ma_crossover({fast},{slow})"),
            Strategy::Donchian { lookback } => format!("donchian({lookback})"),
            Strategy::Rsi { period, entry, .. } => format!("rsi({period},{entry})"),
        }
    }

    pub fn from_spec(spec: &StrategySpec) -> Result<Self, String> {
        let p = &spec.params;
        let get_usize = |key: &str, default: usize| -> Result<usize, String> {
            match p.get(key) {
                None => Ok(default),
                Some(v) => v.as_f64().map(|f| f as usize)
                    .ok_or_else(|| format!("param {key} must be numeric")),
            }
        };
        let get_f64 = |key: &str, default: f64| -> Result<f64, String> {
            match p.get(key) {
                None => Ok(default),
                Some(v) => v.as_f64().ok_or_else(|| format!("param {key} must be numeric")),
            }
        };
        let s = match spec.kind.as_str() {
            "hold" => Strategy::Hold,
            "ma" => {
                let fast = get_usize("fast", 24)?;
                let slow = get_usize("slow", 96)?;
                if fast < 1 || slow < 2 || fast >= slow {
                    return Err("require 1 <= fast < slow".into());
                }
                Strategy::MaCross { fast, slow }
            }
            "donchian" => {
                let lookback = get_usize("lookback", 48)?;
                if lookback < 2 { return Err("lookback must be >= 2".into()); }
                Strategy::Donchian { lookback }
            }
            "rsi" => {
                let period = get_usize("period", 14)?;
                let entry = get_f64("entry", 30.0)?;
                let exit = get_f64("exit", 55.0)?;
                if period < 2 { return Err("period must be >= 2".into()); }
                if !(0.0 < entry && entry < exit && exit < 100.0) {
                    return Err("require 0 < entry < exit < 100".into());
                }
                Strategy::Rsi { period, entry, exit }
            }
            other => return Err(format!("unknown strategy {other:?}")),
        };
        Ok(s)
    }

    /// Target weight per bar in [0, 1], same length as candles, no lookahead.
    pub fn target_weights(&self, candles: &[Candle]) -> Vec<f64> {
        let n = candles.len();
        let close: Vec<f64> = candles.iter().map(|c| c.close).collect();
        match self {
            Strategy::Hold => vec![1.0; n],
            Strategy::MaCross { fast, slow } => {
                let f = rolling_mean(&close, *fast);
                let s = rolling_mean(&close, *slow);
                (0..n).map(|i| match (f[i], s[i]) {
                    (Some(fv), Some(sv)) if fv > sv => 1.0,
                    _ => 0.0,
                }).collect()
            }
            Strategy::Donchian { lookback } => {
                let mut w = vec![0.0; n];
                let mut state = 0.0;
                for i in 0..n {
                    if i + 1 >= *lookback {
                        let win = &close[i + 1 - lookback..=i];
                        let hi = win.iter().cloned().fold(f64::MIN, f64::max);
                        let lo = win.iter().cloned().fold(f64::MAX, f64::min);
                        if close[i] >= hi { state = 1.0; }
                        else if close[i] <= lo { state = 0.0; }
                    }
                    w[i] = state;
                }
                w
            }
            Strategy::Rsi { period, entry, exit } => {
                let rsi = wilder_rsi(&close, *period);
                let mut w = vec![0.0; n];
                let mut state = 0.0;
                for i in 0..n {
                    match rsi[i] {
                        Some(r) => {
                            if r <= *entry { state = 1.0; }
                            else if r >= *exit { state = 0.0; }
                            w[i] = state;
                        }
                        None => w[i] = 0.0,
                    }
                }
                w
            }
        }
    }
}

/// Apply volatility targeted sizing on top of base weights.
pub fn apply_vol_target(
    weights: &mut [f64],
    candles: &[Candle],
    target_annual_vol: f64,
    window: usize,
    periods_per_year: f64,
    max_scale: f64,
) {
    let close: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let rets: Vec<f64> = std::iter::once(0.0)
        .chain(close.windows(2).map(|w| w[1] / w[0] - 1.0))
        .collect();
    for i in 0..weights.len() {
        if i + 1 < window {
            weights[i] = 0.0;
            continue;
        }
        let win = &rets[i + 1 - window..=i];
        let mean = win.iter().sum::<f64>() / win.len() as f64;
        let var = win.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (win.len() - 1) as f64;
        let ann = var.sqrt() * periods_per_year.sqrt();
        let scale = if ann > 0.0 { (target_annual_vol / ann).min(max_scale) } else { 0.0 };
        weights[i] = (weights[i] * scale).clamp(-max_scale, max_scale);
    }
}

fn rolling_mean(xs: &[f64], window: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; xs.len()];
    if window == 0 { return out; }
    let mut sum = 0.0;
    for i in 0..xs.len() {
        sum += xs[i];
        if i >= window { sum -= xs[i - window]; }
        if i + 1 >= window { out[i] = Some(sum / window as f64); }
    }
    out
}

fn wilder_rsi(close: &[f64], period: usize) -> Vec<Option<f64>> {
    let n = close.len();
    let mut out = vec![None; n];
    if n < period + 1 { return out; }
    let alpha = 1.0 / period as f64;
    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for i in 1..n {
        let delta = close[i] - close[i - 1];
        let gain = delta.max(0.0);
        let loss = (-delta).max(0.0);
        if i == 1 { avg_gain = gain; avg_loss = loss; }
        else {
            avg_gain = alpha * gain + (1.0 - alpha) * avg_gain;
            avg_loss = alpha * loss + (1.0 - alpha) * avg_loss;
        }
        if i >= period {
            out[i] = Some(if avg_loss > 0.0 {
                100.0 - 100.0 / (1.0 + avg_gain / avg_loss)
            } else { 100.0 });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn candles(prices: &[f64]) -> Vec<Candle> {
        prices.iter().enumerate().map(|(i, &p)| Candle {
            time: Utc.timestamp_opt(1_700_000_000 + i as i64 * 3600, 0).unwrap(),
            open: p, high: p, low: p, close: p, volume: 1.0,
        }).collect()
    }

    #[test]
    fn spec_validation() {
        let bad = StrategySpec { kind: "ma".into(), params: serde_json::json!({"fast": 96, "slow": 24}).as_object().unwrap().clone(), vol_target: None };
        assert!(Strategy::from_spec(&bad).is_err());
        assert!(Strategy::from_spec(&StrategySpec { kind: "nope".into(), params: Default::default(), vol_target: None }).is_err());
    }

    #[test]
    fn ma_cross_goes_long_in_uptrend() {
        let prices: Vec<f64> = (0..200).map(|i| 100.0 + i as f64).collect();
        let c = candles(&prices);
        let s = Strategy::MaCross { fast: 5, slow: 20 };
        let w = s.target_weights(&c);
        assert_eq!(w.len(), c.len());
        assert_eq!(w[199], 1.0);
        assert_eq!(w[3], 0.0); // before windows exist, flat
    }

    #[test]
    fn weights_bounded_and_finite() {
        let prices: Vec<f64> = (0..300).map(|i| 100.0 + (i as f64 * 0.7).sin() * 10.0).collect();
        let c = candles(&prices);
        for s in [Strategy::Donchian { lookback: 20 }, Strategy::Rsi { period: 14, entry: 30.0, exit: 55.0 }] {
            let w = s.target_weights(&c);
            assert!(w.iter().all(|x| x.is_finite() && (0.0..=1.0).contains(x)), "{}", s.name());
        }
    }

    #[test]
    fn vol_target_shrinks_in_violent_markets() {
        let calm: Vec<f64> = (0..300).map(|i| 100.0 * 1.0001f64.powi(i)).collect();
        let wild: Vec<f64> = (0..300).map(|i| {
            let step: f64 = if i % 2 == 0 { 1.05 } else { 0.955 };
            100.0 * step.powi(i / 2)
        }).collect();
        let (cc, cw) = (candles(&calm), candles(&wild));
        let mut w1 = vec![1.0; 300];
        let mut w2 = vec![1.0; 300];
        apply_vol_target(&mut w1, &cc, 0.3, 48, 8760.0, 1.0);
        apply_vol_target(&mut w2, &cw, 0.3, 48, 8760.0, 1.0);
        assert!(w2[299] < w1[299]);
    }
}
