//! Autonomy modes and the gate that guards live execution.
//!
//! Three modes, in increasing order of consequence:
//!
//! - **Advisory** — the loop runs, journals what it would have done, and
//!   submits nothing. Decisions still get scored, so the system can build a
//!   track record without taking any position at all.
//! - **Paper** — the default. Orders go to the local paper OMS.
//! - **Live** — orders go to a real broker.
//!
//! Live is not a setting you can simply switch on. Every one of the gates below
//! is re-evaluated on *every* loop iteration, and any single failure drops the
//! iteration back to paper. That matters more than it might appear: a gate
//! checked once at enable time protects nothing, because the conditions that
//! justified enabling it — a healthy cohort, an armed breaker, a human paying
//! attention — all decay.
//!
//! The arming step is deliberately time-boxed. An operator arms live trading
//! for a bounded window; when it expires the system falls back to paper without
//! asking. Autonomy that cannot expire is not autonomy, it is abandonment.

use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use serde::{Deserialize, Serialize};

use crate::forecast_candidates::CohortSkill;

static ARMING: OnceLock<Mutex<LiveArming>> = OnceLock::new();
static ARMING_PATH: OnceLock<PathBuf> = OnceLock::new();

fn arming_cell() -> &'static Mutex<LiveArming> {
    ARMING.get_or_init(|| Mutex::new(LiveArming::default()))
}

/// Load the persisted live authorisation.
///
/// Deliberately stored in its own file rather than inside the trader's state
/// journal. That journal verifies each record by re-serializing the decoded
/// struct and comparing digests, so adding any field to a journaled type makes
/// every previously written record fail its integrity check — the app then
/// refuses to start. Authorisation state also has no business sharing a blob
/// with tuning config: it should not be possible to grant live trading as a
/// side effect of persisting an interval change.
pub(crate) fn initialize(data_dir: &std::path::Path) -> Result<(), String> {
    let path = data_dir.join("live-arming.json");
    let existing: LiveArming = if path.exists() {
        fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    } else {
        LiveArming::default()
    };
    let _ = ARMING_PATH.set(path);
    if let Ok(mut guard) = arming_cell().lock() {
        *guard = existing;
    }
    Ok(())
}

/// Current live authorisation. Never fails: an unreadable store means unarmed,
/// which is the safe reading.
pub(crate) fn read_arming() -> LiveArming {
    arming_cell()
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Persist a live authorisation.
pub(crate) fn write_arming(arming: &LiveArming) -> Result<(), String> {
    let mut guard = arming_cell()
        .lock()
        .map_err(|_| "live arming state unavailable")?;
    *guard = arming.clone();
    if let Some(path) = ARMING_PATH.get() {
        let bytes = serde_json::to_vec_pretty(arming)
            .map_err(|error| format!("serialize arming: {error}"))?;
        fs::write(path, bytes).map_err(|error| format!("write arming: {error}"))?;
    }
    Ok(())
}

/// How much consequence the loop is permitted.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AutonomyMode {
    /// Journal the decision; submit nothing anywhere.
    Advisory,
    /// Submit to the local paper OMS.
    #[default]
    Paper,
    /// Submit to a real broker, subject to every gate in this module.
    Live,
}

impl AutonomyMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::Paper => "paper",
            Self::Live => "live",
        }
    }
}

/// Minimum resolved forecasts before a cohort can support live size.
pub(crate) const LIVE_MIN_RESOLVED: usize = 40;

/// Minimum measured skill, in ppm, for live execution. 5 percentage points.
pub(crate) const LIVE_MIN_SKILL_PPM: i64 = 50_000;

/// Longest an operator may arm live trading for, in seconds. Eight hours — one
/// trading session, not one weekend.
pub(crate) const MAX_ARM_SECONDS: u64 = 8 * 60 * 60;

/// What the operator has authorised, and until when.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveArming {
    /// RFC3339 instant after which the arming is void.
    pub(crate) armed_until: Option<String>,
    /// Who armed it, for the audit trail.
    pub(crate) armed_by: Option<String>,
    /// Broker the operator authorised. Orders may go nowhere else.
    pub(crate) broker: Option<String>,
}

/// Inputs the gate reasons over. Gathered by the caller so this stays pure.
#[derive(Clone, Debug)]
pub(crate) struct GateInputs<'a> {
    pub(crate) requested: AutonomyMode,
    pub(crate) arming: &'a LiveArming,
    pub(crate) now: time::OffsetDateTime,
    /// Measured standing of the cohort behind this specific decision.
    pub(crate) skill: Option<CohortSkill>,
    pub(crate) breaker_armed: bool,
    pub(crate) ladder_permits_new: bool,
    pub(crate) budget_remaining_micros: i64,
    /// Whether a broker session is actually connected.
    pub(crate) broker_connected: bool,
}

/// The decision: what mode this iteration may actually use, and why.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GateOutcome {
    /// The mode that will really be used. Never higher than requested.
    pub(crate) effective: AutonomyMode,
    /// Every gate that failed. Empty when the requested mode was granted.
    pub(crate) blocked_by: Vec<String>,
    pub(crate) rationale: String,
}

/// Resolve the mode this iteration is allowed to operate in.
///
/// Downgrades are silent-by-design in one direction only: the effective mode is
/// never *above* the requested one, and every downgrade names its cause.
pub(crate) fn resolve(inputs: &GateInputs<'_>) -> GateOutcome {
    // Advisory and paper carry no live risk, so they pass straight through.
    // Paper still respects the ladder and breaker, but those are enforced by
    // the trader loop itself rather than by this gate.
    if inputs.requested != AutonomyMode::Live {
        return GateOutcome {
            effective: inputs.requested,
            blocked_by: Vec::new(),
            rationale: format!(
                "{} mode carries no live execution",
                inputs.requested.label()
            ),
        };
    }

    let mut blocked = Vec::new();

    match parse_time(inputs.arming.armed_until.as_deref()) {
        Some(expiry) if expiry > inputs.now => {},
        Some(_) => blocked.push("live arming has expired".to_owned()),
        None => blocked.push("live trading has not been armed by an operator".to_owned()),
    }

    if inputs.arming.broker.is_none() {
        blocked.push("no broker was named at arming time".to_owned());
    } else if !inputs.broker_connected {
        blocked.push("the armed broker has no connected session".to_owned());
    }

    match inputs.skill {
        Some(CohortSkill {
            skill_ppm: Some(ppm),
            sample_count,
            ..
        }) => {
            if sample_count < LIVE_MIN_RESOLVED {
                blocked.push(format!(
                    "cohort has {sample_count} resolved forecasts, {LIVE_MIN_RESOLVED} required for live"
                ));
            }
            if ppm < LIVE_MIN_SKILL_PPM {
                blocked.push(format!(
                    "measured skill {:.1}% is below the {:.1}% live threshold",
                    ppm as f64 / 10_000.0,
                    LIVE_MIN_SKILL_PPM as f64 / 10_000.0,
                ));
            }
        },
        _ => blocked.push(
            "cohort has no measured skill — unproven forecasters never trade live".to_owned(),
        ),
    }

    if !inputs.breaker_armed {
        blocked.push("circuit breaker is tripped".to_owned());
    }
    if !inputs.ladder_permits_new {
        blocked.push("drawdown ladder forbids new positions".to_owned());
    }
    if inputs.budget_remaining_micros <= 0 {
        blocked.push("trading budget is exhausted".to_owned());
    }

    if blocked.is_empty() {
        GateOutcome {
            effective: AutonomyMode::Live,
            blocked_by: Vec::new(),
            rationale: format!(
                "live execution authorised until {}",
                inputs.arming.armed_until.as_deref().unwrap_or("unknown")
            ),
        }
    } else {
        // The fallback is paper, never advisory: the operator asked for
        // execution, so the decision should still be recorded as a fill
        // somewhere it can be scored.
        GateOutcome {
            effective: AutonomyMode::Paper,
            rationale: format!("downgraded to paper — {}", blocked.join("; ")),
            blocked_by: blocked,
        }
    }
}

fn parse_time(value: Option<&str>) -> Option<time::OffsetDateTime> {
    time::OffsetDateTime::parse(value?, &time::format_description::well_known::Rfc3339).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> time::OffsetDateTime {
        time::OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap()
    }

    fn rfc3339(offset_seconds: i64) -> String {
        (now() + time::Duration::seconds(offset_seconds))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap()
    }

    fn good_skill() -> Option<CohortSkill> {
        Some(CohortSkill {
            skill_ppm: Some(120_000),
            sample_count: 60,
            state: "stable",
        })
    }

    fn armed() -> LiveArming {
        LiveArming {
            armed_until: Some(rfc3339(3_600)),
            armed_by: Some("operator".into()),
            broker: Some("alpaca".into()),
        }
    }

    fn passing<'a>(arming: &'a LiveArming) -> GateInputs<'a> {
        GateInputs {
            requested: AutonomyMode::Live,
            arming,
            now: now(),
            skill: good_skill(),
            breaker_armed: true,
            ladder_permits_new: true,
            budget_remaining_micros: 1_000_000,
            broker_connected: true,
        }
    }

    #[test]
    fn a_fully_satisfied_gate_grants_live() {
        let arming = armed();
        let outcome = resolve(&passing(&arming));
        assert_eq!(outcome.effective, AutonomyMode::Live);
        assert!(outcome.blocked_by.is_empty());
    }

    #[test]
    fn paper_and_advisory_bypass_the_gate_entirely() {
        let arming = LiveArming::default();
        for mode in [AutonomyMode::Advisory, AutonomyMode::Paper] {
            let mut inputs = passing(&arming);
            inputs.requested = mode;
            let outcome = resolve(&inputs);
            assert_eq!(outcome.effective, mode);
            assert!(outcome.blocked_by.is_empty());
        }
    }

    #[test]
    fn expired_arming_downgrades_to_paper() {
        let arming = LiveArming {
            armed_until: Some(rfc3339(-1)),
            ..armed()
        };
        let outcome = resolve(&passing(&arming));
        assert_eq!(outcome.effective, AutonomyMode::Paper);
        assert!(outcome.blocked_by.iter().any(|b| b.contains("expired")));
    }

    #[test]
    fn unarmed_live_is_refused() {
        let arming = LiveArming::default();
        let outcome = resolve(&passing(&arming));
        assert_eq!(outcome.effective, AutonomyMode::Paper);
        assert!(outcome
            .blocked_by
            .iter()
            .any(|b| b.contains("not been armed")));
    }

    #[test]
    fn an_unproven_cohort_never_trades_live() {
        let arming = armed();
        let mut inputs = passing(&arming);
        inputs.skill = None;
        let outcome = resolve(&inputs);
        assert_eq!(outcome.effective, AutonomyMode::Paper);
        assert!(outcome
            .blocked_by
            .iter()
            .any(|b| b.contains("no measured skill")));
    }

    #[test]
    fn a_thin_but_positive_cohort_is_refused_live() {
        let arming = armed();
        let mut inputs = passing(&arming);
        inputs.skill = Some(CohortSkill {
            skill_ppm: Some(400_000),
            sample_count: LIVE_MIN_RESOLVED - 1,
            state: "stable",
        });
        let outcome = resolve(&inputs);
        assert_eq!(outcome.effective, AutonomyMode::Paper);
        assert!(outcome
            .blocked_by
            .iter()
            .any(|b| b.contains("resolved forecasts")));
    }

    #[test]
    fn skill_below_the_live_threshold_is_refused() {
        let arming = armed();
        let mut inputs = passing(&arming);
        inputs.skill = Some(CohortSkill {
            skill_ppm: Some(LIVE_MIN_SKILL_PPM - 1),
            sample_count: 100,
            state: "stable",
        });
        let outcome = resolve(&inputs);
        assert_eq!(outcome.effective, AutonomyMode::Paper);
        assert!(outcome.blocked_by.iter().any(|b| b.contains("below the")));
    }

    #[test]
    fn each_risk_control_independently_blocks_live() {
        let arming = armed();
        type Mutate = fn(&mut GateInputs<'_>);
        let cases: [(&str, Mutate); 4] = [
            ("circuit breaker", |i| i.breaker_armed = false),
            ("drawdown ladder", |i| i.ladder_permits_new = false),
            ("budget", |i| i.budget_remaining_micros = 0),
            ("connected session", |i| i.broker_connected = false),
        ];
        for (needle, mutate) in cases {
            let mut inputs = passing(&arming);
            mutate(&mut inputs);
            let outcome = resolve(&inputs);
            assert_eq!(
                outcome.effective,
                AutonomyMode::Paper,
                "expected {needle} to block live"
            );
            assert!(
                outcome.blocked_by.iter().any(|b| b.contains(needle)),
                "expected a reason mentioning {needle}, got {:?}",
                outcome.blocked_by
            );
        }
    }

    #[test]
    fn every_failure_is_reported_not_just_the_first() {
        let arming = LiveArming::default();
        let mut inputs = passing(&arming);
        inputs.skill = None;
        inputs.breaker_armed = false;
        let outcome = resolve(&inputs);
        assert!(
            outcome.blocked_by.len() >= 3,
            "expected several reasons, got {:?}",
            outcome.blocked_by
        );
    }

    #[test]
    fn the_effective_mode_is_never_above_the_requested_one() {
        let arming = armed();
        for mode in [
            AutonomyMode::Advisory,
            AutonomyMode::Paper,
            AutonomyMode::Live,
        ] {
            let mut inputs = passing(&arming);
            inputs.requested = mode;
            let effective = resolve(&inputs).effective;
            let rank = |m: AutonomyMode| match m {
                AutonomyMode::Advisory => 0,
                AutonomyMode::Paper => 1,
                AutonomyMode::Live => 2,
            };
            assert!(rank(effective) <= rank(mode));
        }
    }
}
