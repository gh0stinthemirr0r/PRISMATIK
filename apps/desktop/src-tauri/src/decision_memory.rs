//! What the desk decided, and what happened next.
//!
//! Without this the system has no history of its own judgement: the council
//! could argue the same case a hundred times and never learn that it had been
//! wrong the previous ninety-nine. Forecast candidates were already scored, but
//! *decisions* — the abstains, the sizings, the reasons — were not recorded at
//! all, and an abstain that turned out to be right is exactly as informative as
//! a trade that turned out to be wrong.
//!
//! Two properties make this memory worth having rather than merely large:
//!
//! - **Outcomes are attached later, never predicted.** A record is written when
//!   the decision is made, with the price and time it was made at, and marked
//!   again once the horizon has elapsed. Nothing is back-filled.
//! - **Retrieval is by comparability, not recency.** The agents are shown past
//!   episodes of the *same instrument in the same regime*, because "the last
//!   five things that happened" is not evidence about the situation in front of
//!   them.

use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use prismatik_determinism::{Clock, SystemClock};
use serde::{Deserialize, Serialize};

/// Retained decisions. Bounded so the file cannot grow without limit.
const MAX_RECORDS: usize = 2_000;

/// Past episodes shown to an agent for one subject.
const RECALL_LIMIT: usize = 5;

static STORE: OnceLock<Mutex<Vec<DecisionRecord>>> = OnceLock::new();
static PATH: OnceLock<PathBuf> = OnceLock::new();

/// One recorded judgement and, eventually, its outcome.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRecord {
    pub id: String,
    pub symbol: String,
    /// Who decided: the trader loop, the council, or the agent.
    pub source: String,
    /// `long`, `short`, `abstain` or `advise`.
    pub action: String,
    pub rationale: String,
    /// Measured regime at decision time.
    pub regime: Option<String>,
    pub edge_ppm: i32,
    pub conviction: f64,
    /// Price when the decision was made.
    pub decided_price: f64,
    pub decided_at: String,
    /// When the outcome becomes knowable.
    pub resolves_after: String,
    /// Filled in once the horizon elapses. `None` while pending.
    #[serde(default)]
    pub resolved_price: Option<f64>,
    #[serde(default)]
    pub resolved_at: Option<String>,
    /// Move from decision to resolution, in basis points.
    #[serde(default)]
    pub realized_bps: Option<f64>,
    /// Whether the decision was directionally right. `None` for abstains,
    /// which are scored separately — an abstain has no direction to be right
    /// about, only an opportunity cost.
    #[serde(default)]
    pub correct: Option<bool>,
}

fn store() -> &'static Mutex<Vec<DecisionRecord>> {
    STORE.get_or_init(|| Mutex::new(Vec::new()))
}

/// Load persisted decisions. Called once at startup.
pub(crate) fn initialize(data_dir: &std::path::Path) -> Result<(), String> {
    let path = data_dir.join("decision-memory.json");
    let existing: Vec<DecisionRecord> = if path.exists() {
        let bytes = fs::read(&path).map_err(|error| format!("read decision memory: {error}"))?;
        // A corrupt memory file must not prevent the desk from starting; it is
        // an aid to judgement, not a ledger.
        serde_json::from_slice(&bytes).unwrap_or_default()
    } else {
        Vec::new()
    };
    let _ = PATH.set(path);
    if let Ok(mut guard) = store().lock() {
        *guard = existing;
    }
    Ok(())
}

fn persist(records: &[DecisionRecord]) {
    let Some(path) = PATH.get() else { return };
    if let Ok(bytes) = serde_json::to_vec_pretty(records) {
        let _ = fs::write(path, bytes);
    }
}

/// What a caller must supply to record a judgement.
///
/// A struct rather than a long parameter list: nine positional arguments of
/// which three are numbers invites a silent transposition at the call site,
/// and a mis-ordered `edge_ppm`/`conviction` pair would corrupt the track
/// record without failing to compile.
pub(crate) struct Decision<'a> {
    pub(crate) symbol: &'a str,
    /// Who decided: the trader loop, the council, or the agent.
    pub(crate) source: &'a str,
    pub(crate) action: &'a str,
    pub(crate) rationale: &'a str,
    pub(crate) regime: Option<&'a str>,
    pub(crate) edge_ppm: i32,
    pub(crate) conviction: f64,
    pub(crate) decided_price: f64,
    pub(crate) horizon_days: usize,
}

/// Record a decision at the moment it is made.
pub(crate) fn record(decision: Decision<'_>) {
    let Decision {
        symbol,
        source,
        action,
        rationale,
        regime,
        edge_ppm,
        conviction,
        decided_price,
        horizon_days,
    } = decision;
    let now = SystemClock::new().now();
    let record = DecisionRecord {
        id: format!("{symbol}:{source}:{}", now.unix_timestamp_nanos()),
        symbol: symbol.to_owned(),
        source: source.to_owned(),
        action: action.to_owned(),
        rationale: rationale.to_owned(),
        regime: regime.map(str::to_owned),
        edge_ppm,
        conviction,
        decided_price,
        decided_at: now.to_string(),
        resolves_after: (now + time::Duration::days(horizon_days as i64)).to_string(),
        resolved_price: None,
        resolved_at: None,
        realized_bps: None,
        correct: None,
    };
    if let Ok(mut guard) = store().lock() {
        guard.push(record);
        let len = guard.len();
        if len > MAX_RECORDS {
            guard.drain(..len - MAX_RECORDS);
        }
        persist(&guard);
    }
}

/// Attach outcomes to decisions whose horizon has elapsed.
///
/// Marks against the current quote for each symbol. A decision whose symbol is
/// no longer quotable stays pending rather than being marked against a stale
/// price — an unresolved decision is honest, a mismarked one is not.
#[tauri::command]
pub(crate) async fn resolve_decision_memory() -> Result<usize, String> {
    let snapshot = crate::terminal_feed::get_terminal_feed(crate::app_handle()?).await?;
    let now = SystemClock::new().now();
    let mut resolved = 0_usize;

    let mut guard = store().lock().map_err(|_| "decision memory unavailable")?;
    for entry in guard.iter_mut() {
        if entry.resolved_at.is_some() {
            continue;
        }
        let Ok(due) = time::OffsetDateTime::parse(
            &entry.resolves_after,
            &time::format_description::well_known::Rfc3339,
        ) else {
            continue;
        };
        if due > now {
            continue;
        }
        let Some(quote) = snapshot
            .quotes
            .iter()
            .find(|quote| quote.symbol.eq_ignore_ascii_case(&entry.symbol))
        else {
            continue;
        };
        if entry.decided_price <= 0.0 {
            continue;
        }
        let realized = (quote.price - entry.decided_price) / entry.decided_price * 10_000.0;
        entry.resolved_price = Some(quote.price);
        entry.resolved_at = Some(now.to_string());
        entry.realized_bps = Some(realized);
        entry.correct = match entry.action.as_str() {
            "long" => Some(realized > 0.0),
            "short" => Some(realized < 0.0),
            _ => None,
        };
        resolved += 1;
    }
    if resolved > 0 {
        persist(&guard);
    }
    Ok(resolved)
}

/// Past decisions on this instrument, most recent first.
pub(crate) fn recall(symbol: &str, regime: Option<&str>) -> Vec<DecisionRecord> {
    let Ok(guard) = store().lock() else {
        return Vec::new();
    };
    let mut matches: Vec<DecisionRecord> = guard
        .iter()
        .filter(|entry| entry.symbol.eq_ignore_ascii_case(symbol))
        // Same regime when one is known: a decision taken in a crisis tells you
        // little about one taken in a calm trend.
        .filter(|entry| match (regime, entry.regime.as_deref()) {
            (Some(wanted), Some(had)) => wanted == had,
            (Some(_), None) => false,
            (None, _) => true,
        })
        .cloned()
        .collect();
    matches.reverse();
    matches.truncate(RECALL_LIMIT);
    matches
}

/// Render recalled episodes for an evidence packet.
///
/// Returns `None` when there is nothing comparable, so the caller omits the
/// section rather than telling a model "no history" — which models tend to
/// read as "no history exists" rather than "none was retrieved".
pub(crate) fn recall_block(symbol: &str, regime: Option<&str>) -> Option<String> {
    let episodes = recall(symbol, regime);
    if episodes.is_empty() {
        return None;
    }
    let mut block = format!(
        "PRISMATIK's own past decisions on {symbol}{}:\n",
        regime.map_or(String::new(), |r| format!(" in the {r} regime"))
    );
    for entry in &episodes {
        let outcome = match (entry.realized_bps, entry.correct) {
            (Some(bps), Some(correct)) => format!(
                "resolved {bps:+.0} bps — {}",
                if correct { "correct" } else { "wrong" }
            ),
            (Some(bps), None) => format!("resolved {bps:+.0} bps (abstain, no direction to score)"),
            _ => "still pending".to_owned(),
        };
        block.push_str(&format!(
            "- {} [{}] {} (edge {:+.1}pp): {} → {outcome}\n",
            entry.decided_at,
            entry.source,
            entry.action,
            f64::from(entry.edge_ppm) / 10_000.0,
            entry.rationale,
        ));
    }
    block.push_str(
        "\nThese are this system's own prior judgements, not market data. Weigh them as track \
         record: repeated wrong calls in this regime are evidence against the current one.\n",
    );
    Some(block)
}

/// Decisions for the operator, newest first.
#[tauri::command]
pub(crate) fn list_decision_memory(symbol: Option<String>) -> Result<Vec<DecisionRecord>, String> {
    let guard = store().lock().map_err(|_| "decision memory unavailable")?;
    let mut rows: Vec<DecisionRecord> = match symbol {
        Some(symbol) => guard
            .iter()
            .filter(|entry| entry.symbol.eq_ignore_ascii_case(&symbol))
            .cloned()
            .collect(),
        None => guard.clone(),
    };
    rows.reverse();
    rows.truncate(200);
    Ok(rows)
}

/// Aggregate hit rate by regime — the system's own track record.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimeTrackRecord {
    pub regime: String,
    pub decided: usize,
    pub resolved: usize,
    pub correct: usize,
    pub mean_realized_bps: f64,
}

#[tauri::command]
pub(crate) fn decision_track_record() -> Result<Vec<RegimeTrackRecord>, String> {
    let guard = store().lock().map_err(|_| "decision memory unavailable")?;
    let mut by_regime: std::collections::BTreeMap<String, (usize, usize, usize, f64)> =
        std::collections::BTreeMap::new();
    for entry in guard.iter() {
        let key = entry
            .regime
            .clone()
            .unwrap_or_else(|| "unclassified".into());
        let slot = by_regime.entry(key).or_insert((0, 0, 0, 0.0));
        slot.0 += 1;
        if let Some(bps) = entry.realized_bps {
            slot.1 += 1;
            slot.3 += bps;
        }
        if entry.correct == Some(true) {
            slot.2 += 1;
        }
    }
    Ok(by_regime
        .into_iter()
        .map(
            |(regime, (decided, resolved, correct, total_bps))| RegimeTrackRecord {
                regime,
                decided,
                resolved,
                correct,
                mean_realized_bps: if resolved == 0 {
                    0.0
                } else {
                    total_bps / resolved as f64
                },
            },
        )
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The store is process-global, and Rust runs tests in parallel, so these
    /// tests must not interleave: one test's `reset` would otherwise erase
    /// another's fixtures mid-assertion. This lock serialises them.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Serialise and start from an empty store.
    fn guarded() -> std::sync::MutexGuard<'static, ()> {
        // A poisoned lock only means an earlier test panicked; the store is
        // still usable and the next test should still run.
        let guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Ok(mut store) = store().lock() {
            store.clear();
        }
        guard
    }

    #[test]
    fn recall_matches_only_the_same_instrument_and_regime() {
        let _guard = guarded();
        record(Decision {
            symbol: "AAPL",
            source: "trader",
            action: "long",
            rationale: "a",
            regime: Some("calm_trending"),
            edge_ppm: 30_000,
            conviction: 1.0,
            decided_price: 100.0,
            horizon_days: 5,
        });
        record(Decision {
            symbol: "AAPL",
            source: "trader",
            action: "long",
            rationale: "b",
            regime: Some("crisis"),
            edge_ppm: 30_000,
            conviction: 1.0,
            decided_price: 100.0,
            horizon_days: 5,
        });
        record(Decision {
            symbol: "MSFT",
            source: "trader",
            action: "long",
            rationale: "c",
            regime: Some("calm_trending"),
            edge_ppm: 30_000,
            conviction: 1.0,
            decided_price: 100.0,
            horizon_days: 5,
        });

        let same = recall("AAPL", Some("calm_trending"));
        assert_eq!(same.len(), 1);
        assert_eq!(same[0].rationale, "a");

        // Case-insensitive on symbol, strict on regime.
        assert_eq!(recall("aapl", Some("crisis")).len(), 1);
        assert!(recall("AAPL", Some("volatile_trending")).is_empty());
    }

    #[test]
    fn recall_is_capped_and_newest_first() {
        let _guard = guarded();
        for i in 0..(RECALL_LIMIT + 4) {
            record(Decision {
                symbol: "SPY",
                source: "trader",
                action: "long",
                rationale: &format!("r{i}"),
                regime: Some("calm_trending"),
                edge_ppm: 30_000,
                conviction: 1.0,
                decided_price: 100.0,
                horizon_days: 5,
            });
        }
        let recalled = recall("SPY", Some("calm_trending"));
        assert_eq!(recalled.len(), RECALL_LIMIT);
        assert_eq!(
            recalled[0].rationale,
            format!("r{}", RECALL_LIMIT + 3),
            "expected newest first"
        );
    }

    #[test]
    fn an_empty_history_produces_no_block_rather_than_a_no_history_note() {
        let _guard = guarded();
        assert!(recall_block("NOTHING", None).is_none());
    }

    #[test]
    fn a_populated_block_states_that_these_are_our_own_judgements() {
        let _guard = guarded();
        record(Decision {
            symbol: "BTC",
            source: "council",
            action: "short",
            rationale: "bearish",
            regime: Some("crisis"),
            edge_ppm: -40_000,
            conviction: 0.5,
            decided_price: 100.0,
            horizon_days: 5,
        });
        let block = recall_block("BTC", Some("crisis")).unwrap();
        assert!(block.contains("own prior judgements"));
        assert!(block.contains("bearish"));
        assert!(block.contains("still pending"));
    }

    #[test]
    fn the_track_record_counts_only_resolved_decisions_in_its_mean() {
        let _guard = guarded();
        record(Decision {
            symbol: "SPY",
            source: "trader",
            action: "long",
            rationale: "x",
            regime: Some("calm_trending"),
            edge_ppm: 30_000,
            conviction: 1.0,
            decided_price: 100.0,
            horizon_days: 5,
        });
        let rows = decision_track_record().unwrap();
        let row = rows.iter().find(|r| r.regime == "calm_trending").unwrap();
        assert_eq!(row.decided, 1);
        assert_eq!(row.resolved, 0);
        assert_eq!(row.mean_realized_bps, 0.0);
    }
}
