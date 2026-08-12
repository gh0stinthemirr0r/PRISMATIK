//! Reviewed API-provider retention policies; persistence is disabled by default.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use prismatik_application::FileStateJournal;
use prismatik_determinism::{Clock, SystemClock};
use prismatik_market_data::{
    AcquisitionMethod, PayloadRetention, PermittedMetadataField, RedistributionPolicy, SourceId,
    SourcePolicy, SourcePolicyVersion,
};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderPolicyConfig {
    provider_id: String,
    enabled: bool,
    terms_owner: String,
    review_reference: String,
    valid_until: String,
    retain_normalized_payload: bool,
    version: u32,
    recorded_at: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderPolicyDraft {
    provider_id: String,
    enabled: bool,
    terms_owner: String,
    review_reference: String,
    valid_until: String,
    retain_normalized_payload: bool,
}

#[derive(Debug)]
struct PolicyRuntime {
    latest: BTreeMap<String, ProviderPolicyConfig>,
    journal: FileStateJournal<BTreeMap<String, ProviderPolicyConfig>>,
}

static RUNTIME: OnceLock<Mutex<PolicyRuntime>> = OnceLock::new();

pub(crate) fn initialize(data_dir: &Path) -> Result<(), String> {
    let mut journal: FileStateJournal<BTreeMap<String, ProviderPolicyConfig>> =
        FileStateJournal::open(
            data_dir.join("provider-source-policies.jsonl"),
            "prismatik.provider-source-policies.v1",
        )
        .map_err(|error| error.to_string())?;
    let latest = journal.latest().cloned().unwrap_or_default();
    if journal.latest().is_none() {
        journal
            .append("initialized", latest.clone(), SystemClock::new().now())
            .map_err(|error| error.to_string())?;
    }
    RUNTIME
        .set(Mutex::new(PolicyRuntime { latest, journal }))
        .map_err(|_| "provider policy runtime initialized twice".to_owned())
}

#[tauri::command]
pub(crate) fn list_provider_policies() -> Result<Vec<ProviderPolicyConfig>, String> {
    Ok(RUNTIME
        .get()
        .ok_or("provider policy runtime is unavailable")?
        .lock()
        .map_err(|_| "provider policy runtime lock is unavailable".to_owned())?
        .latest
        .values()
        .cloned()
        .collect())
}

#[tauri::command]
pub(crate) fn upsert_provider_policy(
    policy: ProviderPolicyDraft,
) -> Result<ProviderPolicyConfig, String> {
    validate_draft(&policy)?;
    let now = SystemClock::new().now();
    let mut runtime = RUNTIME
        .get()
        .ok_or("provider policy runtime is unavailable")?
        .lock()
        .map_err(|_| "provider policy runtime lock is unavailable".to_owned())?;
    let version = runtime
        .latest
        .get(&policy.provider_id)
        .map_or(1, |existing| existing.version.saturating_add(1));
    let record = ProviderPolicyConfig {
        provider_id: policy.provider_id,
        enabled: policy.enabled,
        terms_owner: policy.terms_owner,
        review_reference: policy.review_reference,
        valid_until: policy.valid_until,
        retain_normalized_payload: policy.retain_normalized_payload,
        version,
        recorded_at: now.to_string(),
    };
    let mut next = runtime.latest.clone();
    next.insert(record.provider_id.clone(), record.clone());
    runtime
        .journal
        .append("policy_version_recorded", next.clone(), now)
        .map_err(|error| error.to_string())?;
    runtime.latest = next;
    Ok(record)
}

fn validate_draft(draft: &ProviderPolicyDraft) -> Result<(), String> {
    if !matches!(
        draft.provider_id.as_str(),
        "coingecko" | "finnhub" | "fred" | "sec-edgar"
    ) {
        return Err("provider does not have a native durable observation adapter".into());
    }
    if draft.enabled
        && (draft.terms_owner.trim().is_empty() || draft.review_reference.trim().is_empty())
    {
        return Err("enabled persistence requires terms owner and review reference".into());
    }
    let now = SystemClock::new().now();
    let until = OffsetDateTime::parse(&draft.valid_until, &Rfc3339)
        .map_err(|_| "validUntil must be RFC3339".to_owned())?;
    if draft.enabled && until <= now {
        return Err("enabled provider policy must expire in the future".into());
    }
    let _ = build_policy(
        &ProviderPolicyConfig {
            provider_id: draft.provider_id.clone(),
            enabled: draft.enabled,
            terms_owner: draft.terms_owner.clone(),
            review_reference: draft.review_reference.clone(),
            valid_until: draft.valid_until.clone(),
            retain_normalized_payload: draft.retain_normalized_payload,
            version: 1,
            recorded_at: now.to_string(),
        },
        now,
    )?;
    Ok(())
}

pub(crate) fn active_policy(provider_id: &str) -> Result<Option<SourcePolicy>, String> {
    let now = SystemClock::new().now();
    let runtime = RUNTIME
        .get()
        .ok_or("provider policy runtime is unavailable")?
        .lock()
        .map_err(|_| "provider policy runtime lock is unavailable".to_owned())?;
    let Some(config) = runtime.latest.get(provider_id) else {
        return Ok(None);
    };
    if !config.enabled {
        return Ok(None);
    }
    Ok(Some(build_policy(config, now)?))
}

fn build_policy(
    config: &ProviderPolicyConfig,
    now: OffsetDateTime,
) -> Result<SourcePolicy, String> {
    let recorded_at = OffsetDateTime::parse(&config.recorded_at, &Rfc3339)
        .map_err(|_| "recorded provider policy time is invalid".to_owned())?;
    let valid_until = OffsetDateTime::parse(&config.valid_until, &Rfc3339)
        .map_err(|_| "provider policy expiry is invalid".to_owned())?;
    SourcePolicy::new(
        SourceId::new(format!("api:{}", config.provider_id)).map_err(|error| error.to_string())?,
        SourcePolicyVersion::new(config.version).map_err(|error| error.to_string())?,
        [AcquisitionMethod::Api],
        if config.retain_normalized_payload {
            PayloadRetention::RawPayloadPermitted
        } else {
            PayloadRetention::MetadataOnly
        },
        RedistributionPolicy::Prohibited,
        [PermittedMetadataField::PublisherRecordId],
        config.terms_owner.clone(),
        recorded_at,
        recorded_at,
        valid_until,
        config.review_reference.clone(),
    )
    .and_then(|policy| {
        if now >= policy.valid_until() {
            Err(prismatik_market_data::CorpusError::PolicyExpired {
                source_id: policy.source_id().clone(),
                version: policy.version(),
            })
        } else {
            Ok(policy)
        }
    })
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence_requires_a_named_review() {
        let draft = ProviderPolicyDraft {
            provider_id: "fred".into(),
            enabled: true,
            terms_owner: String::new(),
            review_reference: String::new(),
            valid_until: "2099-01-01T00:00:00Z".into(),
            retain_normalized_payload: false,
        };
        assert!(validate_draft(&draft).is_err());
    }
}
