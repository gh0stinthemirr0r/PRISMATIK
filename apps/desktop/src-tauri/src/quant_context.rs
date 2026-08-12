//! The deterministic facts every agent should be reasoning *from*.
//!
//! The agent council previously asked a language model to infer the volatility
//! regime from a price snapshot, while the analytics layer sitting beside it
//! computed that regime deterministically — with hysteresis, a variance-ratio
//! and drift test, and a Brier-scorable edge against climatology. The council
//! was doing badly, expensively and unverifiably what the code already did well.
//!
//! This module renders those measured facts into a text block that goes into
//! the evidence packet. It changes what the agents are *for*: they no longer
//! guess the regime, they argue about what a known regime implies. A model is
//! good at the second job and bad at the first.
//!
//! Everything here is labelled as computed rather than observed, and carries
//! its own sample sizes, so a model cannot mistake a thin estimate for a fact.

use crate::{
    signal::{self, DECISION_HORIZON_DAYS},
    tracking::{self, TrackedInstrument},
};

/// Locate a tracked instrument by symbol, case-insensitively.
fn find_tracked(symbol: &str) -> Option<TrackedInstrument> {
    let app = crate::app_handle().ok()?;
    tracking::read_tracked(&app)
        .ok()?
        .into_iter()
        .find(|row| row.symbol.eq_ignore_ascii_case(symbol))
}

/// Deterministic quantitative context for one subject.
///
/// Returns `None` when the subject is not tracked or has no classifiable
/// history — the caller then proceeds without the block rather than
/// substituting a placeholder, so a missing regime is visibly missing.
pub(crate) async fn for_subject(symbol: &str) -> Option<String> {
    let tracked = find_tracked(symbol)?;
    let signal = signal::evaluate(&tracked.symbol, tracked.kind, &tracked.provider_id).await;
    let regime = signal.regime.clone()?;

    let mut block = String::new();
    block.push_str(
        "PRISMATIK computed analytics (deterministic, reproducible from price history — \
         these are computed values, not market observations):\n",
    );
    block.push_str(&format!(
        "- Volatility regime: {regime}, held {} bars.\n",
        signal.regime_run_length
    ));
    block.push_str(&format!(
        "- {DECISION_HORIZON_DAYS}-day conditional probability: {:.1}%; unconditional base rate \
         for the same claim: {:.1}%; edge {:+.1} percentage points over {} historical episodes.\n",
        f64::from(signal.probability_ppm) / 10_000.0,
        f64::from(signal.climatology_ppm) / 10_000.0,
        f64::from(signal.edge_ppm) / 10_000.0,
        signal.sample_size,
    ));
    match signal.skill_ppm {
        Some(ppm) => block.push_str(&format!(
            "- Measured forecaster skill on this instrument: {:.1}% against climatology over {} \
             resolved forecasts. Positive means it has beaten the base rate; zero means it has not.\n",
            ppm as f64 / 10_000.0,
            signal.skill_sample_count,
        )),
        None => block.push_str(
            "- Measured forecaster skill: not yet scored on this instrument. Treat the \
             probability above as unproven.\n",
        ),
    }
    block.push_str(&format!(
        "- PRISMATIK's own gate: {} — {}\n",
        if signal.actionable {
            "actionable"
        } else {
            "abstaining"
        },
        signal.rationale,
    ));
    block.push_str(
        "\nThe edge, not the probability, is the informative quantity: a 58% call against a 58% \
         base rate carries no information. Do not treat a large probability with a small edge as \
         conviction.\n",
    );

    // The desk's own track record on this instrument in this regime, so the
    // agents can weigh their prior judgements rather than starting fresh each
    // time they are asked the same question.
    if let Some(history) = crate::decision_memory::recall_block(&tracked.symbol, Some(&regime)) {
        block.push_str(&format!("\n{history}"));
    }
    Some(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_untracked_subject_yields_no_block() {
        // Without an app handle (as in unit tests) the lookup must fail closed
        // rather than panic, so callers simply proceed without the context.
        assert!(find_tracked("DEFINITELY-NOT-TRACKED").is_none());
    }
}
