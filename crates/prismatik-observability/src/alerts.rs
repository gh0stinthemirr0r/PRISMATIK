//! Enterprise OD **AlertRule** stubs (`P8-OD-03`).
//!
//! These are Alertmanager / ops alert definitions (not the desktop price-threshold
//! rules in `prismatik-application`). Evaluation engines and notification sinks
//! are deferred; this floor freezes ids, severity, and condition text from
//! `DOCS/spec/OBSERVABILITY.md` §5.

use serde::{Deserialize, Serialize};

/// Stable alert rule identifier (Alertmanager rule name).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AlertRuleId(pub String);

impl AlertRuleId {
    /// Wrap an owned id string.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Alertmanager severity tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    /// Page / on-call.
    Critical,
    /// Ticket / backlog.
    Warning,
    /// Log only.
    Info,
}

/// How the rule is evaluated (floor enum for later wiring).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationMode {
    /// Continuous / streaming PromQL-style evaluation.
    Streaming,
    /// Periodic schedule (burn-rate windows, digests).
    Scheduled,
}

/// Enterprise OD alert rule stub.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertRule {
    /// Stable rule id (matches Alertmanager `alert:` name).
    pub id: AlertRuleId,
    /// Human-readable title.
    pub name: String,
    /// Severity tier.
    pub severity: AlertSeverity,
    /// Evaluation mode.
    pub evaluation_mode: EvaluationMode,
    /// Condition prose / PromQL placeholder (not executed at this floor).
    pub condition: String,
    /// Whether the rule is active in the catalog.
    pub enabled: bool,
}

impl AlertRule {
    /// Construct a generic stub rule for tests / future custom catalogs.
    #[must_use]
    pub fn stub(
        id: impl Into<String>,
        severity: AlertSeverity,
        evaluation_mode: EvaluationMode,
        condition: impl Into<String>,
    ) -> Self {
        let id = id.into();
        Self {
            name: id.clone(),
            id: AlertRuleId::new(id),
            severity,
            evaluation_mode,
            condition: condition.into(),
            enabled: true,
        }
    }
}

/// Built-in enterprise alert catalog from `OBSERVABILITY.md` §5.
#[must_use]
pub fn enterprise_alert_catalog() -> Vec<AlertRule> {
    vec![
        // Critical
        AlertRule::stub(
            "AuditTamperDetected",
            AlertSeverity::Critical,
            EvaluationMode::Streaming,
            "audit.verify_all reports tamper_detected = true",
        ),
        AlertRule::stub(
            "AuditAppendLatencyHigh",
            AlertSeverity::Critical,
            EvaluationMode::Streaming,
            "p99 > 5ms for 5 min (audit on sensitive path)",
        ),
        AlertRule::stub(
            "LiveExecutionHalted",
            AlertSeverity::Critical,
            EvaluationMode::Streaming,
            "Session halted in live mode",
        ),
        AlertRule::stub(
            "RiskGateBypassAttempted",
            AlertSeverity::Critical,
            EvaluationMode::Streaming,
            "OrderIntent constructed outside risk kernel",
        ),
        AlertRule::stub(
            "ManifestSignatureInvalid",
            AlertSeverity::Critical,
            EvaluationMode::Streaming,
            "Any manifest fails signature verification on load",
        ),
        AlertRule::stub(
            "ArtifactHashMismatch",
            AlertSeverity::Critical,
            EvaluationMode::Streaming,
            "Loaded artifact hash does not match pinned hash",
        ),
        AlertRule::stub(
            "Unknown",
            AlertSeverity::Critical,
            EvaluationMode::Streaming,
            "Order submission returned Unknown and reconciliation failed",
        ),
        // Warning
        AlertRule::stub(
            "ProviderDegraded",
            AlertSeverity::Warning,
            EvaluationMode::Streaming,
            "Provider health Degraded for >5 min",
        ),
        AlertRule::stub(
            "ProviderChainFailoverRate",
            AlertSeverity::Warning,
            EvaluationMode::Scheduled,
            ">10% of requests failing over for >1 hour",
        ),
        AlertRule::stub(
            "CalibrationDriftDetected",
            AlertSeverity::Warning,
            EvaluationMode::Streaming,
            "drift_detected event with action widen_intervals",
        ),
        AlertRule::stub(
            "PretrainingContaminationDenied",
            AlertSeverity::Warning,
            EvaluationMode::Streaming,
            "Model load denied for contamination",
        ),
        AlertRule::stub(
            "CapabilityDiffBlocked",
            AlertSeverity::Warning,
            EvaluationMode::Streaming,
            "Plugin capability diff gate fired",
        ),
        AlertRule::stub(
            "CoverageBelowNominal",
            AlertSeverity::Warning,
            EvaluationMode::Scheduled,
            "Realized coverage < nominal - sampling error for >24h",
        ),
        // Info
        AlertRule::stub(
            "RateBudgetExhausted",
            AlertSeverity::Info,
            EvaluationMode::Streaming,
            "Provider budget exhausted (expected)",
        ),
        AlertRule::stub(
            "StaleDataDetected",
            AlertSeverity::Info,
            EvaluationMode::Streaming,
            "Stale data detected (UI handles; logged for diagnostics)",
        ),
    ]
}
