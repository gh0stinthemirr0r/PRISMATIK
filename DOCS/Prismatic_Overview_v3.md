# PRISMATIK — Architecture Evolution v0.2

**Document:** `PRISMATIK_Evolution.md`
**Companion to:** `PRISMATIK_Overview.md` v0.1
**Date:** 2026-07-13
**Author:** Aaron Stovall
**Purpose:** Fill the load-bearing architectural gaps identified in the v0.1 overview with build-ready specifications. This document does not repeat what v0.1 already covers — it adds what is missing.

---

## How to Use This Document

Each section maps to a gap in the v0.1 overview. They are ordered by implementation leverage, not document order. The Rust trait definitions in Part I are the single highest-priority addition — they are the actual architectural contract that every workstream in v0.1 §33 depends on.

---

# Part I — Load-Bearing Trait Surfaces

> The v0.1 overview defines one Rust trait (`MarketDataProvider`) out of approximately twelve load-bearing interfaces. This part defines the rest. Each trait is written to be reviewable as a standalone ADR.

## 1. Strategy and Strategy Runtime

The v0.1 overview (§11.1) mandates a typed DSL that "MUST compile to a typed intermediate representation" but never defines the IR or the trait a strategy must implement to be runnable by the backtest engine, paper broker, and live execution boundary. This is the contract that makes a strategy portable across all three runtimes.

### 1.1 Strategy Intermediate Representation

```rust
/// A compiled, type-checked strategy in intermediate representation form.
/// Produced by the DSL compiler, the visual builder codegen, or hand-written
/// Rust strategies via the SDK. All strategies — regardless of origin —
/// converge to this type before they touch a runtime.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyIR {
    pub id: StrategyId,
    pub name: String,
    pub version: SemanticVersion,
    pub universe: UniverseSpec,
    pub entry: Vec<EntryRule>,
    pub exit: Vec<ExitRule>,
    pub risk: RiskBinding,
    pub parameters: ParameterSet,
    pub capabilities: StrategyCapabilities,
    pub source_hash: ContentHash,
    pub compiled_at: OffsetDateTime,
    pub compiler_version: SemanticVersion,
}

/// What a strategy is allowed to do. The runtime enforces these.
/// A backtest strategy gets data access; a live strategy additionally
/// gets order draft access — but never direct submit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyCapabilities {
    pub data_access: DataAccessScope,
    pub can_read_options: bool,
    pub can_read_filings: bool,
    pub can_read_crypto: bool,
    pub can_draft_orders: bool,
    pub can_access_network: bool,  // always false in backtest
    pub max_position_concentration: Option<Ratio>,
    pub max_premium_at_risk: Option<Ratio>,
}
```

### 1.2 The Strategy Trait

```rust
/// The core trait every strategy implements, regardless of authoring mode.
/// The runtime calls `on_event` with a deterministic clock and scoped data.
/// Strategies are stateful but must be reproducible: same events + same
/// seed = same decisions.
#[async_trait::async_trait]
pub trait Strategy: Send + Sync {
    /// Unique identity for this strategy instance.
    fn id(&self) -> StrategyId;

    /// The compiled IR this instance was created from.
    fn ir(&self) -> &StrategyIR;

    /// Called once at the start of a run (backtest, paper, or live).
    /// Provides the deterministic clock, scoped data reader, and seed.
    async fn initialize(
        &mut self,
        ctx: &StrategyContext,
    ) -> Result<(), StrategyError>;

    /// Called for every event the strategy's universe produces.
    /// Market data, option prints, filings, and scheduled triggers
    /// all arrive as typed events. The strategy returns zero or more
    /// order intents — never orders directly.
    async fn on_event(
        &mut self,
        event: &MarketEvent,
        ctx: &StrategyContext,
    ) -> Result<Vec<OrderIntent>, StrategyError>;

    /// Called once at the end of a run. Allows the strategy to
    /// emit summary metadata, close state, and record final
    /// decisions for the journal and audit log.
    async fn finalize(
        &mut self,
        ctx: &StrategyContext,
    ) -> Result<StrategySummary, StrategyError>;
}

/// Scoped context passed to every strategy call. Prevents direct
/// access to the network, the broker, or data outside the strategy's
/// declared universe and capabilities.
pub struct StrategyContext {
    clock: DeterministicClock,
    data: ScopedDataReader,
    rng: SeededRng,
    run_id: RunId,
    manifest: ManifestBuilder,  // accumulates reproducibility metadata
}
```

### 1.3 Why This Design

- **One trait, three runtimes.** The backtest engine, paper broker, and live execution boundary all call `on_event`. The difference is what the `StrategyContext` permits and what happens to the returned `OrderIntent` — in backtest it is simulated, in paper it is recorded against the paper broker, in live it enters the deterministic execution workflow (v0.1 §17.5).
- **Capabilities are structural, not advisory.** `can_access_network` is enforced by the runtime, not by convention. A backtest cannot accidentally call a live API.
- **`OrderIntent`, not `Order`.** Strategies never produce orders. They produce intents that pass through risk checks (§3 below), user approval, and the execution boundary before becoming orders. This is the architectural enforcement of v0.1 §16.2: "Live order submission MUST not be directly available to a general conversational model."

---

## 2. Risk Policy and Pre-Trade Checks

The v0.1 overview (§17.3) lists 13 pre-trade checks but defines no trait, no evaluation model, and no fail-closed return type. Risk enforcement must be structural — a strategy or AI tool cannot bypass it by construction.

### 2.1 Risk Trait

```rust
/// A risk policy is a collection of checks evaluated before any order
/// leaves the system. Checks are short-circuiting: the first hard deny
/// stops evaluation. All checks (including those that passed) are
/// recorded in the audit log.
pub trait RiskPolicy: Send + Sync {
    /// Evaluate an order intent against all applicable checks.
    /// Returns a decision with the full evidence chain.
    fn evaluate(
        &self,
        intent: &OrderIntent,
        portfolio: &PortfolioState,
        ctx: &RiskContext,
    ) -> RiskDecision;
}

/// The decision returned by risk evaluation. Never silent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RiskDecision {
    /// All checks passed. The intent may proceed to the execution boundary.
    Allow {
        checks_evaluated: Vec<CheckResult>,
    },
    /// A soft warning fired but did not block. The intent proceeds,
    /// but the warning is recorded and surfaced to the user.
    Warn {
        warning: CheckWarning,
        checks_evaluated: Vec<CheckResult>,
    },
    /// A hard deny. The intent is blocked. No further processing.
    /// The reason, the check that triggered, and the portfolio state
    /// at evaluation time are all captured for audit.
    Deny {
        reason: DenyReason,
        blocking_check: CheckId,
        portfolio_snapshot: PortfolioSnapshot,
        checks_evaluated: Vec<CheckResult>,
    },
}

/// Individual pre-trade check. Each is a pure function of
/// (intent, portfolio, context) -> CheckResult.
pub trait PreTradeCheck: Send + Sync {
    fn id(&self) -> CheckId;
    fn severity(&self) -> CheckSeverity;  // HardDeny | SoftWarn
    fn evaluate(
        &self,
        intent: &OrderIntent,
        portfolio: &PortfolioState,
        ctx: &RiskContext,
    ) -> CheckResult;
}
```

### 2.2 Check Catalog

Mapping the 13 checks from v0.1 §17.3 to concrete implementations:

| Check ID | Severity | Rule |
|---|---|---|
| `max_position` | HardDeny | Position notional > `RiskPolicy.max_position` |
| `max_premium` | HardDeny | Premium at risk > `RiskPolicy.max_premium_pct` of account |
| `max_account_loss` | HardDeny | Realized + unrealized loss > daily limit |
| `sector_concentration` | HardDeny | Sector exposure > `RiskPolicy.max_sector_pct` |
| `correlation_cluster` | SoftWarn | Adding position raises portfolio correlation > threshold |
| `event_proximity` | SoftWarn | Earnings/macro event within N hours |
| `spread_width` | SoftWarn | Bid-ask spread > `RiskPolicy.max_spread_pct` |
| `liquidity_volume` | SoftWarn | 30-day avg volume < threshold |
| `open_interest` | SoftWarn | Open interest < threshold |
| `account_permissions` | HardDeny | Account not enabled for this asset/strategy type |
| `market_status` | HardDeny | Market closed, halted, or in pre/post with no permission |
| `stale_data` | HardDeny | Quote/filing/IV data older than `RiskPolicy.max_data_age` |
| `duplicate_order` | HardDeny | Idempotency key collision or same-contract intent within window |

---

## 3. Broker Gateway and Execution Boundary

The v0.1 overview (§17.5, §39.2) mandates idempotency keys, reconciliation, and an isolated execution boundary, but defines no gateway trait. This is the contract between PRISMATIK and any broker or exchange.

```rust
/// The execution gateway abstraction. Broker adapters implement this.
/// The execution service wraps every call with idempotency, audit,
/// reconciliation, and the step-up auth gate from v0.1 §17.5.
#[async_trait::async_trait]
pub trait BrokerGateway: Send + Sync {
    fn broker_id(&self) -> BrokerId;
    fn capabilities(&self) -> BrokerCapabilities;

    /// Submit an order that has passed risk checks and user approval.
    /// Uses the idempotency key from the OrderIntent — a retry with
    /// the same key returns the original result, never creates a duplicate.
    async fn submit(
        &self,
        order: ApprovedOrder,
    ) -> Result<SubmissionResult, BrokerError>;

    /// Cancel a working order. Must be idempotent.
    async fn cancel(
        &self,
        order_id: BrokerOrderId,
    ) -> Result<CancelResult, BrokerError>;

    /// Replace a working order (modify quantity, price, or stop).
    /// Not all brokers support this. Check capabilities first.
    async fn replace(
        &self,
        order_id: BrokerOrderId,
        modification: OrderModification,
    ) -> Result<ReplaceResult, BrokerError>;

    /// Reconcile local state with broker state. Called after
    /// timeouts, unknown results, disconnections, and periodically
    /// as a safety net. Returns the delta between local belief
    /// and broker truth.
    async fn reconcile(
        &self,
        local_state: &LocalOrderState,
    ) -> Result<ReconciliationDelta, BrokerError>;

    /// Stream fills and order state changes.
    async fn stream_events(
        &self,
    ) -> BoxStream<'static, BrokerEvent>;
}
```

---

## 4. AI Tool Gateway

The v0.1 overview (§16.2, §31.5) lists 11 controlled tools and 10 gateway enforcement points, but the dispatch trait is undefined. This is the boundary between the conversational AI and the platform's controlled surface area.

```rust
/// The AI tool gateway. Every tool the AI can invoke passes through
/// this trait. The gateway enforces schema validation, rate limits,
/// approval hooks, output filtering, and audit logging — not the tool.
#[async_trait::async_trait]
pub trait ToolGateway: Send + Sync {
    /// Dispatch a tool call from the AI. The gateway validates
    /// the input schema, checks rate limits, optionally requests
    /// human approval, executes the tool, filters the output,
    /// and returns a controlled result.
    async fn dispatch(
        &self,
        call: ToolCall,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError>;

    /// List the tools available to the current session,
    /// respecting trust level and capability scope.
    fn available_tools(&self, trust: TrustLevel) -> Vec<ToolDescriptor>;
}

/// A single tool in the gateway registry.
pub trait ControlledTool: Send + Sync {
    fn descriptor(&self) -> ToolDescriptor;
    fn trust_level(&self) -> TrustLevel;  // ReadOnly | DraftOnly | Sensitive
    fn requires_approval(&self) -> bool;

    async fn execute(
        &self,
        input: ToolInput,
        ctx: &ToolContext,
    ) -> Result<ToolOutput, ToolError>;
}
```

### Tool Catalog with Trust Levels

| Tool | Trust Level | Approval? | Maps to v0.1 §16.2 |
|---|---|---|---|
| `market_query` | ReadOnly | No | Read-only market query |
| `filing_query` | ReadOnly | No | Filing query |
| `cot_query` | ReadOnly | No | COT query |
| `backtest_request` | ReadOnly | No | Backtest request |
| `simulation_request` | ReadOnly | No | Simulation request |
| `chart_snapshot` | ReadOnly | No | Chart snapshot |
| `portfolio_analysis` | ReadOnly | No | Portfolio analysis |
| `journal_search` | ReadOnly | No | Journal search |
| `draft_alert` | DraftOnly | No | Draft alert |
| `draft_strategy` | DraftOnly | No | Draft strategy |
| `draft_order` | Sensitive | Yes — user must approve | Draft order |

Live order submission is intentionally absent from this table. It is not a tool. It is a deterministic workflow (v0.1 §17.5) that the `draft_order` tool feeds into, but the AI never calls submit directly.

---

## 5. Alert Rule Evaluator

The v0.1 overview (§8.8) lists 12 alert targets and 9 delivery channels but defines no rule schema, no evaluator trait, and no streaming-vs-polling boundary.

```rust
/// An alert rule. Compiled from the rule DSL into a typed, evaluable form.
/// Rules are either streaming (evaluated on every event) or scheduled
/// (evaluated on a cron-like trigger).
pub trait AlertRule: Send + Sync {
    fn id(&self) -> AlertRuleId;
    fn evaluation_mode(&self) -> EvaluationMode;  // Streaming | Scheduled
    fn dedup_key(&self, event: &MarketEvent) -> Option<DedupKey>;

    /// Evaluate against the current state. Returns Some(AlertFiring)
    /// if the condition is met, None otherwise.
    fn evaluate(&self, state: &AlertContext) -> Option<AlertFiring>;
}

pub enum EvaluationMode {
    /// Evaluated on every incoming market event. Used for price,
    /// volume, IV, and flow conditions.
    Streaming,
    /// Evaluated on a schedule. Used for filings, COT releases,
    /// and daily risk summaries.
    Scheduled(Schedule),
}
```

### Streaming vs Scheduled Boundary

| Condition | Mode | Rationale |
|---|---|---|
| Price threshold | Streaming | Sub-second relevance |
| Technical indicator | Streaming | Recomputed on every tick/bar |
| Volume / relative volume | Streaming | Intraday signal |
| Options flow | Streaming | Print-level detection |
| Implied volatility | Streaming | Updated with chain refresh |
| Filing arrival | Scheduled | SEC RSS poll or webhook |
| COT extreme | Scheduled | Weekly data, no intraday value |
| Event proximity | Scheduled | Calendar-driven |
| Portfolio risk threshold | Scheduled | Recompute on fill + periodic |

---

## 6. Repository Port Pattern

The v0.1 overview (§18.3) lists "Persistence" as a Rust responsibility but defines no repository traits, leaving domain crates coupled to concrete stores. This is the hexagonal port that keeps the domain pure.

```rust
/// Generic repository port. Domain crates depend on this trait,
/// never on SQLx or a concrete store. The application layer wires
/// concrete implementations (PostgresUserRepository, SqliteUserRepository).
#[async_trait::async_trait]
pub trait Repository<T: Aggregate>: Send + Sync {
    async fn get(&self, id: &T::Id) -> Result<Option<T>, RepoError>;
    async fn save(&self, aggregate: &T) -> Result<(), RepoError>;
    async fn delete(&self, id: &T::Id) -> Result<(), RepoError>;
    async fn exists(&self, id: &T::Id) -> Result<bool, RepoError>;
}

/// Marker trait for domain aggregates that can be persisted.
pub trait Aggregate: Send + Sync + Clone {
    type Id: Clone + Send + Sync;
    fn id(&self) -> Self::Id;
    fn version(&self) -> AggregateVersion;  // optimistic concurrency
}
```

---

# Part II — Pipeline Orchestration and Lineage

> The v0.1 overview (§10.1, §22.1) defines six data layers and lists bus candidates, but never specifies the orchestrator, lineage schema, invalidation model, or freshness targets. This part fills those gaps.

## 7. Orchestrator Architecture

### 7.1 Design Decision

For the desktop deployment (Phase 0–1), use an **in-process Tokio task graph**. No external orchestrator (Temporal, Airflow) is needed for a single-user local-first application. The task graph persists its state to SQLite so it survives restarts.

For the enterprise deployment (Phase 8), evaluate **Temporal** for durable cross-service workflows, keeping the same `TaskRunner` trait so the domain logic is portable.

### 7.2 Core Types

```rust
/// A pipeline task. Idempotent: running it twice with the same inputs
/// produces the same result (or a no-op if already complete).
#[async_trait::async_trait]
pub trait PipelineTask: Send + Sync {
    fn id(&self) -> TaskId;
    fn task_type(&self) -> TaskType;
    fn input_hash(&self) -> ContentHash;

    async fn run(&self, ctx: &PipelineContext) -> Result<TaskOutput, TaskError>;

    /// Whether this task can be safely retried after a partial failure.
    fn idempotent(&self) -> bool { true }
}

/// The task graph. Tasks declare their dependencies; the scheduler
/// resolves execution order. State persists to SQLite for crash recovery.
pub struct TaskGraph {
    tasks: Vec<TaskNode>,
    edges: Vec<(TaskId, TaskId)>,  // (dependency, dependent)
}

pub struct TaskNode {
    task: Box<dyn PipelineTask>,
    trigger: Trigger,
    status: TaskStatus,
    last_run: Option<OffsetDateTime>,
    last_output: Option<TaskOutput>,
}

pub enum Trigger {
    /// Runs on a schedule (cron-like).
    Scheduled(Schedule),
    /// Runs when an upstream task completes.
    Dependency(TaskId),
    /// Runs when new data arrives from a provider.
    DataEvent(ProviderId, EventPattern),
    /// Runs on explicit user request.
    OnDemand,
}
```

### 7.3 Task Types Mapped to v0.1 Layers

| Task Type | Input Layer | Output Layer | Example |
|---|---|---|---|
| `Ingest` | (external) | Raw | Fetch CoinGecko markets snapshot |
| `Normalize` | Raw | Normalized | Parse CoinGecko JSON → canonical `MarketSnapshot` |
| `QualityCheck` | Normalized | Curated | Score freshness, detect outliers, deduplicate |
| `FeatureCompute` | Curated | Feature | Compute RSI, IV rank, relative volume |
| `IntelligenceCompute` | Feature | Intelligence | Score opportunity, run model prediction |
| `Materialize` | Curated/Feature | Presentation | Build user-specific watchlist view |
| `Backfill` | (external) | Raw | Fetch historical OHLC for a new asset |

---

## 8. Lineage Schema

Every record in the curated, feature, and intelligence layers carries lineage back to its raw sources. This is the enforcement of v0.1 §4.1 ("Evidence Before Conclusion") at the data level.

```rust
/// Attached to every curated, feature, and intelligence record.
/// Allows tracing any derived value back to its raw source(s).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lineage {
    /// The raw record IDs this record was computed from.
    pub upstream_record_ids: Vec<RecordId>,
    /// The transform that produced this record.
    pub transform_id: TransformId,
    /// The version of the transform code.
    pub transform_version: SemanticVersion,
    /// When this record was computed.
    pub computed_at: OffsetDateTime,
    /// Hash of all upstream record IDs + transform + version.
    /// If any input changes, this hash changes, and the record
    /// is considered stale.
    pub inputs_hash: ContentHash,
}
```

---

## 9. Invalidation Model

When a raw record is corrected (e.g., a restated 13F, a corrected macro series, a CoinGecko metadata fix), the system must determine which downstream records are affected and whether they need recomputation.

### 9.1 Invalidation Propagation

```
Raw record corrected
    │
    ▼
Find all curated records with this raw ID in upstream_record_ids
    │
    ▼
Mark those curated records as STALE (do not delete — preserve for audit)
    │
    ▼
Find all feature records depending on those curated records
    │
    ▼
Mark feature records as STALE
    │
    ▼
Find all intelligence records (opportunities, predictions) depending on those features
    │
    ▼
Mark intelligence records as STALE
    │
    ▼
Re-trigger the pipeline tasks that produced the stale records
    │
    ▼
New records created with new lineage; old records retained for audit
```

### 9.2 Stale-vs-Source vs Recompute

Not every correction requires recomputation:

| Correction Type | Action | Reason |
|---|---|---|
| Metadata fix (name, description) | Recompute downstream | Low cost, high correctness value |
| Price/bar correction | Recompute + flag affected backtests | Critical — backtest results may change |
| Filing amendment | Recompute intelligence | Opportunities may be invalidated |
| COT revision | Recompute positioning metrics | Crowding scores change |
| Delayed-but-unchanged data | No action | Same data, later arrival — no invalidation |

---

## 10. Per-Layer Freshness Targets

The v0.1 overview (§26.3) lists "Critical provider freshness" as an SLO but does not map targets per dataset. These are the targets for the Phase 0/1 data set:

| Dataset | Layer | Target Freshness | Degradation Behavior |
|---|---|---|---|
| CoinGecko market snapshot (prices, cap, volume) | Raw | ≤ 60 seconds | Show "delayed" badge at 120s; stop scoring at 300s |
| CoinGecko trending | Raw | ≤ 5 minutes | Acceptable to be stale; no degradation |
| CoinGecko metadata | Raw | ≤ 24 hours | Very low change rate; no degradation |
| CoinGecko OHLC (historical) | Raw | Immutable after finalization | N/A — never goes stale |
| SEC filings RSS | Raw | ≤ 5 minutes after publication | Critical for event trading |
| CFTC COT | Raw | ≤ 1 day after Tuesday release | Weekly data; no intraday value |
| FRED macro | Raw | ≤ 1 day after release | Quarterly/monthly cadence |
| Normalized market snapshot | Normalized | ≤ 5 seconds behind raw | Pipeline must be fast |
| Feature: RSI / indicators | Feature | ≤ 10 seconds behind normalized | Recomputed on each bar close |
| Feature: IV rank | Feature | ≤ 30 seconds behind normalized | Depends on chain refresh |
| Intelligence: opportunity score | Intelligence | ≤ 30 seconds behind feature | Recomputed when inputs change |

---

# Part III — Provider Cost Model

> The v0.1 overview (§10.5) lists what the budget governor MUST do but defines no cost unit, no budget policy, and no UI. This part makes it concrete.

## 11. Cost Unit and Budget Architecture

### 11.1 Cost Units

Providers charge in different units. The governor normalizes all of them to a common `CostUnit` for tracking, while preserving the native unit for display.

```rust
/// The native cost unit of a provider.
pub enum CostUnit {
    /// API calls (SEC EDGAR, FRED — free but rate-limited).
    ApiCalls,
    /// Credits (CoinGecko paid tiers — credits per endpoint).
    Credits,
    /// Plan-based (Unusual Whales — flat plan, soft rate limits).
    PlanIncluded,
    /// Dollars (paid per-call providers in later phases).
    Dollars,
}

/// A single provider's cost model.
pub struct ProviderCostModel {
    pub provider_id: ProviderId,
    pub unit: CostUnit,
    pub period: BudgetPeriod,           // Hourly | Daily | Monthly
    pub budget: u64,                     // calls, credits, or cents
    pub remaining: u64,
    pub reservation_pool: u64,           // reserved for critical workflows
    pub endpoints: Vec<EndpointWeight>,  // credit weight per endpoint
}

pub struct EndpointWeight {
    pub endpoint: EndpointPath,
    pub weight: u32,   // e.g., CoinGecko /coins/{id} = 1 credit, /coins/markets = 1, historical = 5
}
```

### 11.2 Budget Policy and Priority Classes

```rust
/// Workflows are classified by priority. When the budget is under
/// pressure, lower-priority requests are shed first.
pub enum PriorityClass {
    /// User-facing dashboard refresh, alert evaluation, live execution.
    /// These always run, even if they exhaust the budget.
    Critical,
    /// Watchlist updates, scanner runs, chart data.
    /// Run unless budget is below 20% remaining.
    Standard,
    /// Background data refresh, historical backfill, model retraining.
    /// Run only when budget is above 50% remaining.
    Background,
    /// Prefetch, warm cache, pre-compute.
    /// Run only when budget is above 80% remaining.
    Opportunistic,
}
```

### 11.3 Rate-Budget UI Specification

The Phase 0 exit criteria require a "Provider health and rate-budget UI." This is what it shows:

**Panel 1 — Provider Health Card (one per connected provider):**
- Provider name and plan tier
- Current status: Healthy / Degraded / Rate-Limited / Down
- Requests in current period / budget
- Credits remaining (for credit-based providers)
- Average latency (p50, p95)
- Error rate
- Last successful request timestamp
- Next reset time

**Panel 2 — Spend Trend:**
- Sparkline of request/credit consumption over the current period
- Projected burn rate (linear extrapolation)
- "At current rate, budget exhausted in X hours" warning

**Panel 3 — Priority Breakdown:**
- Bar chart of requests by PriorityClass (Critical / Standard / Background / Opportunistic)
- Shed count (how many Background/Opportunistic requests were rejected)

**Panel 4 — Endpoint Breakdown (expandable per provider):**
- Table: endpoint path, call count, credit weight, total credits consumed

**Controls:**
- Per-provider pause/resume toggle
- Per-provider budget override (with confirmation)
- "Reserve N credits for critical workflows" slider

---

# Part IV — First-Run, Demo, and Suitability

> The v0.1 overview has no onboarding architecture despite Phase 0 requiring "CoinGecko demo integration" and §17.3 requiring "user-specific suitability filters."

## 12. First-Run Experience

### 12.1 Flow State Machine

```
[Launch]
    │
    ▼
[Welcome Screen]
    │
    ├── "Get Started" ─────────────────▶ [Provider Setup]
    │                                      │
    │                                      ├── Enter CoinGecko key (optional)
    │                                      │   (demo key pre-filled; user can upgrade later)
    │                                      │
    │                                      └── "Skip — use demo mode" ──▶ [Demo Mode Active]
    │
    ▼
[Suitability Profile]
    │
    ├── Experience level: Novice / Intermediate / Advanced / Professional
    ├── Risk tolerance: Conservative / Moderate / Aggressive
    ├── Jurisdiction: Country + State (for feature gating)
    ├── Primary asset interest: Crypto / Equities / Options / Multi-asset
    └── Time horizon: Scalping / Day / Swing / Position / Long-term
    │
    ▼
[Default Workspace Seeded]
    │
    ├── Watchlist: BTC, ETH, SPY, QQQ (+ top 10 by user interest)
    ├── Sample strategy: "EventFlowMomentum" (read-only, for learning)
    ├── Risk policy: Conservative defaults (max 2% per position, max 15% sector)
    └── Layout: "First-Run" named layout with Command Center + one chart + watchlist
    │
    ▼
[Guided Tour (skippable)]
    │
    ├── "This is your Command Center. Cards show..."
    ├── "Every card links to its evidence. Click any source."
    ├── "Probabilities are ranges, not predictions. See how..."
    └── "Your risk limits are set. Here's how to change them."
    │
    ▼
[Ready — Main Application]
```

### 12.2 Demo Mode Architecture

Demo mode allows full exploration without API keys. It is not a separate app — it is a degraded-capability mode that is transparent to the user.

```rust
pub struct DemoMode {
    /// Pre-recorded provider responses, replayed deterministically.
    /// Recorded from CoinGecko demo endpoints with a known timestamp.
    fixtures: FixtureStore,
    /// The "simulation clock" — demo data appears to advance in real-time
    /// but is actually replayed from recorded snapshots at 1-minute granularity.
    sim_clock: DemoClock,
    /// Watermark shown on all data: "DEMO — Data as of 2026-07-10"
    watermark: DemoWatermark,
    /// Features disabled in demo: live execution, real broker sync, alerts to SMS.
    disabled_features: Vec<FeatureGate>,
}
```

**Demo mode rules:**
- Market data shows a watermark on every chart and table: "DEMO — Data as of [date]"
- No data is older than 7 days at time of recording
- Alerts fire against demo data but cannot deliver to SMS, email, or webhooks
- Paper trading works fully against demo data
- The user can enter real API keys at any time to transition out of demo mode seamlessly
- No real credentials are ever stored during demo mode

### 12.3 Suitability Profile Aggregate

```rust
/// Captured during first-run and editable in settings.
/// Consumed by the scanner (filters out unsuitable strategies),
/// the opportunity engine (ranks by user fit), and the compliance
/// layer (gates features by jurisdiction and experience).
pub struct SuitabilityProfile {
    pub experience_level: ExperienceLevel,
    pub risk_tolerance: RiskTolerance,
    pub jurisdiction: Jurisdiction,
    pub primary_interests: Vec<AssetClass>,
    pub time_horizon: TimeHorizon,
    pub acknowledged_disclosures: Vec<DisclosureAcknowledgment>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

---

# Part V — Notification Service

> The v0.1 overview (§8.8) lists alert targets and delivery channels with no service architecture.

## 13. Notification Service Design

```
┌─────────────────────────────────────────────────────────┐
│                    Alert Rules                           │
│  (Streaming evaluator + Scheduled evaluator)             │
└──────────────────────────┬──────────────────────────────┘
                           │ AlertFiring
                           ▼
┌─────────────────────────────────────────────────────────┐
│                  Dedup Window                            │
│  Key = (rule_id, entity, direction, severity)            │
│  If duplicate within window → suppress                   │
│  If same entity, higher severity → escalate              │
└──────────────────────────┬──────────────────────────────┘
                           │ Unique AlertFiring
                           ▼
┌─────────────────────────────────────────────────────────┐
│                Attention Governor                        │
│  Prioritizes based on: portfolio exposure, watchlist     │
│  relevance, strategy fit, event proximity, severity,     │
│  novelty, historical usefulness.                         │
│  Limits repeated low-value alerts.                       │
└──────────────────────────┬──────────────────────────────┘
                           │ Governed AlertFiring
                           ▼
┌─────────────────────────────────────────────────────────┐
│                  Delivery Router                         │
│  Routes to user-preferred channels by severity and time. │
│  Quiet hours suppress non-critical.                      │
│  Critical alerts bypass quiet hours.                     │
└──────┬──────┬──────┬──────┬──────┬──────┬──────┬────────┘
       │      │      │      │      │      │      │
       ▼      ▼      ▼      ▼      ▼      ▼      ▼
    Desktop  Mobile  Email  SMS   Webhook Slack  PagerDuty
    Toast   Push                    /Teams
```

### 13.1 Delivery Channel Abstraction

```rust
#[async_trait::async_trait]
pub trait DeliveryChannel: Send + Sync {
    fn channel_id(&self) -> ChannelId;
    fn min_severity(&self) -> AlertSeverity;

    async fn deliver(
        &self,
        alert: &AlertFiring,
        user_prefs: &NotificationPreferences,
    ) -> Result<DeliveryReceipt, DeliveryError>;

    /// Whether this channel is available (has credentials, is connected).
    async fn is_available(&self) -> bool;
}
```

### 13.2 Dedup and Escalation

| Scenario | Action |
|---|---|
| Same rule fires for same entity within dedup window | Suppress (count incremented on original) |
| Same entity, higher severity than existing alert | Escalate (replace original, notify) |
| Rule fires for entity already alerted → condition clears → refires | New alert (dedup window reset on clear) |
| User dismisses alert; condition persists | Snooze for user-set duration, then re-fire if still active |
| User disables rule | No alerts fire; rule state preserved for re-enable |
| Quiet hours active, severity < Critical | Queue for delivery at quiet-hours end |
| Quiet hours active, severity = Critical | Deliver immediately, bypass quiet hours |

---

# Part VI — Compliance Architecture

> The v0.1 overview (§35.10) treats compliance as legal review. This makes it architecture.

## 14. Jurisdiction Gating

```rust
/// Determined at first-run and verified periodically via IP check
/// (server-side for cloud deployments) or self-declaration (desktop).
/// Gates features that are legally restricted by jurisdiction.
pub struct JurisdictionGate {
    pub jurisdiction: Jurisdiction,
    pub feature_overrides: HashMap<FeatureId, FeatureState>,
    pub last_verified: OffsetDateTime,
}

pub enum FeatureState {
    Enabled,
    Disabled { reason: String },
    RequiresDisclosure { disclosure_id: DisclosureId },
}
```

**Example gating rules:**

| Feature | US | EU | UK | Restricted jurisdictions |
|---|---|---|---|---|
| Crypto market data | Enabled | Enabled | Enabled | Disabled if under OFAC sanctions |
| Options flow | Enabled | Requires disclosure | Requires disclosure | Disabled |
| Congressional trading data | Enabled | Disabled (data licensing) | Disabled | Disabled |
| Live broker execution | Enabled (US brokers) | Enabled (EU brokers) | Enabled (UK brokers) | Disabled |
| AI investment analysis | Requires disclaimer | Requires MiDIS disclaimer | Requires FCA disclaimer | Disabled |

## 15. Disclosure Registry

```rust
/// Every disclosure in the system is registered with an ID,
/// context (where it appears), text, and version. When a
/// disclosure is shown to a user, an audit event records it.
pub struct Disclosure {
    pub id: DisclosureId,
    pub context: DisclosureContext,  // Onboarding | OpportunityCard | RiskPanel | AIResponse | ...
    pub text: DisclosureText,
    pub version: SemanticVersion,
    pub jurisdiction: Option<Jurisdiction>,  // None = all jurisdictions
    pub required_acknowledgment: bool,
}

/// When a disclosure is rendered, this event is emitted.
/// It proves the user saw the disclosure for audit and compliance.
pub struct DisclosureShownEvent {
    pub disclosure_id: DisclosureId,
    pub user_id: UserId,
    pub shown_at: OffsetDateTime,
    pub context: DisclosureContext,
    pub acknowledged: bool,
}
```

---

# Part VII — Model Serving and Feature Store

> The v0.1 overview (§12) thoroughly covers model governance but has no runtime serving or feature store architecture.

## 16. Feature Store

The single most important property of a feature store for a trading platform is **point-in-time correctness** — a feature value used in training or inference must reflect exactly what was known at that timestamp, with no look-ahead.

```rust
/// Point-in-time feature query. Returns the feature value as it
/// existed at `as_of`, not the current value. This is the
/// mechanism that prevents look-ahead bias in backtests and
/// ensures reproducibility in live inference.
pub struct FeatureQuery {
    pub feature_view: FeatureViewId,
    pub entity_keys: Vec<EntityKey>,
    pub as_of: OffsetDateTime,        // the point in time
    pub feature_names: Vec<String>,    // which features to return
}

pub struct FeatureView {
    pub id: FeatureViewId,
    pub entities: Vec<EntityKeySchema>,
    pub features: Vec<FeatureSchema>,
    pub online_store: OnlineStore,    // SQLite (desktop) / Redis (enterprise)
    pub offline_store: OfflineStore,  // Parquet/DuckDB
    pub materialization: MaterializationSchedule,
    pub version: SemanticVersion,
}
```

### 16.1 Online vs Offline Serving

| Store | Used By | Latency Target | Data |
|---|---|---|---|
| Online (SQLite / Redis) | Live inference, streaming alerts, real-time scoring | ≤ 5ms | Latest feature values, ~7 day window |
| Offline (Parquet / DuckDB) | Backtest, model training, batch research | Seconds acceptable | Full history, point-in-time queryable |

### 16.2 Materialization

Features are computed by pipeline tasks (Part II) and written to both stores. The online store retains only the latest value per entity. The offline store retains the full time-series with lineage.

---

## 17. Model Serving

```rust
/// A registered model in the model registry.
pub struct ModelArtifact {
    pub id: ModelId,
    pub name: String,
    pub version: SemanticVersion,
    pub artifact_uri: String,          // local path or object storage
    pub content_hash: ContentHash,
    pub signature: ModelSignature,     // input/output schema
    pub runtime: ModelRuntime,         // Onnx | LlamaCpp | CloudApi
    pub owner: UserId,
    pub validation_report: ValidationReportId,
    pub calibration_report: Option<CalibrationReportId>,
    pub status: ModelStatus,           // Draft | Validated | Promoted | Retired
}

/// Inference trait. The runtime binds the model version to every
/// prediction so that v0.1 §12.4's "model version" requirement
/// is enforced structurally.
#[async_trait::async_trait]
pub trait ModelRuntime: Send + Sync {
    async fn predict(
        &self,
        features: FeatureSet,
    ) -> Result<Prediction, ModelError>;

    /// Metadata bound to every prediction for audit and reproducibility.
    fn metadata(&self) -> ModelMetadata;
}

pub struct Prediction {
    pub model_id: ModelId,
    pub model_version: SemanticVersion,
    pub value: PredictionValue,         // Distribution | Probability | Ranking
    pub confidence_interval: Option<Interval>,
    pub feature_contributions: Vec<FeatureContribution>,  // explainability
    pub training_window: DateRange,
    pub drift_status: DriftStatus,
    pub predicted_at: OffsetDateTime,
}
```

---

# Part VIII — Reproducibility Manifest Schema

> The v0.1 overview mandates "signed result manifests" in sections 4.7, 11, 37.5, and Phase 4 exit criteria, but never defines the format.

## 18. Manifest Schema

```json
{
  "manifest_type": "backtest",
  "manifest_version": 1,
  "manifest_id": "01J...",
  "created_at": "2026-07-13T15:00:00Z",
  "created_by": "user_id",

  "dataset": {
    "snapshot_id": "snap_01J...",
    "snapshot_hash": "sha256:...",
    "as_of": "2026-07-10T00:00:00Z",
    "adjustment_policy": "TOTAL_RETURN",
    "calendar_version": "2026.1",
    "provider_versions": {
      "coingecko": "2026-07-10",
      "sec_edgar": "2026-07-10"
    }
  },

  "strategy": {
    "strategy_id": "strat_01J...",
    "strategy_hash": "sha256:...",
    "strategy_version": "1.2.0",
    "compiler_version": "0.3.0",
    "parameters": { "rsi_period": 14, "iv_rank_max": 45 }
  },

  "execution_assumptions": {
    "commission": "0.65 per contract",
    "slippage_model": "quarter_spread",
    "fill_assumption": "market_open",
    "partial_fill_policy": "proportional"
  },

  "random_seed": 42,

  "code": {
    "commit_hash": "abc123def456",
    "dependency_hash": "sha256:Cargo.lock hash",
    "build_flags": []
  },

  "metrics": {
    "total_return": 0.234,
    "sharpe": 1.82,
    "max_drawdown": -0.087,
    "win_rate": 0.61
  },

  "signatures": [
    {
      "algorithm": "ML-DSA",
      "key_id": "key_01J...",
      "signature": "base64..."
    },
    {
      "algorithm": "Ed25519",
      "key_id": "key_01J...",
      "signature": "base64..."
    }
  ]
}
```

### Verification Protocol

To verify a result reproduces from its manifest:

1. Reconstruct the dataset snapshot from `snapshot_hash`.
2. Load the strategy from `strategy_hash` (content-addressed).
3. Check out code at `commit_hash`.
4. Verify `dependency_hash` matches `Cargo.lock`.
5. Run the backtest with the same parameters, seed, and execution assumptions.
6. Compare metrics within the documented numeric tolerance.
7. Verify both signatures against the trusted key store.

---

# Part IX — Market Infrastructure Services

> The v0.1 overview requires trading calendars, corporate actions, and symbology (§8.3, §10.3, §11.2) but names no sources or services.

## 19. Trading Calendar Service

```rust
/// Provides session boundaries, holidays, and half-days per venue.
/// Used by the backtest engine (replay only on valid sessions),
/// the execution boundary (block orders when market closed),
/// and the chart (show session breaks).
pub trait TradingCalendar: Send + Sync {
    fn is_open(&self, venue: Venue, at: OffsetDateTime) -> bool;
    fn next_open(&self, venue: Venue, from: OffsetDateTime) -> OffsetDateTime;
    fn next_close(&self, venue: Venue, from: OffsetDateTime) -> OffsetDateTime;
    fn session_for(&self, venue: Venue, at: OffsetDateTime) -> Option<Session>;
    fn holidays(&self, venue: Venue, year: i32) -> Vec<Holiday>;
}
```

**Source:** Self-hosted calendar data generated from exchange-published schedules, stored as versioned JSON, updated via signed data patches. Version pinned in reproducibility manifests.

## 20. Corporate Action Service

```rust
/// Corporate actions that affect price series and backtest integrity.
/// Every action specifies how it propagates to historical bars.
pub struct CorporateAction {
    pub id: CorporateActionId,
    pub asset_id: AssetId,
    pub action_type: ActionType,       // Split | Dividend | Spinoff | Merger | Delisting
    pub ex_date: Date,
    pub record_date: Option<Date>,
    pub payable_date: Option<Date>,
    pub ratio: Option<Ratio>,          // split ratio, dividend amount
    pub adjustment_policy: AdjustmentPolicy,
    pub source: ProviderId,
    pub source_reference: String,
    pub parser_version: SemanticVersion,
}
```

### Adjustment Propagation

| Action | Raw bars | Split-adj | Div-adj | Total-return |
|---|---|---|---|---|
| 2:1 split | Unchanged | Price ÷ 2, volume × 2 | Same as split-adj | Same as split-adj + dividend reinvestment |
| $0.50 dividend | Unchanged | Unchanged | Price − $0.50 on ex-date | Dividend reinvested at ex-date close |
| Delisting | Unchanged | Unchanged | Unchanged | Position removed at last available price |

The backtest engine and chart system default to `TotalReturn` adjustment but expose the policy as a parameter. Every reproducibility manifest pins the adjustment policy used.

## 21. Symbology Resolution

```rust
/// Resolves user input ("AAPL", "apple inc", a CUSIP, a FIGI)
/// to a canonical asset ID. Handles ambiguity, delistings, and
/// ticker changes over time.
pub struct SymbologyService {
    /// Tries to resolve a user query to exactly one canonical asset.
    /// Returns multiple candidates if ambiguous, requiring disambiguation.
    pub fn resolve(
        &self,
        query: &str,
        at: Option<OffsetDateTime>,  // None = current; Some = point in time
    ) -> ResolutionResult;
}

pub enum ResolutionResult {
    Unique(AssetId),
    Ambiguous(Vec<AssetCandidate>),    // user must pick
    NotFound,
    Delisted {
        asset_id: AssetId,
        last_valid_date: Date,
        successor: Option<AssetId>,    // if ticker changed
    },
}
```

---

# Part X — Desktop Backup, Restore, and Migration

> For a local-first application, the user's machine is the system of record. Backup and restore must be first-class.

## 22. Backup Bundle

```rust
/// A complete backup of the local-first store. Signed and encrypted.
/// Contains all local databases, the vault (rewrapped to a backup key),
/// configuration, and a manifest of contents.
pub struct BackupBundle {
    pub manifest: BackupManifest,
    pub stores: Vec<StoreBackup>,       // SQLite, DuckDB, Parquet dirs
    pub vault: VaultBackup,             // rewrapped, not re-encrypted
    pub config: ConfigBackup,
    pub signature: BackupSignature,
}

pub struct BackupManifest {
    pub app_version: SemanticVersion,
    pub schema_versions: HashMap<StoreId, SemanticVersion>,
    pub created_at: OffsetDateTime,
    pub total_size_bytes: u64,
    pub item_counts: HashMap<String, u64>,  // {"strategies": 42, "journal_entries": 318}
    pub encryption: EncryptionInfo,
}
```

### Restore Protocol

1. User provides backup file + recovery key (or OS keychain unlock).
2. App verifies backup signature.
3. App checks schema versions against current app version.
4. If schemas differ, run migration sequence (old → current).
5. Restore stores to local paths.
6. Restore vault (rewrap from backup key to device key).
7. Validate integrity (record counts, hash checks).
8. App restarts with restored data.

### Cross-Version Migration

Every local store (SQLite, DuckDB, Parquet layouts) has a `schema_version` table. On app launch, the migration runner checks the stored version against the app's expected version and runs sequential migrations (N → N+1 → N+2 → current). Migrations are signed and idempotent. If a migration fails partway, the runner rolls back to the pre-migration state and surfaces the error — never leaves the store in a half-migrated state.

---

# Closing

This evolution document fills the gaps that would block or degrade implementation of the v0.1 overview. The priority order for implementation is:

1. **Part I — Trait Surfaces** (Strategy, Risk, BrokerGateway, ToolGateway, AlertEvaluator, Repository) — these are the architectural contract. Nothing else can be built correctly until these exist as reviewed ADRs.
2. **Part II — Pipeline Orchestration** — the data pipeline is the backbone of evidence-chained intelligence.
3. **Part III — Cost Model** — required for the Phase 0 exit criteria.
4. **Part IV — First-Run and Suitability** — determines whether the product's pitch lands.
5. **Parts V–X** — fill the remaining gaps as phases progress.

The v0.1 overview remains the authoritative product and security document. This document is its build-ready companion.
