//! # prismatik-prismatik-ai-tools
//!
//! Layer 3 — Extension and AI
//!
//! Spec: DOCS/spec/CRATE_ARCHITECTURE.md
//! Status: PARTIAL — controlled tool gateway contracts.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub use analyst_scope::{AnalystRole, AnalystScope, EvidenceScope, FactBundle};
pub use gateway::{ToolError, ToolGateway, TrustLevel};
pub use router::{
    AiRouter, AuthMethod, DeterminismProfile, EgressPolicy, InferenceCapabilities,
    InferenceProvider, RouterError,
};
pub use tool::{ControlledTool, ToolClass, ToolInvocationRecord};

/// Tool-gateway contracts.
pub mod gateway {
    use crate::tool::{ControlledTool, ToolInvocationRecord};

    /// Trust tier for a tool result.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum TrustLevel {
        /// Trusted deterministic path.
        High,
        /// Human-review suggested.
        Medium,
        /// Untrusted path.
        Low,
    }

    /// Tool execution failure.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ToolError {
        /// Error message.
        pub message: String,
    }

    /// Tool gateway contract.
    pub trait ToolGateway: Send + Sync {
        /// Invoke a controlled tool with serialized input.
        fn invoke(
            &self,
            tool: &dyn ControlledTool,
            payload: &str,
        ) -> Result<ToolInvocationRecord, ToolError>;
    }
}

/// Controlled-tool contracts.
pub mod tool {
    /// Tool class taxonomy.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ToolClass {
        /// Read-only tool.
        ReadOnly,
        /// Risk-reducing tool.
        RiskReducing,
        /// Risk-increasing tool.
        RiskIncreasing,
    }

    /// Controlled tool contract.
    pub trait ControlledTool: Send + Sync {
        /// Tool id.
        fn id(&self) -> &str;
        /// Tool class.
        fn class(&self) -> ToolClass;
    }

    /// Tool invocation audit record.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ToolInvocationRecord {
        /// Tool id.
        pub tool_id: String,
        /// Caller id.
        pub caller: String,
        /// Invocation summary.
        pub summary: String,
    }
}

/// Inference-routing contracts.
pub mod router {
    /// Router error.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct RouterError {
        /// Error message.
        pub message: String,
    }

    /// Authentication method.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum AuthMethod {
        /// OAuth flow.
        OAuth,
        /// API key flow.
        ApiKey,
        /// Local loopback flow.
        Loopback,
    }

    /// Determinism profile for inference.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum DeterminismProfile {
        /// Best-effort deterministic.
        BestEffort,
        /// Strict deterministic.
        Strict,
    }

    /// Egress policy class.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum EgressPolicy {
        /// Loopback-only.
        Loopback,
        /// Restricted remote egress.
        Restricted,
    }

    /// Inference capabilities.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct InferenceCapabilities {
        /// Supports tool calling.
        pub tool_calling: bool,
        /// Supports multimodal input.
        pub multimodal: bool,
    }

    /// Inference provider metadata.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct InferenceProvider {
        /// Provider id.
        pub id: String,
        /// Endpoint URL string.
        pub endpoint: String,
        /// Model id.
        pub model_id: String,
        /// Auth method.
        pub auth: AuthMethod,
        /// Determinism profile.
        pub determinism: DeterminismProfile,
        /// Egress policy.
        pub egress: EgressPolicy,
        /// Capabilities.
        pub capabilities: InferenceCapabilities,
    }

    /// AI router contract.
    pub trait AiRouter: Send + Sync {
        /// Select provider by policy.
        fn select_provider(&self) -> Result<InferenceProvider, RouterError>;
    }
}

/// Analyst-scope contracts.
pub mod analyst_scope {
    /// Analyst role.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum AnalystRole {
        /// Technical analyst.
        Technical,
        /// Fundamental analyst.
        Fundamental,
        /// Sentiment analyst.
        Sentiment,
    }

    /// Shared facts bundle.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct FactBundle {
        /// Facts encoded as key/value tuples.
        pub facts: Vec<(String, String)>,
    }

    /// Private evidence scope.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct EvidenceScope {
        /// Evidence references.
        pub refs: Vec<String>,
    }

    /// Analyst scope declaration.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct AnalystScope {
        /// Analyst role.
        pub role: AnalystRole,
        /// Shared facts.
        pub common_facts: FactBundle,
        /// Role-private evidence.
        pub private_evidence: EvidenceScope,
    }
}

/// Controlled-tool implementations namespace.
pub mod tools {
    use crate::tool::ControlledTool;
    use std::collections::BTreeMap;
    use std::fmt;

    /// In-memory tool registry.
    #[derive(Default)]
    pub struct ToolRegistry {
        tools: BTreeMap<String, Box<dyn ControlledTool>>,
    }

    impl fmt::Debug for ToolRegistry {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("ToolRegistry")
                .field("tool_ids", &self.tools.keys().collect::<Vec<_>>())
                .finish()
        }
    }

    impl ToolRegistry {
        /// Create an empty registry.
        pub fn new() -> Self {
            Self::default()
        }

        /// Register a tool by its declared id.
        pub fn register(&mut self, tool: Box<dyn ControlledTool>) {
            self.tools.insert(tool.id().to_owned(), tool);
        }

        /// Return a tool by id.
        pub fn get(&self, id: &str) -> Option<&dyn ControlledTool> {
            self.tools.get(id).map(Box::as_ref)
        }
    }
}

/// RMCP server/client namespace.
pub mod mcp {
    /// RMCP request envelope.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct McpRequest {
        /// Method name.
        pub method: String,
        /// Serialized payload.
        pub payload: String,
    }

    /// RMCP response envelope.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct McpResponse {
        /// Serialized payload.
        pub payload: String,
    }

    /// RMCP transport contract.
    pub trait McpTransport: Send + Sync {
        /// Send request and receive response.
        fn call(&self, request: &McpRequest) -> Result<McpResponse, String>;
    }
}
