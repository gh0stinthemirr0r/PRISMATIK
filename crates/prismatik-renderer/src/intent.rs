//! Validated UI intents; models never emit markup, script, or vendor configuration.

use serde::{Deserialize, Serialize};

/// Evidence records backing a generated pane.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBinding {
    /// Evidence identifiers.
    pub evidence_ids: Vec<String>,
}

/// Native terminal pane kind.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalPane {
    /// Price/depth chart.
    Chart,
    /// Governed table.
    Table,
    /// Resolution evidence.
    Resolution,
    /// Logical relationship graph.
    ProbabilityGraph,
    /// Transcript monitor.
    Transcript,
    /// Reconciliation exceptions.
    Reconciliation,
    /// Provider health.
    ProviderHealth,
}

/// Governed table request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRequest {
    /// Saved query identifier.
    pub query_id: String,
    /// Maximum rows.
    pub row_limit: u32,
    /// Evidence binding.
    pub evidence: EvidenceBinding,
}

/// Comparison request over canonical identifiers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparisonRequest {
    /// Canonical subjects.
    pub subject_ids: Vec<String>,
    /// Requested pane.
    pub pane: TerminalPane,
    /// Evidence binding.
    pub evidence: EvidenceBinding,
}

/// Model- or user-requested typed native UI operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AiUiIntent {
    /// Render a governed table.
    ShowTable(TableRequest),
    /// Render a comparison.
    ShowComparison(ComparisonRequest),
}

impl AiUiIntent {
    /// Fail closed on orphan evidence, excessive tables, or empty comparisons.
    pub fn validate(&self, maximum_rows: u32) -> Result<(), &'static str> {
        match self {
            Self::ShowTable(request) if request.evidence.evidence_ids.is_empty() => {
                Err("evidence is required")
            },
            Self::ShowTable(request)
                if request.row_limit == 0 || request.row_limit > maximum_rows =>
            {
                Err("row limit exceeds budget")
            },
            Self::ShowComparison(request) if request.evidence.evidence_ids.is_empty() => {
                Err("evidence is required")
            },
            Self::ShowComparison(request) if request.subject_ids.is_empty() => {
                Err("comparison subjects are required")
            },
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_orphan_generated_ui() {
        let intent = AiUiIntent::ShowTable(TableRequest {
            query_id: "q".into(),
            row_limit: 10,
            evidence: EvidenceBinding {
                evidence_ids: vec![],
            },
        });
        assert!(intent.validate(100).is_err());
    }
}
