//! P4-QM-13 floor: streaming `next` sequence equals batch `evaluate`.
//!
//! Spec aspirational path: `tests/property/streaming_eq_batch.rs`.

use prismatik_indicator_core::{
    AtrIndicator, Bar, BollingerLowerIndicator, BollingerMidIndicator, BollingerUpperIndicator,
    CciIndicator, EmaIndicator, Indicator, IndicatorRegistry, MacdHistIndicator, MacdLineIndicator,
    MacdSignalIndicator, MaxIndicator, MedianIndicator, MidPriceIndicator, MinIndicator,
    MomentumIndicator, NativeIndicatorKind, ObvIndicator, PercentChangeIndicator, RangeIndicator,
    ReturnsIndicator, RocIndicator, RsiIndicator, SmaIndicator, StdDevIndicator,
    StochasticDIndicator, StochasticKIndicator, SumIndicator, TrueRangeIndicator,
    TypicalPriceIndicator, VwapIndicator, WilliamsRIndicator, WmaIndicator,
};

const EPS: f64 = 1e-9;
const PERIODS: &[usize] = &[2, 3, 5, 8, 14, 20];
const SERIES_LEN: usize = 64;

fn bar(close: f64) -> Bar {
    Bar {
        open: close,
        high: close + 0.05,
        low: close - 0.05,
        close,
        volume: 1.0 + (close.abs() % 3.0),
    }
}

/// Deterministic synthetic OHLCVs — no proptest / cassette I/O.
fn synthetic_bars(n: usize) -> Vec<Bar> {
    (0..n)
        .map(|i| {
            let t = i as f64;
            // Smooth trend + two harmonics + tiny deterministic dither.
            let close = 100.0
                + 0.15 * t
                + 4.0 * (t * 0.17).sin()
                + 1.5 * (t * 0.41).cos()
                + ((i * 17 + 3) % 11) as f64 * 0.01;
            bar(close)
        })
        .collect()
}

fn assert_nan_eq(a: f64, b: f64, idx: usize, kind: &str, period: usize) {
    let a_nan = a.is_nan();
    let b_nan = b.is_nan();
    assert!(
        a_nan == b_nan,
        "{kind}({period}) NaN mismatch at i={idx}: stream={a:?} batch={b:?}"
    );
}

fn assert_streaming_eq_batch<S, B, F, G>(
    kind: &str,
    period: usize,
    bars: &[Bar],
    mut stream_new: F,
    mut batch_new: G,
) where
    S: Indicator,
    B: Indicator,
    F: FnMut() -> S,
    G: FnMut() -> B,
{
    let mut stream = stream_new();
    let warmup = stream.warmup_len();
    assert_eq!(
        warmup,
        batch_new().warmup_len(),
        "{kind}({period}) warmup mismatch"
    );

    for (i, bar) in bars.iter().enumerate() {
        let streamed = stream.next(bar).expect("stream next");
        let batch = batch_new().evaluate(&bars[..=i]).expect("batch evaluate");

        assert_nan_eq(streamed, batch, i, kind, period);

        // After warmup: finite values must agree within 1e-9; matching NaNs are ok.
        if i + 1 > warmup && !streamed.is_nan() && !batch.is_nan() {
            let delta = (streamed - batch).abs();
            assert!(
                delta <= EPS,
                "{kind}({period}) epsilon {delta} > {EPS} at i={i}: stream={streamed} batch={batch}"
            );
        }
    }
}

#[test]
fn registry_lists_all_native_kinds() {
    let reg = IndicatorRegistry::native();
    assert_eq!(reg.len(), NativeIndicatorKind::ALL.len());
    for kind in NativeIndicatorKind::ALL {
        assert!(reg.contains(*kind), "missing {kind}");
    }
}

#[test]
fn streaming_eq_batch_sma_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "sma",
            period,
            &bars,
            || SmaIndicator::new(period).unwrap(),
            || SmaIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_ema_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "ema",
            period,
            &bars,
            || EmaIndicator::new(period).unwrap(),
            || EmaIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_rsi_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "rsi",
            period,
            &bars,
            || RsiIndicator::new(period).unwrap(),
            || RsiIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_roc_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "roc",
            period,
            &bars,
            || RocIndicator::new(period).unwrap(),
            || RocIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_bollinger_mid_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "bb_mid",
            period,
            &bars,
            || BollingerMidIndicator::new(period).unwrap(),
            || BollingerMidIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_wma_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "wma",
            period,
            &bars,
            || WmaIndicator::new(period).unwrap(),
            || WmaIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_stddev_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "stddev",
            period,
            &bars,
            || StdDevIndicator::new(period).unwrap(),
            || StdDevIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_atr_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "atr",
            period,
            &bars,
            || AtrIndicator::new(period).unwrap(),
            || AtrIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_min_max_range_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "min",
            period,
            &bars,
            || MinIndicator::new(period).unwrap(),
            || MinIndicator::new(period).unwrap(),
        );
        assert_streaming_eq_batch(
            "max",
            period,
            &bars,
            || MaxIndicator::new(period).unwrap(),
            || MaxIndicator::new(period).unwrap(),
        );
        assert_streaming_eq_batch(
            "range",
            period,
            &bars,
            || RangeIndicator::new(period).unwrap(),
            || RangeIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_returns_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "returns",
            period,
            &bars,
            || ReturnsIndicator::new(period).unwrap(),
            || ReturnsIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_macd_line_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        if period < 2 {
            continue;
        }
        assert_streaming_eq_batch(
            "macd_line",
            period,
            &bars,
            || MacdLineIndicator::new(period).unwrap(),
            || MacdLineIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_stoch_k_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "stoch_k",
            period,
            &bars,
            || StochasticKIndicator::new(period).unwrap(),
            || StochasticKIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_vwap_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "vwap",
            period,
            &bars,
            || VwapIndicator::new(period).unwrap(),
            || VwapIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_obv() {
    let bars = synthetic_bars(SERIES_LEN);
    assert_streaming_eq_batch("obv", 0, &bars, ObvIndicator::new, ObvIndicator::new);
}

#[test]
fn streaming_eq_batch_bollinger_bands_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "bb_upper",
            period,
            &bars,
            || BollingerUpperIndicator::new(period).unwrap(),
            || BollingerUpperIndicator::new(period).unwrap(),
        );
        assert_streaming_eq_batch(
            "bb_lower",
            period,
            &bars,
            || BollingerLowerIndicator::new(period).unwrap(),
            || BollingerLowerIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_macd_signal_hist_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        if period < 2 {
            continue;
        }
        assert_streaming_eq_batch(
            "macd_signal",
            period,
            &bars,
            || MacdSignalIndicator::new(period).unwrap(),
            || MacdSignalIndicator::new(period).unwrap(),
        );
        assert_streaming_eq_batch(
            "macd_hist",
            period,
            &bars,
            || MacdHistIndicator::new(period).unwrap(),
            || MacdHistIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_stoch_d_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "stoch_d",
            period,
            &bars,
            || StochasticDIndicator::new(period).unwrap(),
            || StochasticDIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_williams_r_cci_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "williams_r",
            period,
            &bars,
            || WilliamsRIndicator::new(period).unwrap(),
            || WilliamsRIndicator::new(period).unwrap(),
        );
        assert_streaming_eq_batch(
            "cci",
            period,
            &bars,
            || CciIndicator::new(period).unwrap(),
            || CciIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_momentum_median_sum_across_periods() {
    let bars = synthetic_bars(SERIES_LEN);
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "momentum",
            period,
            &bars,
            || MomentumIndicator::new(period).unwrap(),
            || MomentumIndicator::new(period).unwrap(),
        );
        assert_streaming_eq_batch(
            "median",
            period,
            &bars,
            || MedianIndicator::new(period).unwrap(),
            || MedianIndicator::new(period).unwrap(),
        );
        assert_streaming_eq_batch(
            "sum",
            period,
            &bars,
            || SumIndicator::new(period).unwrap(),
            || SumIndicator::new(period).unwrap(),
        );
    }
}

#[test]
fn streaming_eq_batch_true_range_typical_mid_pct() {
    let bars = synthetic_bars(SERIES_LEN);
    assert_streaming_eq_batch(
        "true_range",
        0,
        &bars,
        TrueRangeIndicator::new,
        TrueRangeIndicator::new,
    );
    assert_streaming_eq_batch(
        "typical_price",
        0,
        &bars,
        TypicalPriceIndicator::new,
        TypicalPriceIndicator::new,
    );
    assert_streaming_eq_batch(
        "mid_price",
        0,
        &bars,
        MidPriceIndicator::new,
        MidPriceIndicator::new,
    );
    for &period in PERIODS {
        assert_streaming_eq_batch(
            "pct_change",
            period,
            &bars,
            || PercentChangeIndicator::new(period).unwrap(),
            || PercentChangeIndicator::new(period).unwrap(),
        );
    }
}
