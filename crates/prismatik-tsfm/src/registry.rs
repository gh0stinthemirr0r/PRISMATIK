//! Model registry with license-class hard deny (`P5-QM-05` floor).

use crate::families::{FamilyOnboardingStatus, OnboardedTsfmFamily};
use crate::pretraining::{
    evaluate_contamination, ContaminationVerdict, PretrainingRecord, TsfmSecurityEvent,
};
use crate::runtime::{StubTsfmRuntime, TsfmRuntime};
use crate::tokenizer::{TokenizerBinding, TokenizerError};
use prismatik_determinism::{ArtifactRef, ContentHash, SemanticVersion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;
use time::OffsetDateTime;

/// Opaque stable model identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelId(pub String);

impl ModelId {
    /// Construct from any string-like value.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Broad model family taxonomy (v1.0 §17.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelFamily {
    /// Classical / naive baselines.
    Baseline,
    /// Tree ensembles (GBM, RF, …).
    Tree,
    /// Task-specific neural networks.
    Nn,
    /// Time-series foundation models.
    Tsfm,
    /// Ensembles of other registered models.
    Ensemble,
}

/// License classes for model artifacts (distinct from code licenses).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactLicenseClass {
    /// Apache-2.0, MIT, BSD — distributable in a commercial product.
    PermissiveCommercial,
    /// Requires attribution but otherwise unrestricted.
    AttributionRequired,
    /// Non-commercial (CC BY-NC and relatives). Research registry only.
    NonCommercial,
    /// Custom / vendor terms requiring legal review.
    RequiresLegalReview,
}

/// Runtime backend kind (no ONNX link at this floor).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    /// ONNX Runtime (future; stub only today).
    Onnx,
    /// llama.cpp.
    LlamaCpp,
    /// Remote cloud API.
    CloudApi,
    /// Native Rust path.
    Native,
    /// Explicit floor stub — no inference backend linked.
    Stub,
}

/// Series modality accepted by a TSFM / tokenizer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeriesModality {
    /// OHLCV K-line bars.
    Ohlcv,
    /// Univariate close / mid series.
    Univariate,
    /// Multivariate with covariates.
    Multivariate,
    /// Options flow features.
    OptionsFlow,
    /// Implied-volatility surface slice.
    IvSurface,
}

/// Deployment profile controlling commercial license gates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryProfile {
    /// Revenue / product profile — NonCommercial is hard-denied.
    Commercial,
    /// Research benchmarking — NonCommercial may register but is flagged.
    Research,
}

impl RegistryProfile {
    /// Returns `true` for the commercial product profile.
    #[must_use]
    pub const fn is_commercial(self) -> bool {
        matches!(self, Self::Commercial)
    }
}

/// Governance record for a registered model artifact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelRegistration {
    /// Stable model id.
    pub model_id: ModelId,
    /// Semantic version.
    pub version: SemanticVersion,
    /// Broad family taxonomy.
    pub family: ModelFamily,
    /// Optional upstream TSFM family catalog entry (see [`OnboardedTsfmFamily`]).
    #[serde(default)]
    pub onboarded_family: Option<OnboardedTsfmFamily>,
    /// Weights / artifact reference.
    pub artifact: ArtifactRef,
    /// SPDX license expression for the weights.
    pub license_spdx: String,
    /// Classified license class — gate key.
    pub license_class: ArtifactLicenseClass,
    /// Declared runtime backend.
    pub runtime: RuntimeKind,
    /// Pretraining disclosure (required for TSFM).
    #[serde(default)]
    pub pretraining: Option<PretrainingRecord>,
    /// Expected tokenizer codebook content hash.
    pub expected_codebook_hash: ContentHash,
}

/// Registry / load errors.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RegistryError {
    /// CC BY-NC / NonCommercial artifact denied in commercial profile.
    #[error(
        "non-commercial artifact denied in commercial profile: model `{model_id}` license `{license}`"
    )]
    NonCommercialArtifact {
        /// Denied model.
        model_id: ModelId,
        /// SPDX string that triggered the deny.
        license: String,
    },
    /// Onboarded family marked [`FamilyOnboardingStatus::RejectedCommercial`].
    #[error("family `{family}` is RejectedCommercial — cannot register in commercial profile")]
    RejectedCommercialFamily {
        /// Rejected family name.
        family: String,
    },
    /// Tokenizer / model version or codebook mismatch.
    #[error(transparent)]
    TokenizerMismatch(#[from] TokenizerError),
    /// Pretraining cutoff contaminates the backtest window.
    #[error("pretraining contamination: cutoff {cutoff} >= backtest start {backtest_start}")]
    PretrainingContamination {
        /// Offending cutoff (RFC3339 display).
        cutoff: String,
        /// Backtest start (RFC3339 display).
        backtest_start: String,
    },
    /// Model id already registered.
    #[error("model already registered: {0}")]
    AlreadyRegistered(ModelId),
    /// Model id not found.
    #[error("model not found: {0}")]
    NotFound(ModelId),
    /// Binding missing for a registered model.
    #[error("tokenizer binding missing for model: {0}")]
    BindingMissing(ModelId),
}

/// Plural TSFM / model registry.
#[derive(Clone, Debug)]
pub struct ModelRegistry {
    profile: RegistryProfile,
    entries: BTreeMap<ModelId, ModelRegistration>,
    bindings: BTreeMap<ModelId, TokenizerBinding>,
    security_events: Vec<TsfmSecurityEvent>,
}

impl ModelRegistry {
    /// Create an empty registry for the given profile.
    #[must_use]
    pub fn new(profile: RegistryProfile) -> Self {
        Self {
            profile,
            entries: BTreeMap::new(),
            bindings: BTreeMap::new(),
            security_events: Vec::new(),
        }
    }

    /// Active registry profile.
    #[must_use]
    pub const fn profile(&self) -> RegistryProfile {
        self.profile
    }

    /// Security events emitted so far (lookahead denials, …).
    #[must_use]
    pub fn security_events(&self) -> &[TsfmSecurityEvent] {
        &self.security_events
    }

    /// Drain and return recorded security events.
    pub fn take_security_events(&mut self) -> Vec<TsfmSecurityEvent> {
        std::mem::take(&mut self.security_events)
    }

    /// Register a model. Hard-denies NonCommercial in commercial profile.
    pub fn register(&mut self, registration: ModelRegistration) -> Result<(), RegistryError> {
        self.enforce_license_gate(&registration)?;
        if let Some(family) = registration.onboarded_family {
            if self.profile.is_commercial()
                && family.status() == FamilyOnboardingStatus::RejectedCommercial
            {
                return Err(RegistryError::RejectedCommercialFamily {
                    family: family.as_str().into(),
                });
            }
        }
        if self.entries.contains_key(&registration.model_id) {
            return Err(RegistryError::AlreadyRegistered(registration.model_id));
        }
        self.entries
            .insert(registration.model_id.clone(), registration);
        Ok(())
    }

    /// Attach a tokenizer binding for a previously registered model.
    pub fn bind_tokenizer(&mut self, binding: TokenizerBinding) -> Result<(), RegistryError> {
        let entry = self
            .entries
            .get(&binding.model_id)
            .ok_or_else(|| RegistryError::NotFound(binding.model_id.clone()))?;
        binding.validate(entry.version, entry.expected_codebook_hash)?;
        self.bindings.insert(binding.model_id.clone(), binding);
        Ok(())
    }

    /// Look up a registration.
    pub fn lookup(&self, model_id: &ModelId) -> Result<&ModelRegistration, RegistryError> {
        self.entries
            .get(model_id)
            .ok_or_else(|| RegistryError::NotFound(model_id.clone()))
    }

    /// Binding for a model, if present.
    pub fn binding_for(&self, model_id: &ModelId) -> Result<&TokenizerBinding, RegistryError> {
        self.bindings
            .get(model_id)
            .ok_or_else(|| RegistryError::BindingMissing(model_id.clone()))
    }

    /// Load a TSFM: license gate, tokenizer pairing, optional contamination gate.
    ///
    /// Returns a [`StubTsfmRuntime`] — no ONNX / inference backend is linked.
    pub fn load_tsfm(
        &mut self,
        model_id: &ModelId,
        backtest_window_start: Option<OffsetDateTime>,
    ) -> Result<Arc<dyn TsfmRuntime>, RegistryError> {
        let entry = self.lookup(model_id)?.clone();
        self.enforce_license_gate(&entry)?;

        let binding = self.binding_for(model_id)?.clone();
        binding.validate(entry.version, entry.expected_codebook_hash)?;

        if let (Some(window_start), Some(pretraining)) =
            (backtest_window_start, entry.pretraining.as_ref())
        {
            let verdict = evaluate_contamination(
                model_id.clone(),
                pretraining.pretraining_cutoff,
                window_start,
            );
            if let ContaminationVerdict::Contaminated {
                pretraining_cutoff,
                backtest_window_start,
                ..
            } = verdict
            {
                let event = TsfmSecurityEvent::LookaheadDenied {
                    model_id: model_id.clone(),
                    pretraining_cutoff,
                    backtest_start: backtest_window_start,
                };
                self.security_events.push(event);
                return Err(RegistryError::PretrainingContamination {
                    cutoff: pretraining_cutoff.to_string(),
                    backtest_start: backtest_window_start.to_string(),
                });
            }
        }

        Ok(Arc::new(StubTsfmRuntime::new(model_id.clone())))
    }

    fn enforce_license_gate(&self, entry: &ModelRegistration) -> Result<(), RegistryError> {
        if self.profile.is_commercial()
            && entry.license_class == ArtifactLicenseClass::NonCommercial
        {
            return Err(RegistryError::NonCommercialArtifact {
                model_id: entry.model_id.clone(),
                license: entry.license_spdx.clone(),
            });
        }
        Ok(())
    }
}

/// Synthetic restricted-license NonCommercial registration for negative tests.
#[must_use]
pub fn synthetic_restricted_license_registration() -> ModelRegistration {
    use prismatik_determinism::{ArtifactId, ArtifactKind};
    let version = SemanticVersion::new(2, 0, 0);
    ModelRegistration {
        model_id: ModelId::new("restricted-license-synthetic"),
        version,
        family: ModelFamily::Tsfm,
        onboarded_family: Some(OnboardedTsfmFamily::RestrictedLicense),
        artifact: ArtifactRef {
            artifact_id: ArtifactId::new("restricted-license-weights"),
            kind: ArtifactKind::ModelWeights,
            version,
            content_hash: ContentHash::from_bytes(b"restricted-license-weights-v1"),
            signature: Default::default(),
        },
        license_spdx: "CC-BY-NC-4.0".into(),
        license_class: ArtifactLicenseClass::NonCommercial,
        runtime: RuntimeKind::Stub,
        pretraining: Some(PretrainingRecord::new(
            "synthetic multivariate corpus",
            time::OffsetDateTime::UNIX_EPOCH,
        )),
        expected_codebook_hash: ContentHash::from_bytes(b"restricted-license-codebook"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismatik_determinism::{ArtifactId, ArtifactKind};
    use time::macros::datetime;

    fn permissive_financial_bar() -> ModelRegistration {
        let version = SemanticVersion::new(1, 0, 0);
        let hash = ContentHash::from_bytes(b"financial-bar-codebook-v1");
        ModelRegistration {
            model_id: ModelId::new("financial-bar-base"),
            version,
            family: ModelFamily::Tsfm,
            onboarded_family: Some(OnboardedTsfmFamily::FinancialBar),
            artifact: ArtifactRef {
                artifact_id: ArtifactId::new("financial-bar-weights"),
                kind: ArtifactKind::ModelWeights,
                version,
                content_hash: ContentHash::from_bytes(b"financial-bar-weights-v1"),
                signature: Default::default(),
            },
            license_spdx: "MIT".into(),
            license_class: ArtifactLicenseClass::PermissiveCommercial,
            runtime: RuntimeKind::Stub,
            pretraining: Some(PretrainingRecord::new(
                "K-line corpus",
                datetime!(2023-01-01 00:00:00 UTC),
            )),
            expected_codebook_hash: hash,
        }
    }

    fn financial_bar_binding(reg: &ModelRegistration) -> TokenizerBinding {
        TokenizerBinding {
            model_id: reg.model_id.clone(),
            model_version: reg.version,
            codebook: ArtifactRef {
                artifact_id: ArtifactId::new("financial-bar-codebook"),
                kind: ArtifactKind::TokenizerCodebook,
                version: reg.version,
                content_hash: reg.expected_codebook_hash,
                signature: Default::default(),
            },
            max_context_tokens: 512,
            modalities: vec![SeriesModality::Ohlcv],
            reconstruction_mae: 0.02,
        }
    }

    #[test]
    fn commercial_profile_hard_denies_restricted_license_registration() {
        let mut reg = ModelRegistry::new(RegistryProfile::Commercial);
        let err = reg
            .register(synthetic_restricted_license_registration())
            .expect_err("CC-BY-NC must be denied");
        assert!(matches!(err, RegistryError::NonCommercialArtifact { .. }));
    }

    #[test]
    fn research_profile_allows_noncommercial_registration() {
        let mut reg = ModelRegistry::new(RegistryProfile::Research);
        reg.register(synthetic_restricted_license_registration())
            .unwrap();
    }

    #[test]
    fn commercial_allows_permissive_and_loads_stub() {
        let mut reg = ModelRegistry::new(RegistryProfile::Commercial);
        let entry = permissive_financial_bar();
        let binding = financial_bar_binding(&entry);
        reg.register(entry).unwrap();
        reg.bind_tokenizer(binding).unwrap();
        let runtime = reg
            .load_tsfm(&ModelId::new("financial-bar-base"), None)
            .unwrap();
        assert_eq!(runtime.model_id().0, "financial-bar-base");
    }

    #[test]
    fn contamination_denies_and_emits_security_event() {
        let mut reg = ModelRegistry::new(RegistryProfile::Commercial);
        let entry = permissive_financial_bar();
        let binding = financial_bar_binding(&entry);
        reg.register(entry).unwrap();
        reg.bind_tokenizer(binding).unwrap();

        let window_start = datetime!(2023-01-01 00:00:00 UTC); // equal to cutoff
        let err = reg
            .load_tsfm(&ModelId::new("financial-bar-base"), Some(window_start))
            .unwrap_err();
        assert!(matches!(
            err,
            RegistryError::PretrainingContamination { .. }
        ));
        let events = reg.security_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            TsfmSecurityEvent::LookaheadDenied { .. }
        ));
    }

    #[test]
    fn tokenizer_version_mismatch_fails_bind() {
        let mut reg = ModelRegistry::new(RegistryProfile::Commercial);
        let entry = permissive_financial_bar();
        let mut binding = financial_bar_binding(&entry);
        binding.model_version = SemanticVersion::new(9, 9, 9);
        reg.register(entry).unwrap();
        let err = reg.bind_tokenizer(binding).unwrap_err();
        assert!(matches!(
            err,
            RegistryError::TokenizerMismatch(TokenizerError::VersionMismatch { .. })
        ));
    }
}
