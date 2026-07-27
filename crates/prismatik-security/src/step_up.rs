//! Step-up authorization for live execution enablement (`P7-SS-01` floor).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Privilege levels for execution surfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeLevel {
    /// Read-only research.
    Research = 0,
    /// Paper trading.
    Paper = 1,
    /// Live execution (requires step-up).
    Live = 2,
}

/// Errors from step-up / privilege checks.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum StepUpError {
    /// Live enablement refused without step-up proof.
    #[error("live execution requires step-up authorization")]
    StepUpRequired,
    /// Privilege below required level.
    #[error("insufficient privilege: have {have:?}, need {need:?}")]
    Insufficient {
        /// Current.
        have: PrivilegeLevel,
        /// Required.
        need: PrivilegeLevel,
    },
    /// Emergency halt is active.
    #[error("automation halted")]
    Halted,
}

/// Auditable privilege grant record (floor — not a full OIDC flow).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivilegeGrant {
    /// Subject id (opaque).
    pub subject: String,
    /// Granted level.
    pub level: PrivilegeLevel,
    /// Step-up proof present (PIN / passkey / hardware key — opaque token).
    pub step_up_proof: Option<String>,
    /// Unix micros when granted.
    pub granted_at_micros: u64,
}

/// Session privilege gate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPrivilegeGate {
    /// Current grant.
    pub grant: PrivilegeGrant,
    /// Emergency halt latch.
    pub halted: bool,
}

impl ExecutionPrivilegeGate {
    /// Research-only default.
    pub fn research(subject: impl Into<String>) -> Self {
        Self {
            grant: PrivilegeGrant {
                subject: subject.into(),
                level: PrivilegeLevel::Research,
                step_up_proof: None,
                granted_at_micros: 0,
            },
            halted: false,
        }
    }

    /// Assert `need` is met. Live always requires a non-empty step-up proof.
    pub fn authorize(&self, need: PrivilegeLevel) -> Result<(), StepUpError> {
        if self.halted {
            return Err(StepUpError::Halted);
        }
        if self.grant.level < need {
            return Err(StepUpError::Insufficient {
                have: self.grant.level,
                need,
            });
        }
        if need >= PrivilegeLevel::Live {
            match &self.grant.step_up_proof {
                Some(p) if !p.is_empty() => Ok(()),
                _ => Err(StepUpError::StepUpRequired),
            }
        } else {
            Ok(())
        }
    }

    /// Engage emergency halt (pairs with execution `EmergencyStop`).
    pub fn halt(&mut self) {
        self.halted = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_requires_step_up_proof() {
        let mut gate = ExecutionPrivilegeGate::research("user-1");
        gate.grant.level = PrivilegeLevel::Live;
        assert_eq!(
            gate.authorize(PrivilegeLevel::Live),
            Err(StepUpError::StepUpRequired)
        );
        gate.grant.step_up_proof = Some("passkey-assertion".into());
        assert!(gate.authorize(PrivilegeLevel::Live).is_ok());
    }

    #[test]
    fn halt_blocks_all() {
        let mut gate = ExecutionPrivilegeGate::research("user-1");
        gate.halt();
        assert_eq!(
            gate.authorize(PrivilegeLevel::Research),
            Err(StepUpError::Halted)
        );
    }
}
