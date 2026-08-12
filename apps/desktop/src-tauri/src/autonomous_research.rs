//! User-configured, durable, fail-closed autonomous market research loop.

use std::path::Path;
use std::sync::{OnceLock, RwLock};

use prismatik_application::FileStateJournal;
use prismatik_determinism::{Clock, SystemClock};
use serde::{Deserialize, Serialize};

use crate::model_integrations::MarketResearchResult;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutonomousResearchConfig {
    enabled: bool,
    provider_id: String,
    model: String,
    question: String,
    interval_seconds: u64,
    max_output_tokens: u32,
    max_cost_micros: u64,
    #[serde(default)]
    require_stable_forecasts: bool,
    #[serde(default)]
    required_cohort_id: String,
}

impl Default for AutonomousResearchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider_id: String::new(),
            model: String::new(),
            question: "Identify material cross-asset changes in the current real snapshot, challenge the strongest interpretation, and cite every observation used.".into(),
            interval_seconds: 900,
            max_output_tokens: 700,
            max_cost_micros: 250_000,
            require_stable_forecasts: false,
            required_cohort_id: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutonomousResearchStatus {
    config: AutonomousResearchConfig,
    running: bool,
    next_run_at: Option<String>,
    last_run_at: Option<String>,
    last_error: Option<String>,
    successful_runs: u64,
    failed_runs: u64,
    last_result: Option<MarketResearchResult>,
    #[serde(default)]
    gate_state: String,
    #[serde(default)]
    gate_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LoopState {
    status: AutonomousResearchStatus,
    next_run_unix: Option<i64>,
}

impl Default for LoopState {
    fn default() -> Self {
        Self {
            status: AutonomousResearchStatus {
                config: AutonomousResearchConfig::default(),
                ..AutonomousResearchStatus::default()
            },
            next_run_unix: None,
        }
    }
}

#[derive(Debug)]
struct ResearchRuntime {
    state: LoopState,
    journal: FileStateJournal<LoopState>,
}

static RUNTIME: OnceLock<RwLock<ResearchRuntime>> = OnceLock::new();

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutonomousResearchView {
    #[serde(flatten)]
    status: AutonomousResearchStatus,
    durable: bool,
    journal_records: u64,
    journal_head: String,
    credentials_persisted: bool,
}

fn view(runtime: &ResearchRuntime) -> AutonomousResearchView {
    AutonomousResearchView {
        status: runtime.state.status.clone(),
        durable: true,
        journal_records: runtime.journal.record_count(),
        journal_head: runtime.journal.tip_hash().to_string(),
        credentials_persisted: false,
    }
}

#[tauri::command]
pub(crate) fn get_autonomous_research() -> Result<AutonomousResearchView, String> {
    let runtime = RUNTIME
        .get()
        .ok_or("autonomous research runtime is not initialized")?
        .read()
        .map_err(|_| "autonomous research state is unavailable".to_owned())?;
    Ok(view(&runtime))
}

pub(crate) fn audit_events() -> Result<Vec<crate::audit_timeline::AuditEvent>, String> {
    let runtime = RUNTIME
        .get()
        .ok_or("autonomous research runtime is not initialized")?
        .read()
        .map_err(|_| "autonomous research state is unavailable".to_owned())?;
    let status = &runtime.state.status;
    let Some(occurred_at) = status.last_run_at.clone() else {
        return Ok(Vec::new());
    };
    let (severity, state, summary) = if status.last_error.is_some() {
        (
            "warning",
            "failed",
            format!(
                "{}/{} · failure recorded; detail remains in the gated autonomy pane",
                status.config.provider_id, status.config.model
            ),
        )
    } else if status.gate_state == "paused" {
        (
            "warning",
            "calibration_paused",
            format!(
                "{}/{} · calibration gate paused before model spend",
                status.config.provider_id, status.config.model
            ),
        )
    } else {
        let evidence_count = status
            .last_result
            .as_ref()
            .map_or(0, |result| result.evidence_count);
        (
            "info",
            "completed",
            format!(
                "{}/{} · {} governed evidence record(s) · output retained in autonomous research state",
                status.config.provider_id, status.config.model, evidence_count
            ),
        )
    };
    Ok(vec![crate::audit_timeline::AuditEvent {
        id: format!("autonomous-research:{occurred_at}"),
        occurred_at,
        domain: "autonomy",
        severity,
        state: state.into(),
        title: "Autonomous research cycle".into(),
        summary,
        evidence_id: None,
        route: "/workspace/autonomy",
        durable: true,
    }])
}

#[tauri::command]
pub(crate) fn configure_autonomous_research(
    config: AutonomousResearchConfig,
) -> Result<AutonomousResearchView, String> {
    validate_config(&config)?;
    let now = SystemClock::new().now();
    let mut runtime = RUNTIME
        .get()
        .ok_or("autonomous research runtime is not initialized")?
        .write()
        .map_err(|_| "autonomous research state is unavailable".to_owned())?;
    let mut next = runtime.state.clone();
    next.next_run_unix = config.enabled.then_some(now.unix_timestamp());
    next.status.next_run_at = config.enabled.then(|| now.to_string());
    next.status.config = config;
    next.status.running = false;
    next.status.last_error = None;
    next.status.gate_state = if next.status.config.require_stable_forecasts {
        "checking"
    } else {
        "not_required"
    }
    .into();
    next.status.gate_reason = None;
    runtime
        .journal
        .append("configured", next.clone(), now)
        .map_err(|error| error.to_string())?;
    runtime.state = next;
    Ok(view(&runtime))
}

fn validate_config(config: &AutonomousResearchConfig) -> Result<(), String> {
    if !(300..=604_800).contains(&config.interval_seconds) {
        return Err("autonomous research interval must be between 300 and 604800 seconds".into());
    }
    if config.enabled
        && (config.provider_id.trim().is_empty()
            || config.model.trim().is_empty()
            || config.question.trim().is_empty()
            || config.question.len() > 8_000
            || config.max_cost_micros == 0
            || !(1..=8_192).contains(&config.max_output_tokens))
    {
        return Err("enabled autonomous research requires provider, model, a 1–8,000 character question, token limit, and worst-case cost reservation".into());
    }
    if config.enabled
        && config.require_stable_forecasts
        && config.required_cohort_id.trim().is_empty()
    {
        return Err(
            "a required forecast cohort must be selected when the calibration gate is enabled"
                .into(),
        );
    }
    Ok(())
}

pub(crate) fn initialize(data_dir: &Path) -> Result<(), String> {
    let now = SystemClock::new().now();
    let v2_path = data_dir.join("autonomous-research-v2.jsonl");
    let v2_exists = v2_path.metadata().is_ok_and(|metadata| metadata.len() > 0);
    let mut journal: FileStateJournal<LoopState> =
        FileStateJournal::open(&v2_path, "prismatik.autonomous-research.v2")
            .map_err(|error| error.to_string())?;
    let mut migrated = false;
    let mut state = journal.latest().cloned().unwrap_or_else(|| {
        migrate_v1(data_dir)
            .inspect(|_| {
                migrated = true;
            })
            .unwrap_or_default()
    });
    if journal.latest().is_none() {
        journal
            .append(
                if migrated {
                    "migrated_from_v1"
                } else if v2_exists {
                    "reinitialized_empty_v2"
                } else {
                    "initialized"
                },
                state.clone(),
                now,
            )
            .map_err(|error| error.to_string())?;
    } else {
        let (recovered, event) = recover_startup(state, now);
        state = recovered;
        if let Some(event) = event {
            journal
                .append(event, state.clone(), now)
                .map_err(|error| error.to_string())?;
        }
    }
    RUNTIME
        .set(RwLock::new(ResearchRuntime { state, journal }))
        .map_err(|_| "autonomous research runtime was initialized twice".to_owned())?;
    spawn_loop();
    Ok(())
}

fn migrate_v1(data_dir: &Path) -> Option<LoopState> {
    let path = data_dir.join("autonomous-research.jsonl");
    if !path.exists() {
        return None;
    }
    let journal: FileStateJournal<serde_json::Value> =
        FileStateJournal::open(path, "prismatik.autonomous-research.v1").ok()?;
    serde_json::from_value(journal.latest()?.clone()).ok()
}

fn recover_startup(
    mut state: LoopState,
    now: time::OffsetDateTime,
) -> (LoopState, Option<&'static str>) {
    if state.status.running {
        state.status.running = false;
        state.status.failed_runs = state.status.failed_runs.saturating_add(1);
        state.status.last_error = Some(
            "The previous research cycle was interrupted by shutdown; no result was accepted."
                .into(),
        );
        state.next_run_unix = state.status.config.enabled.then_some(now.unix_timestamp());
        state.status.next_run_at = state.status.config.enabled.then(|| now.to_string());
        (state, Some("interrupted_run_recovered"))
    } else if state.status.config.enabled {
        state.next_run_unix = Some(now.unix_timestamp());
        state.status.next_run_at = Some(now.to_string());
        (state, Some("startup_resumed"))
    } else {
        (state, None)
    }
}

/// Wake an enabled mandate when its configured memory-only provider reconnects.
pub(crate) fn nudge_for_provider(provider_id: &str) {
    let Some(runtime) = RUNTIME.get() else { return };
    let Ok(mut runtime) = runtime.write() else {
        return;
    };
    if !runtime.state.status.config.enabled
        || runtime.state.status.config.provider_id != provider_id
        || runtime.state.status.running
    {
        return;
    }
    let now = SystemClock::new().now();
    let mut next = runtime.state.clone();
    next.next_run_unix = Some(now.unix_timestamp());
    next.status.next_run_at = Some(now.to_string());
    if runtime
        .journal
        .append("provider_reconnected", next.clone(), now)
        .is_ok()
    {
        runtime.state = next;
    }
}

fn spawn_loop() {
    tauri::async_runtime::spawn(async move {
        let clock = SystemClock::new();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            let now = clock.now();
            let config = {
                let Some(runtime) = RUNTIME.get() else {
                    continue;
                };
                let Ok(mut runtime) = runtime.write() else {
                    continue;
                };
                let due = runtime.state.status.config.enabled
                    && !runtime.state.status.running
                    && runtime
                        .state
                        .next_run_unix
                        .is_some_and(|due| due <= now.unix_timestamp());
                if !due {
                    continue;
                }
                if runtime.state.status.config.require_stable_forecasts {
                    if let Err(reason) = crate::forecast_candidates::require_stable_cohort(
                        &runtime.state.status.config.required_cohort_id,
                    ) {
                        let mut next = runtime.state.clone();
                        next.status.gate_state = "paused".into();
                        next.status.gate_reason = Some(reason);
                        let retry = now.unix_timestamp().saturating_add(300);
                        next.next_run_unix = Some(retry);
                        next.status.next_run_at = Some(
                            time::OffsetDateTime::from_unix_timestamp(retry)
                                .unwrap_or(now)
                                .to_string(),
                        );
                        if runtime
                            .journal
                            .append("calibration_gate_paused", next.clone(), now)
                            .is_ok()
                        {
                            runtime.state = next;
                        }
                        continue;
                    }
                }
                let mut next = runtime.state.clone();
                next.status.running = true;
                next.status.last_error = None;
                next.status.gate_state = "active".into();
                next.status.gate_reason = None;
                if runtime
                    .journal
                    .append("run_started", next.clone(), now)
                    .is_err()
                {
                    continue;
                }
                runtime.state = next;
                runtime.state.status.config.clone()
            };
            let result = crate::model_integrations::run_market_research(
                config.provider_id.clone(),
                config.model.clone(),
                config.question.clone(),
                config.max_output_tokens,
                config.max_cost_micros,
            )
            .await;
            let completed_at = clock.now();
            let Some(runtime) = RUNTIME.get() else {
                continue;
            };
            let Ok(mut runtime) = runtime.write() else {
                continue;
            };
            let mut next = runtime.state.clone();
            next.status.running = false;
            next.status.last_run_at = Some(completed_at.to_string());
            let event = match result {
                Ok(result) => {
                    next.status.successful_runs = next.status.successful_runs.saturating_add(1);
                    next.status.last_error = None;
                    next.status.last_result = Some(result);
                    "run_succeeded"
                },
                Err(error) => {
                    next.status.failed_runs = next.status.failed_runs.saturating_add(1);
                    next.status.last_error = Some(error);
                    "run_failed"
                },
            };
            let due = completed_at
                .unix_timestamp()
                .saturating_add(config.interval_seconds as i64);
            next.next_run_unix = next.status.config.enabled.then_some(due);
            next.status.next_run_at = next.status.config.enabled.then(|| {
                time::OffsetDateTime::from_unix_timestamp(due)
                    .unwrap_or(completed_at)
                    .to_string()
            });
            if runtime
                .journal
                .append(event, next.clone(), completed_at)
                .is_ok()
            {
                runtime.state = next;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enabled_config_requires_a_complete_bounded_mandate() {
        let mut config = AutonomousResearchConfig {
            enabled: true,
            ..Default::default()
        };
        assert!(validate_config(&config).is_err());
        config.provider_id = "openai".into();
        config.model = "gpt-test".into();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn interrupted_run_is_failed_closed_and_rescheduled() {
        let mut state = LoopState::default();
        state.status.config.enabled = true;
        state.status.running = true;
        let (state, event) = recover_startup(state, time::OffsetDateTime::UNIX_EPOCH);
        assert_eq!(event, Some("interrupted_run_recovered"));
        assert!(!state.status.running);
        assert_eq!(state.status.failed_runs, 1);
        assert_eq!(state.next_run_unix, Some(0));
        assert!(state.status.last_result.is_none());
    }

    #[test]
    fn stable_forecast_gate_requires_named_cohort() {
        let mut config = AutonomousResearchConfig {
            enabled: true,
            provider_id: "openai".into(),
            model: "model".into(),
            require_stable_forecasts: true,
            ..Default::default()
        };
        assert!(validate_config(&config).is_err());
        config.required_cohort_id = "openai:model:SPY:60m".into();
        assert!(validate_config(&config).is_ok());
    }
}
