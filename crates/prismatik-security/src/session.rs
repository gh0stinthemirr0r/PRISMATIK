//! Session policy floor for OIDC/passkey auth (`P8-SS-01`).
//!
//! Types only — no network OIDC client.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::step_up::{PrivilegeGrant, PrivilegeLevel, StepUpError};

/// Enterprise session constraints (floor — not a full IdP session store).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPolicy {
    /// Maximum session age in seconds before re-authentication is required.
    pub max_age_secs: u64,
    /// When true, live execution enablement requires a passkey step-up proof
    /// (aligned with [`crate::step_up::ExecutionPrivilegeGate`]).
    pub require_passkey_step_up_for_live: bool,
}

/// Offline session policy failures (expiry / live step-up).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    /// Session age is at or past [`SessionPolicy::max_age_secs`].
    #[error("session expired: age_secs={age_secs} max_age_secs={max_age_secs}")]
    Expired {
        /// Observed age in seconds (`now - issued_at`, saturating).
        age_secs: u64,
        /// Policy maximum.
        max_age_secs: u64,
    },
    /// Live privilege / step-up check failed.
    #[error(transparent)]
    StepUp(#[from] StepUpError),
}

impl SessionPolicy {
    /// Conservative enterprise defaults: 8h max age, live always needs passkey step-up.
    pub fn enterprise_default() -> Self {
        Self {
            max_age_secs: 8 * 60 * 60,
            require_passkey_step_up_for_live: true,
        }
    }

    /// Session age in seconds (`now_unix.saturating_sub(issued_at_unix)`).
    pub fn age_secs(&self, issued_at_unix: u64, now_unix: u64) -> u64 {
        now_unix.saturating_sub(issued_at_unix)
    }

    /// `true` when age is at or past [`Self::max_age_secs`] (boundary inclusive).
    ///
    /// Callers must re-authenticate; no clock skew / IdP refresh is performed here.
    pub fn is_expired(&self, issued_at_unix: u64, now_unix: u64) -> bool {
        self.age_secs(issued_at_unix, now_unix) >= self.max_age_secs
    }

    /// Fail closed when the session is expired.
    pub fn require_active(&self, issued_at_unix: u64, now_unix: u64) -> Result<(), SessionError> {
        let age_secs = self.age_secs(issued_at_unix, now_unix);
        if age_secs >= self.max_age_secs {
            return Err(SessionError::Expired {
                age_secs,
                max_age_secs: self.max_age_secs,
            });
        }
        Ok(())
    }

    /// Whether a non-empty step-up proof is present when the policy requires one for live.
    pub fn live_step_up_satisfied(&self, step_up_proof: Option<&str>) -> bool {
        if !self.require_passkey_step_up_for_live {
            return true;
        }
        matches!(step_up_proof, Some(p) if !p.is_empty())
    }

    /// Authorize live privilege against this session policy and a privilege grant.
    ///
    /// Mirrors [`crate::step_up::ExecutionPrivilegeGate::authorize`] for
    /// [`PrivilegeLevel::Live`] when `require_passkey_step_up_for_live` is set.
    pub fn authorize_live(&self, grant: &PrivilegeGrant) -> Result<(), StepUpError> {
        if grant.level < PrivilegeLevel::Live {
            return Err(StepUpError::Insufficient {
                have: grant.level,
                need: PrivilegeLevel::Live,
            });
        }
        if self.require_passkey_step_up_for_live {
            match &grant.step_up_proof {
                Some(p) if !p.is_empty() => Ok(()),
                _ => Err(StepUpError::StepUpRequired),
            }
        } else {
            Ok(())
        }
    }

    /// Authorize live privilege only when the session is still within max age.
    ///
    /// Expiry is checked first (fail closed), then [`Self::authorize_live`].
    pub fn authorize_live_active(
        &self,
        grant: &PrivilegeGrant,
        issued_at_unix: u64,
        now_unix: u64,
    ) -> Result<(), SessionError> {
        self.require_active(issued_at_unix, now_unix)?;
        self.authorize_live(grant)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step_up::ExecutionPrivilegeGate;

    #[test]
    fn live_requires_step_up_flag_matches_gate() {
        let policy = SessionPolicy::enterprise_default();
        assert!(policy.require_passkey_step_up_for_live);

        let mut gate = ExecutionPrivilegeGate::research("user-1");
        gate.grant.level = PrivilegeLevel::Live;

        assert_eq!(
            policy.authorize_live(&gate.grant),
            Err(StepUpError::StepUpRequired)
        );
        assert_eq!(
            gate.authorize(PrivilegeLevel::Live),
            Err(StepUpError::StepUpRequired)
        );

        gate.grant.step_up_proof = Some("passkey-assertion".into());
        assert!(policy.authorize_live(&gate.grant).is_ok());
        assert!(gate.authorize(PrivilegeLevel::Live).is_ok());
        assert!(policy.live_step_up_satisfied(gate.grant.step_up_proof.as_deref()));
    }

    #[test]
    fn session_expiry_negative_rejects_past_max_age() {
        let policy = SessionPolicy {
            max_age_secs: 3_600,
            require_passkey_step_up_for_live: true,
        };
        let issued = 1_000u64;

        // Strictly past max age.
        assert!(policy.is_expired(issued, issued + 3_601));
        assert_eq!(
            policy.require_active(issued, issued + 3_601),
            Err(SessionError::Expired {
                age_secs: 3_601,
                max_age_secs: 3_600,
            })
        );

        // Boundary: age == max_age is expired (re-auth required).
        assert!(policy.is_expired(issued, issued + 3_600));
        assert!(matches!(
            policy.require_active(issued, issued + 3_600),
            Err(SessionError::Expired { .. })
        ));

        // Clock went backwards — age saturates to 0, still active.
        assert!(!policy.is_expired(issued, issued.saturating_sub(1)));
        assert!(policy
            .require_active(issued, issued.saturating_sub(1))
            .is_ok());
    }

    #[test]
    fn session_within_max_age_is_active() {
        let policy = SessionPolicy::enterprise_default();
        let issued = 10_000u64;
        assert!(!policy.is_expired(issued, issued + policy.max_age_secs - 1));
        assert!(policy
            .require_active(issued, issued + policy.max_age_secs - 1)
            .is_ok());
    }

    #[test]
    fn authorize_live_active_fails_closed_on_expiry_before_step_up() {
        let policy = SessionPolicy {
            max_age_secs: 60,
            require_passkey_step_up_for_live: true,
        };
        let mut gate = ExecutionPrivilegeGate::research("user-1");
        gate.grant.level = PrivilegeLevel::Live;
        // Even with a valid step-up proof, expiry wins.
        gate.grant.step_up_proof = Some("passkey-assertion".into());

        assert_eq!(
            policy.authorize_live_active(&gate.grant, 100, 200),
            Err(SessionError::Expired {
                age_secs: 100,
                max_age_secs: 60,
            })
        );
    }

    #[test]
    fn authorize_live_active_rejects_missing_step_up_when_active() {
        let policy = SessionPolicy::enterprise_default();
        let mut gate = ExecutionPrivilegeGate::research("user-1");
        gate.grant.level = PrivilegeLevel::Live;

        assert_eq!(
            policy.authorize_live_active(&gate.grant, 0, 1),
            Err(SessionError::StepUp(StepUpError::StepUpRequired))
        );
    }
}
