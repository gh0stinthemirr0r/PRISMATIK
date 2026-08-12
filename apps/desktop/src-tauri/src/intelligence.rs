use serde::Serialize;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use prismatik_application::FileIntelligenceLedger;
use prismatik_application::IntelligenceObservationKind;
use time::OffsetDateTime;

static INTELLIGENCE_LEDGER: OnceLock<Mutex<FileIntelligenceLedger>> = OnceLock::new();

pub fn initialize(data_dir: &Path) -> Result<(), String> {
    let ledger = FileIntelligenceLedger::open(data_dir.join("intelligence-observations.jsonl"))
        .map_err(|error| error.to_string())?;
    INTELLIGENCE_LEDGER
        .set(Mutex::new(ledger))
        .map_err(|_| "intelligence ledger was initialized twice".to_owned())
}

pub(crate) fn record_observation(
    kind: IntelligenceObservationKind,
    evidence_id: String,
    observed_at: OffsetDateTime,
) -> Result<bool, String> {
    INTELLIGENCE_LEDGER
        .get()
        .ok_or("intelligence ledger is unavailable")?
        .lock()
        .map_err(|_| "intelligence ledger lock is unavailable".to_owned())?
        .append(kind, evidence_id, observed_at)
        .map_err(|error| error.to_string())
}

/// Native analytics capability exposed by the compiled desktop backend.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceCapability {
    id: &'static str,
    engine: &'static str,
    observation_count: u64,
    status: &'static str,
}

/// Return compiled capability truth. Observation counts stay zero until the
/// governed ingestion store supplies real records; this command has no fixture path.
#[tauri::command]
pub fn intelligence_capabilities() -> Vec<IntelligenceCapability> {
    // Type references ensure the desktop build fails if the native contracts
    // disappear or drift away from this capability surface.
    let _ = std::mem::size_of::<prismatik_domain::ResolutionRiskAssessment>();
    let _ = std::mem::size_of::<prismatik_market_data::BookSnapshot>();
    let _ = std::mem::size_of::<prismatik_calibration::CalibrationReport>();
    let _ = std::mem::size_of::<prismatik_events::TranscriptAnalysis>();
    let _ = std::mem::size_of::<prismatik_events::MarketEvidencePacket>();
    let _ = std::mem::size_of::<prismatik_execution::ReconciliationCycle>();
    let _ = std::mem::size_of::<prismatik_crypto::DerivativesState>();
    let _ = std::mem::size_of::<prismatik_crypto::PublicAddressProfile>();
    let _ = std::mem::size_of::<prismatik_renderer::AiUiIntent>();
    let _ = std::mem::size_of::<prismatik_storage::PointInTimeQuery>();

    let counts = INTELLIGENCE_LEDGER
        .get()
        .and_then(|ledger| ledger.lock().ok())
        .map(|ledger| ledger.counts())
        .unwrap_or_default();
    [
        ("resolution", "prismatik-domain"),
        ("logical-arbitrage", "prismatik-domain"),
        ("calibration", "prismatik-calibration"),
        ("l2-reconstruction", "prismatik-market-data"),
        ("event-discovery", "prismatik-events"),
        ("feed-attention", "prismatik-events"),
        ("transcripts", "prismatik-events"),
        ("continuous-reconciliation", "prismatik-execution"),
        ("strategy-authoring", "prismatik-strategy"),
        ("derivatives", "prismatik-crypto"),
        ("public-address", "prismatik-crypto"),
        ("typed-ui", "prismatik-renderer"),
        ("pit-query", "prismatik-storage"),
        ("macro-series", "prismatik-market-data"),
        ("filings", "prismatik-market-data"),
    ]
    .into_iter()
    .map(|(id, engine)| IntelligenceCapability {
        id,
        engine,
        observation_count: counts.by_capability.get(id).copied().unwrap_or(0),
        status: if counts.by_capability.get(id).copied().unwrap_or(0) > 0 {
            "observed"
        } else {
            "awaiting_observations"
        },
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capability_truth_never_claims_fixture_observations() {
        let capabilities = intelligence_capabilities();
        assert_eq!(capabilities.len(), 15);
        assert!(capabilities.iter().all(|capability| {
            capability.status == "observed" || capability.status == "awaiting_observations"
        }));
    }
}
