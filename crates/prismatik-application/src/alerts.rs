//! Threshold alert rules with deterministic in-process deduplication.

use prismatik_storage::{RepositoryError, SqliteBackend};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Mutex;
use time::OffsetDateTime;

/// A simple absolute 24-hour price-change threshold.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AlertRule {
    /// Stable rule identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Trigger when `abs(change_24h_pct)` meets this threshold.
    pub minimum_change_pct: f64,
    /// Whether evaluation is active.
    pub enabled: bool,
}

/// A triggered alert.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AlertEvent {
    /// Rule that triggered.
    pub rule_id: String,
    /// Asset key being evaluated.
    pub asset_key: String,
    /// Parsed change percentage.
    pub change_pct: f64,
    /// Caller-supplied deterministic event time.
    pub occurred_at: OffsetDateTime,
    /// Stable key used to suppress repeated events.
    pub dedup_key: String,
}

/// Alert persistence and evaluation service.
#[derive(Debug)]
pub struct AlertEngine<'a> {
    backend: &'a SqliteBackend,
    seen: Mutex<BTreeSet<String>>,
}

impl<'a> AlertEngine<'a> {
    /// Create an engine over the operational database.
    pub fn new(backend: &'a SqliteBackend) -> Self {
        Self {
            backend,
            seen: Mutex::new(BTreeSet::new()),
        }
    }

    /// Persist a rule in SQLite.
    pub fn save_rule(&self, rule: &AlertRule) -> Result<(), RepositoryError> {
        let json = serde_json::to_string(rule)?;
        self.backend.save_alert_rule(
            &rule.id,
            &rule.name,
            &json,
            &format!("{}:asset", rule.id),
            rule.enabled,
        )
    }

    /// Evaluate a price-change string. A rule/asset pair triggers once per process.
    pub fn evaluate(
        &self,
        rule: &AlertRule,
        asset_key: &str,
        price_change_pct: &str,
        occurred_at: OffsetDateTime,
    ) -> Option<AlertEvent> {
        if !rule.enabled {
            return None;
        }
        let change_pct = price_change_pct.parse::<f64>().ok()?;
        if !change_pct.is_finite() || change_pct.abs() < rule.minimum_change_pct.abs() {
            return None;
        }
        let dedup_key = format!("{}:{asset_key}", rule.id);
        if !self
            .seen
            .lock()
            .expect("alert dedup mutex poisoned")
            .insert(dedup_key.clone())
        {
            return None;
        }
        Some(AlertEvent {
            rule_id: rule.id.clone(),
            asset_key: asset_key.into(),
            change_pct,
            occurred_at,
            dedup_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_alert_deduplicates_rule_and_asset() {
        let db = SqliteBackend::open_in_memory().unwrap();
        let engine = AlertEngine::new(&db);
        let rule = AlertRule {
            id: "move".into(),
            name: "Large move".into(),
            minimum_change_pct: 5.0,
            enabled: true,
        };
        engine.save_rule(&rule).unwrap();
        assert!(engine
            .evaluate(&rule, "BTC", "-6.0", OffsetDateTime::UNIX_EPOCH)
            .is_some());
        assert!(engine
            .evaluate(&rule, "BTC", "-7.0", OffsetDateTime::UNIX_EPOCH)
            .is_none());
        assert!(engine
            .evaluate(&rule, "ETH", "2.0", OffsetDateTime::UNIX_EPOCH)
            .is_none());
    }
}
