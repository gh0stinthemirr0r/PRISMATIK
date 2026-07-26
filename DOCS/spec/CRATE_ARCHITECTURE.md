# PRISMATIK Crate Architecture Specification

**Document:** `spec/CRATE_ARCHITECTURE.md`
**Status:** NORMATIVE — RFC 2119 keywords apply
**Companion to:** `PRISMATIK_Unified_Solution_Architecture_v1.0.md` §10 (workspace layout), §11 (trait registry)
**Date:** 2026-07-26

---

## 0. Purpose and Reading Order

This document specifies the **public API surface, dependency graph, error types, and configuration shape** of every crate in the PRISMATIK workspace. An engineer (human or AI agent) implementing any crate MUST conform to this specification; deviations require an ADR.

Read this document alongside:
- `spec/DATA_SCHEMAS.md` — for storage shapes referenced here
- `spec/IPC_CONTRACTS.md` — for the application layer that wires crates to the frontend
- `spec/MANIFEST_SCHEMA.md` — for the artifact and manifest types

This document is organized by dependency layer. The dependency rule (v1.0 §10) is enforced by a CI check that parses the workspace graph:

> `prismatik-determinism` and `prismatik-identity` depend on nothing else in the workspace. Every other crate MAY depend on them. No crate MAY depend on `prismatik-storage` except `prismatik-application`. Domain crates depend on ports, never on SQLx, never on reqwest, never on a vendor SDK.

---

## 1. Layer 0 — Foundational Crates (no workspace dependencies)

These crates are the substrate. Nothing in the workspace may depend outward beyond these.

### 1.1 `prismatik-determinism`

**Purpose.** The single source of time, entropy, and deterministic ordering for the entire platform. Closes Gap G01.

**Workspace dependencies:** NONE. May depend on `time`, `rand_chacha`, `rustc-hash`, `blake3`, `serde`, `thiserror`, `tracing` only.

**Public API:**

```rust
// Re-exports the trait bundle every deterministic subsystem receives.
pub use clock::{Clock, ClockKind, SystemClock, SimulatedClock, FrozenClock};
pub use entropy::{Entropy, SplitEntropy, EntropyError};
pub use context::{DeterminismContext, RunId, PinnedArtifactSet};
pub use hashing::{DetHasher, DetMap, DetSet};
pub use artifact_ref::{ArtifactRef, ArtifactId, ArtifactKind, ContentHash, SemanticVersion};
pub use signature::{DualSignature, SigningIdentity};

pub mod clock;
pub mod entropy;
pub mod context;
pub mod hashing;
pub mod artifact_ref;
pub mod signature;
```

**`Clock` trait** (normative per v1.0 §12.2):
```rust
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> OffsetDateTime;
    fn monotonic_nanos(&self) -> u64;
    fn sleep_until(&self, deadline: OffsetDateTime) -> BoxFuture<'static, ()>;
    fn kind(&self) -> ClockKind;
}
```
Three implementations and only three: `SystemClock` (permitted only in outermost shell + ingestion for `retrieved_at`), `SimulatedClock` (backtest, replay, DST), `FrozenClock` (unit tests, point-in-time feature materialization).

**`Entropy` trait** (normative per v1.0 §12.2):
```rust
pub trait Entropy: Send + Sync + 'static {
    fn split(&self, label: &str) -> Box<dyn Entropy>;
    fn next_u64(&mut self) -> u64;
    fn next_f64_unit(&mut self) -> f64;
    fn fill_bytes(&mut self, dest: &mut [u8]);
    fn root_seed(&self) -> u64;
    fn stream_path(&self) -> &[String];
}
```
Splittable by design: a child stream derived from a parent with a stable label produces identical values regardless of sibling consumption order. Cryptographic randomness is OUTSIDE this trait — it lives in `prismatik-security` against the OS CSPRNG, because key material MUST never be reproducible.

**Error types:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum DeterminismError {
    #[error("ambient nondeterminism leak: {0}")]
    AmbientLeak(String),
    #[error("artifact hash mismatch: expected {expected}, found {found}")]
    HashMismatch { expected: ContentHash, found: ContentHash },
    #[error("seed replay divergence at seed {seed}")]
    ReplayDivergence { seed: u64 },
}
```

**Configuration:** none at the trait level. Concrete implementations take their config in constructors:
- `SystemClock::new()` — no config
- `SimulatedClock::new(start: OffsetDateTime, tick_step: Duration)`
- `SplitEntropy::from_seed(seed: u64) -> Self`

**CI gates enforced by this crate:**
- `determinism-grep` — no `Instant::now`/`SystemTime::now`/`thread_rng`/default `HashMap` outside the crate's `#![allow]` scope
- Property test: split order independence (10k iterations)
- Property test: same seed + same label → same child stream

---

### 1.2 `prismatik-identity`

**Purpose.** Canonical asset identity, bitemporal symbology, venue registry. Closes Gap G08.

**Workspace dependencies:** `prismatik-determinism` only.

**Public API:**

```rust
pub use asset_id::{AssetId, VenueId, MicCode};
pub use external_id::ExternalIdentifier;
pub use resolver::{SymbologyResolver, SymbologyError, IdentityTransition};
pub use corporate_action::{CorporateAction, CorporateActionId, CorporateActionKind};

pub struct OpenFigiMapper { /* ... */ }

pub mod asset_id;
pub mod external_id;
pub mod resolver;
pub mod corporate_action;
```

**`AssetId`** — opaque, content-addressed (BLAKE3 of canonical bitemporal identity record). Internal canonical identifier; the only identifier domain crates ever see as a key.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId([u8; 32]);  // BLAKE3 digest

impl AssetId {
    pub fn from_canonical_record(rec: &CanonicalIdentityRecord) -> Self { /* ... */ }
}
```

**`ExternalIdentifier` enum** (normative per v1.0 §12.6):
```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ExternalIdentifier {
    Figi(String),
    Isin(String),
    Cusip(String),                              // licensed; gated by entitlement
    TickerAtVenue { ticker: String, mic: MicCode },
    OccOptionSymbol(String),
    CoinGeckoId(String),
    OnChain { chain_id: u64, address: String },
    CftcMarketCode(String),
    SecCik(u64),
}
```

**`SymbologyResolver` trait** (normative per v1.0 §12.6):
```rust
pub trait SymbologyResolver: Send + Sync {
    fn resolve_as_of(
        &self,
        identifier: &ExternalIdentifier,
        as_of: OffsetDateTime,
    ) -> Result<Option<AssetId>, SymbologyError>;

    fn identifiers_as_of(
        &self,
        asset: &AssetId,
        as_of: OffsetDateTime,
    ) -> Result<Vec<ExternalIdentifier>, SymbologyError>;

    fn identity_chain(
        &self,
        asset: &AssetId,
    ) -> Result<Vec<IdentityTransition>, SymbologyError>;

    fn snapshot_artifact(&self) -> &ArtifactRef;
}
```
**There is deliberately no `resolve_now`** — callers MUST state which moment they mean. Bitemporality is mandatory, not an optimization: a 2021 backtest that resolves "FB" using a 2026 mapping produces silently wrong results, and no seed/hash/signature detects it.

**`CorporateActionKind`** (normative per v1.0 §12.7):
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CorporateActionKind {
    Split { numerator: u32, denominator: u32 },
    ReverseSplit { numerator: u32, denominator: u32 },
    CashDividend { amount: Decimal, currency: CurrencyCode },
    StockDividend { ratio: Ratio },
    SpinOff { child: AssetId, ratio: Ratio },
    Merger { surviving: AssetId, terms: MergerTerms },
    TickerChange { from: String, to: String },
    Delisting { reason: DelistingReason },
    OptionAdjustment { memo: OccMemoRef },  // HardDeny for automated options strategies
}
```

**Error types:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum SymbologyError {
    #[error("no mapping for {identifier:?} as of {as_of}")]
    Unmapped { identifier: ExternalIdentifier, as_of: OffsetDateTime },
    #[error("artifact stale: snapshot {snapshot} is before requested as_of {as_of}")]
    ArtifactStale { snapshot: OffsetDateTime, as_of: OffsetDateTime },
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
}
```

**Configuration:** the resolver reads from a pinned `ArtifactRef` (the symbology snapshot). Cold-start loads the artifact, verifies hash + signature, builds an in-memory bitemporal index. Hot-reload is an explicit reviewed event.

---

## 2. Layer 1 — Kernel Extension Crates

These extend the kernel with calendar, audit, manifest, and storage primitives. They depend on Layer 0 only.

### 2.1 `prismatik-calendar`

**Purpose.** Pinned trading-session authority. Closes Gap G07. **Critical invariant: PRISMATIK MUST NOT call a calendar library at runtime.**

**Workspace dependencies:** `prismatik-determinism`.

**Why a pinned artifact, not a library.** Upstream calendar libraries ship sessions as versioned CODE. Point releases have historically corrected years of past sessions across CME, NYSE, EUREX. A backtest reading a calendar at runtime is not reproducible across a dependency upgrade even with an identical seed. The generator script (`scripts/build_calendar_artifact.py`) materializes sessions into Parquet at release time, cross-validates against QuantLib, and fails on any disagreement.

**Public API:**
```rust
pub use session::{Session, SessionKind, Interruption};
pub use trait_def::{SessionCalendar, VenueCalendar};
pub use reader::CalendarArtifactReader;

pub mod session;
pub mod trait_def;
pub mod reader;
```

**`SessionCalendar` trait** (normative per v1.0 §12.4):
```rust
pub trait SessionCalendar: Send + Sync {
    fn venue(&self) -> VenueId;
    fn artifact(&self) -> &ArtifactRef;
    fn is_open(&self, at: OffsetDateTime) -> bool;
    fn session_for(&self, at: OffsetDateTime) -> Option<Session>;
    fn sessions_between(&self, from: OffsetDateTime, to: OffsetDateTime) -> Vec<Session>;
    fn bar_boundaries(&self, session: &Session, resolution: BarResolution) -> Vec<OffsetDateTime>;
}
```

**`Session` record** (normative per v1.0 §12.4):
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub venue: VenueId,
    pub date: Date,
    pub open: OffsetDateTime,
    pub close: OffsetDateTime,
    pub breaks: Vec<(OffsetDateTime, OffsetDateTime)>,  // Asian lunch, futures processing
    pub early_close: bool,
    pub interruptions: Vec<Interruption>,
    pub kind: SessionKind,  // Regular | PreMarket | PostMarket | Holiday
}
```

**Artifact format:** Parquet, one row per session, partitioned by `venue/year`. Schema in `spec/DATA_SCHEMAS.md` §3. The artifact carries the generator version, the QuantLib cross-validation report hash, and the upstream library versions used.

---

### 2.2 `prismatik-audit`

**Purpose.** Append-only, hash-chained (Merkle) audit ledger. Closes Gap G02. The only write path for sensitive operations per Wave 4's audit-as-write-path inversion (v1.0 §14.2).

**Workspace dependencies:** `prismatik-determinism`.

**Public API:**
```rust
pub use trait_def::{AuditLedger, AuditEntry, AuditReceipt, AuditAction, Actor, SubjectRef, Outcome};
pub use proof::{InclusionProof, ConsistencyProof, TreeHead, SignedTreeHead};
pub use redacted::RedactedJson;
pub use error::AuditError;

pub mod trait_def;
pub mod proof;
pub mod redacted;
pub mod error;
pub mod backends;  // SQLite (desktop), Postgres (cloud/enterprise), TigerBeetle (evaluated Wave 4+)
```

**`AuditLedger` trait** (normative per v1.0 §14.1):
```rust
#[async_trait::async_trait]
pub trait AuditLedger: Send + Sync {
    async fn append(&self, entry: AuditEntry) -> Result<AuditReceipt, AuditError>;
    async fn inclusion_proof(
        &self,
        position: u64,
        head: &TreeHead,
    ) -> Result<InclusionProof, AuditError>;
    async fn consistency_proof(
        &self,
        from: &TreeHead,
        to: &TreeHead,
    ) -> Result<ConsistencyProof, AuditError>;
    async fn signed_head(&self) -> Result<SignedTreeHead, AuditError>;
    async fn verify_all(&self) -> Result<VerificationReport, AuditError>;
}
```

**`AuditEntry`:**
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub occurred_at: OffsetDateTime,
    pub actor: Actor,              // User | System | Ai | Plugin | Sidecar
    pub action: AuditAction,
    pub subject: SubjectRef,
    pub outcome: Outcome,          // Allowed | Denied | Failed
    pub prev_hash: ContentHash,
    pub detail: RedactedJson,      // secrets never enter; redaction by type, not filter
}
```

**Redaction discipline.** Secrets never enter the ledger. The redaction is applied by type: a `CredentialAdded` event records `(credential_kind, fingerprint)`, never the value. A `LiveExecutionEnabled` event records `(actor, session_id)`, never the API key. This is enforced by the type system — `AuditAction` variants carry only redacted-shape payloads.

**Performance budget:** 1ms p99 append (verified by `criterion` benchmark, Wave 0 DoD criterion 8).

**Backend selection:**
- **Desktop (Wave 0+):** SQLite-backed Merkle log. Synchronous append is on the sensitive path but p99 <1ms on local NVMe.
- **Cloud/Enterprise (Wave 6+):** batched appends with a bounded window; the window is an explicit, documented risk in the threat model.
- **Wave 4+ evaluation:** TigerBeetle (Rust client shipped April 2026) for the audit-as-write-path pattern — purpose-built financial OLTP, Jepsen-passing.

---

### 2.3 `prismatik-manifest`

**Purpose.** Reproducibility manifest builder, signer, and standalone verifier. Closes Gap G01 (the manifest half).

**Workspace dependencies:** `prismatik-determinism`, `prismatik-audit`.

**Public API:**
```rust
pub use manifest::{Manifest, ManifestV1, ManifestBuilder};
pub use verify::{StandaloneVerifier, VerificationReport};
pub use bundle::{ResearchBundle, BundleError};

pub mod manifest;
pub mod verify;
pub mod bundle;
```

The normative manifest schema is `spec/MANIFEST_SCHEMA.md`. This crate implements it.

**Standalone verifier contract:** `prismatik-cli verify <bundle.tar>` exits 0 if and only if:
1. Manifest schema version is supported
2. Every pinned artifact's hash matches its bytes
3. Dual signature verifies against the configured public key set
4. Audit inclusion proof is valid against the manifest's referenced tree head

The verifier MUST NOT link any application code, MUST NOT require a PRISMATIK install, MUST NOT make network calls. Verified by `tests/standalone_verifier_offline.rs`.

---

### 2.4 `prismatik-storage`

**Purpose.** Storage ports and profile-scoped backends. The ONLY crate permitted to depend on SQLx, SQLite, DuckDB, LanceDB bindings.

**Workspace dependencies:** `prismatik-determinism`, `prismatik-identity`.

**Critical rule:** no crate other than `prismatik-application` may depend on `prismatik-storage`. Domain crates depend on storage *ports* (defined in their own crates), and `prismatik-application` wires ports to backends.

**Public API:**
```rust
pub use repository::{Repository, RepositoryError, RepositoryTx};
pub use profile::{StorageProfile, DesktopProfile, CloudProfile, EnterpriseProfile};
pub use backends::{SqliteBackend, DuckdbBackend, LancedbBackend};

pub mod repository;
pub mod profile;
pub mod backends;
pub mod migrations;  // numbered, idempotent, forward-only
```

**Migration discipline:** forward-only, idempotent, numbered `M{N:04}_{name}.sql`. Down-migrations are forbidden. A migration that fails partway MUST be detected at startup and the database quarantined. Schema is specified normatively in `spec/DATA_SCHEMAS.md` §4.

---

## 3. Layer 2 — Domain Crates

Each domain crate owns a bounded part of the business logic and exposes a port trait. They depend on Layers 0–1 only; they MUST NOT depend on storage backends, HTTP clients, or vendor SDKs.

### 3.1 `prismatik-domain`

**Purpose.** Shared domain primitives: `ProviderId`, `DatasetVersionRef`, `DataLayer`, `DataQualityScore`, `CurrencyCode`, common enums.

**Workspace dependencies:** `prismatik-determinism`, `prismatik-identity`.

```rust
pub use provider_id::ProviderId;
pub use layer::DataLayer;  // Raw | Normalized | Curated | Feature | Intelligence | Presentation
pub use quality::DataQualityScore;
pub use currency::CurrencyCode;
pub use dataset_version::{DatasetVersionRef, DatasetVersionId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderId(pub u16);  // registry-assigned; stable across releases

impl ProviderId {
    pub const COINGECKO: ProviderId = ProviderId(1);
    pub const ALPACA: ProviderId = ProviderId(2);
    pub const SEC_EDGAR: ProviderId = ProviderId(3);
    pub const UNUSUAL_WHALES: ProviderId = ProviderId(4);
    pub const FRED: ProviderId = ProviderId(5);
    pub const CFTC: ProviderId = ProviderId(6);
    pub const POLYGON: ProviderId = ProviderId(7);
    pub const CCXT: ProviderId = ProviderId(8);
    pub const TIINGO: ProviderId = ProviderId(9);
    // ... assigned at intake, never renumbered
}
```

### 3.2 `prismatik-market-data`

**Purpose.** Provider port, ingestion pipeline, evidence/lineage plane.

**Workspace dependencies:** `prismatik-domain`, `prismatik-determinism`.

**Public API:**
```rust
pub use provider::{Provider, ProviderCapabilities, EntitlementSet, ProviderHealth};
pub use request::{ProviderRequest, ProviderResponse, CostUnits, EndpointId};
pub use governor::{BudgetGovernor, AdmissionDecision, PriorityClass, BudgetState};
pub use chain::{ProviderChain, AgreementPolicy, DivergenceAction, FailoverTrigger};
pub use evidence::{EvidenceRef, Concludes, BlindSpot, EvidenceError};
pub use lineage::{Lineage, TransformId};

pub mod provider;
pub mod request;
pub mod governor;
pub mod chain;
pub mod evidence;
pub mod lineage;
pub mod ingest;  // PipelineTask, raw-layer writer
```

**`Provider` trait** (normative per v1.0 §16.1):
```rust
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn capabilities(&self) -> ProviderCapabilities;
    fn entitlements(&self) -> &EntitlementSet;
    fn cost_of(&self, request: &ProviderRequest) -> CostUnits;
    async fn health(&self) -> ProviderHealth;
}
```

**`AdmissionDecision`** (normative per v1.0 §16.2):
```rust
#[derive(Clone, Debug)]
pub enum AdmissionDecision {
    Admit { permit: Permit },
    Defer { retry_at: OffsetDateTime, position: usize },  // exact time; UI shows countdown
    BudgetExhausted { resets_at: OffsetDateTime, class: PriorityClass },
    NotEntitled { required: Entitlement },  // local failure, no request made
}
```

### 3.3 `prismatik-features`

**Purpose.** Point-in-time feature store. Closes part of Gap G05.

**Workspace dependencies:** `prismatik-domain`, `prismatik-market-data`.

**Public API:**
```rust
pub use view::{FeatureView, FeatureViewId, FeatureSpec, EntityKind, OnlineStore, OfflineStore};
pub use store::{FeatureStore, FeatureError, EntityTimeFrame, MaterializationReport};
pub use observation::ObservationDelay;  // the highest-value type in this crate

pub mod view;
pub mod store;
pub mod observation;
```

**The `observation_delay` invariant** (normative per v1.0 §15.3): for any view, entity, and instant, NO returned value may have `event_time + observation_delay > as_of`. Verified by property test (`tests/property/pit.rs`, 10k cases).

### 3.4 `prismatik-analog-store`

**Purpose.** LanceDB-backed embedding store for the Historical Analog Engine. Closes Gap G06.

**Workspace dependencies:** `prismatik-domain`, `prismatik-features`, external `lancedb` crate.

**Public API:**
```rust
pub use store::{AnalogStore, AnalogError, EmbeddingBatch, DatasetVersionRef};
pub use query::{AnalogQuery, AnalogResult, AnalogNeighbour, DistanceMetric, AnalogFilter};
pub use disclosures::{SurvivorshipWarning, SensitivityReport};

pub mod store;
pub mod query;
pub mod disclosures;
```

**Mandatory disclosures.** The `AnalogResult` type carries `sample_size`, `filters_applied`, `survivorship_warning`, `leave_n_out_sensitivity` as NON-OPTIONAL fields. There is no constructor that omits them. The UI MUST NOT render neighbours without all four.

### 3.5 `prismatik-indicator-core`

**Purpose.** Canonical indicator trait, warmup enforcement, YATA adapter, golden vectors. The `BarSampler` (Wave 4) lives here.

**Workspace dependencies:** `prismatik-domain`.

**Public API:**
```rust
pub use indicator::{Indicator, IndicatorDescriptor, IndicatorError, WarmupBehavior};
pub use sampler::{BarSampler, BarInterval};
pub use golden_vectors::GoldenVector;

pub mod indicator;
pub mod sampler;
pub mod yata_adapter;
pub mod golden_vectors;
```

**`Indicator` trait:**
```rust
pub trait Indicator: Send + Sync {
    fn descriptor(&self) -> &IndicatorDescriptor;
    fn warmup_len(&self) -> usize;
    fn evaluate(&self, window: &[Bar]) -> Result<f64, IndicatorError>;
    fn next(&mut self, bar: &Bar) -> Result<f64, IndicatorError>;
}
```

**Streaming equals batch property:** `evaluate(window) == next_sequence_over(window)` for every indicator. Property test mandatory (`tests/property/streaming_eq_batch.rs`).

**`BarSampler`** (normative per spec/CRATE_ARCHITECTURE.md §3.5; from AIAlpha, clean-room):
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BarSampler {
    Time(BarInterval),
    Tick { count: usize },
    Volume { threshold: Decimal },
    Dollar { threshold: Decimal },
}
```

### 3.6 `prismatik-quant-kernel`

**Purpose.** Pricing, greeks, IV solve. Selected RustQuant modules.

**Workspace dependencies:** `prismatik-domain`, external `rustquant` selective.

**Public API:**
```rust
pub use pricing::{Price, PricingModel, PricingError};
pub use greeks::{Greeks, Greek};
pub use iv_solve::{ImpliedVolSolver, IvMethod};

pub mod pricing;
pub mod greeks;
pub mod iv_solve;
```

**Conformance:** 1e-8 relative tolerance against QuantLib sidecar oracle across a 500-case grid. CI failure on any divergence.

### 3.7 `prismatik-strategy`

**Purpose.** `StrategyIR` types, the runtime, the `Strategy` trait. The convergence point for visual builder, DSL, Rust SDK, Python emitter.

**Workspace dependencies:** `prismatik-domain`, `prismatik-determinism`, `prismatik-indicator-core`, `prismatik-features`.

**Public API:**
```rust
pub use ir::{StrategyIR, StrategyCapabilities, StrategyIRError, SchemaVersion};
pub use trait_def::{Strategy, StrategyContext, ExecutionContext};
pub use runtime::{StrategyRuntime, RuntimeError};

pub mod ir;
pub mod trait_def;
pub mod runtime;
pub mod dsl;        // lexer (logos), recursive descent parser, type checker, DataFusion plan emitter
pub mod codegen;    // visual builder → StrategyIR; rust SDK → StrategyIR
```

The normative `StrategyIR` schema is `spec/STRATEGY_IR.md`. This crate implements it.

**`Strategy` trait** (normative per v1.0 §18):
```rust
#[async_trait::async_trait]
pub trait Strategy: Send + Sync {
    fn ir(&self) -> &StrategyIR;
    fn capabilities(&self) -> &StrategyCapabilities;
    async fn on_bar(&mut self, ctx: &mut StrategyContext, bar: &Bar) -> Result<Vec<OrderIntent>, RuntimeError>;
    async fn on_event(&mut self, ctx: &mut StrategyContext, event: &MarketEvent) -> Result<Vec<OrderIntent>, RuntimeError>;
}
```

### 3.8 `prismatik-backtest`

**Purpose.** Event-loop backtest engine over pinned-calendar bar boundaries.

**Workspace dependencies:** `prismatik-strategy`, `prismatik-calendar`, `prismatik-features`.

**Public API:**
```rust
pub use engine::{Backtest, BacktestConfig, BacktestResult, BacktestError};
pub use assumptions::{ExecutionAssumptions, FillModel, SlippageModel, CommissionModel};
pub use metrics::{BacktestMetrics, WalkForwardResult, DeflatedSharpe, ProfitFactor};
pub use fill::{Fill, FillKind};

pub mod engine;
pub mod assumptions;
pub mod metrics;
pub mod fill;
```

**Walk-forward discipline** (normative per Wave 3): purged (drop training samples whose label horizon overlaps test) + embargoed (drop a buffer after test). Without these, overlapping labels leak.

### 3.9 `prismatik-simulation`

**Purpose.** Monte Carlo lab with multiple `SimulationSource` variants.

**Workspace dependencies:** `prismatik-backtest`, `prismatik-determinism`.

**Public API:**
```rust
pub use source::{SimulationSource, Bootstrap, BlockBootstrap, Parametric, JumpDiffusion, StochasticVol, RegimeSwitching, TsfmGenerator};
pub use engine::{MonteCarloEngine, PathSet, QuantileBands, SimulationError};
pub use distributions::{TerminalValueDist, DrawdownDist, RuinProb};

pub mod source;
pub mod engine;
pub mod distributions;
```

**Determinism:** split entropy per path. 1M paths under Rayon MUST reproduce byte-identically across runs AND thread counts (1, 4, 16). Wave 3 DoD criterion 7.

### 3.10 `prismatik-tsfm`

**Purpose.** Plural TSFM registry, tokenizer bindings, ONNX runtime adapter.

**Workspace dependencies:** `prismatik-determinism`, `prismatik-manifest` (for `ArtifactRef`).

**Public API:**
```rust
pub use registry::{ModelRegistry, ModelRegistration, ModelFamily, ModelId, RegistryError};
pub use tokenizer::{SeriesTokenizer, TokenizerBinding, TokenizerError};
pub use runtime::{TsfmRuntime, TsfmForecastRequest, TsfmForecastResponse, OutOfDomainVerdict};
pub use pretraining::{PretrainingRecord, ContaminationVerdict};
pub use calibration_link::{CalibrationRecord, CalibrationMethod};  // type re-export from calibration crate

pub mod registry;
pub mod tokenizer;
pub mod runtime;
pub mod pretraining;
pub mod calibration_link;
```

**Registry hard-deny rules** (normative per v1.0 §17.1 and §6.7):
1. CC BY-NC artifact in commercial profile → `RegistryError::NonCommercialArtifact`
2. Tokenizer/model version mismatch → `RegistryError::TokenizerMismatch`
3. Pretraining cutoff ≥ backtest start → `RegistryError::PretrainingContamination` (also logs security event)
4. Signature failure → `RegistryError::SignatureInvalid`

### 3.11 `prismatik-calibration`

**Purpose.** Conformal calibration types and sidecar client. Closes Gap G03.

**Workspace dependencies:** `prismatik-tsfm` (for types), `prismatik-determinism`.

**Public API:**
```rust
pub use trait_def::{Calibrator, CalibrationRecord, CalibrationMethod, CalibrationError, Coverage, RealizedCoverage};
pub use drift::{DriftDetector, DriftSignal, DriftBaseline, DriftAction, DriftAssessment};

pub mod trait_def;
pub mod drift;
pub mod sidecar_client;  // Arrow over local socket to MAPIE/crepes sidecar
```

**Calibrator trait** (normative per v1.0 §17.3):
```rust
pub trait Calibrator: Send + Sync {
    fn method(&self) -> CalibrationMethod;
    fn conformalize(
        &mut self,
        predictions: &PredictionBatch,
        realizations: &RealizationBatch,
        ctx: &DeterminismContext,
    ) -> Result<CalibrationRecord, CalibrationError>;
    fn calibrate(&self, raw: &RawPrediction, target_coverage: Coverage) -> Result<CalibratedPrediction, CalibrationError>;
    fn update(&mut self, observed: &RealizationBatch) -> Result<CoverageDrift, CalibrationError>;
}
```

**ACI is the default for time series** (normative). Standard split conformal restricted to cross-sectional tasks only.

### 3.12 `prismatik-crypto`, `prismatik-options`, `prismatik-filings`, `prismatik-cot`, `prismatik-events`

**Purpose.** Asset-class-specific and event-type-specific domain crates. Each owns its normalized types, parsers, and provider-port extensions.

**Workspace dependencies:** `prismatik-domain`, `prismatik-market-data`.

**Pattern (same for each):**
```rust
// e.g. prismatik-options
pub use types::{OptionContract, OccSymbol, Greeks, Moneyness, OptionType, OptionAdjustmentFlag};
pub use chain::{OptionsChain, ChainExpiry, ChainStrike};
pub use flow::{FlowPrint, FlowClassification, FlowConfidence};

pub mod types;
pub mod chain;
pub mod flow;
```

**`prismatik-options` specifics:**
- `adjusted_contract` HardDeny check wired to corporate-action ledger (v1.0 §12.7, §19.1).
- OCC symbology parser and validator.
- Flow classification carries explicit `FlowConfidence` and an `Unclassified` variant used freely. A classifier that always decides is a classifier that is often wrong.

### 3.13 `prismatik-risk`

**Purpose.** Risk policy, the 16-check pre-trade catalogue, fixed evaluation order.

**Workspace dependencies:** `prismatik-domain`, `prismatik-portfolio`.

**Public API:**
```rust
pub use policy::{RiskPolicy, RiskError};
pub use checks::{PreTradeCheck, CheckId, CheckResult, Severity, CheckContext};
pub use catalog::PreTradeCatalog;
pub use approved::{RiskApproved, RiskApprovedOrderIntent};  // unconstructable outside this module

pub mod policy;
pub mod checks;
pub mod catalog;
pub mod approved;
```

**`RiskApprovedOrderIntent` newtype** (normative per v1.0 §20.1 and v1.2 §3.4): constructible ONLY inside the `prismatik-risk` crate's `approved` module. The OMS (`prismatik-execution`) accepts ONLY this type. "The agent cannot bypass risk" is then a compile-time property rather than a policy statement.

```rust
// In prismatik-risk::approved
pub struct RiskApprovedOrderIntent(OrderIntent);

impl RiskApprovedOrderIntent {
    pub(crate) fn approve(intent: OrderIntent, catalog: &CheckCatalog) -> Result<Self, RiskError> {
        // runs the 16 checks in fixed order; all results recorded including passes
        // ...
    }
}

// In prismatik-execution
pub async fn submit(intent: RiskApprovedOrderIntent) -> Result<SubmissionResult, ...> { /* ... */ }
// There is NO submit(OrderIntent) signature. Compile-time enforcement.
```

**Fixed evaluation order** (normative per v1.0 §19.1): cheapest/most-deterministic first; all HardDeny checks before any SoftWarn. The order is declared in `catalog::PreTradeCatalog::ORDER`. A deny MUST NOT depend on how far evaluation got.

### 3.14 `prismatik-portfolio`

**Purpose.** Portfolio model: positions, lots, cost basis, realized/unrealized PnL. Read model projection from audit ledger (Wave 4+).

**Workspace dependencies:** `prismatik-domain`, `prismatik-identity`.

**Public API:**
```rust
pub use position::{Position, PositionRisk, AvgCostSource, Multiplier};
pub use lot::{Lot, LotId, RealizedLot};
pub use pnl::{RealizedPnl, UnrealizedPnl, PnlError};
pub use projection::{PortfolioProjection, ProjectionError, ReconciliationStatus};

pub mod position;
pub mod lot;
pub mod pnl;
pub mod projection;
```

**`Position` type** (normative per v1.2 §3.8 / clean-room ADR-024):
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub asset_id: AssetId,
    pub currency: CurrencyCode,
    pub side: PositionSide,  // Long | Short
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub market_price: Decimal,
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub multiplier: Multiplier,           // REQUIRED, never implicit 1
    pub avg_cost_source: AvgCostSource,   // Broker | Wallet — REQUIRED, makes synthesized bases visible
    pub risk: Option<PositionRisk>,       // None for spot / non-leveraged; nested, not flattened
}
```

### 3.15 `prismatik-execution`

**Purpose.** Broker gateway, submission, reconciliation, broker error taxonomy.

**Workspace dependencies:** `prismatik-domain`, `prismatik-identity`, `prismatik-risk` (for `RiskApprovedOrderIntent`).

**Public API:**
```rust
pub use gateway::{BrokerGateway, BrokerError, BrokerErrorCode, SubmissionResult, BrokerRejection};
pub use idempotency::{IdempotencyKey, IdempotencyError};
pub use reconcile::{Reconciler, ReconciliationDelta, QuarantineReason};
pub use broker_state::{LocalOrderState, BrokerOrderState, OrderStateError};

pub mod gateway;
pub mod idempotency;
pub mod reconcile;
pub mod broker_state;
pub mod adapters;  // alpaca, ccxt, ibkr, coinbase
```

**`BrokerErrorCode`** (normative per v1.2 §3.8, clean-room from OpenAlice ADR-024):
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerErrorCode {
    Config,        // permanent — disables the account
    Auth,          // permanent — disables the account
    Network,       // transient — auto-recover
    Exchange,      // transient — venue rejected, do not retry blindly
    MarketClosed,  // transient — expected, not a failure
    Connecting,    // data pending, retry shortly — NOT a failure
    Unknown,
}

impl BrokerErrorCode {
    pub fn permanent(self) -> bool {
        matches!(self, BrokerErrorCode::Config | BrokerErrorCode::Auth)
    }
}
```

**Classification order (CRITICAL):** `MarketClosed` MUST be classified before `Auth`. Venues return 403 for both; getting the order wrong disables healthy accounts on routine after-hours reads.

**`SubmissionResult`** (normative per v1.0 §19.2):
```rust
#[derive(Clone, Debug)]
pub enum SubmissionResult {
    Accepted { broker_order_id: BrokerOrderId, accepted_at: OffsetDateTime },
    Rejected { reason: BrokerRejection },
    Unknown { idempotency_key: IdempotencyKey, last_known_state: LocalOrderState },
}
```
**The `Unknown` variant MUST NOT trigger blind retry.** The path: quarantine instrument → reconcile → compare delta → append resolution to ledger → release quarantine only on success.

### 3.16 `prismatik-journal`

**Purpose.** Thesis journal with evidence linkage and the 3-layer memory loop.

**Workspace dependencies:** `prismatik-domain`, `prismatik-market-data` (for `EvidenceRef`).

**Public API:**
```rust
pub use entry::{JournalEntry, EntryId, Thesis, Outcome, OutcomeTag};
pub use memory::{MemoryLoop, MemoryLayer, MemoryRecord, TriggerWinRate};
pub use feedback::{FeedbackSignal, ScoreAdjustment};

pub mod entry;
pub mod memory;
pub mod feedback;
```

**3-layer memory loop** (normative per Wave 4, clean-room from prism-insight):
- Layer 1 (0–7d): detailed records
- Layer 2 (8–30d): `"{sector} + {trigger} → {action} → {result}"`
- Layer 3 (31+d): `"{condition} = {principle}"` with hit-rate stats

---

## 4. Layer 3 — Extension and AI Crates

### 4.1 `prismatik-plugin-host`

**Purpose.** Capability-scoped WASM execution via wasmtime. Closes Gap G04.

**Workspace dependencies:** `prismatik-determinism`, `prismatik-audit`, external `wasmtime`.

**Public API:**
```rust
pub use host::{PluginHost, PluginError, InstalledPlugin, PluginId};
pub use capability::{CapabilitySet, DataScope, HostFunctionId, NetworkScope, PathScope};
pub use limits::{ResourceLimits};
pub use signed::SignedPluginModule;
pub use config::hardened_engine;

pub mod host;
pub mod capability;
pub mod limits;
pub mod signed;
pub mod config;
```

**Mandatory wasmtime config** (normative per v1.0 §20.4 and v1.2 §4.4):
- `Strategy::Cranelift` ONLY. Winch prohibited (RUSTSEC-2026-0095).
- `signals_based_traps(true)` and `guard_before_linear_memory(true)`.
- `consume_fuel(true)` and `epoch_interruption(true)`.
- `wasm_threads(false)`, `wasm_simd(true)`, `wasm_relaxed_simd(false)` (relaxed SIMD permits implementation-defined results; disqualifying).
- `cranelift_nan_canonicalization(true)`.
- Verified by `tests/engine_config_assertion.rs`.

**Capability diffing gate** (normative per v1.2 §4.4): a plugin update that newly imports a network capability MUST fail the gate automatically, not await human review.

### 4.2 `prismatik-ai-tools`

**Purpose.** AI tool gateway, rmcp server/client, the controlled-tool catalogue.

**Workspace dependencies:** `prismatik-domain`, `prismatik-market-data`, `prismatik-portfolio`, external `rmcp`.

**Public API:**
```rust
pub use gateway::{ToolGateway, ToolError, TrustLevel};
pub use tool::{ControlledTool, ToolClass, ToolInvocationRecord};
pub use router::{AiRouter, RouterError, InferenceProvider, AuthMethod, InferenceCapabilities, DeterminismProfile, EgressPolicy};
pub use analyst_scope::{AnalystScope, FactBundle, EvidenceScope, AnalystRole};

pub mod gateway;
pub mod tool;
pub mod router;
pub mod analyst_scope;
pub mod tools;  // the controlled-tool implementations
pub mod mcp;    // rmcp server + client
```

**Tool class taxonomy** (normative per v1.2 §3.4):
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolClass {
    ReadOnly,         // may be invoked freely; metrics only in audit
    RiskReducing,     // agents may take directly (e.g. stop_strategy)
    RiskIncreasing,   // DENY BY DEFAULT; behind approval gate
}
```
Every tool registration MUST declare a class. Risk-increasing tools are deny-by-default.

**`InferenceProvider`** (normative per v1.2 §4.5):
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferenceProvider {
    pub id: ProviderId,
    pub kind: InferenceProviderKind,  // OAuth { .. } | ApiKey | LmStudioLocal | OpenAiCompatible
    pub auth: AuthMethod,             // OAuth { token_source, scopes } | ConsoleApiKey { .. } | Loopback
    pub endpoint: Url,
    pub model_id: String,
    pub model_digest: ContentHash,    // pinned weights; required for reproducibility
    pub context_window: usize,
    pub capabilities: InferenceCapabilities,
    pub determinism: DeterminismProfile,
    pub egress: EgressPolicy,         // Loopback for local — ENFORCED, not assumed
    pub cost_model: CostModel,
    pub max_tokens_per_call: usize,
}
```

**`AnalystScope`** (normative per v1.2 §3.3, clean-room from ai-trading-claude MIT):
```rust
pub struct AnalystScope {
    pub common_facts: FactBundle,        // shared: asset identity, current quote, calendar. Facts, not interpretations.
    pub private_evidence: EvidenceScope, // disjoint per analyst; technical MUST NOT see sentiment's news
}
```

### 4.3 `prismatik-security`

**Purpose.** Key hierarchy, OS keychain integration, capability enforcement for secrets.

**Workspace dependencies:** `prismatik-determinism` (but NOT `Entropy` for key material — uses OS CSPRNG directly).

**Public API:**
```rust
pub use keychain::{OsKeychain, KeychainError};
pub use keys::{DeviceRootKey, DerivedKey, KeyPurpose, SecretRef};
pub use envelope::{Envelope, EnvelopeError};  // AES-GCM envelope encryption
pub use zeroize_secret::ZeroizeSecret;

pub mod keychain;
pub mod keys;
pub mod envelope;
```

**Key hierarchy:**
- `DeviceRootKey` — generated on first run, stored in OS keychain (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux).
- `DerivedKey` — derived from root per `KeyPurpose` (e.g. `KeyPurpose::AuditLedger`, `KeyPurpose::BrokerCredential`, `KeyPurpose::LocalCache`). HKDF-SHA256 with purpose as info.
- All secrets held in `secrecy` wrappers, `zeroize`d on drop.

---

## 5. Layer 4 — Application and Shell

### 5.1 `prismatik-application`

**Purpose.** Composition root. Wires ports to backends, owns the in-process Tokio task graph, persists task state to SQLite.

**Workspace dependencies:** ALL Layer 2 + Layer 3 crates, `prismatik-storage`. **The ONLY crate permitted to depend on `prismatik-storage`.**

**Public API:**
```rust
pub use app::{PrismatikApp, AppConfig, AppError};
pub use task_graph::{TaskGraph, PipelineTask, TaskId, TaskKind, Trigger};
pub use profile::{AppProfile, DesktopApp, CloudApp, EnterpriseApp};

pub mod app;
pub mod task_graph;
pub mod profile;
pub mod wiring;  // dependency injection; ports → concrete backends
```

### 5.2 `prismatik-observability`

**Purpose.** OpenTelemetry setup, determinism telemetry attributes, span taxonomy.

**Workspace dependencies:** `prismatik-determinism`, external `opentelemetry`.

See `spec/OBSERVABILITY.md` for the span catalog and attribute taxonomy.

### 5.3 `prismatik-renderer`

**Purpose.** wgpu in-process renderer for volatility surfaces, Monte Carlo path clouds, correlation matrices. Zero-copy buffer sharing with the core process.

**Workspace dependencies:** `prismatik-domain`, external `wgpu`.

**Mandatory fallback:** CPU + Canvas fallback for every GPU surface. A GPU failure MUST NOT take down the UI.

### 5.4 `prismatik-oss-registry`

**Purpose.** Third-party component manifests, license-class gate, attribution/notice generator.

**Workspace dependencies:** `prismatik-determinism`.

**Public API:**
```rust
pub use manifest::{ThirdPartyComponentManifest, IntegrationMode, ExecutionAllowed};
pub use license::{LicenseClass, LicenseGate, LicenseError};
pub use notice::{NoticeGenerator, NoticeFile};

pub mod manifest;
pub mod license;
pub mod notice;
```

**Modes** (carried from v0.4): A (Direct Dependency), B (Controlled Internal Fork), C (Isolated Sidecar), D (Compatibility Adapter), E (Validation Oracle / Reference), F (Reject).

### 5.5 `prismatik-cli`

**Purpose.** Standalone CLI. Includes the `verify` subcommand that MUST NOT link any application code.

**Workspace dependencies:** `prismatik-manifest` (verify only), `clap`, `tracing`.

```rust
// Subcommands:
// prismatik verify <bundle.tar>      — standalone research-bundle verifier
// prismatik manifest inspect <id>    — manifest inspection
// prismatik audit verify             — startup audit-chain verification
// prismatik calendar generate        — calendar artifact generator (release-time tool)
```

---

## 6. The `apps/`, `packages/`, `services/`, `artifacts/` Trees

### 6.1 `apps/`
- `apps/desktop/` — Tauri 2 (≥2.12 per CVE-2026-42184) + SvelteKit. The primary shell.
- `apps/web/` — optional browser companion.
- `apps/admin/` — enterprise operations console (Wave 6+).

### 6.2 `packages/`
- `packages/ui/` — Svelte component library (see `spec/DESIGN_SYSTEM.md`).
- `packages/design-tokens/` — CSS variables, light/dark themes.
- `packages/api-client/` — generated by tauri-specta. **NEVER hand-edited.** Drift is a CI failure.
- `packages/schemas/` — JSON Schema definitions published under Apache-2.0.
- `packages/chart-contracts/` — `ChartDocument`, `ChartBackend` contracts (Apache-2.0).

### 6.3 `services/`
- `services/ingest/` — ingestion worker (separate process; crash isolation for malformed payloads).
- `services/quant-worker/` — long-running CPU-bound jobs (separate process; isolation prevents starvation of UI).
- `services/notification/` — delivery channel abstraction.
- `services/cloud-api/` — team cloud / enterprise API.
- `services/sidecars/`:
  - `ccxt-gateway/` — Python; no credentials, no data-plane writes, output treated as provider data.
  - `quantlib-oracle/` — Python; conformance oracle.
  - `qlib-research/` — Python; research bridge.
  - `calibration/` — Python; MAPIE ACI/EnbPI + crepes Mondrian.
  - `kronos-tsfm/` — Python; isolated TSFM worker.
  - `lm-studio/` — external; local inference (the LM Studio app).

### 6.4 `artifacts/`
- `artifacts/calendars/` — content-addressed, signed calendar artifacts.
- `artifacts/codebooks/` — tokenizer codebooks.
- `artifacts/models/` — model weights.
- `artifacts/manifests/` — signed manifest corpus (golden + production).

---

## 7. CI-Enforced Workspace Rules

These rules are checked by `scripts/check_workspace_graph.py` and run on every CI build. Failure is blocking.

| Rule | Check |
|---|---|
| Layer 0 independence | `prismatik-determinism` and `prismatik-identity` have no workspace deps |
| Storage isolation | Only `prismatik-application` depends on `prismatik-storage` |
| Domain purity | No domain crate depends on `sqlx`, `reqwest`, or vendor SDKs |
| API surface stability | Public API diffs require a `api-change` label on the PR |
| License allowlist | `deny.toml` passes for all workspace crates |
| Determinism grep | No ambient time/entropy/hashmap outside the determinism crate's allowlist |

---

## 8. Cross-References

| Topic | Document |
|---|---|
| Parquet/LanceDB/SQLite schemas | `spec/DATA_SCHEMAS.md` |
| Tauri command/event catalog | `spec/IPC_CONTRACTS.md` |
| `StrategyIR` JSON schema and AST | `spec/STRATEGY_IR.md` |
| Reproducibility manifest format | `spec/MANIFEST_SCHEMA.md` |
| Provider endpoint/auth/cassette specs | `spec/PROVIDER_ADAPTERS.md` |
| Property test catalog | `spec/TESTING.md` |
| CI workflow definitions | `spec/CI_WORKFLOWS.md` |
| Threat model | `spec/SECURITY_THREAT_MODEL.md` |
| Design system / component library | `spec/DESIGN_SYSTEM.md` |
| Observability spans and attributes | `spec/OBSERVABILITY.md` |
| AI router dispatch and replay | `spec/AI_ROUTER_INTERNALS.md` |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
