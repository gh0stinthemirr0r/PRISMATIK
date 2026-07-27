//! Tenant isolation floor types (`P8-SS-02` / row-level key scoping).
//!
//! Types only — no Postgres RLS wiring.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Opaque tenant identifier for row-level isolation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(String);

/// Errors constructing tenant-scoped identities.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TenantError {
    /// Empty or whitespace-only tenant id.
    #[error("tenant id must be non-empty")]
    EmptyId,
    /// Empty or whitespace-only key purpose.
    #[error("tenant key purpose must be non-empty")]
    EmptyPurpose,
}

impl TenantId {
    /// Construct a non-empty tenant id.
    pub fn new(id: impl Into<String>) -> Result<Self, TenantError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(TenantError::EmptyId);
        }
        Ok(Self(id))
    }

    /// Borrow the underlying id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Key material scope bound to a single tenant and purpose label.
///
/// Distinct tenants must never share the same scoped key identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantScopedKey {
    /// Owning tenant.
    pub tenant: TenantId,
    /// Purpose label (e.g. `"row_seal"`, `"cache"`).
    pub purpose: String,
}

impl TenantScopedKey {
    /// Construct a tenant-scoped key identity.
    pub fn new(tenant: TenantId, purpose: impl Into<String>) -> Result<Self, TenantError> {
        let purpose = purpose.into();
        if purpose.trim().is_empty() {
            return Err(TenantError::EmptyPurpose);
        }
        Ok(Self { tenant, purpose })
    }

    /// True when both keys belong to the same tenant (isolation check helper).
    pub fn same_tenant(&self, other: &Self) -> bool {
        self.tenant == other.tenant
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_tenants_do_not_share_scoped_keys() {
        let a = TenantScopedKey::new(TenantId::new("tenant-a").unwrap(), "row_seal").unwrap();
        let b = TenantScopedKey::new(TenantId::new("tenant-b").unwrap(), "row_seal").unwrap();
        assert!(!a.same_tenant(&b));
        assert_ne!(a, b);
    }

    #[test]
    fn rejects_empty_tenant_or_purpose() {
        assert_eq!(TenantId::new("  "), Err(TenantError::EmptyId));
        let t = TenantId::new("t1").unwrap();
        assert_eq!(TenantScopedKey::new(t, ""), Err(TenantError::EmptyPurpose));
    }
}
