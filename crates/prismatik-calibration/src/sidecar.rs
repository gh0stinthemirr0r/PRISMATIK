//! Calibration sidecar client (`P5-QM-12` deepen).
//!
//! Typed request/response envelopes for method-native conformal backends
//! (typical upstream: MAPIE / crepes) over Arrow IPC transport.
//!
//! - [`SidecarTransport::Loopback`] — in-process [`NullCalibrator`] floor;
//!   round-trips without linking Python.
//! - [`SidecarTransport::Remote`] — subprocess / network invoke fails closed
//!   with [`SidecarError::NotLinked`].

use crate::trait_def::{
    CalibrationMethod, CalibrationRecord, CategorizerId, NullCalibrator, PredictionBatch,
    RealizationBatch,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Which sidecar backend would serve the request.
///
/// Method-native names only (ADR-0032) — do not reintroduce Mapie/Crepes type names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarBackend {
    /// Adaptive conformal inference.
    AdaptiveConformal,
    /// Ensemble batch prediction intervals (EnbPI).
    EnsembleBatch,
    /// Mondrian categorical conformal.
    Mondrian,
}

impl SidecarBackend {
    /// Map to the in-crate [`CalibrationMethod`] taxonomy.
    #[must_use]
    pub fn to_method(self) -> CalibrationMethod {
        match self {
            Self::AdaptiveConformal => CalibrationMethod::AdaptiveConformal {
                learning_rate: 0.01,
            },
            Self::EnsembleBatch => CalibrationMethod::EnbPI,
            Self::Mondrian => CalibrationMethod::MondrianConformal {
                categorizer: CategorizerId("mondrian".into()),
            },
        }
    }
}

/// How [`CalibrationSidecarClient::invoke`] is served.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarTransport {
    /// In-process floor using [`NullCalibrator`] — no Python / MAPIE link.
    Loopback,
    /// Remote subprocess or network endpoint — fails closed until linked.
    #[default]
    Remote,
}

/// Arrow IPC payload handle (bytes owned at floor — no arrow crate link required).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrowIpcBlob {
    /// Opaque Arrow IPC stream bytes.
    pub bytes: Vec<u8>,
    /// Schema fingerprint (BLAKE3 hex or similar).
    pub schema_fingerprint: String,
}

impl ArrowIpcBlob {
    /// Construct a blob.
    #[must_use]
    pub fn new(bytes: Vec<u8>, schema_fingerprint: impl Into<String>) -> Self {
        Self {
            bytes,
            schema_fingerprint: schema_fingerprint.into(),
        }
    }

    /// Empty placeholder used in tests.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            bytes: Vec::new(),
            schema_fingerprint: "empty".into(),
        }
    }
}

/// Sidecar calibrate request.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarCalibrateRequest {
    /// Backend.
    pub backend: SidecarBackend,
    /// Predictions as Arrow IPC (preferred transport).
    pub predictions_ipc: ArrowIpcBlob,
    /// Realizations as Arrow IPC.
    pub realizations_ipc: ArrowIpcBlob,
    /// Optional in-process batches when IPC is empty (loopback / test helper).
    #[serde(default)]
    pub predictions: Option<PredictionBatch>,
    /// Optional realizations batch.
    #[serde(default)]
    pub realizations: Option<RealizationBatch>,
}

/// Sidecar response carrying a [`CalibrationRecord`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SidecarCalibrateResponse {
    /// Produced record.
    pub record: CalibrationRecord,
}

/// Sidecar client errors.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SidecarError {
    /// Sidecar process not linked.
    #[error("calibration sidecar not linked at P5-QM-12 floor: {0}")]
    NotLinked(String),
    /// Transport / schema failure.
    #[error("sidecar transport: {0}")]
    Transport(String),
}

/// Calibration sidecar client (`P5-QM-12`).
///
/// Use [`CalibrationSidecarClient::loopback`] for an in-memory round-trip, or
/// [`CalibrationSidecarClient::new`] / [`CalibrationSidecarClient::remote`] for
/// the fail-closed remote path.
#[derive(Clone, Debug)]
pub struct CalibrationSidecarClient {
    /// Endpoint URL when remote (unused until a process transport lands).
    pub endpoint: Option<String>,
    /// Serving mode.
    transport: SidecarTransport,
}

impl Default for CalibrationSidecarClient {
    fn default() -> Self {
        Self::remote(None)
    }
}

impl CalibrationSidecarClient {
    /// Construct a remote client with optional endpoint (fail-closed invoke).
    #[must_use]
    pub fn new(endpoint: Option<String>) -> Self {
        Self::remote(endpoint)
    }

    /// Remote / subprocess client — [`invoke`](Self::invoke) fails closed.
    #[must_use]
    pub fn remote(endpoint: Option<String>) -> Self {
        Self {
            endpoint,
            transport: SidecarTransport::Remote,
        }
    }

    /// In-process loopback using [`NullCalibrator`] (no Python).
    #[must_use]
    pub fn loopback() -> Self {
        Self {
            endpoint: None,
            transport: SidecarTransport::Loopback,
        }
    }

    /// Active transport mode.
    #[must_use]
    pub fn transport(&self) -> SidecarTransport {
        self.transport
    }

    /// Invoke the sidecar.
    ///
    /// Loopback succeeds with in-process batches. Remote always returns
    /// [`SidecarError::NotLinked`].
    pub fn invoke(
        &self,
        request: &SidecarCalibrateRequest,
    ) -> Result<SidecarCalibrateResponse, SidecarError> {
        match self.transport {
            SidecarTransport::Loopback => invoke_loopback(request),
            SidecarTransport::Remote => Err(SidecarError::NotLinked(format!(
                "backend={:?} endpoint={:?}",
                request.backend, self.endpoint
            ))),
        }
    }
}

fn resolve_batches(
    request: &SidecarCalibrateRequest,
) -> Result<(PredictionBatch, RealizationBatch), SidecarError> {
    match (&request.predictions, &request.realizations) {
        (Some(predictions), Some(realizations)) => Ok((predictions.clone(), realizations.clone())),
        _ => {
            let _ = (&request.predictions_ipc, &request.realizations_ipc);
            Err(SidecarError::Transport(
                "loopback requires in-process prediction/realization batches (Arrow IPC decode not linked)"
                    .into(),
            ))
        },
    }
}

fn invoke_loopback(
    request: &SidecarCalibrateRequest,
) -> Result<SidecarCalibrateResponse, SidecarError> {
    let (predictions, realizations) = resolve_batches(request)?;
    let method = request.backend.to_method();
    let mut calibrator = NullCalibrator::with_method(method);
    let record = calibrator
        .conformalize_batches(&predictions, &realizations)
        .map_err(|e| SidecarError::Transport(e.to_string()))?;
    Ok(SidecarCalibrateResponse { record })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request(backend: SidecarBackend) -> SidecarCalibrateRequest {
        SidecarCalibrateRequest {
            backend,
            predictions_ipc: ArrowIpcBlob::empty(),
            realizations_ipc: ArrowIpcBlob::empty(),
            predictions: Some(PredictionBatch {
                points: vec![1.0, 2.0, 3.0],
            }),
            realizations: Some(RealizationBatch {
                values: vec![1.1, 1.9, 3.2],
            }),
        }
    }

    #[test]
    fn backend_maps_to_calibration_method() {
        assert!(matches!(
            SidecarBackend::AdaptiveConformal.to_method(),
            CalibrationMethod::AdaptiveConformal { .. }
        ));
        assert_eq!(
            SidecarBackend::EnsembleBatch.to_method(),
            CalibrationMethod::EnbPI
        );
        assert!(matches!(
            SidecarBackend::Mondrian.to_method(),
            CalibrationMethod::MondrianConformal { .. }
        ));
    }

    #[test]
    fn remote_invoke_fails_closed() {
        let client = CalibrationSidecarClient::new(Some("http://127.0.0.1:9".into()));
        assert_eq!(client.transport(), SidecarTransport::Remote);
        let req = sample_request(SidecarBackend::Mondrian);
        let err = client.invoke(&req).expect_err("remote must fail closed");
        assert!(matches!(err, SidecarError::NotLinked(_)));
    }

    #[test]
    fn loopback_round_trips_adaptive_conformal() {
        let client = CalibrationSidecarClient::loopback();
        assert_eq!(client.transport(), SidecarTransport::Loopback);
        let resp = client
            .invoke(&sample_request(SidecarBackend::AdaptiveConformal))
            .expect("loopback must succeed");
        assert!(matches!(
            resp.record.method,
            CalibrationMethod::AdaptiveConformal { .. }
        ));
        assert_eq!(resp.record.sample_size, 3);
        assert_eq!(resp.record.calibrator_artifact_id, "null-calibrator");
    }

    #[test]
    fn loopback_stamps_ensemble_and_mondrian_methods() {
        let client = CalibrationSidecarClient::loopback();
        let enbpi = client
            .invoke(&sample_request(SidecarBackend::EnsembleBatch))
            .expect("loopback EnsembleBatch");
        assert_eq!(enbpi.record.method, CalibrationMethod::EnbPI);

        let mondrian = client
            .invoke(&sample_request(SidecarBackend::Mondrian))
            .expect("loopback Mondrian");
        assert!(matches!(
            mondrian.record.method,
            CalibrationMethod::MondrianConformal { .. }
        ));
    }

    #[test]
    fn loopback_without_batches_fails_transport() {
        let client = CalibrationSidecarClient::loopback();
        let req = SidecarCalibrateRequest {
            backend: SidecarBackend::AdaptiveConformal,
            predictions_ipc: ArrowIpcBlob::empty(),
            realizations_ipc: ArrowIpcBlob::empty(),
            predictions: None,
            realizations: None,
        };
        let err = client.invoke(&req).expect_err("IPC-only not linked");
        assert!(matches!(err, SidecarError::Transport(_)));
    }
}
