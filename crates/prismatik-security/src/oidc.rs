//! OIDC claim validation floor (`P8-SS-01`).
//!
//! Offline typed checks only — no live IdP / JWKS / network client.
//! Live OIDC discovery and token exchange remain author-ops residual.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Provider configuration for offline audience / expiry checks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OidcProviderConfig {
    /// Issuer URL (documented; not fetched in this floor).
    pub issuer: String,
    /// OAuth / OIDC client id for this deployment.
    pub client_id: String,
    /// Accepted `aud` values (deny unless claims.aud is listed).
    pub audiences: Vec<String>,
}

/// Subset of ID/access token claims used by the offline floor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OidcTokenClaims {
    /// Subject identifier.
    pub sub: String,
    /// Audience claim (single value at this floor).
    pub aud: String,
    /// Expiration time as Unix seconds.
    pub exp: u64,
}

/// Offline claim validation failures.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum OidcClaimsError {
    /// `aud` is not in [`OidcProviderConfig::audiences`].
    #[error("oidc audience rejected: {aud}")]
    AudienceRejected {
        /// Observed audience.
        aud: String,
    },
    /// Token `exp` is not strictly after `now_unix`.
    #[error("oidc token expired: exp={exp} now={now}")]
    Expired {
        /// Claim expiration.
        exp: u64,
        /// Validation clock (Unix seconds).
        now: u64,
    },
}

/// Errors from live IdP operations (discovery / token exchange / JWKS fetch).
///
/// Live wiring remains author-ops; callers must fail closed on [`Self::NotLinked`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum OidcLiveError {
    /// No live IdP / JWKS / token endpoint client is linked in this build.
    #[error("oidc live IdP not linked (author-ops residual): {0}")]
    NotLinked(String),
}

/// Validate `aud` membership and `exp` against `now_unix` with no network I/O.
///
/// Does not verify signatures, issuer fetch, or nonce — those remain author-ops.
pub fn validate_claims_offline(
    claims: &OidcTokenClaims,
    config: &OidcProviderConfig,
    now_unix: u64,
) -> Result<(), OidcClaimsError> {
    if !config.audiences.iter().any(|a| a == &claims.aud) {
        return Err(OidcClaimsError::AudienceRejected {
            aud: claims.aud.clone(),
        });
    }
    if claims.exp <= now_unix {
        return Err(OidcClaimsError::Expired {
            exp: claims.exp,
            now: now_unix,
        });
    }
    Ok(())
}

/// Attempt live IdP discovery / token exchange for `config`.
///
/// Always fails with [`OidcLiveError::NotLinked`] — typed fail-closed until
/// author-ops links a real discovery + JWKS client. No network I/O.
pub fn try_live_idp(config: &OidcProviderConfig) -> Result<(), OidcLiveError> {
    Err(OidcLiveError::NotLinked(format!(
        "issuer={} client_id={}",
        config.issuer, config.client_id
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> OidcProviderConfig {
        OidcProviderConfig {
            issuer: "https://idp.example/".into(),
            client_id: "prismatik-desktop".into(),
            audiences: vec!["prismatik-api".into(), "prismatik-desktop".into()],
        }
    }

    fn claims(aud: &str, exp: u64) -> OidcTokenClaims {
        OidcTokenClaims {
            sub: "user-1".into(),
            aud: aud.into(),
            exp,
        }
    }

    #[test]
    fn validate_claims_offline_passes_matching_aud_and_future_exp() {
        let cfg = config();
        let c = claims("prismatik-api", 2_000);
        assert!(validate_claims_offline(&c, &cfg, 1_000).is_ok());
    }

    #[test]
    fn validate_claims_offline_rejects_wrong_aud() {
        let cfg = config();
        let c = claims("other-service", 2_000);
        assert_eq!(
            validate_claims_offline(&c, &cfg, 1_000),
            Err(OidcClaimsError::AudienceRejected {
                aud: "other-service".into(),
            })
        );
    }

    #[test]
    fn validate_claims_offline_rejects_expired_exp() {
        let cfg = config();
        let c = claims("prismatik-desktop", 500);
        assert_eq!(
            validate_claims_offline(&c, &cfg, 1_000),
            Err(OidcClaimsError::Expired {
                exp: 500,
                now: 1_000,
            })
        );
        // Boundary: exp == now is expired.
        let c2 = claims("prismatik-desktop", 1_000);
        assert!(matches!(
            validate_claims_offline(&c2, &cfg, 1_000),
            Err(OidcClaimsError::Expired { .. })
        ));
    }

    #[test]
    fn try_live_idp_fails_closed_not_linked() {
        let cfg = config();
        let err = try_live_idp(&cfg).expect_err("must stay unlinked");
        assert!(matches!(err, OidcLiveError::NotLinked(_)));
        assert!(err.to_string().contains("author-ops residual"));
        assert!(err.to_string().contains("idp.example"));
    }
}
