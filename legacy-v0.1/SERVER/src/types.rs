//! Shared types. Author: Aaron Stovall · Version 0.1.0 · 2026-07-07

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Candle {
    pub time: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Metrics {
    pub total_return: f64,
    pub cagr: f64,
    pub ann_volatility: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub max_drawdown: f64,
    pub calmar: f64,
    pub n_periods: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct EquityPoint {
    pub time: DateTime<Utc>,
    pub value: f64,
}

pub fn downsample(points: &[EquityPoint], max_points: usize) -> Vec<EquityPoint> {
    if points.len() <= max_points || points.is_empty() {
        return points.to_vec();
    }
    let step = points.len() / max_points;
    let mut out: Vec<EquityPoint> = points.iter().step_by(step.max(1)).cloned().collect();
    let last = points[points.len() - 1].clone();
    if out.last().map(|p| p.time) != Some(last.time) {
        out.push(last);
    }
    out
}
