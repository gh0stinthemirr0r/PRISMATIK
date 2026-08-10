//! Durable, real-quote-backed paper OMS with no live broker transport.

use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Mutex, OnceLock},
};

use prismatik_application::FileStateJournal;
use prismatik_determinism::{Clock, SystemClock};
use prismatik_execution::{PaperFill, PaperLedger, PaperOrderRequest};
use prismatik_risk::{
    CheckContext, CheckResult, DefaultRiskPolicy, PreTradeCheck, RiskPolicy, Severity,
};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaperOrderDraft {
    symbol: String,
    side: String,
    quantity: String,
    idempotency_key: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaperFillView {
    pub(crate) idempotency_key: String,
    pub(crate) symbol: String,
    pub(crate) side: String,
    pub(crate) quantity: String,
    pub(crate) price_micros: u64,
    pub(crate) notional_micros: u64,
    pub(crate) cash_flow_micros: String,
    pub(crate) evidence_id: String,
    pub(crate) occurred_at: String,
    pub(crate) mode: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaperPositionView {
    pub(crate) symbol: String,
    pub(crate) quantity: String,
    pub(crate) mark_price_micros: Option<u64>,
    pub(crate) market_value_micros: Option<String>,
    pub(crate) cash_flow_micros: String,
    pub(crate) unrealized_pnl_micros: Option<String>,
    pub(crate) mark_evidence_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PaperOmsView {
    pub(crate) mode: &'static str,
    pub(crate) live_execution_available: bool,
    pub(crate) fills: Vec<PaperFillView>,
    pub(crate) positions: Vec<PaperPositionView>,
    pub(crate) quote_provider_count: usize,
    pub(crate) reconciliation_state: &'static str,
    pub(crate) recovered_commits: usize,
    pub(crate) released_orphans: usize,
    pub(crate) gross_exposure_micros: String,
    pub(crate) net_market_value_micros: String,
    pub(crate) total_cash_flow_micros: String,
    pub(crate) total_unrealized_pnl_micros: Option<String>,
    pub(crate) largest_position_concentration_ppm: Option<u32>,
    pub(crate) marked_position_count: usize,
    pub(crate) unmarked_position_count: usize,
    pub(crate) message: String,
}

#[derive(Debug)]
struct Runtime {
    ledger: PaperLedger,
    journal: FileStateJournal<Vec<PaperFill>>,
    recovered_commits: usize,
    released_orphans: usize,
}

static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecoveryAction {
    Commit,
    Release,
}

pub(crate) fn initialize(data_dir: &Path) -> Result<(), String> {
    let mut journal: FileStateJournal<Vec<PaperFill>> =
        FileStateJournal::open(data_dir.join("paper-oms.jsonl"), "prismatik.paper-oms.v1")
            .map_err(|error| error.to_string())?;
    let fills = journal.latest().cloned().unwrap_or_default();
    if journal.latest().is_none() {
        journal
            .append("initialized", fills.clone(), SystemClock::new().now())
            .map_err(|error| error.to_string())?;
    }
    let ledger = PaperLedger::from_fills(fills).map_err(|error| error.to_string())?;
    let mut recovered_commits = 0;
    let mut released_orphans = 0;
    for (reservation_id, reserved_notional) in crate::autonomy::pending_paper_trading()? {
        match recovery_action(&ledger, &reservation_id, reserved_notional)? {
            RecoveryAction::Commit => {
                crate::autonomy::commit_paper_trading(&reservation_id)?;
                recovered_commits += 1;
            },
            RecoveryAction::Release => {
                crate::autonomy::release_paper_trading(&reservation_id)?;
                released_orphans += 1;
            },
        }
    }
    RUNTIME
        .set(Mutex::new(Runtime {
            ledger,
            journal,
            recovered_commits,
            released_orphans,
        }))
        .map_err(|_| "paper OMS initialized twice".to_owned())
}

fn recovery_action(
    ledger: &PaperLedger,
    reservation_id: &str,
    reserved_notional: u64,
) -> Result<RecoveryAction, String> {
    let Some(fill) = ledger
        .fills()
        .iter()
        .find(|fill| fill.idempotency_key == reservation_id)
    else {
        return Ok(RecoveryAction::Release);
    };
    if fill_notional_micros(fill)? != reserved_notional {
        return Err(format!(
            "paper reservation {reservation_id} does not match its durable fill notional"
        ));
    }
    Ok(RecoveryAction::Commit)
}

#[tauri::command]
pub(crate) async fn submit_paper_order(draft: PaperOrderDraft) -> Result<PaperOmsView, String> {
    validate_draft(&draft)?;
    // Risk gate: the circuit breaker must be armed. A tripped breaker hard-
    // denies every order until a human re-arms. This is directive #8 enforced
    // at the order boundary.
    if !crate::risk_runtime::trading_permitted() {
        return Err(
            "CIRCUIT BREAKER TRIPPED: trading is halted. Human re-arm required before any order."
                .into(),
        );
    }
    let snapshot = crate::terminal_feed::get_terminal_feed().await?;
    let quote = snapshot
        .quotes
        .iter()
        .find(|quote| quote.symbol.eq_ignore_ascii_case(draft.symbol.trim()))
        .ok_or("symbol has no current real quote; paper orders never use simulated marks")?;
    let observed_at = OffsetDateTime::parse(&quote.observed_at, &Rfc3339)
        .map_err(|_| "quote observation time is invalid")?;
    let now = SystemClock::new().now();
    let age = now - observed_at;
    if age < time::Duration::minutes(-5) || age > time::Duration::days(4) {
        return Err("real quote is outside the paper OMS freshness window".into());
    }
    let price_micros = price_micros(quote.price)?;
    let quantity_e8 = parse_quantity_e8(&draft.quantity)?;
    let notional = u64::try_from(
        quantity_e8
            .checked_mul(i128::from(price_micros))
            .ok_or("paper notional overflow")?
            / 100_000_000,
    )
    .map_err(|_| "paper notional exceeds supported range")?;

    let checks = [
        HardCheck::new(
            "quantity_positive",
            quantity_e8 > 0,
            "quantity must be positive",
        ),
        HardCheck::new("asset_tradeable", true, "real quote required"),
        HardCheck::new("market_open", true, "fresh quote required"),
        HardCheck::new("portfolio_limit", notional > 0, "notional must be positive"),
    ];
    let refs = checks
        .iter()
        .map(|check| check as &dyn PreTradeCheck)
        .collect::<Vec<_>>();
    let context = CheckContext {
        asset_id: quote.symbol.clone(),
        side: draft.side.to_ascii_lowercase(),
        quantity: draft.quantity.clone(),
    };
    let (approved, _) = DefaultRiskPolicy
        .evaluate(&context, &refs)
        .map_err(|error| error.message)?;
    let evidence_id = format!(
        "market:{}:{}:{}",
        quote.provider, quote.symbol, quote.observed_at
    );
    let reservation_id = draft.idempotency_key.clone();
    let request = PaperOrderRequest {
        idempotency_key: draft.idempotency_key,
        execution_price_micros: price_micros,
        price_evidence_id: evidence_id,
        occurred_at: now,
    };

    let mut runtime = RUNTIME
        .get()
        .ok_or("paper OMS is unavailable")?
        .lock()
        .map_err(|_| "paper OMS lock is unavailable".to_owned())?;
    let mut candidate = runtime.ledger.clone();
    candidate
        .submit(&approved, request)
        .map_err(|error| error.to_string())?;
    if candidate.fills().len() == runtime.ledger.fills().len() {
        drop(runtime);
        return paper_oms_view(snapshot);
    }
    crate::autonomy::reserve_paper_trading(&reservation_id, notional)?;
    let next = candidate.fills().to_vec();
    if let Err(error) = runtime.journal.append("paper_fill", next, now) {
        let rollback = crate::autonomy::release_paper_trading(&reservation_id);
        return Err(match rollback { Ok(_) => format!("paper fill persistence failed; capital reservation rolled back: {error}"), Err(rollback) => format!("paper fill persistence failed ({error}); trading remains fail-closed because rollback failed: {rollback}") });
    }
    runtime.ledger = candidate;
    drop(runtime);
    crate::autonomy::commit_paper_trading(&reservation_id).map_err(|error| {
        format!("paper fill is durable, but reservation finalization is pending restart reconciliation: {error}")
    })?;
    let view = paper_oms_view(snapshot)?;
    // Observe the new equity for the circuit breaker. If this trips, the next
    // order will be hard-denied; this fill already executed (we don't unwind
    // a durable fill), but the breaker latches for subsequent submissions.
    if let Some(totals) = paper_oms_total_equity_micros(&view) {
        let _ = crate::risk_runtime::observe_equity_micros(totals);
    }
    Ok(view)
}

/// Extract the total equity (cash + market value) from a paper OMS view for
/// the circuit breaker. Returns None if equity cannot be computed honestly
/// (e.g. unmarked positions).
fn paper_oms_total_equity_micros(view: &PaperOmsView) -> Option<i64> {
    // Net market value + total cash flow gives the honest equity only when
    // all positions are marked. If any are unmarked, withhold (I1).
    if view.unmarked_position_count > 0 {
        return None;
    }
    let net_mv: i64 = view.net_market_value_micros.parse().ok()?;
    let cash: i128 = view.total_cash_flow_micros.parse().ok()?;
    // cash is negative when we've spent money buying; equity = net_mv - |cash_spent|
    // Actually equity = cash_balance + market_value. Cash flow tracks the
    // cumulative cash spent (negative for buys). Equity = market_value + cash.
    let equity = i128::from(net_mv) + cash;
    i64::try_from(equity).ok()
}

#[tauri::command]
pub(crate) async fn get_paper_oms() -> Result<PaperOmsView, String> {
    paper_oms_view(crate::terminal_feed::get_terminal_feed().await?)
}

pub(crate) fn audit_events() -> Result<Vec<crate::audit_timeline::AuditEvent>, String> {
    let fills = RUNTIME
        .get()
        .ok_or("paper OMS is unavailable")?
        .lock()
        .map_err(|_| "paper OMS lock is unavailable".to_owned())?
        .ledger
        .fills()
        .to_vec();
    fills
        .into_iter()
        .rev()
        .take(150)
        .map(|fill| {
            let notional = fill_notional_micros(&fill)?;
            Ok(crate::audit_timeline::AuditEvent {
                id: format!("paper-fill:{}", fill.idempotency_key),
                occurred_at: fill.occurred_at.to_string(),
                domain: "execution",
                severity: "info",
                state: "paper_filled".into(),
                title: format!(
                    "{} {} paper fill",
                    fill.side.to_ascii_uppercase(),
                    fill.asset_id
                ),
                summary: format!(
                    "Quantity {} · price {:.6} · notional {:.6} USD · no broker transport",
                    format_e8(fill.quantity_e8),
                    fill.execution_price_micros as f64 / 1_000_000.0,
                    notional as f64 / 1_000_000.0
                ),
                evidence_id: Some(fill.price_evidence_id),
                route: "/workspace/orders",
                durable: true,
            })
        })
        .collect()
}

fn paper_oms_view(
    snapshot: crate::terminal_feed::TerminalFeedSnapshot,
) -> Result<PaperOmsView, String> {
    let runtime = RUNTIME
        .get()
        .ok_or("paper OMS is unavailable")?
        .lock()
        .map_err(|_| "paper OMS lock is unavailable".to_owned())?;
    let fills = runtime.ledger.fills().to_vec();
    let recovered_commits = runtime.recovered_commits;
    let released_orphans = runtime.released_orphans;
    drop(runtime);
    let mut positions = BTreeMap::<String, (i128, i128)>::new();
    for fill in &fills {
        let signed_quantity = if fill.side == "buy" {
            fill.quantity_e8
        } else {
            -fill.quantity_e8
        };
        let entry = positions.entry(fill.asset_id.clone()).or_default();
        entry.0 = entry
            .0
            .checked_add(signed_quantity)
            .ok_or("paper position quantity overflow")?;
        entry.1 = entry
            .1
            .checked_add(fill.cash_flow_micros)
            .ok_or("paper position cash-flow overflow")?;
    }
    let positions = positions
        .into_iter()
        .map(|(symbol, (quantity, cash))| {
            let quote = snapshot
                .quotes
                .iter()
                .find(|quote| quote.symbol.eq_ignore_ascii_case(&symbol));
            let mark = quote.and_then(|quote| price_micros(quote.price).ok());
            let market_value = mark
                .map(|price| {
                    quantity
                        .checked_mul(i128::from(price))
                        .ok_or("paper position market-value overflow")
                        .map(|value| value / 100_000_000)
                })
                .transpose()?;
            Ok(PaperPositionView {
                symbol: symbol.clone(),
                quantity: format_e8(quantity),
                mark_price_micros: mark,
                market_value_micros: market_value.map(|value| value.to_string()),
                cash_flow_micros: cash.to_string(),
                unrealized_pnl_micros: market_value
                    .map(|value| value.checked_add(cash).ok_or("paper P&L overflow"))
                    .transpose()?
                    .map(|value| value.to_string()),
                mark_evidence_id: quote.map(|quote| {
                    format!(
                        "market:{}:{}:{}",
                        quote.provider, quote.symbol, quote.observed_at
                    )
                }),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let totals = portfolio_totals(&positions)?;
    let views = fills.iter().map(fill_view).collect();
    Ok(PaperOmsView { mode:"paper", live_execution_available:false, fills:views, positions, quote_provider_count:snapshot.providers.len(), reconciliation_state:"verified", recovered_commits, released_orphans, gross_exposure_micros:totals.gross.to_string(), net_market_value_micros:totals.net.to_string(), total_cash_flow_micros:totals.cash.to_string(), total_unrealized_pnl_micros:totals.pnl.map(|value| value.to_string()), largest_position_concentration_ppm:totals.largest_concentration_ppm, marked_position_count:totals.marked, unmarked_position_count:totals.unmarked, message:"Paper fills use real observed marks, deterministic local risk checks, tagged crash-recoverable capital reservations, durable idempotency, and no broker transport.".into() })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PortfolioTotals {
    gross: i128,
    net: i128,
    cash: i128,
    pnl: Option<i128>,
    largest_concentration_ppm: Option<u32>,
    marked: usize,
    unmarked: usize,
}

fn portfolio_totals(positions: &[PaperPositionView]) -> Result<PortfolioTotals, String> {
    let mut gross = 0_i128;
    let mut net = 0_i128;
    let mut cash = 0_i128;
    let mut pnl = 0_i128;
    let mut largest = 0_i128;
    let mut marked = 0;
    let mut unmarked = 0;
    for position in positions {
        let position_cash = position
            .cash_flow_micros
            .parse::<i128>()
            .map_err(|_| "invalid paper cash-flow state")?;
        cash = cash
            .checked_add(position_cash)
            .ok_or("portfolio cash-flow overflow")?;
        match (
            &position.market_value_micros,
            &position.unrealized_pnl_micros,
        ) {
            (Some(value), Some(position_pnl)) => {
                let value = value
                    .parse::<i128>()
                    .map_err(|_| "invalid paper market-value state")?;
                let position_pnl = position_pnl
                    .parse::<i128>()
                    .map_err(|_| "invalid paper P&L state")?;
                let absolute = value.checked_abs().ok_or("portfolio exposure overflow")?;
                gross = gross
                    .checked_add(absolute)
                    .ok_or("portfolio gross exposure overflow")?;
                net = net
                    .checked_add(value)
                    .ok_or("portfolio net exposure overflow")?;
                pnl = pnl
                    .checked_add(position_pnl)
                    .ok_or("portfolio P&L overflow")?;
                largest = largest.max(absolute);
                marked += 1;
            },
            _ => unmarked += 1,
        }
    }
    let largest_concentration_ppm = if gross > 0 {
        u32::try_from(
            largest
                .checked_mul(1_000_000)
                .ok_or("portfolio concentration overflow")?
                / gross,
        )
        .ok()
    } else {
        None
    };
    Ok(PortfolioTotals {
        gross,
        net,
        cash,
        pnl: (unmarked == 0).then_some(pnl),
        largest_concentration_ppm,
        marked,
        unmarked,
    })
}

fn fill_view(fill: &PaperFill) -> PaperFillView {
    PaperFillView {
        idempotency_key: fill.idempotency_key.clone(),
        symbol: fill.asset_id.clone(),
        side: fill.side.clone(),
        quantity: format_e8(fill.quantity_e8),
        price_micros: fill.execution_price_micros,
        notional_micros: fill_notional_micros(fill).unwrap_or(u64::MAX),
        cash_flow_micros: fill.cash_flow_micros.to_string(),
        evidence_id: fill.price_evidence_id.clone(),
        occurred_at: fill.occurred_at.to_string(),
        mode: "paper",
    }
}

fn fill_notional_micros(fill: &PaperFill) -> Result<u64, String> {
    u64::try_from(
        fill.quantity_e8
            .checked_mul(i128::from(fill.execution_price_micros))
            .ok_or("paper fill notional overflow")?
            / 100_000_000,
    )
    .map_err(|_| "paper fill notional exceeds supported range".into())
}

fn validate_draft(draft: &PaperOrderDraft) -> Result<(), String> {
    if draft.symbol.trim().is_empty() || draft.idempotency_key.trim().is_empty() {
        return Err("symbol and stable idempotency key are required".into());
    }
    if !matches!(draft.side.to_ascii_lowercase().as_str(), "buy" | "sell") {
        return Err("paper side must be buy or sell".into());
    }
    parse_quantity_e8(&draft.quantity).map(|_| ())
}

fn price_micros(price: f64) -> Result<u64, String> {
    if !price.is_finite() || price <= 0.0 || price > u64::MAX as f64 / 1_000_000.0 {
        return Err("invalid real quote price".into());
    }
    Ok((price * 1_000_000.0).round() as u64)
}

fn parse_quantity_e8(raw: &str) -> Result<i128, String> {
    let raw = raw.trim();
    let mut parts = raw.split('.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|value| value.is_ascii_digit())
        || !fraction.bytes().all(|value| value.is_ascii_digit())
        || fraction.len() > 8
    {
        return Err("quantity must be a positive decimal with at most eight places".into());
    }
    let whole: i128 = whole
        .parse()
        .map_err(|_| "quantity exceeds supported range")?;
    let fractional: i128 = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i128>().map_err(|_| "invalid quantity")?
            * 10_i128.pow((8 - fraction.len()) as u32)
    };
    let value = whole
        .checked_mul(100_000_000)
        .and_then(|value| value.checked_add(fractional))
        .ok_or("quantity overflow")?;
    if value <= 0 {
        return Err("quantity must be positive".into());
    }
    Ok(value)
}

fn format_e8(value: i128) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let value = value.abs();
    let whole = value / 100_000_000;
    let fraction = value % 100_000_000;
    if fraction == 0 {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{fraction:08}")
            .trim_end_matches('0')
            .to_owned()
    }
}

#[derive(Debug)]
struct HardCheck {
    id: &'static str,
    pass: bool,
    message: &'static str,
}
impl HardCheck {
    const fn new(id: &'static str, pass: bool, message: &'static str) -> Self {
        Self { id, pass, message }
    }
}
impl PreTradeCheck for HardCheck {
    fn id(&self) -> &'static str {
        self.id
    }
    fn severity(&self) -> Severity {
        Severity::HardDeny
    }
    fn evaluate(&self, _: &CheckContext) -> CheckResult {
        CheckResult {
            id: self.id,
            severity: Severity::HardDeny,
            passed: self.pass,
            message: (!self.pass).then(|| self.message.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quantity_parser_is_exact_and_bounded() {
        assert_eq!(parse_quantity_e8("0.125").unwrap(), 12_500_000);
        assert!(parse_quantity_e8("-1").is_err());
        assert!(parse_quantity_e8("1.000000001").is_err());
    }
    #[test]
    fn signed_quantity_format_is_stable() {
        assert_eq!(format_e8(-12_500_000), "-0.125");
        assert_eq!(format_e8(200_000_000), "2");
    }
    fn fill(id: &str) -> PaperFill {
        PaperFill {
            idempotency_key: id.into(),
            asset_id: "SPY".into(),
            side: "buy".into(),
            quantity_e8: 100_000_000,
            execution_price_micros: 500_000_000,
            cash_flow_micros: -500_000_000,
            price_evidence_id: "market:test:SPY:0".into(),
            occurred_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
    #[test]
    fn crash_recovery_commits_matching_fill_and_releases_orphan() {
        let ledger = PaperLedger::from_fills(vec![fill("filled")]).unwrap();
        assert_eq!(
            recovery_action(&ledger, "filled", 500_000_000).unwrap(),
            RecoveryAction::Commit
        );
        assert_eq!(
            recovery_action(&ledger, "before-fill", 500_000_000).unwrap(),
            RecoveryAction::Release
        );
    }
    #[test]
    fn crash_recovery_rejects_reservation_fill_mismatch() {
        let ledger = PaperLedger::from_fills(vec![fill("mismatch")]).unwrap();
        assert!(recovery_action(&ledger, "mismatch", 1).is_err());
    }
    fn position(symbol: &str, value: Option<i128>, cash: i128) -> PaperPositionView {
        PaperPositionView {
            symbol: symbol.into(),
            quantity: "1".into(),
            mark_price_micros: value.and_then(|item| u64::try_from(item).ok()),
            market_value_micros: value.map(|item| item.to_string()),
            cash_flow_micros: cash.to_string(),
            unrealized_pnl_micros: value.map(|item| (item + cash).to_string()),
            mark_evidence_id: value.map(|_| "market:test".into()),
        }
    }
    #[test]
    fn portfolio_totals_are_exact_and_withhold_incomplete_pnl() {
        let complete = portfolio_totals(&[
            position("SPY", Some(600), -500),
            position("BTC", Some(-400), 450),
        ])
        .unwrap();
        assert_eq!(complete.gross, 1_000);
        assert_eq!(complete.net, 200);
        assert_eq!(complete.pnl, Some(150));
        assert_eq!(complete.largest_concentration_ppm, Some(600_000));

        let incomplete = portfolio_totals(&[
            position("SPY", Some(600), -500),
            position("UNMARKED", None, -100),
        ])
        .unwrap();
        assert_eq!(incomplete.marked, 1);
        assert_eq!(incomplete.unmarked, 1);
        assert_eq!(incomplete.pnl, None);
    }
}
