//! Backtest runtime: exposes the simulation engine + seed strategy library
//! to the desktop UI. Lets a user select a built-in strategy, supply a bar
//! series (from terminal-feed history or a fixture), and view the full
//! metrics bundle with honest positive/non-positive Sharpe verdict.

use prismatik_backtest::{
    simulate, CalendarKind, Donchian, ExecutionAssumptions, FillModel, MeanReversion, Momentum,
    OhlcBar, OpeningRangeBreakout, Signal, SignalStrategy, SimulationResult,
};
use serde::{Deserialize, Serialize};

/// Built-in seed strategy selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeedStrategy {
    /// Opening Range Breakout (ES-futures 30-min default).
    Orb,
    /// Long-only 200-SMA trend momentum.
    Momentum,
    /// RSI(2) mean reversion in an uptrend.
    MeanReversion,
    /// Donchian(55) channel breakout.
    Donchian,
}

/// Calendar / asset-class selector for annualization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarSelector {
    /// US equity daily/intraday.
    EquityUs,
    /// US equity futures (ES, NQ, etc.).
    EquityFutureUs,
    /// Crypto spot (24/7).
    CryptoDaily,
    /// Forex (24h session).
    Forex,
}

impl CalendarSelector {
    fn to_kind(self) -> CalendarKind {
        match self {
            CalendarSelector::EquityUs => CalendarKind::EquityUs,
            CalendarSelector::EquityFutureUs => CalendarKind::EquityFutureUs,
            CalendarSelector::CryptoDaily => CalendarKind::CryptoDaily,
            CalendarSelector::Forex => CalendarKind::Forex,
        }
    }
}

/// Backtest request from the UI.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestRequest {
    /// Which seed strategy to run.
    pub strategy: SeedStrategy,
    /// OHLCV bars (chronological). May come from terminal-feed history or a
    /// user-supplied series.
    pub bars: Vec<BarInput>,
    /// Starting capital in USD.
    pub starting_capital: f64,
    /// Calendar for annualization.
    pub calendar: CalendarSelector,
    /// Bar interval in seconds.
    pub bar_interval_seconds: u64,
    /// Slippage in basis points.
    pub slippage_bps: Option<u32>,
    /// Commission per share in currency micros.
    pub commission_per_share_micros: Option<u64>,
}

/// OHLCV bar input from the UI.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BarInput {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Option<f64>,
    pub timestamp_secs: Option<i64>,
}

impl From<BarInput> for OhlcBar {
    fn from(b: BarInput) -> Self {
        OhlcBar {
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume.unwrap_or(0.0),
            timestamp_secs: b.timestamp_secs,
        }
    }
}

/// Backtest result view for the UI.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestResultView {
    pub strategy_id: String,
    pub bars_processed: usize,
    pub starting_capital: f64,
    pub ending_equity: f64,
    pub total_return: f64,
    pub annualized_return: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub calmar: f64,
    pub max_drawdown: f64,
    pub profit_factor: f64,
    pub win_rate: f64,
    pub trade_count: usize,
    pub fill_count: usize,
    pub verdict: String,
    pub message: String,
    /// Sampled equity curve (every Nth point to keep payload small).
    pub equity_curve_sampled: Vec<EquityPointView>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityPointView {
    pub bar_index: usize,
    pub equity: f64,
    pub position: f64,
}

fn select_strategy(seed: SeedStrategy) -> Box<dyn SignalStrategy> {
    match seed {
        SeedStrategy::Orb => Box::new(OpeningRangeBreakout::es_default()),
        SeedStrategy::Momentum => Box::new(Momentum::default_equity()),
        SeedStrategy::MeanReversion => Box::new(MeanReversion::default_equity()),
        SeedStrategy::Donchian => Box::new(Donchian::default_equity()),
    }
}

/// Run a backtest over the supplied bar series with the selected seed
/// strategy. Returns the full metrics bundle. No synthetic data is ever
/// substituted for missing bars — an empty series is an error.
#[tauri::command]
pub(crate) async fn run_backtest(req: BacktestRequest) -> Result<BacktestResultView, String> {
    if req.bars.len() < 2 {
        return Err("backtest requires at least 2 bars".into());
    }
    if !req.starting_capital.is_finite() || req.starting_capital <= 0.0 {
        return Err("starting capital must be positive".into());
    }
    if req.bar_interval_seconds == 0 {
        return Err("bar interval must be positive".into());
    }
    // Validate bars are well-formed.
    for (i, bar) in req.bars.iter().enumerate() {
        if !bar.close.is_finite() || bar.close <= 0.0 {
            return Err(format!("bar {i} has invalid close price"));
        }
        if bar.high < bar.low {
            return Err(format!("bar {i} has high < low"));
        }
    }
    let strategy = select_strategy(req.strategy);
    let bars: Vec<OhlcBar> = req.bars.into_iter().map(Into::into).collect();
    let execution = ExecutionAssumptions {
        fill_model: FillModel::Close,
        slippage_bps: req.slippage_bps.unwrap_or(2),
        commission_per_share_micros: req.commission_per_share_micros.unwrap_or(500),
        assignment_enabled: false,
    };
    let result: SimulationResult = simulate(
        &bars,
        strategy.as_ref(),
        req.starting_capital,
        &execution,
        req.calendar.to_kind(),
        req.bar_interval_seconds,
    )
    .map_err(|error| format!("backtest failed: {error}"))?;
    let message = if result.sharpe > 0.0 {
        format!(
            "Positive Sharpe ({:.2}). Strategy earned {:.2}% return with {:.2}% max drawdown over {} bars.",
            result.sharpe,
            result.total_return * 100.0,
            result.max_drawdown * 100.0,
            result.bars_processed
        )
    } else {
        format!(
            "Non-positive Sharpe ({:.2}). This strategy does not show an edge on this data — review the regime fit.",
            result.sharpe
        )
    };
    // Sample the equity curve to at most 200 points for UI payload.
    let equity_curve_sampled = sample_equity_curve(&result.equity_curve, 200);
    Ok(BacktestResultView {
        strategy_id: result.strategy_id,
        bars_processed: result.bars_processed,
        starting_capital: result.starting_capital,
        ending_equity: result.ending_equity,
        total_return: result.total_return,
        annualized_return: result.annualized_return,
        sharpe: result.sharpe,
        sortino: result.sortino,
        calmar: result.calmar,
        max_drawdown: result.max_drawdown,
        profit_factor: result.profit_factor,
        win_rate: result.win_rate,
        trade_count: result.trade_count,
        fill_count: result.fill_count,
        verdict: result.verdict.clone(),
        message,
        equity_curve_sampled,
    })
}

fn sample_equity_curve(
    curve: &[prismatik_backtest::EquityPoint],
    max_points: usize,
) -> Vec<EquityPointView> {
    if curve.len() <= max_points {
        return curve
            .iter()
            .map(|p| EquityPointView {
                bar_index: p.bar_index,
                equity: p.equity,
                position: p.position,
            })
            .collect();
    }
    let step = curve.len().div_ceil(max_points);
    curve
        .iter()
        .step_by(step)
        .map(|p| EquityPointView {
            bar_index: p.bar_index,
            equity: p.equity,
            position: p.position,
        })
        .collect()
}

/// List the available seed strategies with metadata for the UI.
#[tauri::command]
pub(crate) async fn list_seed_strategies() -> Result<Vec<SeedStrategyInfo>, String> {
    Ok(vec![
        SeedStrategyInfo {
            id: "orb".into(),
            kind: SeedStrategy::Orb,
            name: "Opening Range Breakout".into(),
            description: "ES-futures 30-minute opening-range breakout. Long above the OR high, short below the OR low, flatten at session close. The classic intraday momentum strategy.".into(),
            asset_class: "futures".into(),
            best_regime: "trending intraday sessions with a clean opening range".into(),
        },
        SeedStrategyInfo {
            id: "momentum".into(),
            kind: SeedStrategy::Momentum,
            name: "Trend Momentum (200-SMA)".into(),
            description: "Long-only when price is above its 200-period SMA. The most robust trend-following filter — captures regime drift while sitting out downtrends.".into(),
            asset_class: "multi-asset".into(),
            best_regime: "regime-shifting series with sustained trends".into(),
        },
        SeedStrategyInfo {
            id: "mean_reversion".into(),
            kind: SeedStrategy::MeanReversion,
            name: "RSI(2) Mean Reversion".into(),
            description: "Long when RSI(2) falls below 10 in an uptrend (price above 200-SMA). Hold through the recovery band. Profits from overshoots in established uptrends.".into(),
            asset_class: "equity".into(),
            best_regime: "mean-reverting series with strong pull-to-mean".into(),
        },
        SeedStrategyInfo {
            id: "donchian".into(),
            kind: SeedStrategy::Donchian,
            name: "Donchian(55) Breakout".into(),
            description: "Long when price breaks above the 55-bar high, exit on the 55-bar low. The turtle-system long-period entry — filters noise that destroys shorter channels.".into(),
            asset_class: "multi-asset".into(),
            best_regime: "strong directional trends with low noise".into(),
        },
    ])
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedStrategyInfo {
    pub id: String,
    pub kind: SeedStrategy,
    pub name: String,
    pub description: String,
    pub asset_class: String,
    pub best_regime: String,
}

// Used only to keep the Signal import live for documentation of the strategy
// surface; the actual signal evaluation happens inside the backtest crate.
#[allow(dead_code)]
fn _signal_doc_anchor() -> Signal {
    Signal::Flat
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uptrend(n: usize) -> Vec<BarInput> {
        (0..n)
            .map(|i| BarInput {
                open: 99.0 + i as f64,
                high: 101.0 + i as f64,
                low: 98.0 + i as f64,
                close: 100.0 + i as f64,
                volume: Some(1000.0),
                timestamp_secs: None,
            })
            .collect()
    }

    #[tokio::test]
    async fn run_backtest_on_uptrend_is_positive() {
        let req = BacktestRequest {
            strategy: SeedStrategy::Momentum,
            bars: uptrend(300),
            starting_capital: 100_000.0,
            calendar: CalendarSelector::EquityUs,
            bar_interval_seconds: 86_400,
            slippage_bps: Some(2),
            commission_per_share_micros: Some(500),
        };
        let result = run_backtest(req).await.unwrap();
        assert!(result.sharpe > 0.0, "expected positive Sharpe");
        assert_eq!(result.verdict, "positive_sharpe");
        assert!(result.total_return > 0.0);
    }

    #[tokio::test]
    async fn run_backtest_rejects_empty_series() {
        let req = BacktestRequest {
            strategy: SeedStrategy::Momentum,
            bars: vec![],
            starting_capital: 100_000.0,
            calendar: CalendarSelector::EquityUs,
            bar_interval_seconds: 86_400,
            slippage_bps: None,
            commission_per_share_micros: None,
        };
        assert!(run_backtest(req).await.is_err());
    }

    #[tokio::test]
    async fn list_strategies_returns_four() {
        let list = list_seed_strategies().await.unwrap();
        assert_eq!(list.len(), 4);
        assert!(list.iter().any(|s| s.id == "orb"));
    }
}
