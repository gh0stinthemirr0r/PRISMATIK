//! Chart indicator computation — exposes the native indicator library to the
//! desktop chart for on-chart overlays.
//!
//! Computes indicator series from a bar array using the deterministic Rust
//! indicator crate (`prismatik-indicator-core`). The chart calls this when
//! the displayed candle series changes (symbol/timeframe switch, new bar),
//! caches the result, and draws from cache each frame — no per-frame IPC.

use prismatik_indicator_core::{
    AtrIndicator, Bar, BollingerLowerIndicator, BollingerMidIndicator, BollingerUpperIndicator,
    EmaIndicator, Indicator, MacdHistIndicator, MacdLineIndicator, MacdSignalIndicator,
    RsiIndicator, SmaIndicator, StochasticDIndicator, StochasticKIndicator, VwapIndicator,
};
use serde::{Deserialize, Serialize};

/// Chart bar input (matches the JS `Candle` shape with single-letter keys
/// mapped to full names for clarity over IPC).
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartBar {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl From<ChartBar> for Bar {
    fn from(b: ChartBar) -> Self {
        Bar {
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
        }
    }
}

/// Indicator kind selector (mirrors the most chart-useful subset of the
/// native catalog).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartIndicatorKind {
    /// Simple moving average (price-pane overlay).
    Sma,
    /// Exponential moving average (price-pane overlay).
    Ema,
    /// Bollinger upper band.
    BbUpper,
    /// Bollinger middle band (the SMA basis).
    BbMid,
    /// Bollinger lower band.
    BbLower,
    /// Volume-weighted average price.
    Vwap,
    /// RSI — rendered in a sub-pane (oscillator).
    Rsi,
    /// MACD line — sub-pane.
    MacdLine,
    /// MACD signal — sub-pane.
    MacdSignal,
    /// MACD histogram — sub-pane.
    MacdHist,
    /// Stochastic %K — sub-pane.
    StochK,
    /// Stochastic %D — sub-pane.
    StochD,
    /// ATR — sub-pane (volatility).
    Atr,
}

/// Where the indicator renders: on the price pane or in a sub-pane below.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorPane {
    /// Overlaid on the price chart (SMA, EMA, BB, VWAP).
    Price,
    /// Drawn in a separate oscillator pane below (RSI, MACD, Stoch, ATR).
    Sub,
}

impl ChartIndicatorKind {
    /// Which pane this indicator belongs on.
    pub fn pane(self) -> IndicatorPane {
        match self {
            ChartIndicatorKind::Sma
            | ChartIndicatorKind::Ema
            | ChartIndicatorKind::BbUpper
            | ChartIndicatorKind::BbMid
            | ChartIndicatorKind::BbLower
            | ChartIndicatorKind::Vwap => IndicatorPane::Price,
            ChartIndicatorKind::Rsi
            | ChartIndicatorKind::MacdLine
            | ChartIndicatorKind::MacdSignal
            | ChartIndicatorKind::MacdHist
            | ChartIndicatorKind::StochK
            | ChartIndicatorKind::StochD
            | ChartIndicatorKind::Atr => IndicatorPane::Sub,
        }
    }

    /// Display label.
    #[allow(dead_code, reason = "used by the frontend via the label field")]
    pub fn label(self) -> &'static str {
        match self {
            ChartIndicatorKind::Sma => "SMA",
            ChartIndicatorKind::Ema => "EMA",
            ChartIndicatorKind::BbUpper => "BB Upper",
            ChartIndicatorKind::BbMid => "BB Mid",
            ChartIndicatorKind::BbLower => "BB Lower",
            ChartIndicatorKind::Vwap => "VWAP",
            ChartIndicatorKind::Rsi => "RSI",
            ChartIndicatorKind::MacdLine => "MACD Line",
            ChartIndicatorKind::MacdSignal => "MACD Signal",
            ChartIndicatorKind::MacdHist => "MACD Histogram",
            ChartIndicatorKind::StochK => "Stochastic %K",
            ChartIndicatorKind::StochD => "Stochastic %D",
            ChartIndicatorKind::Atr => "ATR",
        }
    }
}

/// A single indicator computation request.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorRequest {
    pub kind: ChartIndicatorKind,
    /// Period (ignored for VWAP which is cumulative).
    pub period: usize,
}

/// The computed indicator series. `values[i]` corresponds to `bars[i]`;
/// `null` during warmup.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorSeries {
    pub kind: ChartIndicatorKind,
    pub label: String,
    pub period: usize,
    pub pane: IndicatorPane,
    /// One value per input bar; `None` during warmup.
    pub values: Vec<Option<f64>>,
}

/// Compute one or more indicator series from a bar array. Pure and
/// deterministic — same bars + requests always produce identical output.
#[tauri::command]
pub(crate) async fn compute_chart_indicators(
    bars: Vec<ChartBar>,
    requests: Vec<IndicatorRequest>,
) -> Result<Vec<IndicatorSeries>, String> {
    if bars.is_empty() {
        return Ok(Vec::new());
    }
    let rust_bars: Vec<Bar> = bars.into_iter().map(Into::into).collect();
    let mut results = Vec::with_capacity(requests.len());
    for req in requests {
        if let Some(series) = compute_one(&rust_bars, req) {
            results.push(series);
        }
    }
    Ok(results)
}

fn compute_one(bars: &[Bar], req: IndicatorRequest) -> Option<IndicatorSeries> {
    let period = req.period.max(1);
    let kind = req.kind;
    let label = format!("{} ({period})", kind.label());
    let values: Vec<Option<f64>> = match kind {
        ChartIndicatorKind::Sma => series(bars, period, |p| SmaIndicator::new(p).ok()),
        ChartIndicatorKind::Ema => series(bars, period, |p| EmaIndicator::new(p).ok()),
        ChartIndicatorKind::BbUpper => {
            series(bars, period, |p| BollingerUpperIndicator::new(p).ok())
        },
        ChartIndicatorKind::BbMid => series(bars, period, |p| BollingerMidIndicator::new(p).ok()),
        ChartIndicatorKind::BbLower => {
            series(bars, period, |p| BollingerLowerIndicator::new(p).ok())
        },
        ChartIndicatorKind::Vwap => {
            // VWAP is cumulative over the series; use a large period.
            series(bars, bars.len(), |p| VwapIndicator::new(p).ok())
        },
        ChartIndicatorKind::Rsi => series(bars, period, |p| RsiIndicator::new(p).ok()),
        ChartIndicatorKind::MacdLine => series(bars, period, |p| MacdLineIndicator::new(p).ok()),
        ChartIndicatorKind::MacdSignal => {
            series(bars, period, |p| MacdSignalIndicator::new(p).ok())
        },
        ChartIndicatorKind::MacdHist => series(bars, period, |p| MacdHistIndicator::new(p).ok()),
        ChartIndicatorKind::StochK => series(bars, period, |p| StochasticKIndicator::new(p).ok()),
        ChartIndicatorKind::StochD => series(bars, period, |p| StochasticDIndicator::new(p).ok()),
        ChartIndicatorKind::Atr => series(bars, period, |p| AtrIndicator::new(p).ok()),
    };
    if values.is_empty() {
        return None;
    }
    Some(IndicatorSeries {
        kind,
        label,
        period,
        pane: kind.pane(),
        values,
    })
}

/// Compute a streaming indicator series: for each bar index, feed bars up to
/// and including that index and record the output. Pushes `None` during
/// warmup (when `next()` errors) — no need to guess the boundary.
fn series<I: Indicator>(
    bars: &[Bar],
    period: usize,
    make: impl Fn(usize) -> Option<I>,
) -> Vec<Option<f64>> {
    let mut indicator = match make(period) {
        Some(i) => i,
        None => return Vec::new(),
    };
    let mut out = Vec::with_capacity(bars.len());
    for bar in bars {
        match indicator.next(bar) {
            Ok(v) if v.is_finite() => out.push(Some(v)),
            _ => out.push(None),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars(n: usize) -> Vec<ChartBar> {
        (0..n)
            .map(|i| {
                let p = 100.0 + i as f64;
                ChartBar {
                    open: p - 0.5,
                    high: p + 0.5,
                    low: p - 1.0,
                    close: p,
                    volume: 1000.0,
                }
            })
            .collect()
    }

    #[tokio::test]
    async fn sma_series_has_warmup_then_values() {
        let req = IndicatorRequest {
            kind: ChartIndicatorKind::Sma,
            period: 5,
        };
        let result = compute_chart_indicators(bars(20), vec![req]).await.unwrap();
        assert_eq!(result.len(), 1);
        let s = &result[0];
        assert_eq!(s.kind, ChartIndicatorKind::Sma);
        assert_eq!(s.values.len(), 20);
        // Early bars are None (warmup), later bars have values.
        assert!(s.values[0].is_none(), "first bar should be in warmup");
        let some_count = s.values.iter().filter(|v| v.is_some()).count();
        assert!(some_count > 0, "should produce values after warmup");
        assert!(some_count < 20, "should have warmup period");
        // The last value should be a real finite number.
        assert!(s.values.last().unwrap().is_some());
    }

    #[tokio::test]
    async fn bb_upper_above_mid_above_lower() {
        let reqs = vec![
            IndicatorRequest {
                kind: ChartIndicatorKind::BbUpper,
                period: 10,
            },
            IndicatorRequest {
                kind: ChartIndicatorKind::BbMid,
                period: 10,
            },
            IndicatorRequest {
                kind: ChartIndicatorKind::BbLower,
                period: 10,
            },
        ];
        let result = compute_chart_indicators(bars(30), reqs).await.unwrap();
        assert_eq!(result.len(), 3);
        let upper = result[0].values[25].unwrap();
        let mid = result[1].values[25].unwrap();
        let lower = result[2].values[25].unwrap();
        assert!(upper > mid, "upper {upper} should be > mid {mid}");
        assert!(mid > lower, "mid {mid} should be > lower {lower}");
    }

    #[tokio::test]
    async fn rsi_in_zero_to_hundred_range() {
        let req = IndicatorRequest {
            kind: ChartIndicatorKind::Rsi,
            period: 14,
        };
        let result = compute_chart_indicators(bars(30), vec![req]).await.unwrap();
        let vals: Vec<f64> = result[0].values.iter().flatten().copied().collect();
        assert!(!vals.is_empty());
        for v in &vals {
            assert!(*v >= 0.0 && *v <= 100.0, "RSI {v} out of range");
        }
    }

    #[tokio::test]
    async fn empty_bars_returns_empty() {
        let result = compute_chart_indicators(vec![], vec![]).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn pane_classification_correct() {
        assert_eq!(ChartIndicatorKind::Sma.pane(), IndicatorPane::Price);
        assert_eq!(ChartIndicatorKind::Vwap.pane(), IndicatorPane::Price);
        assert_eq!(ChartIndicatorKind::Rsi.pane(), IndicatorPane::Sub);
        assert_eq!(ChartIndicatorKind::MacdHist.pane(), IndicatorPane::Sub);
    }

    #[tokio::test]
    async fn determinism_same_input_same_output() {
        let req = IndicatorRequest {
            kind: ChartIndicatorKind::Ema,
            period: 12,
        };
        let b = bars(25);
        let r1 = compute_chart_indicators(b.clone(), vec![req])
            .await
            .unwrap();
        let r2 = compute_chart_indicators(
            b,
            vec![IndicatorRequest {
                kind: ChartIndicatorKind::Ema,
                period: 12,
            }],
        )
        .await
        .unwrap();
        assert_eq!(r1[0].values.len(), r2[0].values.len());
        for (a, b) in r1[0].values.iter().zip(r2[0].values.iter()) {
            assert_eq!(a.is_some(), b.is_some());
            if let (Some(a), Some(b)) = (a, b) {
                assert!((a - b).abs() < 1e-12, "non-deterministic EMA");
            }
        }
    }
}
