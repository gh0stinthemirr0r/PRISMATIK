//! RBAC policy floor (`P8-SS-02`).
//!
//! Deny-by-default role → permission evaluation. No ABAC network / IdP calls.

use serde::{Deserialize, Serialize};

/// Coarse enterprise roles (floor set).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Read-only workspace access.
    Viewer,
    /// Research write access.
    Analyst,
    /// Paper/live operator (live still gated by session step-up).
    Operator,
    /// Full administrative grants.
    Admin,
}

/// Discrete permissions evaluated by [`RbacPolicy`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Read tenant workspace data.
    ReadWorkspace,
    /// Write research notes / manifests.
    WriteResearch,
    /// Manage paper-trading sessions.
    ManagePaperTrading,
    /// Request live execution enablement (still requires session step-up).
    EnableLiveExecution,
    /// Manage tenant membership / invites.
    ManageTenant,
}

/// Static RBAC evaluator — deny by default unless a subject role grants the permission.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RbacPolicy;

impl RbacPolicy {
    /// Construct the default (static) policy.
    pub const fn new() -> Self {
        Self
    }

    /// Return `true` iff any of `subject_roles` grants `permission`.
    ///
    /// Empty role sets always deny. [`Role::Admin`] grants every permission.
    pub fn evaluate(&self, subject_roles: &[Role], permission: Permission) -> bool {
        if subject_roles.is_empty() {
            return false;
        }
        if subject_roles.contains(&Role::Admin) {
            return true;
        }
        subject_roles
            .iter()
            .any(|role| role_grants(*role, permission))
    }
}

fn role_grants(role: Role, permission: Permission) -> bool {
    match role {
        Role::Admin => true,
        Role::Viewer => matches!(permission, Permission::ReadWorkspace),
        Role::Analyst => matches!(
            permission,
            Permission::ReadWorkspace | Permission::WriteResearch
        ),
        Role::Operator => matches!(
            permission,
            Permission::ReadWorkspace
                | Permission::WriteResearch
                | Permission::ManagePaperTrading
                | Permission::EnableLiveExecution
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_PERMISSIONS: &[Permission] = &[
        Permission::ReadWorkspace,
        Permission::WriteResearch,
        Permission::ManagePaperTrading,
        Permission::EnableLiveExecution,
        Permission::ManageTenant,
    ];

    /// Expected grants per role (Admin handled separately — grants every permission).
    fn expected_grants(role: Role) -> &'static [Permission] {
        match role {
            Role::Viewer => &[Permission::ReadWorkspace],
            Role::Analyst => &[Permission::ReadWorkspace, Permission::WriteResearch],
            Role::Operator => &[
                Permission::ReadWorkspace,
                Permission::WriteResearch,
                Permission::ManagePaperTrading,
                Permission::EnableLiveExecution,
            ],
            Role::Admin => ALL_PERMISSIONS,
        }
    }

    #[test]
    fn deny_by_default() {
        let policy = RbacPolicy::new();
        assert!(!policy.evaluate(&[], Permission::ReadWorkspace));
        assert!(!policy.evaluate(&[Role::Viewer], Permission::EnableLiveExecution));
        assert!(!policy.evaluate(&[Role::Analyst], Permission::ManageTenant));
        assert!(!policy.evaluate(&[Role::Operator], Permission::ManageTenant));
    }

    #[test]
    fn deny_matrix_role_missing_permission() {
        let policy = RbacPolicy::new();
        for role in [Role::Viewer, Role::Analyst, Role::Operator] {
            let grants = expected_grants(role);
            for &perm in ALL_PERMISSIONS {
                let allowed = grants.contains(&perm);
                assert_eq!(
                    policy.evaluate(&[role], perm),
                    allowed,
                    "role={role:?} permission={perm:?} expected_allow={allowed}"
                );
            }
        }
    }

    #[test]
    fn admin_grants_all() {
        let policy = RbacPolicy::new();
        let admin = [Role::Admin];
        for perm in ALL_PERMISSIONS {
            assert!(
                policy.evaluate(&admin, *perm),
                "admin should grant {perm:?}"
            );
        }
    }

    #[test]
    fn operator_can_request_live_permission() {
        let policy = RbacPolicy::new();
        assert!(policy.evaluate(&[Role::Operator], Permission::EnableLiveExecution));
    }

    #[test]
    fn multi_role_union_still_denies_ungranted() {
        let policy = RbacPolicy::new();
        // Viewer ∪ Analyst still cannot manage tenant or enable live.
        let roles = [Role::Viewer, Role::Analyst];
        assert!(!policy.evaluate(&roles, Permission::ManageTenant));
        assert!(!policy.evaluate(&roles, Permission::EnableLiveExecution));
        assert!(!policy.evaluate(&roles, Permission::ManagePaperTrading));
        assert!(policy.evaluate(&roles, Permission::WriteResearch));
    }
}
