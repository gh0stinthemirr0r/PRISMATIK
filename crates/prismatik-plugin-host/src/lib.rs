//! # prismatik-prismatik-plugin-host
//!
//! Layer 3 — Extension and AI
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — plugin host contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use capability::{CapabilitySet, DataScope, HostFunctionId, NetworkScope, PathScope};
pub use config::hardened_engine;
pub use host::{InstalledPlugin, PluginError, PluginHost, PluginId};
pub use limits::ResourceLimits;
pub use signed::SignedPluginModule;

/// Host contracts.
pub mod host {
    use crate::capability::CapabilitySet;
    use crate::limits::ResourceLimits;
    use crate::signed::SignedPluginModule;

    /// Stable plugin identifier.
    pub type PluginId = String;

    /// Installed plugin metadata.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct InstalledPlugin {
        /// Plugin id.
        pub id: PluginId,
        /// Version string.
        pub version: String,
        /// Capability declaration.
        pub capabilities: CapabilitySet,
    }

    /// Plugin host error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PluginError {
        /// Error message.
        pub message: String,
    }

    /// Plugin host contract.
    pub trait PluginHost: Send + Sync {
        /// Install a signed plugin module.
        fn install(
            &self,
            module: SignedPluginModule,
            limits: ResourceLimits,
        ) -> Result<InstalledPlugin, PluginError>;
    }
}

/// Capability contracts.
pub mod capability {
    /// Host function identifier.
    pub type HostFunctionId = String;

    /// Network egress scope.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct NetworkScope {
        /// Allowed hosts.
        pub hosts: Vec<String>,
    }

    /// Filesystem scope.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct PathScope {
        /// Allowed root paths.
        pub roots: Vec<String>,
    }

    /// Data access scope.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct DataScope {
        /// Allowed dataset scopes.
        pub datasets: Vec<String>,
    }

    /// Plugin capability set.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CapabilitySet {
        /// Allowed host functions.
        pub host_functions: Vec<HostFunctionId>,
        /// Network scope.
        pub network: NetworkScope,
        /// Path scope.
        pub paths: PathScope,
        /// Data scope.
        pub data: DataScope,
    }
}

/// Resource-limit contracts.
pub mod limits {
    /// Resource limits for sandbox execution.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ResourceLimits {
        /// Fuel budget per invocation.
        pub fuel: u64,
        /// Memory limit in bytes.
        pub memory_bytes: u64,
    }
}

/// Signed-module contracts.
pub mod signed {
    /// Signed plugin module wrapper.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SignedPluginModule {
        /// Raw wasm bytes.
        pub wasm: Vec<u8>,
        /// Signature bytes.
        pub signature: Vec<u8>,
    }
}

/// Hardened-engine config contracts.
pub mod config {
    /// Hardened engine marker.
    #[derive(Debug, Default, Clone, Copy)]
    pub struct HardenedEngine;

    /// Build hardened plugin host engine configuration.
    pub fn hardened_engine() -> HardenedEngine {
        HardenedEngine
    }
}
