//! P4-QM-14: two-implementation conformance harness integration tests.
//!
//! Inline fixtures only. Future: MIT ai-algotrading-agent `hist-10m/*.csv` corpus
//! (see `DOCS/spec/TESTING.md` §4.4 and `src/conformance.rs`).

use prismatik_indicator_core::{run_case, Bar, ConformanceCase, SmaIndicator, NUMERICAL_TOLERANCE};

fn bar(close: f64) -> Bar {
    Bar {
        open: close,
        high: close,
        low: close,
        close,
        volume: 1.0,
    }
}

fn inline_sma_cases() -> Vec<ConformanceCase> {
    vec![
        ConformanceCase {
            name: "sma3_uniform_floor",
            period: 3,
            bars: vec![bar(10.0), bar(10.0), bar(10.0)],
            expected: 10.0,
        },
        ConformanceCase {
            name: "sma5_uptrend_tail",
            period: 5,
            bars: vec![
                bar(100.0),
                bar(101.0),
                bar(102.0),
                bar(103.0),
                bar(104.0),
                bar(105.0),
            ],
            // Last five closes: 101..=105 → mean 103.0
            expected: 103.0,
        },
        ConformanceCase {
            name: "sma2_algotrading_tick_style",
            period: 2,
            // ai-algotrading-agent style short tick-replay ladder
            bars: vec![bar(50.0), bar(50.25), bar(50.5), bar(50.75)],
            expected: 50.625,
        },
        ConformanceCase {
            name: "sma3_warmup_nan",
            period: 3,
            bars: vec![bar(1.0), bar(2.0)],
            expected: f64::NAN,
        },
    ]
}

#[test]
fn two_impl_conformance_sma_inline_fixtures() {
    for case in inline_sma_cases() {
        let period = case.period;
        run_case(&case, || SmaIndicator::new(period).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", case.name));
    }
}

#[test]
fn numerical_tolerance_is_1e8() {
    assert!((NUMERICAL_TOLERANCE - 1e-8).abs() < f64::EPSILON);
}
