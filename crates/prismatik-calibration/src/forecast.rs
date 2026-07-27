//! Forecast types that cannot omit [`CalibrationRecord`] (Invariant I2 gate).

use serde::{Deserialize, Serialize};

use crate::trait_def::CalibrationRecord;

/// Opaque point / series forecast payload (floor placeholder).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawForecast {
    /// Model / series identifier.
    pub model_id: String,
    /// Point forecast value (placeholder units).
    pub point: f64,
}

/// User-facing forecast envelope — calibration is mandatory by construction.
///
/// There is no public constructor that omits [`CalibrationRecord`]. Fields are
/// private; use [`ForecastWithCalibration::new`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ForecastWithCalibration {
    raw: RawForecast,
    calibration: CalibrationRecord,
}

impl ForecastWithCalibration {
    /// Construct a forecast that is guaranteed to carry a calibration record.
    pub fn new(raw: RawForecast, calibration: CalibrationRecord) -> Self {
        Self { raw, calibration }
    }

    /// Underlying raw forecast.
    pub fn raw(&self) -> &RawForecast {
        &self.raw
    }

    /// Attached calibration record (Invariant I2).
    pub fn calibration(&self) -> &CalibrationRecord {
        &self.calibration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trait_def::CalibrationMethod;

    #[test]
    fn constructor_requires_calibration_record() {
        let forecast = ForecastWithCalibration::new(
            RawForecast {
                model_id: "m1".into(),
                point: 42.0,
            },
            CalibrationRecord::floor_stub(CalibrationMethod::default_temporal()),
        );
        assert_eq!(forecast.raw().point, 42.0);
        assert_eq!(forecast.calibration().sample_size, 1);
    }
}
