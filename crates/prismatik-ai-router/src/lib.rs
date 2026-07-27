//! Capability-routed AI providers with recorded effects and evidence scopes.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Mutex;
use thiserror::Error;
use url::Url;

/// A provider capability selected by the router.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRoute {
    /// Stable capability name, such as `market_analysis`.
    pub capability: String,
    /// Preferred provider name.
    pub provider: String,
}

/// Structured inference input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceRequest {
    /// User or system prompt.
    pub prompt: String,
    /// Capability required by this request.
    pub capability: String,
}

/// Structured inference output. Epistemic caveats are never optional.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InferenceResponse {
    /// Main answer.
    pub content: String,
    /// Confidence uncertainty in the inclusive range 0..=1.
    pub uncertainty: f64,
    /// Statements that conflict with one another.
    #[serde(default)]
    pub contradictions: Vec<String>,
    /// Concrete checks a user can perform.
    #[serde(default)]
    pub suggested_verification: Vec<String>,
    /// Claims not grounded in supplied evidence.
    #[serde(default)]
    pub orphan_claims: Vec<String>,
}

/// AI routing failures.
#[derive(Debug, Error)]
pub enum AiError {
    /// Provider credentials or live implementation are unavailable.
    #[error("provider {0} is not configured")]
    NotConfigured(&'static str),
    /// The provider configuration is unsafe or incomplete.
    #[error("invalid provider configuration: {0}")]
    InvalidConfiguration(String),
    /// Structured output could not be decoded.
    #[error("invalid structured response: {0}")]
    InvalidResponse(#[from] serde_json::Error),
}

/// Common interface implemented by all AI providers.
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Stable provider name.
    fn name(&self) -> &'static str;
    /// Perform one structured inference.
    async fn infer(&self, request: &InferenceRequest) -> Result<InferenceResponse, AiError>;
}

/// A replayable, externally visible inference effect.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecordedEffect {
    /// Caller-provided effect identifier.
    pub effect_id: String,
    /// Provider name.
    pub provider: String,
    /// Exact request.
    pub request: InferenceRequest,
    /// Exact structured response.
    pub response: InferenceResponse,
}

/// Storage port for inference side effects.
pub trait RecordedEffectStore: Send + Sync {
    /// Record an effect.
    fn record(&self, effect: RecordedEffect);
    /// Replay an effect by identifier.
    fn get(&self, effect_id: &str) -> Option<RecordedEffect>;
}

/// Thread-safe in-memory effect store.
#[derive(Debug, Default)]
pub struct InMemoryRecordedEffectStore {
    effects: Mutex<BTreeMap<String, RecordedEffect>>,
}

impl RecordedEffectStore for InMemoryRecordedEffectStore {
    fn record(&self, effect: RecordedEffect) {
        self.effects
            .lock()
            .expect("recorded effect mutex poisoned")
            .insert(effect.effect_id.clone(), effect);
    }

    fn get(&self, effect_id: &str) -> Option<RecordedEffect> {
        self.effects
            .lock()
            .expect("recorded effect mutex poisoned")
            .get(effect_id)
            .cloned()
    }
}

/// Local LM Studio provider. Only loopback endpoints are permitted.
#[derive(Clone, Debug)]
pub struct LmStudioProvider {
    base_url: Url,
    model_digest: String,
    replay_fixture: Option<String>,
}

impl LmStudioProvider {
    /// Create a local provider. `model_digest` is mandatory.
    pub fn new(
        base_url: &str,
        model_digest: impl Into<String>,
        replay_fixture: Option<String>,
    ) -> Result<Self, AiError> {
        let base_url = Url::parse(base_url)
            .map_err(|error| AiError::InvalidConfiguration(error.to_string()))?;
        let loopback = match base_url.host() {
            Some(url::Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            None => false,
        };
        if !loopback {
            return Err(AiError::InvalidConfiguration(
                "LM Studio endpoint must be loopback".into(),
            ));
        }
        let model_digest = model_digest.into();
        if model_digest.trim().is_empty() {
            return Err(AiError::InvalidConfiguration(
                "LM Studio model digest is required".into(),
            ));
        }
        Ok(Self {
            base_url,
            model_digest,
            replay_fixture,
        })
    }

    /// Pinned model digest.
    pub fn model_digest(&self) -> &str {
        &self.model_digest
    }

    /// Configured loopback URL.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }
}

#[async_trait]
impl AiProvider for LmStudioProvider {
    fn name(&self) -> &'static str {
        "lm_studio"
    }

    async fn infer(&self, _request: &InferenceRequest) -> Result<InferenceResponse, AiError> {
        match &self.replay_fixture {
            Some(fixture) => Ok(serde_json::from_str(fixture)?),
            None => Err(AiError::NotConfigured("lm_studio_live")),
        }
    }
}

macro_rules! credential_provider {
    ($name:ident, $provider:literal, $field:ident) => {
        #[doc = concat!("Wave 1 ", $provider, " provider boundary.")]
        #[derive(Clone, Debug, Default)]
        pub struct $name {
            #[doc = "Credential used when the provider is enabled."]
            pub $field: Option<String>,
        }

        #[async_trait]
        impl AiProvider for $name {
            fn name(&self) -> &'static str {
                $provider
            }

            async fn infer(
                &self,
                request: &InferenceRequest,
            ) -> Result<InferenceResponse, AiError> {
                if self
                    .$field
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .is_none()
                {
                    return Err(AiError::NotConfigured($provider));
                }
                Ok(InferenceResponse {
                    content: format!("{} stub response for {}", $provider, request.capability),
                    uncertainty: 1.0,
                    contradictions: Vec::new(),
                    suggested_verification: vec![
                        "Verify against primary evidence before acting".into()
                    ],
                    orphan_claims: Vec::new(),
                })
            }
        }
    };
}

credential_provider!(OpenRouterProvider, "openrouter", api_key);
credential_provider!(GeminiProvider, "gemini", api_key);
credential_provider!(AnthropicProvider, "anthropic", api_key);
credential_provider!(OpenAiProvider, "openai", oauth_token);

/// Inference modality used when selecting a calibrated route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceModality {
    /// Text / chat completion.
    Text,
    /// Image or multimodal vision input.
    Vision,
    /// Schema-constrained structured output.
    Structured,
}

/// A typed route target: pinned model plus modality.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceRoute {
    /// Stable model identifier (provider-local or registry id).
    pub model_id: String,
    /// Modality the caller is requesting.
    pub modality: InferenceModality,
}

/// Router policy knobs. Uncalibrated models are denied by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouterPolicy {
    /// When true, [`route`] fails closed if the model lacks a calibration record.
    pub deny_uncalibrated: bool,
}

impl RouterPolicy {
    /// Strict policy: refuse routes without calibration evidence.
    #[must_use]
    pub fn deny_uncalibrated() -> Self {
        Self {
            deny_uncalibrated: true,
        }
    }

    /// Explicit opt-in to allow uncalibrated models (tests / research only).
    #[must_use]
    pub fn allow_uncalibrated() -> Self {
        Self {
            deny_uncalibrated: false,
        }
    }
}

impl Default for RouterPolicy {
    fn default() -> Self {
        Self::deny_uncalibrated()
    }
}

/// Calibration lookup failures and routing denials.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RouterError {
    /// Policy requires calibration and none is registered for this route.
    #[error("calibration missing for model `{model_id}` modality `{modality:?}`")]
    CalibrationMissing {
        /// Model that lacked a calibration record.
        model_id: String,
        /// Modality that was requested.
        modality: InferenceModality,
    },
}

/// In-memory set of calibrated `(model_id, modality)` pairs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CalibrationIndex {
    calibrated: BTreeMap<String, Vec<InferenceModality>>,
}

impl CalibrationIndex {
    /// Empty index — every route is uncalibrated until marked.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `model_id` is calibrated for `modality`.
    pub fn mark_calibrated(&mut self, model_id: impl Into<String>, modality: InferenceModality) {
        let entry = self.calibrated.entry(model_id.into()).or_default();
        if !entry.contains(&modality) {
            entry.push(modality);
        }
    }

    /// Whether a calibration record exists for this route.
    #[must_use]
    pub fn is_calibrated(&self, model_id: &str, modality: InferenceModality) -> bool {
        self.calibrated
            .get(model_id)
            .is_some_and(|modalities| modalities.contains(&modality))
    }
}

/// Successful routing decision after policy checks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutedInference {
    /// Accepted route.
    pub route: InferenceRoute,
    /// Policy that authorized the route.
    pub policy: RouterPolicy,
}

/// Select a route under [`RouterPolicy`].
///
/// When [`RouterPolicy::deny_uncalibrated`] is active and the
/// [`CalibrationIndex`] has no record for the route, returns
/// [`RouterError::CalibrationMissing`].
pub fn route(
    inference_route: &InferenceRoute,
    policy: RouterPolicy,
    calibration: &CalibrationIndex,
) -> Result<RoutedInference, RouterError> {
    if policy.deny_uncalibrated
        && !calibration.is_calibrated(&inference_route.model_id, inference_route.modality)
    {
        return Err(RouterError::CalibrationMissing {
            model_id: inference_route.model_id.clone(),
            modality: inference_route.modality,
        });
    }
    Ok(RoutedInference {
        route: inference_route.clone(),
        policy,
    })
}

/// Evidence visibility boundary for collaborating analysts.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AnalystScope {
    /// Shared facts visible to all analysts.
    CommonFacts,
    /// Evidence private to one analyst.
    PrivateEvidence(String),
}

/// Small scoped evidence store enforcing common/private isolation.
#[derive(Debug, Default)]
pub struct AnalystEvidenceStore {
    common: BTreeMap<String, String>,
    private: BTreeMap<String, BTreeMap<String, String>>,
}

impl AnalystEvidenceStore {
    /// Insert evidence into a scope.
    pub fn insert(
        &mut self,
        scope: AnalystScope,
        key: impl Into<String>,
        value: impl Into<String>,
    ) {
        let key = key.into();
        let value = value.into();
        match scope {
            AnalystScope::CommonFacts => {
                self.common.insert(key, value);
            },
            AnalystScope::PrivateEvidence(analyst) => {
                self.private.entry(analyst).or_default().insert(key, value);
            },
        }
    }

    /// Read evidence visible from a scope.
    pub fn get(&self, scope: &AnalystScope, key: &str) -> Option<&str> {
        match scope {
            AnalystScope::CommonFacts => self.common.get(key).map(String::as_str),
            AnalystScope::PrivateEvidence(analyst) => self
                .private
                .get(analyst)
                .and_then(|facts| facts.get(key))
                .or_else(|| self.common.get(key))
                .map(String::as_str),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
        "content":"Price moved",
        "uncertainty":0.25,
        "contradictions":[],
        "suggested_verification":["check exchange feed"],
        "orphan_claims":["cause is unknown"]
    }"#;

    #[test]
    fn lm_studio_rejects_non_loopback_hosts() {
        assert!(LmStudioProvider::new("https://example.com/v1", "sha256:model", None).is_err());
    }

    #[tokio::test]
    async fn replay_preserves_epistemic_fields() {
        let provider = LmStudioProvider::new(
            "http://127.0.0.1:1234/v1",
            "sha256:model",
            Some(FIXTURE.into()),
        )
        .unwrap();
        let response = provider
            .infer(&InferenceRequest {
                prompt: "analyze".into(),
                capability: "market_analysis".into(),
            })
            .await
            .unwrap();
        assert_eq!(response.uncertainty, 0.25);
        assert!(!response.suggested_verification.is_empty());
        assert!(!response.orphan_claims.is_empty());
    }

    #[test]
    fn common_scope_cannot_read_private_evidence() {
        let mut store = AnalystEvidenceStore::default();
        store.insert(
            AnalystScope::PrivateEvidence("alice".into()),
            "private-thesis",
            "secret",
        );
        assert_eq!(
            store.get(&AnalystScope::CommonFacts, "private-thesis"),
            None
        );
        assert_eq!(
            store.get(
                &AnalystScope::PrivateEvidence("alice".into()),
                "private-thesis"
            ),
            Some("secret")
        );
    }

    #[test]
    fn route_denies_uncalibrated_model_by_default() {
        let inference_route = InferenceRoute {
            model_id: "phi-4-mini".into(),
            modality: InferenceModality::Text,
        };
        let err = route(
            &inference_route,
            RouterPolicy::deny_uncalibrated(),
            &CalibrationIndex::new(),
        )
        .expect_err("uncalibrated model must be denied");
        assert_eq!(
            err,
            RouterError::CalibrationMissing {
                model_id: "phi-4-mini".into(),
                modality: InferenceModality::Text,
            }
        );
    }

    #[test]
    fn route_accepts_calibrated_model() {
        let mut calibration = CalibrationIndex::new();
        calibration.mark_calibrated("qwen3-14b", InferenceModality::Structured);
        let inference_route = InferenceRoute {
            model_id: "qwen3-14b".into(),
            modality: InferenceModality::Structured,
        };
        let routed = route(
            &inference_route,
            RouterPolicy::deny_uncalibrated(),
            &calibration,
        )
        .expect("calibrated model must route");
        assert_eq!(routed.route, inference_route);
        assert!(routed.policy.deny_uncalibrated);
    }

    #[test]
    fn route_modality_mismatch_is_still_uncalibrated() {
        let mut calibration = CalibrationIndex::new();
        calibration.mark_calibrated("vision-model", InferenceModality::Text);
        let inference_route = InferenceRoute {
            model_id: "vision-model".into(),
            modality: InferenceModality::Vision,
        };
        assert!(matches!(
            route(&inference_route, RouterPolicy::default(), &calibration,),
            Err(RouterError::CalibrationMissing { .. })
        ));
    }

    #[test]
    fn allow_uncalibrated_policy_bypasses_calibration_gate() {
        let inference_route = InferenceRoute {
            model_id: "experimental".into(),
            modality: InferenceModality::Text,
        };
        let routed = route(
            &inference_route,
            RouterPolicy::allow_uncalibrated(),
            &CalibrationIndex::new(),
        )
        .expect("permissive policy must allow uncalibrated");
        assert!(!routed.policy.deny_uncalibrated);
    }
}
