//! Durable, unsigned StrategyIR authoring for the desktop workbench.

use std::{
    path::Path,
    sync::{OnceLock, RwLock},
};

use prismatik_application::FileStateJournal;
use prismatik_determinism::{Clock, SystemClock};
use prismatik_strategy::{AuthoringContract, StrategyIr, StrategyValidation, UniverseSpec};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StrategyDraftRecord {
    strategy: StrategyIr,
    created_at: String,
    updated_at: String,
    state: String,
    execution_eligible: bool,
}

#[derive(Debug)]
struct Runtime {
    journal: FileStateJournal<Vec<StrategyDraftRecord>>,
    drafts: Vec<StrategyDraftRecord>,
}

static RUNTIME: OnceLock<RwLock<Runtime>> = OnceLock::new();

pub(crate) fn initialize(data_dir: &Path) -> Result<(), String> {
    let mut journal: FileStateJournal<Vec<StrategyDraftRecord>> = FileStateJournal::open(
        data_dir.join("strategy-drafts.jsonl"),
        "prismatik.strategy-drafts.v1",
    )
    .map_err(|error| error.to_string())?;
    let drafts = journal.latest().cloned().unwrap_or_default();
    if journal.latest().is_none() {
        journal
            .append("initialized", drafts.clone(), SystemClock::new().now())
            .map_err(|error| error.to_string())?;
    }
    RUNTIME
        .set(RwLock::new(Runtime { journal, drafts }))
        .map_err(|_| "strategy authoring initialized twice".to_owned())
}

#[tauri::command]
pub(crate) fn get_strategy_authoring_contract() -> AuthoringContract {
    AuthoringContract::native()
}

#[tauri::command]
pub(crate) fn validate_strategy(strategy: StrategyIr) -> StrategyValidation {
    validate(&strategy)
}

#[tauri::command]
pub(crate) fn list_strategy_drafts() -> Result<Vec<StrategyDraftRecord>, String> {
    let mut drafts = RUNTIME
        .get()
        .ok_or("strategy authoring is unavailable")?
        .read()
        .map_err(|_| "strategy authoring lock is unavailable".to_owned())?
        .drafts
        .clone();
    drafts.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.strategy.strategy_id.cmp(&right.strategy.strategy_id))
    });
    Ok(drafts)
}

#[tauri::command]
pub(crate) fn save_strategy(strategy: StrategyIr) -> Result<StrategyDraftRecord, String> {
    let validation = validate(&strategy);
    if !validation.valid {
        return Err(validation
            .issues
            .into_iter()
            .map(|issue| format!("{}: {}", issue.code, issue.message))
            .collect::<Vec<_>>()
            .join("; "));
    }
    let now = SystemClock::new().now();
    let now_text = now.format(&Rfc3339).map_err(|error| error.to_string())?;
    let mut runtime = RUNTIME
        .get()
        .ok_or("strategy authoring is unavailable")?
        .write()
        .map_err(|_| "strategy authoring lock is unavailable".to_owned())?;
    let created_at = runtime
        .drafts
        .iter()
        .find(|draft| draft.strategy.strategy_id == strategy.strategy_id)
        .map_or_else(|| now_text.clone(), |draft| draft.created_at.clone());
    let record = StrategyDraftRecord {
        strategy,
        created_at,
        updated_at: now_text,
        state: "unsigned_draft".into(),
        execution_eligible: false,
    };
    let mut next = runtime.drafts.clone();
    if let Some(index) = next
        .iter()
        .position(|draft| draft.strategy.strategy_id == record.strategy.strategy_id)
    {
        next[index] = record.clone();
    } else {
        next.push(record.clone());
    }
    runtime
        .journal
        .append("draft_saved", next.clone(), now)
        .map_err(|error| format!("strategy draft was not persisted: {error}"))?;
    runtime.drafts = next;
    Ok(record)
}

fn validate(strategy: &StrategyIr) -> StrategyValidation {
    let contract = AuthoringContract::native();
    let mut result = contract.validate(strategy);
    let mut push = |code: &str, message: &str| {
        result.issues.push(prismatik_strategy::ValidationIssue {
            code: code.into(),
            message: message.into(),
        });
    };
    if strategy.strategy_id.trim().is_empty() || strategy.strategy_id.len() > 128 {
        push(
            "strategy_id",
            "strategy id must contain 1 to 128 characters",
        );
    }
    if strategy.name.trim().is_empty() || strategy.name.len() > 160 {
        push("name", "strategy name must contain 1 to 160 characters");
    }
    if strategy.indicators.len() > 128 {
        push(
            "indicators",
            "strategy drafts support at most 128 indicators",
        );
    }
    match &strategy.universe {
        UniverseSpec::Static { static_members } => {
            if static_members.is_empty() {
                push(
                    "universe",
                    "static universe must contain at least one asset identifier",
                );
            } else if static_members.len() > 1_000 {
                push("universe", "static universe supports at most 1,000 assets");
            } else if static_members
                .iter()
                .any(|asset| asset.trim().is_empty() || asset.len() > 128)
            {
                push(
                    "universe",
                    "asset identifiers must contain 1 to 128 characters",
                );
            }
        },
        UniverseSpec::DynamicQuery { .. } => {
            push(
                "universe",
                "dynamic query universes are not enabled in the desktop draft floor",
            );
        },
    }
    if !strategy.rules.is_object() {
        push("rules", "rules must be a bounded JSON object");
    } else if ["entries", "exits"].iter().any(|key| {
        !strategy
            .rules
            .get(key)
            .is_some_and(serde_json::Value::is_array)
    }) {
        push("rules", "rules must contain entries and exits arrays");
    }
    if serde_json::to_vec(strategy).map_or(true, |bytes| bytes.len() > 262_144) {
        push("size", "serialized StrategyIR must not exceed 256 KiB");
    }
    result.valid = result.issues.is_empty();
    result
}

pub(crate) fn audit_events() -> Result<Vec<crate::audit_timeline::AuditEvent>, String> {
    Ok(list_strategy_drafts()?
        .into_iter()
        .take(100)
        .filter_map(|draft| {
            OffsetDateTime::parse(&draft.updated_at, &Rfc3339)
                .ok()
                .map(|_| {
                    let asset_count = match &draft.strategy.universe {
                        UniverseSpec::Static { static_members } => static_members.len(),
                        UniverseSpec::DynamicQuery { .. } => 0,
                    };
                    crate::audit_timeline::AuditEvent {
                        id: format!(
                            "strategy-draft:{}:{}",
                            draft.strategy.strategy_id, draft.updated_at
                        ),
                        occurred_at: draft.updated_at,
                        domain: "strategy",
                        severity: "info",
                        state: draft.state,
                        title: draft.strategy.name,
                        summary: format!(
                            "Unsigned StrategyIR draft · {asset_count} asset(s) · {} indicator(s) · execution blocked",
                            draft.strategy.indicators.len()
                        ),
                        evidence_id: None,
                        route: "/workspace/strategy",
                        durable: true,
                    }
                })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::validate;
    use prismatik_strategy::{StrategyIr, UniverseSpec};

    #[test]
    fn empty_universe_is_rejected() {
        let strategy = StrategyIr::minimal("id", "name");
        let result = validate(&strategy);
        assert!(!result.valid);
        assert!(result.issues.iter().any(|issue| issue.code == "universe"));
    }

    #[test]
    fn deterministic_static_draft_is_accepted() {
        let mut strategy = StrategyIr::minimal("id", "name");
        let UniverseSpec::Static { static_members } = &mut strategy.universe else {
            unreachable!()
        };
        static_members.push("BTC/USD".into());
        assert!(validate(&strategy).valid);
    }
}
