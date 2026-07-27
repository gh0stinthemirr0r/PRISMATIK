//! Managed desktop market state (GCRA budget + determinism clock).

use std::sync::Arc;

use prismatik_determinism::{
    DeterminismContext, PinnedArtifactSet, RunId, SplitEntropy, SystemClock,
};
use prismatik_domain::ProviderId;
use prismatik_market_data::{
    AdmissionDecision, BudgetGovernor, BudgetState, CostUnits, EndpointId, Entitlement,
    GcraBudgetGovernor, PriorityClass,
};
use serde::Serialize;
use tauri::State;
use time::OffsetDateTime;

/// Shared shell state for rate-budget UI.
pub struct MarketRuntime {
    /// GCRA governor (CoinGecko demo defaults).
    pub governor: GcraBudgetGovernor,
    /// Wall-clock determinism context for the desktop shell.
    pub ctx: DeterminismContext,
}

impl MarketRuntime {
    /// Construct with SystemClock (shell-only; Layer 2 stays cassette-deterministic).
    pub fn new() -> Self {
        Self {
            governor: GcraBudgetGovernor::coingecko_demo(),
            ctx: DeterminismContext {
                clock: Arc::new(SystemClock::new()),
                entropy: Box::new(SplitEntropy::from_seed(0x50524953)),
                run_id: RunId::test(),
                pinned: PinnedArtifactSet::default(),
            },
        }
    }
}

/// UI budget snapshot.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiBudgetState {
    pub provider: String,
    pub remaining_ratio: f64,
    pub next_permit_at: Option<String>,
    pub period_resets_at: String,
    pub ready: bool,
}

/// Result of spending a rate-budget permit.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAdmitResult {
    pub kind: String,
    pub remaining_ratio: f64,
    pub next_permit_at: Option<String>,
    pub message: String,
}

fn fmt_rfc3339(t: OffsetDateTime) -> String {
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| t.to_string())
}

fn map_budget(provider: ProviderId, s: BudgetState) -> UiBudgetState {
    UiBudgetState {
        provider: provider.to_string(),
        remaining_ratio: s.remaining_ratio,
        next_permit_at: s.next_permit_at.map(fmt_rfc3339),
        period_resets_at: fmt_rfc3339(s.period_resets_at),
        ready: s.next_permit_at.is_none() && s.remaining_ratio > 0.05,
    }
}

/// Current CoinGecko GCRA budget for the rate-budget meter.
#[tauri::command]
pub fn get_rate_budget_state(state: State<'_, MarketRuntime>) -> UiBudgetState {
    let s = state
        .governor
        .budget_state(ProviderId::COINGECKO, &state.ctx);
    map_budget(ProviderId::COINGECKO, s)
}

/// Spend one interactive permit against CoinGecko (exact defer times on deny).
#[tauri::command]
pub async fn spend_rate_budget(state: State<'_, MarketRuntime>) -> Result<UiAdmitResult, String> {
    let decision = state
        .governor
        .admit(
            ProviderId::COINGECKO,
            EndpointId::new("ui.spend"),
            CostUnits::new(1),
            PriorityClass::UserInitiated,
            Some(Entitlement::CoinGeckoDemo),
            &[Entitlement::CoinGeckoDemo, Entitlement::CoinGeckoPro],
            &state.ctx,
        )
        .await;

    let snap = state
        .governor
        .budget_state(ProviderId::COINGECKO, &state.ctx);

    let result = match decision {
        AdmissionDecision::Admit { permit } => UiAdmitResult {
            kind: "admit".into(),
            remaining_ratio: snap.remaining_ratio,
            next_permit_at: None,
            message: format!("permit #{} granted", permit.id),
        },
        AdmissionDecision::Defer { retry_at, position } => UiAdmitResult {
            kind: "defer".into(),
            remaining_ratio: snap.remaining_ratio,
            next_permit_at: Some(fmt_rfc3339(retry_at)),
            message: format!("deferred · queue {position}"),
        },
        AdmissionDecision::BudgetExhausted { resets_at, class } => UiAdmitResult {
            kind: "exhausted".into(),
            remaining_ratio: snap.remaining_ratio,
            next_permit_at: Some(fmt_rfc3339(resets_at)),
            message: format!("exhausted · class {class:?}"),
        },
        AdmissionDecision::NotEntitled { required } => UiAdmitResult {
            kind: "not_entitled".into(),
            remaining_ratio: snap.remaining_ratio,
            next_permit_at: None,
            message: format!("not entitled: {required:?}"),
        },
    };
    Ok(result)
}
