//! P4-QM-14 floor: two-implementation conformance harness.
//!
//! Each [`ConformanceCase`] runs the same OHLCV window through:
//! - **ReferenceImpl** — batch [`Indicator::evaluate`] on the full window
//! - **CandidateImpl** — streaming [`Indicator::next`] bar-by-bar
//!
//! Both paths must agree with the declared expected value within
//! [`NUMERICAL_TOLERANCE`] (1e-8 relative, matching Wave 2 quant gates).
//!
//! ## Future external corpus
//!
//! MIT [ai-algotrading-agent](https://github.com/IvoPetiz/ai-algotrading-agent)
//! `hist-10m/*.csv` fixtures are the planned external reference corpus
//! (see `DOCS/spec/TESTING.md` §4.4). Vendor those CSVs and extend this
//! harness with SMA-cross and trailing-stop cases once ported. Inline fixtures
//! are used until the corpus is vendored.

use crate::{Bar, Indicator, IndicatorError};

/// Relative tolerance for indicator conformance (Wave 2 / P4-QM-14 gate).
pub const NUMERICAL_TOLERANCE: f64 = 1e-8;

/// One inline or future vendored conformance vector.
#[derive(Clone, Debug, PartialEq)]
pub struct ConformanceCase {
    /// Human-readable case id (stable for CI logs).
    pub name: &'static str,
    /// Indicator lookback period (e.g. SMA length).
    pub period: usize,
    /// OHLCV window ending at the bar under test.
    pub bars: Vec<Bar>,
    /// Expected indicator value at the last bar (`NaN` during warmup).
    pub expected: f64,
}

/// Batch evaluation path — reference implementation.
pub struct ReferenceImpl<'a, I> {
    indicator: I,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<I> std::fmt::Debug for ReferenceImpl<'_, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReferenceImpl").finish_non_exhaustive()
    }
}

impl<I: Indicator> ReferenceImpl<'_, I> {
    /// Wrap an indicator for batch evaluation.
    pub fn new(indicator: I) -> Self {
        Self {
            indicator,
            _marker: std::marker::PhantomData,
        }
    }

    /// Evaluate over the full window (reference path).
    pub fn value(&self, bars: &[Bar]) -> Result<f64, IndicatorError> {
        self.indicator.evaluate(bars)
    }
}

/// Streaming update path — candidate / second code path.
pub struct CandidateImpl<I> {
    indicator: I,
}

impl<I> std::fmt::Debug for CandidateImpl<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CandidateImpl").finish_non_exhaustive()
    }
}

impl<I: Indicator> CandidateImpl<I> {
    /// Wrap an indicator for streaming evaluation.
    pub fn new(indicator: I) -> Self {
        Self { indicator }
    }

    /// Feed bars sequentially and return the value at the last bar.
    pub fn value(&mut self, bars: &[Bar]) -> Result<f64, IndicatorError> {
        let mut last = f64::NAN;
        for bar in bars {
            last = self.indicator.next(bar)?;
        }
        Ok(last)
    }
}

/// Run one case: reference vs candidate vs expected, all within tolerance.
pub fn run_case<I, F>(case: &ConformanceCase, make_indicator: F) -> Result<(), String>
where
    I: Indicator,
    F: Fn() -> I,
{
    let reference = ReferenceImpl::new(make_indicator())
        .value(&case.bars)
        .map_err(|e| format!("{} reference: {e}", case.name))?;
    let mut candidate = CandidateImpl::new(make_indicator());
    let candidate_val = candidate
        .value(&case.bars)
        .map_err(|e| format!("{} candidate: {e}", case.name))?;

    assert_close(case.name, "reference_vs_expected", reference, case.expected)?;
    assert_close(
        case.name,
        "candidate_vs_expected",
        candidate_val,
        case.expected,
    )?;
    assert_close(
        case.name,
        "reference_vs_candidate",
        reference,
        candidate_val,
    )?;
    Ok(())
}

fn assert_close(case: &str, field: &str, actual: f64, expected: f64) -> Result<(), String> {
    if actual.is_nan() && expected.is_nan() {
        return Ok(());
    }
    if actual.is_nan() || expected.is_nan() {
        return Err(format!(
            "{case} {field}: NaN mismatch actual={actual:?} expected={expected:?}"
        ));
    }
    let scale = expected.abs().max(1e-12);
    let relative = (actual - expected).abs() / scale;
    if relative > NUMERICAL_TOLERANCE {
        return Err(format!(
            "{case} {field}: actual={actual:.16}, expected={expected:.16}, rel={relative:.3e}"
        ));
    }
    Ok(())
}
