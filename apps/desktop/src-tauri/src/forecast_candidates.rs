//! Evidence-bound, quarantined model forecast candidates.

use std::{
    collections::BTreeSet,
    path::Path,
    sync::{Mutex, OnceLock},
};

use prismatik_application::FileStateJournal;
use prismatik_calibration::{calibration_report, CalibrationReport, ForecastObservation};
use prismatik_determinism::{Clock, ContentHash, SystemClock};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForecastCandidateRequest {
    provider_id: String,
    model: String,
    target: String,
    horizon_minutes: u32,
    thesis: String,
    max_output_tokens: u32,
    max_cost_micros: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ModelForecastPayload {
    direction: String,
    probability_ppm: u32,
    summary: String,
    evidence_ids: Vec<String>,
    drivers: Vec<String>,
    risks: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ForecastCandidate {
    id: String,
    provider_id: String,
    model: String,
    target: String,
    horizon_minutes: u32,
    direction: String,
    probability_ppm: u32,
    summary: String,
    evidence_ids: Vec<String>,
    drivers: Vec<String>,
    risks: Vec<String>,
    generated_at: String,
    provider_request_id: Option<String>,
    reserved_cost_micros: u64,
    calibration_status: String,
    calibration_sample_size: u64,
    execution_eligible: bool,
    status: String,
    #[serde(default)]
    baseline_price: Option<f64>,
    #[serde(default)]
    baseline_observed_at: Option<String>,
    #[serde(default)]
    resolves_after: Option<String>,
    #[serde(default)]
    resolved_at: Option<String>,
    #[serde(default)]
    resolution_price: Option<f64>,
    #[serde(default)]
    outcome: Option<bool>,
    #[serde(default)]
    brier_ppm: Option<u32>,
    #[serde(default)]
    calibration_report: Option<CalibrationReport>,
    /// Unconditional base rate for this claim at the time it was made, in ppm.
    ///
    /// Stored on the candidate rather than recomputed at scoring time: the base
    /// rate drifts as history accumulates, and skill must be judged against the
    /// baseline the forecast actually had to beat. `None` for forecasters that
    /// do not supply one (the LLM path), which simply score without a skill
    /// figure rather than against a fabricated one.
    #[serde(default)]
    climatology_ppm: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolutionReport {
    resolved: usize,
    pending: usize,
    unavailable: usize,
    candidates: Vec<ForecastCandidate>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CalibrationHealth {
    cohort_id: String,
    provider_id: String,
    model: String,
    target: String,
    horizon_minutes: u32,
    sample_count: usize,
    overall_brier_ppm: Option<u32>,
    overall_ece_ppm: Option<u32>,
    baseline_brier_ppm: Option<u32>,
    recent_brier_ppm: Option<u32>,
    drift_ppm: Option<i64>,
    /// Brier of always predicting the base rate, over the same resolved set.
    climatology_brier_ppm: Option<u32>,
    /// Brier skill score against climatology, in ppm. Positive means the
    /// forecaster genuinely beat the base rate; zero means it added nothing;
    /// negative means it was worse than saying "the usual thing happens".
    ///
    /// This is the number that says whether the cohort is worth running.
    /// `baseline_brier_ppm` above compares the model against an *earlier
    /// window of itself*, which detects drift but cannot detect a forecaster
    /// that has been uselessly consistent from the start.
    skill_ppm: Option<i64>,
    /// Resolved candidates that carried a base rate.
    skill_sample_count: usize,
    state: &'static str,
    execution_eligible: bool,
}

#[derive(Debug)]
struct Runtime {
    candidates: Vec<ForecastCandidate>,
    journal: FileStateJournal<Vec<ForecastCandidate>>,
}

static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();

pub(crate) fn initialize(data_dir: &Path) -> Result<(), String> {
    let mut journal: FileStateJournal<Vec<ForecastCandidate>> = FileStateJournal::open(
        data_dir.join("forecast-candidates.jsonl"),
        "prismatik.forecast-candidates.v1",
    )
    .map_err(|error| error.to_string())?;
    let candidates = journal.latest().cloned().unwrap_or_default();
    if journal.latest().is_none() {
        journal
            .append("initialized", candidates.clone(), SystemClock::new().now())
            .map_err(|error| error.to_string())?;
    }
    RUNTIME
        .set(Mutex::new(Runtime {
            candidates,
            journal,
        }))
        .map_err(|_| "forecast candidate runtime initialized twice".to_owned())?;
    start_resolver();
    Ok(())
}

fn start_resolver() {
    std::thread::Builder::new()
        .name("prismatik-forecast-resolver".into())
        .spawn(|| loop {
            let now = SystemClock::new().now();
            if has_matured_candidate(now) {
                let _ = tauri::async_runtime::block_on(resolve_due_forecast_candidates());
            }
            std::thread::sleep(std::time::Duration::from_secs(300));
        })
        .expect("spawn forecast resolver");
}

fn has_matured_candidate(now: time::OffsetDateTime) -> bool {
    RUNTIME
        .get()
        .and_then(|runtime| runtime.lock().ok())
        .is_some_and(|runtime| {
            runtime.candidates.iter().any(|candidate| {
                candidate.outcome.is_none()
                    && candidate
                        .resolves_after
                        .as_deref()
                        .and_then(parse_time)
                        .is_some_and(|maturity| maturity <= now)
            })
        })
}

#[tauri::command]
pub(crate) fn list_forecast_candidates() -> Result<Vec<ForecastCandidate>, String> {
    Ok(RUNTIME
        .get()
        .ok_or("forecast candidate runtime is unavailable")?
        .lock()
        .map_err(|_| "forecast candidate lock is unavailable".to_owned())?
        .candidates
        .clone())
}

#[tauri::command]
pub(crate) fn forecast_calibration_health() -> Result<Vec<CalibrationHealth>, String> {
    let candidates = list_forecast_candidates()?;
    Ok(calibration_health(&candidates))
}

pub(crate) fn audit_events() -> Result<Vec<crate::audit_timeline::AuditEvent>, String> {
    let candidates = list_forecast_candidates()?;
    let mut events = Vec::new();
    for candidate in candidates.into_iter().rev().take(150) {
        events.push(crate::audit_timeline::AuditEvent {
            id: format!("forecast:{}:generated", candidate.id),
            occurred_at: candidate.generated_at.clone(),
            domain: "forecast",
            severity: if candidate.status.contains("regressed") {
                "warning"
            } else {
                "info"
            },
            state: candidate.status.clone(),
            title: format!(
                "{} {}m forecast candidate",
                candidate.target, candidate.horizon_minutes
            ),
            summary: format!(
                "{} at {:.1}% · {}/{} · {} validated citation(s) · execution blocked",
                candidate.direction.to_ascii_uppercase(),
                f64::from(candidate.probability_ppm) / 10_000.0,
                candidate.provider_id,
                candidate.model,
                candidate.evidence_ids.len()
            ),
            evidence_id: Some(candidate.id.clone()),
            route: "/workspace/models",
            durable: true,
        });
        if let Some(resolved_at) = candidate.resolved_at {
            events.push(crate::audit_timeline::AuditEvent {
                id: format!("forecast:{}:resolved", candidate.id),
                occurred_at: resolved_at,
                domain: "forecast",
                severity: if candidate.status.contains("regressed") {
                    "warning"
                } else {
                    "info"
                },
                state: candidate.status,
                title: format!("{} forecast outcome resolved", candidate.target),
                summary: format!(
                    "Outcome {} · Brier {} · calibration n={} · execution blocked",
                    candidate.outcome.map_or("unavailable", |value| if value {
                        "correct"
                    } else {
                        "incorrect"
                    }),
                    candidate.brier_ppm.map_or_else(
                        || "unavailable".into(),
                        |value| format!("{:.1}%", f64::from(value) / 10_000.0)
                    ),
                    candidate.calibration_sample_size
                ),
                evidence_id: Some(candidate.id),
                route: "/workspace/models",
                durable: true,
            });
        }
    }
    Ok(events)
}

pub(crate) fn require_stable_cohort(cohort_id: &str) -> Result<(), String> {
    let health = forecast_calibration_health()?;
    let cohort = health
        .iter()
        .find(|row| row.cohort_id == cohort_id)
        .ok_or_else(|| format!("required forecast cohort '{cohort_id}' does not exist"))?;
    if cohort.state != "stable" {
        return Err(format!("required forecast cohort is {}; 40 resolved observations and no material 20-vs-20 regression are required", cohort.state));
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn generate_forecast_candidate(
    request: ForecastCandidateRequest,
) -> Result<ForecastCandidate, String> {
    validate_request(&request)?;
    let prompt = format!(
        "Create a structured forecast candidate for target '{}' over {} minutes. Thesis/context: {}. Return ONLY one JSON object with exactly: direction ('up','down','flat'), probabilityPpm (0..1000000), summary, evidenceIds, drivers, risks. Cite only evidenceId values present in the packet. Do not include markdown or trading instructions.",
        request.target, request.horizon_minutes, request.thesis
    );
    let research = crate::model_integrations::run_market_research(
        request.provider_id.clone(),
        request.model.clone(),
        prompt,
        request.max_output_tokens,
        request.max_cost_micros,
    )
    .await?;
    let payload = parse_payload(&research.text)?;
    validate_payload(&payload, &research.evidence_ids)?;
    let baseline = research
        .evidence_quotes
        .iter()
        .find(|quote| quote.symbol.eq_ignore_ascii_case(&request.target))
        .ok_or("target has no real quote in the governed evidence packet")?;
    let generated = SystemClock::new().now();
    let generated_at = generated.to_string();
    let identity = serde_json::json!({"provider":request.provider_id,"model":request.model,"target":request.target,"horizon":request.horizon_minutes,"payload":payload,"generatedAt":generated_at});
    let candidate = ForecastCandidate {
        id: ContentHash::from_bytes(identity.to_string().as_bytes()).to_string(),
        provider_id: request.provider_id,
        model: request.model,
        target: request.target,
        horizon_minutes: request.horizon_minutes,
        direction: payload.direction,
        probability_ppm: payload.probability_ppm,
        summary: payload.summary,
        evidence_ids: payload.evidence_ids,
        drivers: payload.drivers,
        risks: payload.risks,
        generated_at,
        provider_request_id: research.provider_request_id,
        reserved_cost_micros: research.reserved_cost_micros,
        calibration_status: "unvalidated_candidate".into(),
        calibration_sample_size: 0,
        execution_eligible: false,
        status: "quarantined".into(),
        baseline_price: Some(baseline.price),
        baseline_observed_at: Some(baseline.observed_at.clone()),
        resolves_after: Some(
            (generated + time::Duration::minutes(i64::from(request.horizon_minutes))).to_string(),
        ),
        resolved_at: None,
        resolution_price: None,
        outcome: None,
        brier_ppm: None,
        calibration_report: None,
        climatology_ppm: None,
    };
    let mut runtime = RUNTIME
        .get()
        .ok_or("forecast candidate runtime is unavailable")?
        .lock()
        .map_err(|_| "forecast candidate lock is unavailable".to_owned())?;
    let mut next = runtime.candidates.clone();
    next.push(candidate.clone());
    if next.len() > 200 {
        next.drain(..next.len() - 200);
    }
    runtime
        .journal
        .append(
            "candidate_quarantined",
            next.clone(),
            SystemClock::new().now(),
        )
        .map_err(|error| error.to_string())?;
    runtime.candidates = next;
    Ok(candidate)
}

#[tauri::command]
pub(crate) async fn resolve_due_forecast_candidates() -> Result<ResolutionReport, String> {
    let now = SystemClock::new().now();
    let current = list_forecast_candidates()?;
    if !has_matured_candidate(now) {
        let pending = current
            .iter()
            .filter(|candidate| candidate.outcome.is_none() && candidate.resolves_after.is_some())
            .count();
        let unavailable = current
            .iter()
            .filter(|candidate| candidate.outcome.is_none() && candidate.resolves_after.is_none())
            .count();
        return Ok(ResolutionReport {
            resolved: 0,
            pending,
            unavailable,
            candidates: current,
        });
    }
    let snapshot = crate::terminal_feed::get_terminal_feed(crate::app_handle()?).await?;
    if snapshot.quotes.is_empty() {
        return Err("no fresh real market quotes are available for resolution".into());
    }
    let mut runtime = RUNTIME
        .get()
        .ok_or("forecast candidate runtime is unavailable")?
        .lock()
        .map_err(|_| "forecast candidate lock is unavailable".to_owned())?;
    let mut next = runtime.candidates.clone();
    let (mut resolved, mut pending, mut unavailable) = (0, 0, 0);
    for candidate in &mut next {
        if candidate.outcome.is_some() {
            continue;
        }
        let Some(resolve_at) = candidate.resolves_after.as_deref().and_then(parse_time) else {
            unavailable += 1;
            continue;
        };
        if resolve_at > now {
            pending += 1;
            continue;
        }
        let Some(quote) = snapshot
            .quotes
            .iter()
            .find(|quote| quote.symbol.eq_ignore_ascii_case(&candidate.target))
        else {
            unavailable += 1;
            continue;
        };
        let chronology = candidate
            .baseline_observed_at
            .as_deref()
            .and_then(parse_time)
            .zip(parse_time(&quote.observed_at));
        if chronology.is_none_or(|(baseline, current)| current <= baseline) {
            unavailable += 1;
            continue;
        }
        let Some(baseline) = candidate
            .baseline_price
            .filter(|price| price.is_finite() && *price > 0.0)
        else {
            unavailable += 1;
            continue;
        };
        let change_bps = (quote.price - baseline) / baseline * 10_000.0;
        let outcome = match candidate.direction.as_str() {
            "up" => change_bps > 5.0,
            "down" => change_bps < -5.0,
            "flat" => change_bps.abs() <= 5.0,
            _ => false,
        };
        candidate.outcome = Some(outcome);
        candidate.resolution_price = Some(quote.price);
        candidate.resolved_at = Some(now.to_string());
        candidate.status = "resolved_uncalibrated".into();
        candidate.brier_ppm = Some(brier(candidate.probability_ppm, outcome));
        resolved += 1;
    }
    apply_calibration(&mut next);
    if resolved > 0 {
        runtime
            .journal
            .append("outcomes_resolved", next.clone(), now)
            .map_err(|error| error.to_string())?;
        runtime.candidates = next.clone();
    }
    Ok(ResolutionReport {
        resolved,
        pending,
        unavailable,
        candidates: next,
    })
}

/// Provider identity used for statistically generated forecasts.
///
/// Kept distinct from any model provider so the empirical forecaster forms its
/// own calibration cohorts and can be compared against the LLM forecasters on
/// equal terms, by Brier score, on the same targets and horizons.
pub(crate) const EMPIRICAL_PROVIDER: &str = "prismatik.regime";
/// The sequence estimator (`tape.rs`).
pub(crate) const TAPE_PROVIDER: &str = "prismatik.tape";
/// The population estimator (`crowd.rs`).
pub(crate) const CROWD_PROVIDER: &str = "prismatik.crowd";
/// Version the method, not just the provider: changing the classifier changes
/// what the numbers mean, and old scores must not be pooled with new ones.
/// Sourced from the prompt registry so the version lives in one place.
pub(crate) fn empirical_model() -> String {
    crate::prompts::EMPIRICAL_FORECASTER.cohort_tag()
}

/// A trading day, in minutes, for horizon bookkeeping.
const MINUTES_PER_TRADING_DAY: u32 = 24 * 60;

/// File one empirical forecast candidate.
///
/// Returns `Ok(false)` when an unresolved candidate for the same target and
/// horizon is already open — re-filing on every sweep would flood the cohort
/// with near-duplicate observations and make the calibration statistics
/// meaningless.
pub(crate) fn file_empirical_candidate(
    target: &str,
    forecast: &prismatik_regime::EmpiricalForecast,
    horizon_days: usize,
    baseline_price: f64,
    baseline_observed_at: &str,
    bar_count: usize,
) -> Result<bool, String> {
    if !baseline_price.is_finite() || baseline_price <= 0.0 {
        return Err("baseline quote is not a usable price".into());
    }
    let horizon_minutes = (horizon_days as u32).saturating_mul(MINUTES_PER_TRADING_DAY);

    let mut runtime = RUNTIME
        .get()
        .ok_or("forecast candidate runtime is unavailable")?
        .lock()
        .map_err(|_| "forecast candidate lock is unavailable".to_owned())?;

    let already_open = runtime.candidates.iter().any(|candidate| {
        candidate.provider_id == EMPIRICAL_PROVIDER
            && candidate.model == empirical_model()
            && candidate.target.eq_ignore_ascii_case(target)
            && candidate.horizon_minutes == horizon_minutes
            && candidate.outcome.is_none()
    });
    if already_open {
        return Ok(false);
    }

    let direction = match forecast.direction {
        prismatik_regime::ForecastDirection::Up => "up",
        prismatik_regime::ForecastDirection::Down => "down",
        prismatik_regime::ForecastDirection::Flat => "flat",
    };
    let generated = SystemClock::new().now();
    let generated_at = generated.to_string();
    let summary = format!(
        "{target} is in {} (held {} bars). Across {} historical episodes of this regime over {horizon_days}d: {:.1}% up, {:.1}% down, {:.1}% flat; median {:+.0} bps, 10-90 range {:+.0} to {:+.0} bps.",
        forecast.regime.id(),
        forecast.sample_size,
        forecast.sample_size,
        f64::from(forecast.probability_up_ppm) / 10_000.0,
        f64::from(forecast.probability_down_ppm) / 10_000.0,
        f64::from(forecast.probability_flat_ppm) / 10_000.0,
        forecast.median_move_bps,
        forecast.p10_move_bps,
        forecast.p90_move_bps,
    );
    let identity = serde_json::json!({
        "provider": EMPIRICAL_PROVIDER,
        "model": empirical_model(),
        "target": target,
        "horizon": horizon_minutes,
        "regime": forecast.regime.id(),
        "generatedAt": generated_at,
    });

    let candidate = ForecastCandidate {
        id: ContentHash::from_bytes(identity.to_string().as_bytes()).to_string(),
        provider_id: EMPIRICAL_PROVIDER.to_owned(),
        model: empirical_model(),
        target: target.to_owned(),
        horizon_minutes,
        direction: direction.to_owned(),
        // Probability of the *stated direction* — the resolver scores that claim.
        probability_ppm: forecast.probability_ppm.min(1_000_000),
        summary,
        // The evidence is the price history itself, identified by the span used.
        evidence_ids: vec![format!("ohlcv:{target}:{bar_count}bars")],
        drivers: vec![
            format!("regime {}", forecast.regime.id()),
            format!("{} historical episodes", forecast.sample_size),
            format!(
                "regime prevalence {:.1}%",
                forecast.regime_prevalence * 100.0
            ),
        ],
        risks: vec![
            "Empirical distribution assumes the regime persists over the horizon".to_owned(),
            "Overlapping historical windows are correlated observations".to_owned(),
        ],
        generated_at,
        provider_request_id: None,
        reserved_cost_micros: 0,
        calibration_status: "unvalidated_candidate".into(),
        calibration_sample_size: 0,
        execution_eligible: false,
        status: "quarantined".into(),
        baseline_price: Some(baseline_price),
        baseline_observed_at: Some(baseline_observed_at.to_owned()),
        resolves_after: Some(
            (generated + time::Duration::minutes(i64::from(horizon_minutes))).to_string(),
        ),
        resolved_at: None,
        resolution_price: None,
        outcome: None,
        brier_ppm: None,
        calibration_report: None,
        climatology_ppm: Some(forecast.climatology_ppm),
    };

    let mut next = runtime.candidates.clone();
    next.push(candidate);
    if next.len() > 200 {
        next.drain(..next.len() - 200);
    }
    runtime
        .journal
        .append("candidate_quarantined", next.clone(), generated)
        .map_err(|error| error.to_string())?;
    runtime.candidates = next;
    Ok(true)
}

/// One estimator's directional claim, ready to be filed and scored.
pub(crate) struct EstimatorClaim<'a> {
    pub(crate) provider_id: &'a str,
    pub(crate) model: &'a str,
    pub(crate) target: &'a str,
    pub(crate) direction: prismatik_regime::ForecastDirection,
    /// Probability of the *stated direction* — what the resolver scores.
    pub(crate) probability_ppm: u32,
    /// Base rate for that same direction, from `climatology_for`.
    pub(crate) climatology_ppm: u32,
    pub(crate) summary: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) drivers: Vec<String>,
    pub(crate) risks: Vec<String>,
}

/// File a directional claim from Tape, Crowd, or any future estimator.
///
/// This is `file_empirical_candidate` with the regime-specific parts lifted
/// out. Everything that decides whether a forecast counts — the horizon
/// arithmetic, the one-open-candidate rule, quarantine on arrival, the
/// climatology the Brier skill is taken against — is shared, so a new
/// estimator cannot accidentally grade itself on an easier curve.
///
/// Returns `Ok(false)` when an unresolved claim for the same cohort, target
/// and horizon is already open.
pub(crate) fn file_estimator_candidate(
    claim: EstimatorClaim<'_>,
    horizon_days: usize,
    baseline_price: f64,
    baseline_observed_at: &str,
) -> Result<bool, String> {
    if !baseline_price.is_finite() || baseline_price <= 0.0 {
        return Err("baseline quote is not a usable price".into());
    }
    let horizon_minutes = (horizon_days as u32).saturating_mul(MINUTES_PER_TRADING_DAY);

    let mut runtime = RUNTIME
        .get()
        .ok_or("forecast candidate runtime is unavailable")?
        .lock()
        .map_err(|_| "forecast candidate lock is unavailable".to_owned())?;

    let already_open = runtime.candidates.iter().any(|candidate| {
        candidate.provider_id == claim.provider_id
            && candidate.model == claim.model
            && candidate.target.eq_ignore_ascii_case(claim.target)
            && candidate.horizon_minutes == horizon_minutes
            && candidate.outcome.is_none()
    });
    if already_open {
        return Ok(false);
    }

    let direction = match claim.direction {
        prismatik_regime::ForecastDirection::Up => "up",
        prismatik_regime::ForecastDirection::Down => "down",
        prismatik_regime::ForecastDirection::Flat => "flat",
    };
    let generated = SystemClock::new().now();
    let generated_at = generated.to_string();
    let identity = serde_json::json!({
        "provider": claim.provider_id,
        "model": claim.model,
        "target": claim.target,
        "horizon": horizon_minutes,
        "direction": direction,
        "generatedAt": generated_at,
    });

    let candidate = ForecastCandidate {
        id: ContentHash::from_bytes(identity.to_string().as_bytes()).to_string(),
        provider_id: claim.provider_id.to_owned(),
        model: claim.model.to_owned(),
        target: claim.target.to_owned(),
        horizon_minutes,
        direction: direction.to_owned(),
        probability_ppm: claim.probability_ppm.min(1_000_000),
        summary: claim.summary,
        evidence_ids: claim.evidence_ids,
        drivers: claim.drivers,
        risks: claim.risks,
        generated_at,
        provider_request_id: None,
        reserved_cost_micros: 0,
        calibration_status: "unvalidated_candidate".into(),
        calibration_sample_size: 0,
        execution_eligible: false,
        status: "quarantined".into(),
        baseline_price: Some(baseline_price),
        baseline_observed_at: Some(baseline_observed_at.to_owned()),
        resolves_after: Some(
            (generated + time::Duration::minutes(i64::from(horizon_minutes))).to_string(),
        ),
        resolved_at: None,
        resolution_price: None,
        outcome: None,
        brier_ppm: None,
        calibration_report: None,
        climatology_ppm: Some(claim.climatology_ppm),
    };

    let mut next = runtime.candidates.clone();
    next.push(candidate);
    if next.len() > 200 {
        next.drain(..next.len() - 200);
    }
    runtime
        .journal
        .append("candidate_quarantined", next.clone(), generated)
        .map_err(|error| error.to_string())?;
    runtime.candidates = next;
    Ok(true)
}

/// The fields `consensus` needs from an open claim.
///
/// A narrow projection rather than exposing `ForecastCandidate` itself: the
/// candidate carries resolution state and journal bookkeeping that nothing
/// outside this module should be reading, let alone acting on.
pub(crate) struct OpenClaim {
    pub(crate) model: String,
    pub(crate) direction: String,
    pub(crate) probability_ppm: u32,
    pub(crate) climatology_ppm: u32,
    pub(crate) summary: String,
    pub(crate) generated_at: String,
}

/// The most recent unresolved claim from one estimator on one target.
///
/// Resolved candidates are excluded deliberately: they are history, and
/// showing one beside live claims would misreport what the desk currently
/// believes.
pub(crate) fn latest_open_claim(
    provider_id: &str,
    target: &str,
    horizon_minutes: u32,
) -> Option<OpenClaim> {
    let candidates = list_forecast_candidates().ok()?;
    candidates
        .into_iter()
        .filter(|candidate| {
            candidate.provider_id == provider_id
                && candidate.target.eq_ignore_ascii_case(target)
                && candidate.horizon_minutes == horizon_minutes
                && candidate.outcome.is_none()
        })
        .max_by(|a, b| a.generated_at.cmp(&b.generated_at))
        .map(|candidate| OpenClaim {
            model: candidate.model,
            direction: candidate.direction,
            probability_ppm: candidate.probability_ppm,
            climatology_ppm: candidate.climatology_ppm.unwrap_or(0),
            summary: candidate.summary,
            generated_at: candidate.generated_at,
        })
}

fn parse_time(value: &str) -> Option<time::OffsetDateTime> {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).ok()
}

fn brier(probability_ppm: u32, outcome: bool) -> u32 {
    let target = if outcome { 1_000_000_i64 } else { 0 };
    let error = i64::from(probability_ppm) - target;
    u32::try_from((error * error) / 1_000_000).unwrap_or(1_000_000)
}

fn apply_calibration(candidates: &mut [ForecastCandidate]) {
    for index in 0..candidates.len() {
        let observations = candidates
            .iter()
            .filter(|candidate| {
                candidate.provider_id == candidates[index].provider_id
                    && candidate.model == candidates[index].model
                    && candidate.target == candidates[index].target
                    && candidate.horizon_minutes == candidates[index].horizon_minutes
            })
            .filter_map(|candidate| {
                candidate.outcome.map(|outcome| ForecastObservation {
                    participant_id: format!("{}:{}", candidate.provider_id, candidate.model),
                    category: candidate.target.clone(),
                    horizon: format!("{}m", candidate.horizon_minutes),
                    probability_ppm: candidate.probability_ppm,
                    outcome,
                })
            })
            .collect::<Vec<_>>();
        candidates[index].calibration_sample_size = observations.len() as u64;
        if observations.len() >= 20 {
            candidates[index].calibration_report = calibration_report(&observations, 10);
            candidates[index].calibration_status = "cohort_calibrated".into();
            if candidates[index].outcome.is_some() {
                candidates[index].status = "resolved_calibrated".into();
            }
        }
        if observations.len() >= 40 {
            let split = observations.len() - 20;
            let baseline = calibration_report(&observations[split - 20..split], 10);
            let recent = calibration_report(&observations[split..], 10);
            if baseline.zip(recent).is_some_and(|(before, after)| {
                after.brier_ppm > before.brier_ppm.saturating_add(50_000)
                    || after.expected_calibration_error_ppm
                        > before.expected_calibration_error_ppm.saturating_add(50_000)
            }) {
                candidates[index].calibration_status = "calibration_regressed".into();
                if candidates[index].outcome.is_some() {
                    candidates[index].status = "resolved_regressed".into();
                }
            }
        }
    }
}

/// Measured skill of the empirical forecaster on one target and horizon.
///
/// This is what the autonomous trader sizes on: not the model's own stated
/// confidence, but whether this cohort has historically beaten the base rate.
/// `None` means the cohort has not been scored yet — the caller must treat that
/// as unproven, never as neutral.
pub(crate) fn empirical_skill(target: &str, horizon_days: usize) -> Option<CohortSkill> {
    let horizon_minutes = (horizon_days as u32).saturating_mul(MINUTES_PER_TRADING_DAY);
    let candidates = list_forecast_candidates().ok()?;
    calibration_health(&candidates).into_iter().find_map(|row| {
        let matches = row.provider_id == EMPIRICAL_PROVIDER
            && row.model == empirical_model()
            && row.target.eq_ignore_ascii_case(target)
            && row.horizon_minutes == horizon_minutes;
        matches.then_some(CohortSkill {
            skill_ppm: row.skill_ppm,
            sample_count: row.skill_sample_count,
            state: row.state,
        })
    })
}

/// Scored standing of one forecasting cohort.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CohortSkill {
    /// Brier skill against climatology, in ppm. `None` until scored.
    pub(crate) skill_ppm: Option<i64>,
    /// Resolved forecasts behind the score.
    pub(crate) sample_count: usize,
    /// Calibration state of the cohort.
    pub(crate) state: &'static str,
}

/// Measured skill of one specialist analyst's cohort.
///
/// Same scoring path as the statistical forecaster, so an analyst's standing is
/// directly comparable with it — which is the only way to answer "should I
/// listen to this analyst or to the model?"
pub(crate) fn analyst_skill(cohort_model: &str, horizon_days: usize) -> Option<CohortSkill> {
    estimator_skill(
        crate::analysts::ANALYST_PROVIDER,
        cohort_model,
        horizon_days,
    )
}

/// Measured skill for any cohort, keyed the same way for every estimator.
///
/// Tape, Crowd, the analysts and the regime forecaster all resolve through
/// this one function deliberately: three estimators that computed their own
/// standing three different ways would not be comparable, and comparing them
/// is the reason they all exist.
pub(crate) fn estimator_skill(
    provider_id: &str,
    cohort_model: &str,
    horizon_days: usize,
) -> Option<CohortSkill> {
    let horizon_minutes = (horizon_days as u32).saturating_mul(MINUTES_PER_TRADING_DAY);
    let candidates = list_forecast_candidates().ok()?;
    calibration_health(&candidates).into_iter().find_map(|row| {
        let matches = row.provider_id == provider_id
            && row.model == cohort_model
            && row.horizon_minutes == horizon_minutes;
        matches.then_some(CohortSkill {
            skill_ppm: row.skill_ppm,
            sample_count: row.skill_sample_count,
            state: row.state,
        })
    })
}

fn calibration_health(candidates: &[ForecastCandidate]) -> Vec<CalibrationHealth> {
    let mut keys = BTreeSet::new();
    for candidate in candidates {
        keys.insert((
            candidate.provider_id.clone(),
            candidate.model.clone(),
            candidate.target.clone(),
            candidate.horizon_minutes,
        ));
    }
    keys.into_iter()
        .map(|(provider_id, model, target, horizon_minutes)| {
            let observations = candidates
                .iter()
                .filter(|candidate| {
                    candidate.provider_id == provider_id
                        && candidate.model == model
                        && candidate.target == target
                        && candidate.horizon_minutes == horizon_minutes
                })
                .filter_map(|candidate| {
                    candidate.outcome.map(|outcome| ForecastObservation {
                        participant_id: format!("{provider_id}:{model}"),
                        category: target.clone(),
                        horizon: format!("{horizon_minutes}m"),
                        probability_ppm: candidate.probability_ppm,
                        outcome,
                    })
                })
                .collect::<Vec<_>>();
            // Skill against climatology, over the resolved candidates that
            // recorded a base rate. Both Briers are computed on exactly the
            // same set, so the ratio is a fair comparison.
            let scored: Vec<(u32, u32, bool)> = candidates
                .iter()
                .filter(|candidate| {
                    candidate.provider_id == provider_id
                        && candidate.model == model
                        && candidate.target == target
                        && candidate.horizon_minutes == horizon_minutes
                })
                .filter_map(|candidate| {
                    Some((
                        candidate.probability_ppm,
                        candidate.climatology_ppm?,
                        candidate.outcome?,
                    ))
                })
                .collect();
            let (climatology_brier_ppm, skill_ppm) = if scored.is_empty() {
                (None, None)
            } else {
                let count = scored.len() as u64;
                let model_brier: u64 = scored
                    .iter()
                    .map(|(p, _, outcome)| u64::from(brier(*p, *outcome)))
                    .sum::<u64>()
                    / count;
                let clim_brier: u64 = scored
                    .iter()
                    .map(|(_, c, outcome)| u64::from(brier(*c, *outcome)))
                    .sum::<u64>()
                    / count;
                let skill = if clim_brier == 0 {
                    // Climatology was perfect; the model cannot improve on it.
                    None
                } else {
                    Some(1_000_000 - (model_brier as i64 * 1_000_000) / clim_brier as i64)
                };
                (u32::try_from(clim_brier).ok(), skill)
            };

            let overall = calibration_report(&observations, 10);
            let (baseline, recent) = if observations.len() >= 40 {
                let split = observations.len() - 20;
                (
                    calibration_report(&observations[split - 20..split], 10),
                    calibration_report(&observations[split..], 10),
                )
            } else {
                (None, None)
            };
            let drift_ppm = baseline
                .as_ref()
                .zip(recent.as_ref())
                .map(|(before, after)| i64::from(after.brier_ppm) - i64::from(before.brier_ppm));
            let regressed =
                baseline
                    .as_ref()
                    .zip(recent.as_ref())
                    .is_some_and(|(before, after)| {
                        after.brier_ppm > before.brier_ppm.saturating_add(50_000)
                            || after.expected_calibration_error_ppm
                                > before.expected_calibration_error_ppm.saturating_add(50_000)
                    });
            CalibrationHealth {
                cohort_id: format!("{provider_id}:{model}:{target}:{horizon_minutes}m"),
                provider_id,
                model,
                target,
                horizon_minutes,
                sample_count: observations.len(),
                overall_brier_ppm: overall.as_ref().map(|report| report.brier_ppm),
                overall_ece_ppm: overall
                    .as_ref()
                    .map(|report| report.expected_calibration_error_ppm),
                baseline_brier_ppm: baseline.as_ref().map(|report| report.brier_ppm),
                recent_brier_ppm: recent.as_ref().map(|report| report.brier_ppm),
                drift_ppm,
                climatology_brier_ppm,
                skill_ppm,
                skill_sample_count: scored.len(),
                state: if regressed {
                    "regressed"
                } else if observations.len() >= 40 {
                    "stable"
                } else if observations.len() >= 20 {
                    "calibrated_insufficient_drift_window"
                } else {
                    "insufficient_sample"
                },
                execution_eligible: false,
            }
        })
        .collect()
}

fn validate_request(request: &ForecastCandidateRequest) -> Result<(), String> {
    if request.target.trim().is_empty()
        || request.target.len() > 120
        || request.thesis.trim().is_empty()
        || request.thesis.len() > 4_000
    {
        return Err("target and bounded thesis are required".into());
    }
    if !(1..=43_200).contains(&request.horizon_minutes) {
        return Err("forecast horizon must be between 1 minute and 30 days".into());
    }
    if !(128..=4_096).contains(&request.max_output_tokens) || request.max_cost_micros == 0 {
        return Err(
            "forecast requires 128–4096 output tokens and a non-zero cost reservation".into(),
        );
    }
    Ok(())
}

fn parse_payload(text: &str) -> Result<ModelForecastPayload, String> {
    let start = text
        .find('{')
        .ok_or("model returned no JSON forecast object")?;
    let end = text
        .rfind('}')
        .ok_or("model returned incomplete JSON forecast object")?;
    serde_json::from_str(&text[start..=end])
        .map_err(|error| format!("invalid forecast JSON: {error}"))
}

fn validate_payload(payload: &ModelForecastPayload, allowed: &[String]) -> Result<(), String> {
    if !matches!(payload.direction.as_str(), "up" | "down" | "flat")
        || payload.probability_ppm > 1_000_000
    {
        return Err("model forecast direction or probability is invalid".into());
    }
    if payload.summary.trim().is_empty()
        || payload.summary.len() > 2_000
        || payload.evidence_ids.is_empty()
        || payload.evidence_ids.len() > 32
        || payload.drivers.len() > 12
        || payload.risks.len() > 12
    {
        return Err("model forecast exceeds bounded artifact limits".into());
    }
    if payload.evidence_ids.iter().any(|id| !allowed.contains(id)) {
        return Err("model cited an evidence ID absent from the governed packet".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citations_must_exist_in_packet() {
        let payload = ModelForecastPayload {
            direction: "up".into(),
            probability_ppm: 600_000,
            summary: "candidate".into(),
            evidence_ids: vec!["invented".into()],
            drivers: vec![],
            risks: vec![],
        };
        assert!(validate_payload(&payload, &["real".into()]).is_err());
    }

    #[test]
    fn parses_bounded_json_without_accepting_prose_as_fields() {
        let payload = parse_payload(r#"{"direction":"flat","probabilityPpm":500000,"summary":"x","evidenceIds":["e1"],"drivers":[],"risks":[]}"#).unwrap();
        assert_eq!(payload.probability_ppm, 500_000);
    }

    #[test]
    fn brier_scoring_rewards_correct_confidence() {
        assert_eq!(brier(1_000_000, true), 0);
        assert_eq!(brier(0, true), 1_000_000);
        assert_eq!(brier(500_000, true), 250_000);
    }

    fn resolved_candidate(probability_ppm: u32, outcome: bool) -> ForecastCandidate {
        ForecastCandidate {
            id: format!("{probability_ppm}-{outcome}"),
            provider_id: "provider".into(),
            model: "model".into(),
            target: "SPY".into(),
            horizon_minutes: 60,
            direction: "up".into(),
            probability_ppm,
            summary: "test".into(),
            evidence_ids: vec!["e1".into()],
            drivers: vec![],
            risks: vec![],
            generated_at: "2026-01-01T00:00:00Z".into(),
            provider_request_id: None,
            reserved_cost_micros: 1,
            calibration_status: "cohort_calibrated".into(),
            calibration_sample_size: 40,
            execution_eligible: false,
            status: "resolved_calibrated".into(),
            baseline_price: Some(100.0),
            baseline_observed_at: Some("2026-01-01T00:00:00Z".into()),
            resolves_after: Some("2026-01-01T01:00:00Z".into()),
            resolved_at: Some("2026-01-01T02:00:00Z".into()),
            resolution_price: Some(101.0),
            outcome: Some(outcome),
            brier_ppm: Some(brier(probability_ppm, outcome)),
            calibration_report: None,
            climatology_ppm: None,
        }
    }

    fn scored_against_climatology(
        probability_ppm: u32,
        climatology_ppm: u32,
        outcome: bool,
    ) -> ForecastCandidate {
        let mut candidate = resolved_candidate(probability_ppm, outcome);
        candidate.id = format!("{probability_ppm}-{climatology_ppm}-{outcome}");
        candidate.climatology_ppm = Some(climatology_ppm);
        candidate
    }

    #[test]
    fn a_forecaster_that_only_restates_the_base_rate_scores_zero_skill() {
        // The failure this whole metric exists to catch: confident-looking
        // probabilities that are pure climatology.
        let candidates: Vec<_> = (0..40)
            .map(|i| scored_against_climatology(600_000, 600_000, i % 10 < 6))
            .collect();
        let health = calibration_health(&candidates);
        let cohort = health.first().expect("one cohort");
        assert_eq!(
            cohort.skill_ppm,
            Some(0),
            "identical forecasts must show no skill"
        );
        assert_eq!(cohort.skill_sample_count, 40);
    }

    #[test]
    fn beating_the_base_rate_scores_positive_skill() {
        // Base rate says 50/50; the forecaster calls each outcome correctly.
        let candidates: Vec<_> = (0..40)
            .map(|i| {
                let outcome = i % 2 == 0;
                let confident = if outcome { 900_000 } else { 100_000 };
                scored_against_climatology(confident, 500_000, outcome)
            })
            .collect();
        let cohort = calibration_health(&candidates).into_iter().next().unwrap();
        assert!(
            cohort.skill_ppm.is_some_and(|s| s > 0),
            "expected positive skill, got {:?}",
            cohort.skill_ppm,
        );
    }

    #[test]
    fn losing_to_the_base_rate_scores_negative_skill() {
        // Confidently wrong against an accurate base rate.
        let candidates: Vec<_> = (0..40)
            .map(|i| {
                let outcome = i % 2 == 0;
                let wrong = if outcome { 100_000 } else { 900_000 };
                scored_against_climatology(wrong, 500_000, outcome)
            })
            .collect();
        let cohort = calibration_health(&candidates).into_iter().next().unwrap();
        assert!(
            cohort.skill_ppm.is_some_and(|s| s < 0),
            "expected negative skill, got {:?}",
            cohort.skill_ppm,
        );
    }

    #[test]
    fn cohorts_without_a_recorded_base_rate_report_no_skill() {
        // The LLM path files no climatology; it must score without a skill
        // figure rather than against an invented one.
        let candidates: Vec<_> = (0..40)
            .map(|i| resolved_candidate(600_000, i % 2 == 0))
            .collect();
        let cohort = calibration_health(&candidates).into_iter().next().unwrap();
        assert_eq!(cohort.skill_ppm, None);
        assert_eq!(cohort.skill_sample_count, 0);
    }

    #[test]
    fn recent_regression_is_detected_and_never_execution_eligible() {
        let mut candidates = (0..20)
            .map(|_| resolved_candidate(900_000, true))
            .collect::<Vec<_>>();
        candidates.extend((0..20).map(|_| resolved_candidate(900_000, false)));
        let health = calibration_health(&candidates);
        assert_eq!(health[0].state, "regressed");
        assert!(!health[0].execution_eligible);
        apply_calibration(&mut candidates);
        assert!(candidates
            .iter()
            .all(|candidate| candidate.calibration_status == "calibration_regressed"));
    }
}
