//! Canonical metric name registry (enterprise OD floor).
//!
//! Names follow `prismatik.<domain>.<signal>` so Prometheus / OTel exporters
//! share one vocabulary. Desktop may emit a subset; Team Cloud + Enterprise
//! scrape the full catalog.

use serde::{Deserialize, Serialize};

/// Instrument kind for registry metadata (floor; not wired to OTel yet).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    /// Cumulative counter.
    Counter,
    /// Instantaneous gauge.
    Gauge,
    /// Latency / size histogram.
    Histogram,
}

/// Stable metric name string registered for enterprise OD.
///
/// Serialize-only at this floor (`&'static str` catalog entries).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct MetricName {
    name: &'static str,
    kind: MetricKind,
}

impl MetricName {
    /// Borrow the Prometheus / OTel metric name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.name
    }

    /// Instrument kind.
    #[must_use]
    pub const fn kind(self) -> MetricKind {
        self.kind
    }
}

macro_rules! metric {
    ($name:literal, $kind:expr) => {
        MetricName {
            name: $name,
            kind: $kind,
        }
    };
}

/// Floor catalog aligned with `DOCS/spec/OBSERVABILITY.md` dashboards / SLOs.
pub const METRIC_CATALOG: &[MetricName] = &[
    metric!("prismatik.ipc.command.latency_ms", MetricKind::Histogram),
    metric!("prismatik.ipc.command.errors_total", MetricKind::Counter),
    metric!("prismatik.audit.append.latency_ms", MetricKind::Histogram),
    metric!("prismatik.audit.tamper_detected_total", MetricKind::Counter),
    metric!("prismatik.provider.requests_total", MetricKind::Counter),
    metric!("prismatik.provider.failover_total", MetricKind::Counter),
    metric!("prismatik.provider.disagreement_total", MetricKind::Counter),
    metric!("prismatik.provider.availability_ratio", MetricKind::Gauge),
    metric!("prismatik.dst.pass_ratio", MetricKind::Gauge),
    metric!("prismatik.service.availability_ratio", MetricKind::Gauge),
    metric!("prismatik.cold_start.interactive_ms", MetricKind::Histogram),
];

/// Lookup table over [`METRIC_CATALOG`].
#[derive(Clone, Debug)]
pub struct MetricNameRegistry {
    entries: &'static [MetricName],
}

impl MetricNameRegistry {
    /// Built-in floor registry.
    #[must_use]
    pub const fn floor() -> Self {
        Self {
            entries: METRIC_CATALOG,
        }
    }

    /// Number of registered names.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty (always false for the floor catalog).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Resolve a metric by exact name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<MetricName> {
        self.entries.iter().copied().find(|m| m.as_str() == name)
    }

    /// Whether `name` is registered.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Iterate registered names.
    pub fn iter(&self) -> impl Iterator<Item = MetricName> + '_ {
        self.entries.iter().copied()
    }
}
