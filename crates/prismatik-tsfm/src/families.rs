//! Onboarded TSFM family catalogue (`P5-QM-09` floor).
//!
//! Variants are **PRISMATIK role names** (ADR-0032 amendment): we do not brand
//! the catalog after third-party GitHub / research repos we are not integrating
//! as product dependencies. License posture and capability roles stay explicit.

use serde::{Deserialize, Serialize};

/// Commercial onboarding status for a named TSFM family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FamilyOnboardingStatus {
    /// Allowed into commercial-profile registries (permissive license).
    Onboarded,
    /// Hard-rejected for commercial registration (e.g. CC BY-NC).
    RejectedCommercial,
}

/// Closed set of TSFM **capability roles** tracked by the plural registry.
///
/// Five roles are commercially onboarded; [`Self::RestrictedLicense`] exists so
/// the registry can name and reject NonCommercial artifacts explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnboardedTsfmFamily {
    /// OHLCV / bar-series specialist (tokenizer + path synthesis role).
    FinancialBar,
    /// General forecaster with covariate support.
    GeneralCovariate,
    /// Streaming / recurrent sequence role.
    StreamingSequence,
    /// Large-scale / high-capacity forecasting role.
    LargeScaleForecast,
    /// Probabilistic baseline role.
    ProbabilisticBaseline,
    /// NonCommercial / restricted-license placeholder — commercial registration denied.
    RestrictedLicense,
}

impl OnboardedTsfmFamily {
    /// Commercial onboarding status for this family.
    #[must_use]
    pub const fn status(self) -> FamilyOnboardingStatus {
        match self {
            Self::FinancialBar
            | Self::GeneralCovariate
            | Self::StreamingSequence
            | Self::LargeScaleForecast
            | Self::ProbabilisticBaseline => FamilyOnboardingStatus::Onboarded,
            Self::RestrictedLicense => FamilyOnboardingStatus::RejectedCommercial,
        }
    }

    /// SPDX expression expected for commercially onboarded roles (floor).
    #[must_use]
    pub const fn license_spdx(self) -> &'static str {
        match self {
            Self::FinancialBar => "MIT",
            Self::GeneralCovariate
            | Self::StreamingSequence
            | Self::LargeScaleForecast
            | Self::ProbabilisticBaseline => "Apache-2.0",
            Self::RestrictedLicense => "CC-BY-NC-4.0",
        }
    }

    /// Stable display name (Prismatik role id).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FinancialBar => "FinancialBar",
            Self::GeneralCovariate => "GeneralCovariate",
            Self::StreamingSequence => "StreamingSequence",
            Self::LargeScaleForecast => "LargeScaleForecast",
            Self::ProbabilisticBaseline => "ProbabilisticBaseline",
            Self::RestrictedLicense => "RestrictedLicense",
        }
    }

    /// All catalogue entries in fixed order.
    pub const ALL: [Self; 6] = [
        Self::FinancialBar,
        Self::GeneralCovariate,
        Self::StreamingSequence,
        Self::LargeScaleForecast,
        Self::ProbabilisticBaseline,
        Self::RestrictedLicense,
    ];

    /// Commercially onboarded families only.
    pub const COMMERCIAL: [Self; 5] = [
        Self::FinancialBar,
        Self::GeneralCovariate,
        Self::StreamingSequence,
        Self::LargeScaleForecast,
        Self::ProbabilisticBaseline,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restricted_license_is_rejected_commercial() {
        assert_eq!(
            OnboardedTsfmFamily::RestrictedLicense.status(),
            FamilyOnboardingStatus::RejectedCommercial
        );
        assert_eq!(
            OnboardedTsfmFamily::RestrictedLicense.license_spdx(),
            "CC-BY-NC-4.0"
        );
    }

    #[test]
    fn five_families_onboarded() {
        assert_eq!(OnboardedTsfmFamily::COMMERCIAL.len(), 5);
        for f in OnboardedTsfmFamily::COMMERCIAL {
            assert_eq!(f.status(), FamilyOnboardingStatus::Onboarded);
        }
    }
}
