//! Deterministic news attention, novelty, diffusion, and silence features.

use std::collections::{BTreeMap, BTreeSet};

/// One normalized, provenance-linked feed observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedObservation {
    /// Stable observation id.
    pub id: String,
    /// Reviewed source id.
    pub source_id: String,
    /// Canonical URL.
    pub canonical_url: String,
    /// Headline or title.
    pub title: String,
    /// Extracted normalized entity keys.
    pub entities: Vec<String>,
    /// Observation timestamp in Unix seconds.
    pub observed_at: i64,
}

/// Point-in-time attention features suitable for deterministic models or cited LLM context.
#[derive(Clone, Debug, PartialEq)]
pub struct AttentionSignal {
    /// Entity key.
    pub entity: String,
    /// Number of independent sources in the active window.
    pub source_breadth: usize,
    /// Observations in the recent window.
    pub recent_mentions: usize,
    /// Recent mentions divided by baseline mentions (bounded when baseline is zero).
    pub velocity_ratio: f64,
    /// Fraction of recent canonical URLs not present in the baseline window.
    pub novelty_ratio: f64,
    /// Seconds since last observation; useful as an explicit silence feature.
    pub silence_seconds: i64,
    /// Observation ids supporting the signal.
    pub evidence_ids: Vec<String>,
}

/// Compute point-in-time-safe attention features without interpreting future observations.
pub fn analyze_attention(
    observations: &[FeedObservation],
    as_of: i64,
    recent_window_secs: i64,
    baseline_window_secs: i64,
) -> Vec<AttentionSignal> {
    let recent_start = as_of - recent_window_secs;
    let baseline_start = recent_start - baseline_window_secs;
    let mut by_entity: BTreeMap<String, Vec<&FeedObservation>> = BTreeMap::new();
    for observation in observations
        .iter()
        .filter(|row| row.observed_at <= as_of && row.observed_at >= baseline_start)
    {
        for entity in &observation.entities {
            by_entity
                .entry(entity.clone())
                .or_default()
                .push(observation);
        }
    }
    by_entity
        .into_iter()
        .map(|(entity, rows)| {
            let recent: Vec<_> = rows
                .iter()
                .copied()
                .filter(|row| row.observed_at >= recent_start)
                .collect();
            let baseline: Vec<_> = rows
                .iter()
                .copied()
                .filter(|row| row.observed_at < recent_start)
                .collect();
            let sources: BTreeSet<_> = recent.iter().map(|row| row.source_id.as_str()).collect();
            let baseline_urls: BTreeSet<_> = baseline
                .iter()
                .map(|row| row.canonical_url.as_str())
                .collect();
            let novel = recent
                .iter()
                .filter(|row| !baseline_urls.contains(row.canonical_url.as_str()))
                .count();
            let last = rows
                .iter()
                .map(|row| row.observed_at)
                .max()
                .unwrap_or(as_of);
            AttentionSignal {
                entity,
                source_breadth: sources.len(),
                recent_mentions: recent.len(),
                velocity_ratio: recent.len() as f64 / baseline.len().max(1) as f64,
                novelty_ratio: novel as f64 / recent.len().max(1) as f64,
                silence_seconds: as_of.saturating_sub(last),
                evidence_ids: recent.iter().map(|row| row.id.clone()).collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excludes_future_observations_and_counts_source_breadth() {
        let rows = vec![
            FeedObservation {
                id: "a".into(),
                source_id: "s1".into(),
                canonical_url: "u1".into(),
                title: "x".into(),
                entities: vec!["NVDA".into()],
                observed_at: 90,
            },
            FeedObservation {
                id: "b".into(),
                source_id: "s2".into(),
                canonical_url: "u2".into(),
                title: "x".into(),
                entities: vec!["NVDA".into()],
                observed_at: 95,
            },
            FeedObservation {
                id: "future".into(),
                source_id: "s3".into(),
                canonical_url: "u3".into(),
                title: "x".into(),
                entities: vec!["NVDA".into()],
                observed_at: 101,
            },
        ];
        let signal = analyze_attention(&rows, 100, 20, 60).pop().unwrap();
        assert_eq!(signal.source_breadth, 2);
        assert!(!signal.evidence_ids.contains(&"future".to_owned()));
    }
}
