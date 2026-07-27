//! # prismatik-prismatik-calibration
//!
//! Layer 2 — Domain
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — calibration contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use drift::{DriftAction, DriftAssessment, DriftBaseline, DriftDetector, DriftSignal};
pub use trait_def::{
    CalibrationError, CalibrationMethod, CalibrationRecord, Calibrator, Coverage, RealizedCoverage,
};

/// Calibration trait contracts.
pub mod trait_def {
    /// Coverage target in [0, 1].
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Coverage(pub f64);

    /// Realized coverage in [0, 1].
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct RealizedCoverage(pub f64);

    /// Calibration method enum.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum CalibrationMethod {
        /// Adaptive conformal inference.
        Aci,
        /// EnbPI.
        Enbpi,
        /// Split conformal.
        SplitConformal,
    }

    /// Calibration record.
    #[derive(Clone, Debug, PartialEq)]
    pub struct CalibrationRecord {
        /// Calibration method.
        pub method: CalibrationMethod,
        /// Target coverage.
        pub target: Coverage,
        /// Realized coverage.
        pub realized: RealizedCoverage,
    }

    /// Calibration error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CalibrationError {
        /// Error message.
        pub message: String,
    }

    /// Calibrator contract.
    pub trait Calibrator: Send + Sync {
        /// Return method.
        fn method(&self) -> CalibrationMethod;
        /// Calibrate raw prediction.
        fn calibrate(
            &self,
            raw: f64,
            target_coverage: Coverage,
        ) -> Result<(f64, f64), CalibrationError>;
    }
}

/// Drift-detection contracts.
pub mod drift {
    /// Drift signal type.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum DriftSignal {
        /// No signal.
        None,
        /// Warning signal.
        Warning,
        /// Critical signal.
        Critical,
    }

    /// Drift baseline metadata.
    #[derive(Clone, Debug, PartialEq)]
    pub struct DriftBaseline {
        /// Window size in observations.
        pub window: usize,
        /// Baseline miss-rate.
        pub miss_rate: f64,
    }

    /// Drift response action.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum DriftAction {
        /// Keep strategy live.
        Keep,
        /// Increase caution.
        Degrade,
        /// Halt strategy.
        Halt,
    }

    /// Drift assessment output.
    #[derive(Clone, Debug, PartialEq)]
    pub struct DriftAssessment {
        /// Signal.
        pub signal: DriftSignal,
        /// Action.
        pub action: DriftAction,
    }

    /// Drift detector contract.
    pub trait DriftDetector: Send + Sync {
        /// Assess latest realized coverage.
        fn assess(&self, realized: f64, baseline: &DriftBaseline) -> DriftAssessment;
    }
}

/// Sidecar client contracts.
pub mod sidecar_client {
    use crate::trait_def::{
        CalibrationError, CalibrationMethod, CalibrationRecord, Coverage, RealizedCoverage,
    };

    /// Sidecar calibration request.
    #[derive(Clone, Debug, PartialEq)]
    pub struct SidecarCalibrationRequest {
        /// Raw point prediction.
        pub raw_prediction: f64,
        /// Target coverage.
        pub target: Coverage,
        /// Calibration method.
        pub method: CalibrationMethod,
    }

    /// Sidecar calibration response.
    #[derive(Clone, Debug, PartialEq)]
    pub struct SidecarCalibrationResponse {
        /// Lower bound.
        pub lower: f64,
        /// Upper bound.
        pub upper: f64,
        /// Calibration record.
        pub record: CalibrationRecord,
    }

    /// Sidecar client contract.
    pub trait CalibrationSidecarClient: Send + Sync {
        /// Request calibrated interval from sidecar.
        fn calibrate(
            &self,
            request: &SidecarCalibrationRequest,
        ) -> Result<SidecarCalibrationResponse, CalibrationError>;
    }

    /// In-process deterministic sidecar stub implementation.
    #[derive(Clone, Debug, Default)]
    pub struct LocalCalibrationSidecarClient;

    impl CalibrationSidecarClient for LocalCalibrationSidecarClient {
        fn calibrate(
            &self,
            request: &SidecarCalibrationRequest,
        ) -> Result<SidecarCalibrationResponse, CalibrationError> {
            if !(0.0..=1.0).contains(&request.target.0) {
                return Err(CalibrationError {
                    message: "target coverage must be within [0, 1]".to_owned(),
                });
            }
            let width = 1.0 - request.target.0;
            Ok(SidecarCalibrationResponse {
                lower: request.raw_prediction - width,
                upper: request.raw_prediction + width,
                record: CalibrationRecord {
                    method: request.method,
                    target: request.target,
                    realized: RealizedCoverage(request.target.0),
                },
            })
        }
    }
}
