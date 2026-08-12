//! Logical constraints and fee-aware inconsistency detection across prediction contracts.

use crate::{ContractId, ProbabilityPpm};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Declared relationship between prediction contracts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogicalConstraint {
    /// Outcomes cannot both occur.
    MutuallyExclusive(Vec<ContractId>),
    /// Exactly one supplied outcome must occur.
    Exhaustive(Vec<ContractId>),
    /// If antecedent occurs, consequent must occur.
    Implies {
        /// Antecedent contract.
        antecedent: ContractId,
        /// Consequent contract.
        consequent: ContractId,
    },
    /// Child outcome is contained by a broader parent outcome.
    Subset {
        /// Narrow contract.
        child: ContractId,
        /// Broad contract.
        parent: ContractId,
    },
}

/// Explainable probability inconsistency.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbabilityInconsistency {
    /// Constraint that failed.
    pub constraint: LogicalConstraint,
    /// Magnitude beyond permitted fees/tolerance.
    pub excess_ppm: u32,
    /// Contracts needed to evaluate the finding.
    pub contracts: Vec<ContractId>,
}

/// Detect deterministic inconsistencies after a caller-supplied fee/tolerance allowance.
pub fn detect_probability_inconsistencies(
    probabilities: &BTreeMap<ContractId, ProbabilityPpm>,
    constraints: &[LogicalConstraint],
    allowance_ppm: u32,
) -> Vec<ProbabilityInconsistency> {
    let mut findings = Vec::new();
    for constraint in constraints {
        let (contracts, excess) = match constraint {
            LogicalConstraint::MutuallyExclusive(ids) => {
                let sum = sum(ids, probabilities);
                (
                    ids.clone(),
                    sum.saturating_sub(1_000_000_u64 + u64::from(allowance_ppm)),
                )
            },
            LogicalConstraint::Exhaustive(ids) => {
                let sum = sum(ids, probabilities);
                let delta = sum.abs_diff(1_000_000);
                (ids.clone(), delta.saturating_sub(u64::from(allowance_ppm)))
            },
            LogicalConstraint::Implies {
                antecedent,
                consequent,
            }
            | LogicalConstraint::Subset {
                child: antecedent,
                parent: consequent,
            } => {
                let Some(left) = probabilities.get(antecedent) else {
                    continue;
                };
                let Some(right) = probabilities.get(consequent) else {
                    continue;
                };
                (
                    vec![antecedent.clone(), consequent.clone()],
                    u64::from(left.get())
                        .saturating_sub(u64::from(right.get()) + u64::from(allowance_ppm)),
                )
            },
        };
        if excess > 0 {
            findings.push(ProbabilityInconsistency {
                constraint: constraint.clone(),
                excess_ppm: u32::try_from(excess).unwrap_or(u32::MAX),
                contracts,
            });
        }
    }
    findings
}

fn sum(ids: &[ContractId], values: &BTreeMap<ContractId, ProbabilityPpm>) -> u64 {
    ids.iter()
        .filter_map(|id| values.get(id))
        .map(|value| u64::from(value.get()))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_dutch_book_and_implication_violation() {
        let a = ContractId::new("a").unwrap();
        let b = ContractId::new("b").unwrap();
        let probabilities = BTreeMap::from([
            (a.clone(), ProbabilityPpm::new(700_000).unwrap()),
            (b.clone(), ProbabilityPpm::new(500_000).unwrap()),
        ]);
        let findings = detect_probability_inconsistencies(
            &probabilities,
            &[
                LogicalConstraint::Exhaustive(vec![a.clone(), b.clone()]),
                LogicalConstraint::Implies {
                    antecedent: a,
                    consequent: b,
                },
            ],
            10_000,
        );
        assert_eq!(findings.len(), 2);
    }
}
