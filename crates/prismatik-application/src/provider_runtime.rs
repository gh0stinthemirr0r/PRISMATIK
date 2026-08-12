//! Shared provider runtime for interactive and scheduled live ingestion.
//!
//! Registration binds an endpoint to a provider capability, entitlement,
//! source policy, and persistent governor. Execution always traverses
//! [`crate::IngestCommand`], so no registered endpoint can bypass admission,
//! lawful-retention policy, durable observation storage, or audit emission.

use crate::{FileObservationStore, IngestCommand, IngestError, IngestOutcome, PayloadFetcher};
use prismatik_audit::InMemoryAuditLedger;
use prismatik_determinism::Clock;
use prismatik_market_data::{
    AcquisitionMethod, BudgetGovernor, BudgetState, Capability, Entitlement, IngestManifestId,
    PriorityClass, Provider, ProviderHealth, RetrievalAttemptId, SourcePolicy,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Immutable endpoint registration.
#[derive(Clone)]
pub struct ProviderEndpoint {
    /// Stable runtime endpoint key.
    pub id: String,
    /// Provider implementation advertising capability and entitlement state.
    pub provider: Arc<dyn Provider>,
    /// Capability required to invoke this endpoint.
    pub capability: Capability,
    /// Account entitlement required to invoke this endpoint, when applicable.
    pub entitlement: Option<Entitlement>,
    /// Reviewed acquisition and retention policy.
    pub policy: SourcePolicy,
    /// Approved acquisition mechanism.
    pub acquisition_method: AcquisitionMethod,
    /// Canonical provider URI retained in observation provenance.
    pub source_uri: String,
    /// Persistent per-provider budget governor.
    pub governor: Arc<dyn BudgetGovernor>,
}

impl std::fmt::Debug for ProviderEndpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderEndpoint")
            .field("id", &self.id)
            .field("provider", &self.provider.id())
            .field("capability", &self.capability)
            .field("entitlement", &self.entitlement)
            .field("policy", &self.policy)
            .field("acquisition_method", &self.acquisition_method)
            .field("source_uri", &self.source_uri)
            .finish_non_exhaustive()
    }
}

/// Honest runtime status sourced from the provider and its live governor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderRuntimeStatus {
    /// Endpoint registration key.
    pub endpoint_id: String,
    /// Provider-reported health without synthesized telemetry.
    pub health: ProviderHealth,
    /// Current interactive request budget.
    pub interactive_budget: BudgetState,
    /// Current background request budget.
    pub background_budget: BudgetState,
}

/// Provider runtime registration or execution failure.
#[derive(Debug, Error)]
pub enum ProviderRuntimeError {
    /// Endpoint id or source URI was empty.
    #[error("provider endpoint requires non-empty id and source URI")]
    EmptyEndpoint,
    /// Endpoint id was already registered.
    #[error("provider endpoint is already registered: {0}")]
    DuplicateEndpoint(String),
    /// Provider did not advertise the required capability.
    #[error("provider does not advertise capability {0:?}")]
    MissingCapability(Capability),
    /// Provider account did not advertise the required entitlement.
    #[error("provider account lacks entitlement {0:?}")]
    MissingEntitlement(Entitlement),
    /// Acquisition method was absent from the reviewed source policy.
    #[error("source policy does not authorize acquisition method {0:?}")]
    AcquisitionDenied(AcquisitionMethod),
    /// Endpoint id was not registered.
    #[error("provider endpoint is not registered: {0}")]
    UnknownEndpoint(String),
    /// The shared runtime registry lock was poisoned.
    #[error("provider runtime registry lock was poisoned")]
    RegistryPoisoned,
    /// Governed ingestion failed.
    #[error(transparent)]
    Ingest(#[from] IngestError),
}

/// Thread-safe registry and composition root for all live provider work.
pub struct ProviderRuntime {
    clock: Arc<dyn Clock>,
    store: Arc<FileObservationStore>,
    audit: Arc<InMemoryAuditLedger>,
    endpoints: RwLock<BTreeMap<String, ProviderEndpoint>>,
}

impl std::fmt::Debug for ProviderRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderRuntime")
            .field("clock_kind", &self.clock.kind())
            .field("store", &self.store)
            .field("audit", &self.audit)
            .field("endpoints", &self.endpoints)
            .finish()
    }
}

impl ProviderRuntime {
    /// Construct a runtime over shared deterministic time, storage, and audit.
    pub fn new(
        clock: Arc<dyn Clock>,
        store: Arc<FileObservationStore>,
        audit: Arc<InMemoryAuditLedger>,
    ) -> Self {
        Self {
            clock,
            store,
            audit,
            endpoints: RwLock::new(BTreeMap::new()),
        }
    }

    /// Register an endpoint after capability, entitlement, and policy checks.
    pub fn register(&self, endpoint: ProviderEndpoint) -> Result<(), ProviderRuntimeError> {
        if endpoint.id.trim().is_empty() || endpoint.source_uri.trim().is_empty() {
            return Err(ProviderRuntimeError::EmptyEndpoint);
        }
        if !endpoint
            .provider
            .capabilities()
            .contains(endpoint.capability)
        {
            return Err(ProviderRuntimeError::MissingCapability(endpoint.capability));
        }
        if let Some(required) = &endpoint.entitlement {
            if !endpoint.provider.entitlements().contains(required) {
                return Err(ProviderRuntimeError::MissingEntitlement(required.clone()));
            }
        }
        if !endpoint
            .policy
            .allowed_methods()
            .contains(&endpoint.acquisition_method)
        {
            return Err(ProviderRuntimeError::AcquisitionDenied(
                endpoint.acquisition_method,
            ));
        }
        let mut endpoints = self
            .endpoints
            .write()
            .map_err(|_| ProviderRuntimeError::RegistryPoisoned)?;
        if endpoints.contains_key(&endpoint.id) {
            return Err(ProviderRuntimeError::DuplicateEndpoint(endpoint.id));
        }
        endpoints.insert(endpoint.id.clone(), endpoint);
        Ok(())
    }

    /// Return endpoint ids in stable registration-key order.
    pub fn endpoint_ids(&self) -> Result<Vec<String>, ProviderRuntimeError> {
        Ok(self
            .endpoints
            .read()
            .map_err(|_| ProviderRuntimeError::RegistryPoisoned)?
            .keys()
            .cloned()
            .collect())
    }

    /// Execute one endpoint through its persistent governor and durable ingest path.
    pub async fn execute(
        &self,
        endpoint_id: &str,
        fetcher: &dyn PayloadFetcher,
        priority: PriorityClass,
        attempt_id: RetrievalAttemptId,
        manifest_id: IngestManifestId,
    ) -> Result<IngestOutcome, ProviderRuntimeError> {
        let endpoint = self
            .endpoints
            .read()
            .map_err(|_| ProviderRuntimeError::RegistryPoisoned)?
            .get(endpoint_id)
            .cloned()
            .ok_or_else(|| ProviderRuntimeError::UnknownEndpoint(endpoint_id.into()))?;
        let command = IngestCommand::new(
            self.clock.clone(),
            endpoint.governor,
            self.store.clone(),
            self.audit.clone(),
        );
        Ok(command
            .execute(
                fetcher,
                endpoint.policy.source_id().clone(),
                endpoint.policy.version(),
                endpoint.acquisition_method,
                priority,
                &endpoint.policy,
                attempt_id,
                manifest_id,
                &endpoint.source_uri,
            )
            .await?)
    }

    /// Probe registered providers and expose real health plus persistent budgets.
    pub async fn statuses(&self) -> Result<Vec<ProviderRuntimeStatus>, ProviderRuntimeError> {
        let endpoints = self
            .endpoints
            .read()
            .map_err(|_| ProviderRuntimeError::RegistryPoisoned)?
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut statuses = Vec::with_capacity(endpoints.len());
        for endpoint in endpoints {
            statuses.push(ProviderRuntimeStatus {
                endpoint_id: endpoint.id,
                health: endpoint.provider.health().await,
                interactive_budget: endpoint.governor.state(PriorityClass::Interactive),
                background_budget: endpoint.governor.state(PriorityClass::Background),
            });
        }
        Ok(statuses)
    }

    /// Number of durable observations visible to this runtime.
    pub fn observation_count(&self) -> usize {
        self.store.len()
    }

    /// Number of audit events emitted by this runtime.
    pub fn audit_event_count(&self) -> usize {
        self.audit.entries().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::FetchedPayload;
    use prismatik_determinism::FrozenClock;
    use prismatik_domain::ProviderId;
    use prismatik_market_data::{
        EntitlementSet, GcraBudgetGovernor, PayloadRetention, PermittedMetadataField,
        ProviderCapabilities, RedistributionPolicy, SourceId, SourcePolicyVersion,
    };
    use std::collections::BTreeSet;
    use time::{Duration, OffsetDateTime};

    #[derive(Debug)]
    struct TestProvider {
        capabilities: ProviderCapabilities,
        entitlements: EntitlementSet,
    }

    #[async_trait::async_trait]
    impl Provider for TestProvider {
        fn id(&self) -> ProviderId {
            ProviderId::COINGECKO
        }

        fn capabilities(&self) -> ProviderCapabilities {
            self.capabilities.clone()
        }

        fn entitlements(&self) -> &EntitlementSet {
            &self.entitlements
        }

        fn cost_of(
            &self,
            _request: &prismatik_market_data::ProviderRequest,
        ) -> prismatik_market_data::CostUnits {
            prismatik_market_data::CostUnits::new(1)
        }

        async fn health(&self) -> ProviderHealth {
            ProviderHealth::healthy()
        }
    }

    #[derive(Debug)]
    struct TestFetcher;

    #[async_trait::async_trait]
    impl PayloadFetcher for TestFetcher {
        async fn fetch(
            &self,
        ) -> Result<FetchedPayload, prismatik_market_data::http::TransportError> {
            Ok(FetchedPayload {
                bytes: br#"{"price":"100"}"#.to_vec(),
                media_type: "application/json".into(),
                event_time: Some(OffsetDateTime::UNIX_EPOCH),
                publication_time: None,
                publisher_record_id: None,
            })
        }
    }

    fn endpoint(clock: Arc<dyn Clock>) -> ProviderEndpoint {
        let provider = TestProvider {
            capabilities: [Capability::CryptoMarkets].into(),
            entitlements: [Entitlement::CoinGeckoDemo].into(),
        };
        let policy = SourcePolicy::new(
            SourceId::new("coingecko-markets").unwrap(),
            SourcePolicyVersion::new(1).unwrap(),
            [AcquisitionMethod::Api],
            PayloadRetention::RawPayloadPermitted,
            RedistributionPolicy::Prohibited,
            BTreeSet::<PermittedMetadataField>::new(),
            "CoinGecko",
            OffsetDateTime::UNIX_EPOCH - Duration::days(1),
            OffsetDateTime::UNIX_EPOCH - Duration::hours(1),
            OffsetDateTime::UNIX_EPOCH + Duration::days(1),
            "review:coingecko-test",
        )
        .unwrap();
        ProviderEndpoint {
            id: "coingecko.markets".into(),
            provider: Arc::new(provider),
            capability: Capability::CryptoMarkets,
            entitlement: Some(Entitlement::CoinGeckoDemo),
            policy,
            acquisition_method: AcquisitionMethod::Api,
            source_uri: "https://api.coingecko.test/coins/markets".into(),
            governor: Arc::new(GcraBudgetGovernor::new(
                prismatik_market_data::GovernorQuota {
                    burst: 5,
                    per_period_cells: 5,
                    per_period_secs: 1,
                },
                prismatik_market_data::GovernorQuota {
                    burst: 5,
                    per_period_cells: 5,
                    per_period_secs: 1,
                },
                clock,
            )),
        }
    }

    #[tokio::test]
    async fn registered_execution_is_durable_audited_and_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FrozenClock::new(OffsetDateTime::UNIX_EPOCH));
        let runtime = ProviderRuntime::new(
            clock.clone(),
            Arc::new(FileObservationStore::open(temp.path()).unwrap()),
            Arc::new(InMemoryAuditLedger::new()),
        );
        runtime.register(endpoint(clock)).unwrap();
        let first = runtime
            .execute(
                "coingecko.markets",
                &TestFetcher,
                PriorityClass::Interactive,
                RetrievalAttemptId::from_bytes([1; 32]),
                IngestManifestId::from_bytes([2; 32]),
            )
            .await
            .unwrap();
        assert!(matches!(
            first.append,
            crate::DurableAppendOutcome::Inserted { .. }
        ));
        assert_eq!(runtime.observation_count(), 1);
        assert_eq!(runtime.audit_event_count(), 1);
        assert_eq!(runtime.endpoint_ids().unwrap(), ["coingecko.markets"]);
    }

    #[test]
    fn registration_fails_closed_on_missing_capability() {
        let temp = tempfile::tempdir().unwrap();
        let clock: Arc<dyn Clock> = Arc::new(FrozenClock::new(OffsetDateTime::UNIX_EPOCH));
        let runtime = ProviderRuntime::new(
            clock.clone(),
            Arc::new(FileObservationStore::open(temp.path()).unwrap()),
            Arc::new(InMemoryAuditLedger::new()),
        );
        let mut invalid = endpoint(clock);
        invalid.capability = Capability::Filings;
        assert!(matches!(
            runtime.register(invalid),
            Err(ProviderRuntimeError::MissingCapability(Capability::Filings))
        ));
    }
}
