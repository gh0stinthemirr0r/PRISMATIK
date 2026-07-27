//! Plugin marketplace submission floor (`P9-SS-01` / `P9-SS-02`).

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{commercial_license_gate, ComponentRecord, LicenseClass, RegistryError};

/// Marketplace submission lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketplaceStatus {
    /// Awaiting review.
    Submitted,
    /// Under capability review.
    InReview,
    /// Approved for install.
    Approved,
    /// Revoked (signature or policy).
    Revoked,
}

/// Publisher identity floor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublisherIdentity {
    /// Stable publisher id.
    pub publisher_id: String,
    /// Display name.
    pub display_name: String,
    /// Ed25519 public key hex (opaque at floor).
    pub signing_pubkey_hex: String,
}

/// Marketplace listing (pre-install capability disclosure required).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketplaceListing {
    /// Plugin artifact id.
    pub plugin_id: String,
    /// Semver.
    pub version: String,
    /// Publisher.
    pub publisher: PublisherIdentity,
    /// Declared capabilities (string names at floor).
    pub capabilities: Vec<String>,
    /// License class.
    pub license_class: LicenseClass,
    /// Status.
    pub status: MarketplaceStatus,
}

/// Marketplace gate errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MarketplaceError {
    /// License refused.
    #[error(transparent)]
    License(#[from] RegistryError),
    /// Not approved for install.
    #[error("plugin not approved for install: {0:?}")]
    NotApproved(MarketplaceStatus),
    /// Capability disclosure missing.
    #[error("capability disclosure required before install")]
    MissingCapabilityDisclosure,
    /// Revoked.
    #[error("plugin revoked")]
    Revoked,
}

impl MarketplaceListing {
    /// Pre-install gate: approved + commercial license + non-empty capability list.
    pub fn assert_installable(&self) -> Result<(), MarketplaceError> {
        if self.status == MarketplaceStatus::Revoked {
            return Err(MarketplaceError::Revoked);
        }
        if self.status != MarketplaceStatus::Approved {
            return Err(MarketplaceError::NotApproved(self.status));
        }
        if self.capabilities.is_empty() {
            return Err(MarketplaceError::MissingCapabilityDisclosure);
        }
        let record = ComponentRecord {
            name: self.plugin_id.clone(),
            license_spdx: "Apache-2.0".into(),
            license_class: self.license_class,
            version: self.version.clone(),
        };
        commercial_license_gate(&record)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listing(status: MarketplaceStatus) -> MarketplaceListing {
        MarketplaceListing {
            plugin_id: "demo".into(),
            version: "1.0.0".into(),
            publisher: PublisherIdentity {
                publisher_id: "pub1".into(),
                display_name: "Demo".into(),
                signing_pubkey_hex: "00".into(),
            },
            capabilities: vec!["clock".into()],
            license_class: LicenseClass::PermissiveCommercial,
            status,
        }
    }

    #[test]
    fn approved_install_ok() {
        assert!(listing(MarketplaceStatus::Approved)
            .assert_installable()
            .is_ok());
    }

    #[test]
    fn submitted_not_installable() {
        assert!(matches!(
            listing(MarketplaceStatus::Submitted).assert_installable(),
            Err(MarketplaceError::NotApproved(_))
        ));
    }

    #[test]
    fn noncommercial_denied() {
        let mut l = listing(MarketplaceStatus::Approved);
        l.license_class = LicenseClass::NonCommercial;
        assert!(l.assert_installable().is_err());
    }
}
