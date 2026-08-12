//! Native deterministic technical indicators with equivalent batch and stream paths.

use crate::{Bar, Indicator, IndicatorDescriptor, IndicatorError};
use std::sync::LazyLock;

fn validate_period(period: usize) -> Result<(), IndicatorError> {
    if period == 0 {
        Err(IndicatorError::InvalidInput(
            "period must be positive".into(),
        ))
    } else {
        Ok(())
    }
}

fn tail(bars: &[Bar], period: usize) -> Option<&[Bar]> {
    (bars.len() >= period).then(|| &bars[bars.len() - period..])
}

fn closes(bars: &[Bar], period: usize) -> Option<Vec<f64>> {
    tail(bars, period).map(|rows| rows.iter().map(|bar| bar.close).collect())
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn ema(values: impl IntoIterator<Item = f64>, period: usize) -> f64 {
    let alpha = 2.0 / (period as f64 + 1.0);
    let mut values = values.into_iter();
    let Some(mut result) = values.next() else {
        return f64::NAN;
    };
    for value in values {
        result = value * alpha + result * (1.0 - alpha);
    }
    result
}

fn standard_deviation(values: &[f64]) -> f64 {
    let average = mean(values);
    (values
        .iter()
        .map(|value| (value - average).powi(2))
        .sum::<f64>()
        / values.len() as f64)
        .sqrt()
}

macro_rules! periodic_indicator {
    ($name:ident, $id:literal, $label:literal, $compute:ident) => {
        #[doc = concat!($label, " indicator.")]
        #[derive(Clone, Debug)]
        pub struct $name {
            period: usize,
            history: Vec<Bar>,
        }

        impl $name {
            #[doc = concat!("Construct a ", $label, " indicator.")]
            pub fn new(period: usize) -> Result<Self, IndicatorError> {
                validate_period(period)?;
                Ok(Self {
                    period,
                    history: Vec::new(),
                })
            }
        }

        impl Indicator for $name {
            fn descriptor(&self) -> &IndicatorDescriptor {
                static DESCRIPTOR: LazyLock<IndicatorDescriptor> =
                    LazyLock::new(|| IndicatorDescriptor {
                        id: $id.into(),
                        name: $label.into(),
                    });
                &DESCRIPTOR
            }

            fn warmup_len(&self) -> usize {
                self.period
            }

            fn evaluate(&self, window: &[Bar]) -> Result<f64, IndicatorError> {
                Ok($compute(window, self.period))
            }

            fn next(&mut self, bar: &Bar) -> Result<f64, IndicatorError> {
                self.history.push(*bar);
                Ok($compute(&self.history, self.period))
            }
        }
    };
}

fn sma_value(bars: &[Bar], period: usize) -> f64 {
    closes(bars, period).map_or(f64::NAN, |values| mean(&values))
}

fn ema_value(bars: &[Bar], period: usize) -> f64 {
    if bars.len() < period {
        f64::NAN
    } else {
        ema(bars.iter().map(|bar| bar.close), period)
    }
}

fn rsi_value(bars: &[Bar], period: usize) -> f64 {
    if bars.len() <= period {
        return f64::NAN;
    }
    let rows = &bars[bars.len() - period - 1..];
    let (gain, loss) = rows.windows(2).fold((0.0, 0.0), |(gain, loss), pair| {
        let change = pair[1].close - pair[0].close;
        (gain + change.max(0.0), loss + (-change).max(0.0))
    });
    if loss == 0.0 {
        100.0
    } else {
        100.0 - 100.0 / (1.0 + gain / loss)
    }
}

fn roc_value(bars: &[Bar], period: usize) -> f64 {
    if bars.len() <= period {
        return f64::NAN;
    }
    let previous = bars[bars.len() - period - 1].close;
    if previous == 0.0 {
        f64::NAN
    } else {
        (bars.last().expect("non-empty").close / previous - 1.0) * 100.0
    }
}

fn returns_value(bars: &[Bar], period: usize) -> f64 {
    roc_value(bars, period) / 100.0
}

fn percent_change_value(bars: &[Bar], period: usize) -> f64 {
    roc_value(bars, period)
}

fn wma_value(bars: &[Bar], period: usize) -> f64 {
    tail(bars, period).map_or(f64::NAN, |rows| {
        let denominator = (period * (period + 1) / 2) as f64;
        rows.iter()
            .enumerate()
            .map(|(index, bar)| bar.close * (index + 1) as f64)
            .sum::<f64>()
            / denominator
    })
}

fn stddev_value(bars: &[Bar], period: usize) -> f64 {
    closes(bars, period).map_or(f64::NAN, |values| standard_deviation(&values))
}

fn true_range_at(bars: &[Bar], index: usize) -> f64 {
    let current = bars[index];
    if index == 0 {
        current.high - current.low
    } else {
        let previous_close = bars[index - 1].close;
        (current.high - current.low)
            .max((current.high - previous_close).abs())
            .max((current.low - previous_close).abs())
    }
}

fn atr_value(bars: &[Bar], period: usize) -> f64 {
    if bars.len() < period {
        return f64::NAN;
    }
    let start = bars.len() - period;
    (start..bars.len())
        .map(|index| true_range_at(bars, index))
        .sum::<f64>()
        / period as f64
}

fn macd_value(bars: &[Bar], period: usize) -> f64 {
    if bars.len() < period {
        return f64::NAN;
    }
    let fast = (period / 2).max(1);
    ema(bars.iter().map(|bar| bar.close), fast) - ema(bars.iter().map(|bar| bar.close), period)
}

fn macd_series(bars: &[Bar], period: usize) -> Vec<f64> {
    (period..=bars.len())
        .map(|length| macd_value(&bars[..length], period))
        .collect()
}

fn macd_signal_value(bars: &[Bar], period: usize) -> f64 {
    if bars.len() < period * 2 {
        return f64::NAN;
    }
    ema(macd_series(bars, period), period)
}

fn macd_hist_value(bars: &[Bar], period: usize) -> f64 {
    let signal = macd_signal_value(bars, period);
    if signal.is_nan() {
        f64::NAN
    } else {
        macd_value(bars, period) - signal
    }
}

fn stochastic_k_value(bars: &[Bar], period: usize) -> f64 {
    tail(bars, period).map_or(f64::NAN, |rows| {
        let high = rows
            .iter()
            .map(|bar| bar.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let low = rows.iter().map(|bar| bar.low).fold(f64::INFINITY, f64::min);
        if high == low {
            0.0
        } else {
            (rows.last().expect("non-empty").close - low) / (high - low) * 100.0
        }
    })
}

fn stochastic_d_value(bars: &[Bar], period: usize) -> f64 {
    if bars.len() < period + 2 {
        return f64::NAN;
    }
    (bars.len() - 2..bars.len() + 1)
        .map(|length| stochastic_k_value(&bars[..length], period))
        .sum::<f64>()
        / 3.0
}

fn obv_value(bars: &[Bar], _period: usize) -> f64 {
    bars.windows(2).fold(0.0, |value, pair| {
        value
            + if pair[1].close > pair[0].close {
                pair[1].volume
            } else if pair[1].close < pair[0].close {
                -pair[1].volume
            } else {
                0.0
            }
    })
}

fn vwap_value(bars: &[Bar], period: usize) -> f64 {
    tail(bars, period).map_or(f64::NAN, |rows| {
        let volume = rows.iter().map(|bar| bar.volume).sum::<f64>();
        if volume == 0.0 {
            f64::NAN
        } else {
            rows.iter()
                .map(|bar| (bar.high + bar.low + bar.close) / 3.0 * bar.volume)
                .sum::<f64>()
                / volume
        }
    })
}

fn min_value(bars: &[Bar], period: usize) -> f64 {
    closes(bars, period).map_or(f64::NAN, |values| {
        values.into_iter().fold(f64::INFINITY, f64::min)
    })
}

fn max_value(bars: &[Bar], period: usize) -> f64 {
    closes(bars, period).map_or(f64::NAN, |values| {
        values.into_iter().fold(f64::NEG_INFINITY, f64::max)
    })
}

fn range_value(bars: &[Bar], period: usize) -> f64 {
    max_value(bars, period) - min_value(bars, period)
}

fn bollinger_mid_value(bars: &[Bar], period: usize) -> f64 {
    sma_value(bars, period)
}
fn bollinger_upper_value(bars: &[Bar], period: usize) -> f64 {
    sma_value(bars, period) + 2.0 * stddev_value(bars, period)
}
fn bollinger_lower_value(bars: &[Bar], period: usize) -> f64 {
    sma_value(bars, period) - 2.0 * stddev_value(bars, period)
}

fn williams_r_value(bars: &[Bar], period: usize) -> f64 {
    tail(bars, period).map_or(f64::NAN, |rows| {
        let high = rows
            .iter()
            .map(|bar| bar.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let low = rows.iter().map(|bar| bar.low).fold(f64::INFINITY, f64::min);
        if high == low {
            0.0
        } else {
            (high - rows.last().expect("non-empty").close) / (high - low) * -100.0
        }
    })
}

fn cci_value(bars: &[Bar], period: usize) -> f64 {
    tail(bars, period).map_or(f64::NAN, |rows| {
        let typical = rows
            .iter()
            .map(|bar| (bar.high + bar.low + bar.close) / 3.0)
            .collect::<Vec<_>>();
        let average = mean(&typical);
        let deviation = typical
            .iter()
            .map(|value| (value - average).abs())
            .sum::<f64>()
            / period as f64;
        if deviation == 0.0 {
            0.0
        } else {
            (typical.last().expect("non-empty") - average) / (0.015 * deviation)
        }
    })
}

fn momentum_value(bars: &[Bar], period: usize) -> f64 {
    if bars.len() <= period {
        f64::NAN
    } else {
        bars.last().expect("non-empty").close - bars[bars.len() - period - 1].close
    }
}

fn median_value(bars: &[Bar], period: usize) -> f64 {
    closes(bars, period).map_or(f64::NAN, |mut values| {
        values.sort_by(f64::total_cmp);
        if period % 2 == 0 {
            (values[period / 2 - 1] + values[period / 2]) / 2.0
        } else {
            values[period / 2]
        }
    })
}

fn sum_value(bars: &[Bar], period: usize) -> f64 {
    closes(bars, period).map_or(f64::NAN, |values| values.iter().sum())
}

periodic_indicator!(SmaIndicator, "sma", "Simple Moving Average", sma_value);
periodic_indicator!(EmaIndicator, "ema", "Exponential Moving Average", ema_value);
periodic_indicator!(RsiIndicator, "rsi", "Relative Strength Index", rsi_value);
periodic_indicator!(RocIndicator, "roc", "Rate of Change", roc_value);
periodic_indicator!(
    BollingerMidIndicator,
    "bb_mid",
    "Bollinger Middle Band",
    bollinger_mid_value
);
periodic_indicator!(WmaIndicator, "wma", "Weighted Moving Average", wma_value);
periodic_indicator!(
    StdDevIndicator,
    "stddev",
    "Standard Deviation",
    stddev_value
);
periodic_indicator!(AtrIndicator, "atr", "Average True Range", atr_value);
periodic_indicator!(MacdLineIndicator, "macd_line", "MACD Line", macd_value);
periodic_indicator!(
    StochasticKIndicator,
    "stoch_k",
    "Stochastic K",
    stochastic_k_value
);
periodic_indicator!(
    VwapIndicator,
    "vwap",
    "Volume Weighted Average Price",
    vwap_value
);
periodic_indicator!(MinIndicator, "min", "Rolling Minimum", min_value);
periodic_indicator!(MaxIndicator, "max", "Rolling Maximum", max_value);
periodic_indicator!(RangeIndicator, "range", "Rolling Range", range_value);
periodic_indicator!(ReturnsIndicator, "returns", "Returns", returns_value);
periodic_indicator!(
    BollingerUpperIndicator,
    "bb_upper",
    "Bollinger Upper Band",
    bollinger_upper_value
);
periodic_indicator!(
    BollingerLowerIndicator,
    "bb_lower",
    "Bollinger Lower Band",
    bollinger_lower_value
);
periodic_indicator!(
    MacdSignalIndicator,
    "macd_signal",
    "MACD Signal",
    macd_signal_value
);
periodic_indicator!(
    MacdHistIndicator,
    "macd_hist",
    "MACD Histogram",
    macd_hist_value
);
periodic_indicator!(
    StochasticDIndicator,
    "stoch_d",
    "Stochastic D",
    stochastic_d_value
);
periodic_indicator!(
    WilliamsRIndicator,
    "williams_r",
    "Williams R",
    williams_r_value
);
periodic_indicator!(CciIndicator, "cci", "Commodity Channel Index", cci_value);
periodic_indicator!(MomentumIndicator, "momentum", "Momentum", momentum_value);
periodic_indicator!(MedianIndicator, "median", "Rolling Median", median_value);
periodic_indicator!(SumIndicator, "sum", "Rolling Sum", sum_value);
periodic_indicator!(
    PercentChangeIndicator,
    "pct_change",
    "Percent Change",
    percent_change_value
);

macro_rules! unbounded_indicator {
    ($name:ident, $id:literal, $label:literal, $compute:ident) => {
        #[doc = concat!($label, " indicator.")]
        #[derive(Clone, Debug, Default)]
        pub struct $name {
            history: Vec<Bar>,
        }
        impl $name {
            #[doc = concat!("Construct a ", $label, " indicator.")]
            pub fn new() -> Self {
                Self::default()
            }
        }
        impl Indicator for $name {
            fn descriptor(&self) -> &IndicatorDescriptor {
                static DESCRIPTOR: LazyLock<IndicatorDescriptor> =
                    LazyLock::new(|| IndicatorDescriptor {
                        id: $id.into(),
                        name: $label.into(),
                    });
                &DESCRIPTOR
            }
            fn warmup_len(&self) -> usize {
                0
            }
            fn evaluate(&self, window: &[Bar]) -> Result<f64, IndicatorError> {
                Ok($compute(window, 0))
            }
            fn next(&mut self, bar: &Bar) -> Result<f64, IndicatorError> {
                self.history.push(*bar);
                Ok($compute(&self.history, 0))
            }
        }
    };
}

fn true_range_value(bars: &[Bar], _period: usize) -> f64 {
    bars.len()
        .checked_sub(1)
        .map_or(f64::NAN, |index| true_range_at(bars, index))
}
fn typical_price_value(bars: &[Bar], _period: usize) -> f64 {
    bars.last()
        .map_or(f64::NAN, |bar| (bar.high + bar.low + bar.close) / 3.0)
}
fn mid_price_value(bars: &[Bar], _period: usize) -> f64 {
    bars.last()
        .map_or(f64::NAN, |bar| (bar.high + bar.low) / 2.0)
}

unbounded_indicator!(ObvIndicator, "obv", "On Balance Volume", obv_value);
unbounded_indicator!(
    TrueRangeIndicator,
    "true_range",
    "True Range",
    true_range_value
);
unbounded_indicator!(
    TypicalPriceIndicator,
    "typical_price",
    "Typical Price",
    typical_price_value
);
unbounded_indicator!(MidPriceIndicator, "mid_price", "Mid Price", mid_price_value);
