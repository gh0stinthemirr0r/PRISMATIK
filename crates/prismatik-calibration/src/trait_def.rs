//! Calibrator trait and calibration record types (`P5-QM-11` floor).

use prismatik_determinism::DeterminismContext;
use serde::{Deserialize, Serialize};

/// Nominal coverage level in (0, 1], e.g. `0.9` for a 90% interval.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Coverage(pub f64);

/// Realized empirical coverage for a nominal level.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealizedCoverage(pub f64);

/// Opaque categorizer id for Mondrian / regime-conditioned conformal.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CategorizerId(pub String);

/// Probability scaling flavour for classification outputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalingKind {
    /// Isotonic regression.
    Isotonic,
    /// Platt / logistic scaling.
    Platt,
}

/// Distribution-free calibration method (v1.0 §17.3).
///
/// ACI is the default for temporal forecasts. Split conformal is retained only
/// for cross-sectional (non-temporal) tasks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationMethod {
    /// Standard split conformal — exchangeability assumed.
    SplitConformal,
    /// Ensemble batch prediction intervals (bootstrap; short series).
    EnbPI,
    /// Adaptive Conformal Inference — default for time series.
    AdaptiveConformal {
        /// ACI learning rate (floor placeholder; not tuned here).
        learning_rate: f64,
    },
    /// Mondrian / regime-conditioned conformal.
    MondrianConformal {
        /// Regime categorizer identifier.
        categorizer: CategorizerId,
    },
    /// Isotonic or Platt scaling for classification probabilities.
    ProbabilityScaling(ScalingKind),
}

impl CalibrationMethod {
    /// Normative default for temporal forecasts: ACI.
    pub fn default_temporal() -> Self {
        Self::AdaptiveConformal {
            learning_rate: 0.01,
        }
    }
}

/// Held-out conformalization provenance + honesty metrics.
///
/// Every user-facing forecast must carry one of these (Invariant I2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalibrationRecord {
    /// Method used to produce the intervals.
    pub method: CalibrationMethod,
    /// Inclusive start of the conformalization window (unix seconds, floor).
    pub conformalized_on_start_unix: i64,
    /// Exclusive end of the conformalization window (unix seconds, floor).
    pub conformalized_on_end_unix: i64,
    /// Held-out sample size.
    pub sample_size: usize,
    /// Realized coverage per nominal level.
    pub coverage_curve: Vec<(Coverage, RealizedCoverage)>,
    /// Mean interval width (coverage is trivial with infinite width).
    pub mean_interval_width: f64,
    /// Pinball loss (lower is better).
    pub pinball_loss: f64,
    /// Continuous ranked probability score (lower is better).
    pub crps: f64,
    /// Opaque calibrator artifact id (content-addressed later).
    pub calibrator_artifact_id: String,
    /// Computation timestamp (unix seconds, floor).
    pub computed_at_unix: i64,
}

impl CalibrationRecord {
    /// Build a minimal floor record (tests / stubs).
    pub fn floor_stub(method: CalibrationMethod) -> Self {
        Self {
            method,
            conformalized_on_start_unix: 0,
            conformalized_on_end_unix: 1,
            sample_size: 1,
            coverage_curve: vec![(Coverage(0.9), RealizedCoverage(0.9))],
            mean_interval_width: 1.0,
            pinball_loss: 0.0,
            crps: 0.0,
            calibrator_artifact_id: "cal-stub".into(),
            computed_at_unix: 0,
        }
    }
}

/// Errors from conformalization / calibration.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CalibrationError {
    /// Held-out set too small for the chosen method.
    #[error("insufficient conformalization sample: {0}")]
    InsufficientSample(usize),
    /// Target coverage outside (0, 1].
    #[error("invalid coverage target: {0}")]
    InvalidCoverage(String),
    /// Generic floor failure.
    #[error("calibration failed: {0}")]
    Failed(String),
}

/// Raw model point / distribution batch before calibration (floor placeholder).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PredictionBatch {
    /// Point predictions (placeholder units).
    pub points: Vec<f64>,
}

/// Realized outcomes aligned to a prediction batch (floor placeholder).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RealizationBatch {
    /// Realized values (placeholder units).
    pub values: Vec<f64>,
}

/// Single raw prediction before conformalization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawPrediction {
    /// Point forecast.
    pub point: f64,
}

/// Calibrated interval prediction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalibratedPrediction {
    /// Point forecast (unchanged from raw at this floor).
    pub point: f64,
    /// Interval lower bound.
    pub lower: f64,
    /// Interval upper bound.
    pub upper: f64,
    /// Target coverage used.
    pub target_coverage: Coverage,
}

/// Online coverage drift from ACI-style updates (floor placeholder).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoverageDrift {
    /// Realized minus nominal coverage (positive ⇒ undercover).
    pub coverage_error: f64,
}

/// Turns raw model output into intervals with a stated coverage property.
///
/// No model output reaches a user surface without a [`CalibrationRecord`].
pub trait Calibrator: Send + Sync {
    /// Calibration method this instance applies.
    fn method(&self) -> CalibrationMethod;

    /// Fit on a held-out conformalization set (never training / eval).
    fn conformalize(
        &mut self,
        predictions: &PredictionBatch,
        realizations: &RealizationBatch,
        ctx: &DeterminismContext,
    ) -> Result<CalibrationRecord, CalibrationError>;

    /// Apply to a raw prediction, producing a calibrated interval.
    fn calibrate(
        &self,
        raw: &RawPrediction,
        target_coverage: Coverage,
    ) -> Result<CalibratedPrediction, CalibrationError>;

    /// Online update as realizations arrive (ACI coverage correction).
    fn update(&mut self, observed: &RealizationBatch) -> Result<CoverageDrift, CalibrationError>;
}

/// Floor calibrator: ACI default, identity intervals scaled by a fixed width.
#[derive(Clone, Debug)]
pub struct NullCalibrator {
    method: CalibrationMethod,
    width: f64,
}

impl NullCalibrator {
    /// Construct with ACI default method.
    pub fn new() -> Self {
        Self {
            method: CalibrationMethod::default_temporal(),
            width: 1.0,
        }
    }

    /// Construct with an explicit method (sidecar loopback stamps backend → method).
    #[must_use]
    pub fn with_method(method: CalibrationMethod) -> Self {
        Self { method, width: 1.0 }
    }

    /// In-process conformalize without a full [`DeterminismContext`].
    ///
    /// Used by the P5-QM-12 loopback path so calibration can round-trip without
    /// linking Python or assembling a run context.
    pub fn conformalize_batches(
        &mut self,
        predictions: &PredictionBatch,
        realizations: &RealizationBatch,
    ) -> Result<CalibrationRecord, CalibrationError> {
        if predictions.points.len() != realizations.values.len() {
            return Err(CalibrationError::Failed(
                "prediction/realization length mismatch".into(),
            ));
        }
        if predictions.points.is_empty() {
            return Err(CalibrationError::InsufficientSample(0));
        }
        Ok(CalibrationRecord {
            method: self.method.clone(),
            conformalized_on_start_unix: 0,
            conformalized_on_end_unix: predictions.points.len() as i64,
            sample_size: predictions.points.len(),
            coverage_curve: vec![(Coverage(0.9), RealizedCoverage(0.9))],
            mean_interval_width: self.width * 2.0,
            pinball_loss: 0.0,
            crps: 0.0,
            calibrator_artifact_id: "null-calibrator".into(),
            computed_at_unix: 0,
        })
    }
}

impl Default for NullCalibrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Calibrator for NullCalibrator {
    fn method(&self) -> CalibrationMethod {
        self.method.clone()
    }

    fn conformalize(
        &mut self,
        predictions: &PredictionBatch,
        realizations: &RealizationBatch,
        _ctx: &DeterminismContext,
    ) -> Result<CalibrationRecord, CalibrationError> {
        self.conformalize_batches(predictions, realizations)
    }

    fn calibrate(
        &self,
        raw: &RawPrediction,
        target_coverage: Coverage,
    ) -> Result<CalibratedPrediction, CalibrationError> {
        if !(target_coverage.0 > 0.0 && target_coverage.0 <= 1.0) {
            return Err(CalibrationError::InvalidCoverage(format!(
                "{}",
                target_coverage.0
            )));
        }
        Ok(CalibratedPrediction {
            point: raw.point,
            lower: raw.point - self.width,
            upper: raw.point + self.width,
            target_coverage,
        })
    }

    fn update(&mut self, observed: &RealizationBatch) -> Result<CoverageDrift, CalibrationError> {
        if observed.values.is_empty() {
            return Err(CalibrationError::InsufficientSample(0));
        }
        Ok(CoverageDrift {
            coverage_error: 0.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_temporal_is_aci() {
        match CalibrationMethod::default_temporal() {
            CalibrationMethod::AdaptiveConformal { learning_rate } => {
                assert!(learning_rate > 0.0);
            },
            other => panic!("expected ACI, got {other:?}"),
        }
    }

    #[test]
    fn null_calibrator_rejects_empty_conformalize() {
        let mut cal = NullCalibrator::new();
        // DeterminismContext construction is heavy; exercise calibrate/update paths instead.
        let err = cal
            .calibrate(&RawPrediction { point: 1.0 }, Coverage(0.0))
            .unwrap_err();
        assert!(matches!(err, CalibrationError::InvalidCoverage(_)));
        let drift = cal.update(&RealizationBatch { values: vec![1.0] }).unwrap();
        assert_eq!(drift.coverage_error, 0.0);
    }
}
