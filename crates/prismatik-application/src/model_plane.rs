//! Governed cloud/local model routing without consumer-session credential reuse.

use crate::{AutonomyBudget, AutonomyBudgetSnapshot, BudgetError, BudgetLane};
use serde::{Deserialize, Serialize};

/// Supported model service families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProvider {
    /// OpenAI API platform.
    OpenAi,
    /// Anthropic API platform.
    Anthropic,
    /// Google Gemini API / Vertex AI.
    Google,
    /// Microsoft Azure OpenAI.
    AzureOpenAi,
    /// Amazon Bedrock.
    AmazonBedrock,
    /// OpenRouter API.
    OpenRouter,
    /// User-controlled local OpenAI-compatible endpoint.
    LocalCompatible,
}

/// Documented authentication mechanism. Secret values are referenced, not embedded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ModelAuth {
    /// Provider API key held by approved secret storage.
    ApiKey {
        /// Reference into approved secret storage.
        secret_ref: String,
    },
    /// Official OAuth authorization-code connection where supported.
    OAuthAuthorizationCode {
        /// Reference to the refreshable connection record.
        connection_ref: String,
    },
    /// Cloud workload identity or service account.
    WorkloadIdentity {
        /// Reference to the cloud identity configuration.
        connection_ref: String,
    },
    /// No credential for a loopback/local endpoint.
    Local,
}

/// Intended inference purpose used for routing and audit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPurpose {
    /// Evidence summarization with citations.
    Summarization,
    /// Adversarial research critique.
    Critique,
    /// Forecast feature interpretation, never outcome truth.
    ForecastResearch,
    /// Strategy draft generation for simulation.
    StrategyDraft,
}

/// A budgeted inference plan. Creating this does not perform network I/O.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInvocationPlan {
    /// Provider.
    pub provider: ModelProvider,
    /// Provider model identifier.
    pub model: String,
    /// Declared purpose.
    pub purpose: ModelPurpose,
    /// Maximum output tokens.
    pub max_output_tokens: u32,
    /// Reserved worst-case cost in micro-units of the budget currency.
    pub reserved_cost_micros: u64,
    /// Budget state after reservation.
    pub budget: AutonomyBudgetSnapshot,
}

/// Model-plane configuration or admission error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelPlaneError {
    /// Model id, secret reference, or connection reference was empty.
    InvalidConfiguration,
    /// Authentication mechanism is not officially supported for this provider.
    UnsupportedAuthentication,
    /// Monetary budget rejected the reservation.
    Budget(BudgetError),
}

impl From<BudgetError> for ModelPlaneError {
    fn from(value: BudgetError) -> Self {
        Self::Budget(value)
    }
}

/// Validate auth/provider compatibility and reserve worst-case cost before inference.
pub fn plan_model_invocation(
    budget: &AutonomyBudget,
    provider: ModelProvider,
    auth: &ModelAuth,
    model: impl Into<String>,
    purpose: ModelPurpose,
    max_output_tokens: u32,
    estimated_cost_micros: u64,
) -> Result<ModelInvocationPlan, ModelPlaneError> {
    let model = model.into();
    let reference_valid = match auth {
        ModelAuth::ApiKey { secret_ref } => !secret_ref.trim().is_empty(),
        ModelAuth::OAuthAuthorizationCode { connection_ref }
        | ModelAuth::WorkloadIdentity { connection_ref } => !connection_ref.trim().is_empty(),
        ModelAuth::Local => true,
    };
    if model.trim().is_empty() || max_output_tokens == 0 || !reference_valid {
        return Err(ModelPlaneError::InvalidConfiguration);
    }
    let compatible = match provider {
        ModelProvider::LocalCompatible => {
            matches!(auth, ModelAuth::Local | ModelAuth::ApiKey { .. })
        },
        ModelProvider::Google => matches!(
            auth,
            ModelAuth::ApiKey { .. } | ModelAuth::WorkloadIdentity { .. }
        ),
        ModelProvider::AzureOpenAi | ModelProvider::AmazonBedrock => matches!(
            auth,
            ModelAuth::ApiKey { .. } | ModelAuth::WorkloadIdentity { .. }
        ),
        ModelProvider::OpenAi | ModelProvider::Anthropic | ModelProvider::OpenRouter => {
            matches!(auth, ModelAuth::ApiKey { .. })
        },
    };
    if !compatible {
        return Err(ModelPlaneError::UnsupportedAuthentication);
    }
    let snapshot = budget.reserve(BudgetLane::Operations, estimated_cost_micros)?;
    Ok(ModelInvocationPlan {
        provider,
        model,
        purpose,
        max_output_tokens,
        reserved_cost_micros: estimated_cost_micros,
        budget: snapshot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AutonomyBudgetPolicy;
    fn budget() -> AutonomyBudget {
        AutonomyBudget::new(AutonomyBudgetPolicy {
            currency: "USD".into(),
            operations_limit_micros: 1000,
            trading_limit_micros: 0,
            per_operation_limit_micros: 1000,
            per_trade_limit_micros: 1,
        })
        .unwrap()
    }
    #[test]
    fn openai_does_not_accept_undocumented_oauth_connection() {
        assert_eq!(
            plan_model_invocation(
                &budget(),
                ModelProvider::OpenAi,
                &ModelAuth::OAuthAuthorizationCode {
                    connection_ref: "consumer-session".into()
                },
                "gpt",
                ModelPurpose::Critique,
                10,
                1
            ),
            Err(ModelPlaneError::UnsupportedAuthentication)
        );
    }
    #[test]
    fn planning_reserves_operating_budget() {
        let plan = plan_model_invocation(
            &budget(),
            ModelProvider::Google,
            &ModelAuth::WorkloadIdentity {
                connection_ref: "vertex-project".into(),
            },
            "gemini",
            ModelPurpose::Summarization,
            100,
            25,
        )
        .unwrap();
        assert_eq!(plan.budget.operations_used_micros, 25);
    }
}
