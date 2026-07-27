//! SLO definition registry (Team Cloud + Enterprise; desktop has no SLOs).
//!
//! Floor definitions mirror `DOCS/spec/OBSERVABILITY.md` §4. Evaluation /
//! burn-rate alerting is deferred to P8-OD-03 runbook wiring.

use serde::{Deserialize, Serialize};

/// Stable SLO identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SloId {
    /// Trusted-core service availability.
    Availability,
    /// IPC command p95 latency.
    IpcP95Latency,
    /// IPC command p99 latency.
    IpcP99Latency,
    /// Audit append p99 latency.
    AuditAppendP99Latency,
    /// Cold start to interactive.
    ColdStartInteractive,
    /// Provider failover rate (per provider).
    ProviderFailoverRate,
    /// Provider disagreement rate (N-of-M).
    ProviderDisagreementRate,
    /// DST seed corpus pass rate (release gate).
    DstPassRate,
}

/// One SLO target + window pair.
///
/// Serialize-only at this floor (`&'static str` catalog entries).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SloDefinition {
    /// Stable id.
    pub id: SloId,
    /// Human-readable label.
    pub name: &'static str,
    /// Target expression (string floor; not a parsed threshold DSL yet).
    pub target: &'static str,
    /// Evaluation window.
    pub window: &'static str,
    /// Primary metric name this SLO is measured against (when applicable).
    pub metric_name: Option<&'static str>,
}

/// Floor SLO catalog from `OBSERVABILITY.md` §4.
pub const SLO_CATALOG: &[SloDefinition] = &[
    SloDefinition {
        id: SloId::Availability,
        name: "Availability",
        target: "99.9%",
        window: "30 days",
        metric_name: Some("prismatik.service.availability_ratio"),
    },
    SloDefinition {
        id: SloId::IpcP95Latency,
        name: "IPC command p95 latency",
        target: "<100ms",
        window: "30 days",
        metric_name: Some("prismatik.ipc.command.latency_ms"),
    },
    SloDefinition {
        id: SloId::IpcP99Latency,
        name: "IPC command p99 latency",
        target: "<500ms",
        window: "30 days",
        metric_name: Some("prismatik.ipc.command.latency_ms"),
    },
    SloDefinition {
        id: SloId::AuditAppendP99Latency,
        name: "Audit append p99 latency",
        target: "<1ms",
        window: "30 days",
        metric_name: Some("prismatik.audit.append.latency_ms"),
    },
    SloDefinition {
        id: SloId::ColdStartInteractive,
        name: "Cold start to interactive",
        target: "<2.0s",
        window: "per release",
        metric_name: Some("prismatik.cold_start.interactive_ms"),
    },
    SloDefinition {
        id: SloId::ProviderFailoverRate,
        name: "Provider failover rate",
        target: "<5% of requests",
        window: "30 days",
        metric_name: Some("prismatik.provider.failover_total"),
    },
    SloDefinition {
        id: SloId::ProviderDisagreementRate,
        name: "Provider disagreement rate",
        target: "<0.1% of N-of-M reads",
        window: "30 days",
        metric_name: Some("prismatik.provider.disagreement_total"),
    },
    SloDefinition {
        id: SloId::DstPassRate,
        name: "DST pass rate",
        target: "100% of seed corpus",
        window: "each release",
        metric_name: Some("prismatik.dst.pass_ratio"),
    },
];

/// Lookup table over [`SLO_CATALOG`].
#[derive(Clone, Debug)]
pub struct SloRegistry {
    entries: &'static [SloDefinition],
}

impl SloRegistry {
    /// Built-in floor registry.
    #[must_use]
    pub const fn floor() -> Self {
        Self {
            entries: SLO_CATALOG,
        }
    }

    /// Number of registered SLOs.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Resolve by [`SloId`].
    #[must_use]
    pub fn get(&self, id: SloId) -> Option<&'static SloDefinition> {
        self.entries.iter().find(|s| s.id == id)
    }

    /// Iterate definitions.
    pub fn iter(&self) -> impl Iterator<Item = &'static SloDefinition> + '_ {
        self.entries.iter()
    }
}
