//! Hardware soak bench plan floor (Wave 1 DoD cold-start / chart soak).
//!
//! Typed plan + stub result only. Full multi-hour hardware soaks are
//! **author-ops** and waived under turbo (`TURBO_GATE_POLICY.md`).
//!
//! # Author-ops residual
//!
//! Committing machine-specific soak timings (cold start p95, 1M-point chart)
//! requires author hardware passes. [`SoakResultStub`] documents that residual
//! so CI stays hermetic.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Plan for a soak / endurance bench run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoakBenchPlan {
    /// Human-readable bench name (e.g. `cold_start_p95`, `chart_1m_points`).
    pub name: String,
    /// Planned soak duration in seconds — must be `> 0`.
    pub duration_secs: u64,
    /// Gate / DoD criterion id this soak evidences (e.g. `W1-DoD-06`).
    pub criterion_id: String,
}

/// Errors from validating a [`SoakBenchPlan`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SoakPlanError {
    /// `duration_secs` was zero.
    #[error("soak plan duration_secs must be > 0 (name={name}, criterion_id={criterion_id})")]
    NonPositiveDuration {
        /// Plan name that failed validation.
        name: String,
        /// Criterion id from the plan.
        criterion_id: String,
    },
}

impl SoakBenchPlan {
    /// Validate that `duration_secs > 0`.
    pub fn validate(&self) -> Result<(), SoakPlanError> {
        if self.duration_secs == 0 {
            return Err(SoakPlanError::NonPositiveDuration {
                name: self.name.clone(),
                criterion_id: self.criterion_id.clone(),
            });
        }
        Ok(())
    }
}

/// Stub soak result documenting the turbo waiver.
///
/// Hardware soak remains author-ops; this type records that the run was not
/// executed in CI and is waived under turbo gate policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoakResultStub {
    /// Plan that would have been executed on author hardware.
    pub plan: SoakBenchPlan,
    /// Always `true` on the turbo floor — hardware soak waived.
    pub waived_under_turbo: bool,
    /// Residual documenting author-ops hardware soak.
    pub residual: String,
}

impl SoakResultStub {
    /// Build a stub result for a validated plan (hardware soak not run).
    pub fn waived(plan: SoakBenchPlan) -> Result<Self, SoakPlanError> {
        plan.validate()?;
        Ok(Self {
            plan,
            waived_under_turbo: true,
            residual: "hardware soak is author-ops; waived under TURBO_GATE_POLICY.md".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soak_plan_rejects_zero_duration() {
        let plan = SoakBenchPlan {
            name: "cold_start_p95".into(),
            duration_secs: 0,
            criterion_id: "W1-DoD-06".into(),
        };
        let err = plan.validate().expect_err("duration 0 must fail");
        assert!(matches!(err, SoakPlanError::NonPositiveDuration { .. }));
    }

    #[test]
    fn soak_plan_accepts_positive_duration() {
        let plan = SoakBenchPlan {
            name: "chart_1m_points".into(),
            duration_secs: 3_600,
            criterion_id: "W1-DoD-07".into(),
        };
        plan.validate().expect("positive duration must pass");
        let stub = SoakResultStub::waived(plan).expect("validated plan yields stub");
        assert!(stub.waived_under_turbo);
        assert!(stub.residual.contains("author-ops"));
    }
}
