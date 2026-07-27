//! # prismatik-prismatik-oss-registry
//!
//! Layer 3 — Extension and AI
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — OSS registry contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use license::{LicenseClass, LicenseError, LicenseGate};
pub use manifest::{ExecutionAllowed, IntegrationMode, ThirdPartyComponentManifest};
pub use notice::{NoticeFile, NoticeGenerator};

/// Component-manifest contracts.
pub mod manifest {
    /// Integration mode classification.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum IntegrationMode {
        /// Direct dependency.
        A,
        /// Controlled internal fork.
        B,
        /// Isolated sidecar.
        C,
        /// Compatibility adapter.
        D,
        /// Validation oracle.
        E,
        /// Rejected.
        F,
    }

    /// Execution permission marker.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ExecutionAllowed {
        /// Allowed to execute.
        Yes,
        /// Not allowed to execute.
        No,
    }

    /// Third-party component manifest.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ThirdPartyComponentManifest {
        /// Component id.
        pub component_id: String,
        /// SPDX license identifier.
        pub license: String,
        /// Integration mode.
        pub mode: IntegrationMode,
        /// Execution permission.
        pub execution_allowed: ExecutionAllowed,
    }
}

/// License-gate contracts.
pub mod license {
    use crate::manifest::ThirdPartyComponentManifest;

    /// License class.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum LicenseClass {
        /// Allowed without restrictions.
        Allow,
        /// Allowed with obligations.
        Conditional,
        /// Prohibited.
        Deny,
    }

    /// License gate failure.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct LicenseError {
        /// Error message.
        pub message: String,
    }

    /// License gate contract.
    pub trait LicenseGate: Send + Sync {
        /// Evaluate a component manifest.
        fn evaluate(
            &self,
            manifest: &ThirdPartyComponentManifest,
        ) -> Result<LicenseClass, LicenseError>;
    }
}

/// Notice-generation contracts.
pub mod notice {
    use crate::manifest::ThirdPartyComponentManifest;

    /// Notice file artifact.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct NoticeFile {
        /// File path.
        pub path: String,
        /// File content.
        pub content: String,
    }

    /// Notice generator contract.
    pub trait NoticeGenerator: Send + Sync {
        /// Generate notices for components.
        fn generate(&self, manifests: &[ThirdPartyComponentManifest]) -> NoticeFile;
    }
}
