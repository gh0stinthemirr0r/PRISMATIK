//! Performance statistics for backtest evaluation.
//!
//! All functions are pure, deterministic, and use numerically stable
//! single-pass algorithms (Welford for variance, Neumaier compensated
//! summation for totals). Non-associative float addition is avoided by
//! construction — there is no parallelism and no work-stealing here, so
//! identical inputs always produce byte-identical outputs across machines
//! (invariant I3).
//!
//! Annualization convention: the caller supplies `periods_per_year`, which
//! must be derived from the bar interval and the trading calendar of the
//! underlying asset class. The helpers in [`crate::simulation`] compute this
//! automatically.

/// Neumaier-compensated sum — strictly more accurate than naive summation
/// and deterministic (single-threaded, fixed order).
fn compensated_sum(values: &[f64]) -> f64 {
    let mut sum = 0.0_f64;
    let mut c = 0.0_f64; // compensation
    for &value in values {
        let t = sum + value;
        if sum.abs() >= value.abs() {
            c += (sum - t) + value;
        } else {
            c += (value - t) + sum;
        }
        sum = t;
    }
    sum + c
}

/// Arithmetic mean using compensated summation.
fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    compensated_sum(values) / (values.len() as f64)
}

/// Population standard deviation (Welford online algorithm).
fn std_dev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let n = values.len() as f64;
    let mut m = 0.0_f64; // running mean
    let mut s = 0.0_f64; // running M2
    for (k, &x) in values.iter().enumerate() {
        let k1 = (k as f64) + 1.0;
        let delta = x - m;
        m += delta / k1;
        s += delta * (x - m);
    }
    (s / n).sqrt()
}

/// Downside deviation — root-mean-square of returns below `mar` (minimum
/// acceptable return, typically 0).
fn downside_deviation(returns: &[f64], mar: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let downsides: Vec<f64> = returns
        .iter()
        .map(|&r| if r < mar { mar - r } else { 0.0 })
        .collect();
    let n = returns.len() as f64;
    (compensated_sum(&downsides.iter().map(|d| d * d).collect::<Vec<_>>()) / n).sqrt()
}

/// Annualized Sharpe ratio from a per-period return series.
///
/// `periods_per_year` must match the return series frequency (252 for daily
/// equity, 365 for daily crypto, etc.). Risk-free rate defaults to 0; a
/// constant per-period risk-free rate can be supplied to subtract drift.
pub fn sharpe_ratio(returns: &[f64], periods_per_year: f64, rf_per_period: f64) -> f64 {
    if returns.is_empty() || periods_per_year <= 0.0 {
        return 0.0;
    }
    let excess: Vec<f64> = returns.iter().map(|&r| r - rf_per_period).collect();
    let mu = mean(&excess);
    let sigma = std_dev(&excess);
    if sigma == 0.0 {
        return 0.0;
    }
    (mu / sigma) * periods_per_year.sqrt()
}

/// Annualized Sortino ratio — like Sharpe but only penalizes downside
/// volatility.
pub fn sortino_ratio(returns: &[f64], periods_per_year: f64, rf_per_period: f64) -> f64 {
    if returns.is_empty() || periods_per_year <= 0.0 {
        return 0.0;
    }
    let excess: Vec<f64> = returns.iter().map(|&r| r - rf_per_period).collect();
    let mu = mean(&excess);
    let dd = downside_deviation(returns, rf_per_period);
    if dd == 0.0 {
        return 0.0;
    }
    (mu / dd) * periods_per_year.sqrt()
}

/// Maximum drawdown of an equity curve, returned as a positive fraction
/// (0.25 = 25% peak-to-trough decline).
pub fn max_drawdown(equity_curve: &[f64]) -> f64 {
    if equity_curve.is_empty() {
        return 0.0;
    }
    let mut peak = equity_curve[0];
    let mut max_dd = 0.0_f64;
    for &value in equity_curve {
        if value > peak {
            peak = value;
        }
        if peak > 0.0 {
            let dd = (peak - value) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

/// Calmar ratio — annualized return divided by maximum drawdown.
pub fn calmar_ratio(annualized_return: f64, max_dd: f64) -> f64 {
    if max_dd <= 0.0 {
        return 0.0;
    }
    annualized_return / max_dd
}

/// Annualized return from total return and number of periods.
pub fn annualized_return(total_return: f64, periods: usize, periods_per_year: f64) -> f64 {
    if periods == 0 || total_return <= -1.0 {
        return 0.0;
    }
    let years = (periods as f64) / periods_per_year;
    if years <= 0.0 {
        return 0.0;
    }
    // CAGR = (1 + total_return)^(1/years) - 1
    let base = 1.0 + total_return;
    if base <= 0.0 {
        return -1.0;
    }
    base.powf(1.0 / years) - 1.0
}

/// Profit factor — gross profit divided by gross loss (absolute value).
/// Returns `f64::INFINITY` if there are no losing trades.
pub fn profit_factor(trade_pnl: &[f64]) -> f64 {
    let gross_profit = compensated_sum(
        &trade_pnl
            .iter()
            .filter(|&&p| p > 0.0)
            .copied()
            .collect::<Vec<_>>(),
    );
    let gross_loss = compensated_sum(
        &trade_pnl
            .iter()
            .filter(|&&p| p < 0.0)
            .map(|p| -p)
            .collect::<Vec<_>>(),
    );
    if gross_loss == 0.0 {
        return if gross_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };
    }
    gross_profit / gross_loss
}

/// Win rate — fraction of trades with positive P&L (0.0 to 1.0).
pub fn win_rate(trade_pnl: &[f64]) -> f64 {
    if trade_pnl.is_empty() {
        return 0.0;
    }
    let winners = trade_pnl.iter().filter(|&&p| p > 0.0).count();
    winners as f64 / trade_pnl.len() as f64
}

/// Trading calendar kind — determines the annualization factor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarKind {
    /// US equity sessions: 252 trading days/year.
    EquityUs,
    /// US equity futures (e.g. ES): nearly 24h electronic session, 252 days.
    EquityFutureUs,
    /// Crypto: 365 days/year, 24h.
    CryptoDaily,
    /// Crypto intraday (24/7).
    CryptoIntraday,
    /// Forex: 252 days/year, 24h session.
    Forex,
}

impl CalendarKind {
    /// Trading periods per year for the given bar interval (in seconds).
    pub fn periods_per_year(&self, bar_interval_seconds: u64) -> f64 {
        if bar_interval_seconds == 0 {
            return 252.0;
        }
        let secs = bar_interval_seconds as f64;
        match self {
            // 252 trading days × 6.5h session × 3600 s/h
            CalendarKind::EquityUs => {
                let session_seconds = 6.5 * 3600.0;
                let bars_per_day = (session_seconds / secs).ceil();
                252.0 * bars_per_day
            },
            // ES futures: 252 days × ~23h electronic session
            CalendarKind::EquityFutureUs => {
                let session_seconds = 23.0 * 3600.0;
                let bars_per_day = (session_seconds / secs).ceil();
                252.0 * bars_per_day
            },
            // Crypto: 365 × 24h
            CalendarKind::CryptoDaily => {
                let day_seconds = 24.0 * 3600.0;
                let bars_per_day = (day_seconds / secs).ceil();
                365.0 * bars_per_day
            },
            CalendarKind::CryptoIntraday => {
                let day_seconds = 24.0 * 3600.0;
                let bars_per_day = (day_seconds / secs).ceil();
                365.0 * bars_per_day
            },
            // Forex: 252 × 24h
            CalendarKind::Forex => {
                let day_seconds = 24.0 * 3600.0;
                let bars_per_day = (day_seconds / secs).ceil();
                252.0 * bars_per_day
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compensated_sum_is_accurate() {
        // Large + small values — naive summation loses precision
        let values = [1e16, 1.0, -1e16, 1.0];
        let result = compensated_sum(&values);
        assert!((result - 2.0).abs() < 1e-6, "expected 2.0, got {result}");
    }

    #[test]
    fn sharpe_ratio_positive_for_consistent_positive_returns() {
        let returns = [0.001, 0.002, 0.001, 0.002, 0.001, 0.002];
        let sharpe = sharpe_ratio(&returns, 252.0, 0.0);
        assert!(
            sharpe > 0.0,
            "positive consistent returns => positive Sharpe"
        );
        // Should be a large positive number (low variance, positive mean)
        assert!(sharpe > 10.0, "Sharpe should be very high, got {sharpe}");
    }

    #[test]
    fn sharpe_ratio_zero_for_flat_series() {
        let returns = [0.0; 10];
        assert_eq!(sharpe_ratio(&returns, 252.0, 0.0), 0.0);
    }

    #[test]
    fn sortino_penalizes_only_downside() {
        let returns = [0.01, -0.02, 0.05, 0.01, 0.03];
        let sharpe = sharpe_ratio(&returns, 252.0, 0.0);
        let sortino = sortino_ratio(&returns, 252.0, 0.0);
        // Sortino should be >= Sharpe when upside variance is high
        // (it doesn't penalize the large positive return)
        assert!(
            sortino >= sharpe,
            "Sortino ({sortino}) should be >= Sharpe ({sharpe})"
        );
    }

    #[test]
    fn max_drawdown_simple_case() {
        // Peak 100 -> trough 75 -> recover 90
        let equity = [100.0, 90.0, 75.0, 85.0, 90.0];
        let dd = max_drawdown(&equity);
        assert!((dd - 0.25).abs() < 1e-9, "25% drawdown expected, got {dd}");
    }

    #[test]
    fn max_drawdown_no_drawdown() {
        let equity = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(max_drawdown(&equity), 0.0);
    }

    #[test]
    fn profit_factor_basic() {
        let trades = [100.0, -50.0, 200.0, -30.0];
        let pf = profit_factor(&trades);
        // (100 + 200) / (50 + 30) = 300/80 = 3.75
        assert!((pf - 3.75).abs() < 1e-9, "expected 3.75, got {pf}");
    }

    #[test]
    fn profit_factor_all_winners() {
        let trades = [10.0, 20.0, 30.0];
        assert!(profit_factor(&trades).is_infinite());
    }

    #[test]
    fn win_rate_half() {
        let trades = [10.0, -5.0, 20.0, -10.0];
        assert!((win_rate(&trades) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn calmar_ratio_basic() {
        let calmar = calmar_ratio(0.20, 0.10);
        assert!((calmar - 2.0).abs() < 1e-9);
    }

    #[test]
    fn calmar_zero_when_no_drawdown() {
        assert_eq!(calmar_ratio(0.20, 0.0), 0.0);
    }

    #[test]
    fn annualized_return_doubles_in_one_year() {
        // 100% total return over exactly 1 year of daily bars (252 periods)
        let ar = annualized_return(1.0, 252, 252.0);
        assert!((ar - 1.0).abs() < 1e-9, "expected 1.0, got {ar}");
    }

    #[test]
    fn calendar_periods_equity_daily() {
        // 252 days, 1 bar/day
        assert_eq!(CalendarKind::EquityUs.periods_per_year(86_400), 252.0);
    }

    #[test]
    fn calendar_periods_es_future_5min() {
        // 252 days × ceil(23*3600/300) = 252 × 276 = 69,552
        let p = CalendarKind::EquityFutureUs.periods_per_year(300);
        assert!((p - 69_552.0).abs() < 0.5, "got {p}");
    }

    #[test]
    fn calendar_periods_crypto_daily() {
        assert_eq!(CalendarKind::CryptoDaily.periods_per_year(86_400), 365.0);
    }

    #[test]
    fn determinism_same_input_same_output() {
        let returns = [0.01, -0.003, 0.005, 0.02, -0.01, 0.008];
        let s1 = sharpe_ratio(&returns, 252.0, 0.0);
        let s2 = sharpe_ratio(&returns, 252.0, 0.0);
        assert_eq!(s1.to_bits(), s2.to_bits(), "deterministic bit-identical");
    }
}
