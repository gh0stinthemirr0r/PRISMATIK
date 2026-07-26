# PRISMATIK Architecture Evolution v0.4
## Open-Source Trading, Visualization, Quantitative, and Connector Ecosystem

**Document:** `PRISMATIK_Architecture_Evolution_v0.4_Open_Source_Trading_Ecosystem.md`  
**Companion to:** `PRISMATIK_Overview.md` v0.1, Architecture Evolution v0.2, and TSFM Integration v0.3  
**Organization:** Mythos Systems  
**Author:** Aaron Stovall  
**Year:** 2026  
**Status:** Proposed architecture and open-source adoption standard  
**Verification date:** 2026-07-24  

---

## Executive Decision

PRISMATIK should absorb useful capabilities, design patterns, test vectors, connectors, and implementation techniques from the open-source trading ecosystem—but it must not become an uncontrolled aggregation of unrelated repositories.

The governing strategy is:

> **Own every load-bearing trust boundary; adopt or bridge commodity capabilities behind stable PRISMATIK contracts.**

PRISMATIK must own:

- Canonical asset identity.
- Market-data normalization.
- Data lineage and point-in-time correctness.
- Strategy intermediate representation.
- Deterministic strategy runtime.
- Risk policy and pre-trade enforcement.
- Order-intent and execution governance.
- Portfolio accounting.
- Audit and reproducibility manifests.
- Model governance.
- Provider entitlements.
- Security, signing, secrets, and cryptographic agility.
- User-facing evidence and uncertainty contracts.

PRISMATIK may adopt, wrap, bridge, or use as validation references:

- Financial chart renderers.
- General-purpose visualization engines.
- Streaming analytical grids.
- Technical-indicator formula libraries.
- Numerical-finance libraries.
- Research engines.
- Crypto venue connector frameworks.
- Backtesting conformance engines.
- Market microstructure simulators.
- Community examples whose provenance and licenses are verified.

This document evolves the TradingView ecosystem list into a governed **PRISMATIK Open-Source Trading Ecosystem Plane**.

---

# Part I — What `awesome-tradingview` Is and Is Not

## 1. Repository Assessment

`tradingview/awesome-tradingview` is a curated catalog of TradingView widgets, Advanced Charts, Trading Platform, Lightweight Charts, Pine Script resources, plugins, wrappers, data-feed examples, and community indicator packages.

It is useful as a **discovery index** and periodic scouting source.

It is not:

- A production framework.
- A dependency.
- A security-reviewed software distribution.
- A guarantee that linked projects are licensed safely.
- A guarantee of maintenance or compatibility.
- A guarantee that community Pine ports have clean provenance.
- A substitute for architecture or vendor due diligence.

PRISMATIK should ingest the repository only as **catalog metadata**, never as trusted executable code.

## 2. Recommended Treatment

Create an internal registry entry:

```yaml
catalog:
  id: github.tradingview.awesome-tradingview
  type: discovery_catalog
  execution_allowed: false
  automatic_dependency_install: false
  polling_interval: 30d
  requires_human_triage: true
  source: https://github.com/tradingview/awesome-tradingview
```

A scheduled scouting workflow may:

1. Fetch the pinned repository commit.
2. Parse newly added links.
3. Produce a change report.
4. Enrich entries with license and repository metadata.
5. Open review candidates.
6. Never install or execute discovered packages automatically.

---

# Part II — Open-Source Integration Doctrine

## 3. Six Allowed Integration Modes

Every external repository must be assigned exactly one primary integration mode.

### Mode A — Direct Dependency

Use when the license is permissive, the package is maintained, the API is stable enough, security posture is acceptable, it does not own a PRISMATIK trust boundary, and transitive risk is manageable.

Examples:

- TradingView Lightweight Charts.
- Apache ECharts.
- Perspective.
- Selected Rust numerical or indicator libraries.

### Mode B — Controlled Internal Fork

Use when the component is strategically important, upstream velocity is uncertain, PRISMATIK requires security or determinism changes, the license permits modification, and Mythos Systems accepts maintenance ownership.

Forks must preserve upstream lineage and cannot become silent source copies.

### Mode C — Isolated Sidecar or Worker

Use when the project is written in another runtime, provides mature functionality, should not enter the trusted Rust core, requires process/network/filesystem isolation, or has license obligations best contained at a service boundary.

Examples:

- Microsoft Qlib research jobs.
- QuantConnect LEAN comparison runs.
- CCXT connector workers.
- Hummingbot strategy or connector workers.

### Mode D — Compatibility Adapter

Use when PRISMATIK wants to support an ecosystem contract without embedding the project.

Examples:

- Pine-like indicator compatibility.
- TradingView UDF-compatible market-data endpoints.
- Broker or exchange API compatibility.
- External strategy import.

### Mode E — Validation Oracle or Reference Only

Use when independent comparison improves correctness, but license, language, maturity, or architecture makes adoption undesirable.

Examples:

- QuantLib for option-pricing conformance.
- LEAN for backtest comparison.
- NautilusTrader for event-driven architecture study.
- `hftbacktest` for microstructure simulation comparisons.

### Mode F — Reject or Prohibit

Use when licensing conflicts with the product model, provenance is unclear, the project is abandoned, it introduces insecure execution, it relies on undocumented scraping, it requests excessive permissions, or it provides no advantage over a safer implementation.

---

# Part III — Proposed Architecture Plane

## 4. Open-Source Ecosystem Plane

```text
PRISMATIK OPEN-SOURCE ECOSYSTEM PLANE

Discovery Registry
    ↓
Qualification and License Intelligence
    ↓
Quarantine Build Environment
    ↓
Security / Quality / Conformance Evaluation
    ↓
Adoption Decision
    ├── Direct Dependency
    ├── Internal Fork
    ├── Sidecar
    ├── Compatibility Adapter
    ├── Validation Oracle
    └── Reject
    ↓
Signed Component Registry
    ↓
Runtime Capability and Policy Enforcement
```

## 5. New Crates and Packages

```text
crates/
  prismatik-oss-registry/
  prismatik-chart-kernel/
  prismatik-indicator-core/
  prismatik-quant-kernel/
  prismatik-research-bridge/
  prismatik-connector-gateway/
  prismatik-conformance/
  prismatik-sandbox/
  prismatik-third-party-audit/

packages/
  chart-svelte/
  perspective-svelte/
  echarts-svelte/
  chart-contracts/
  indicator-contracts/
  plugin-sdk/
```

## 6. Component Manifest

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThirdPartyComponentManifest {
    pub component_id: ThirdPartyComponentId,
    pub name: String,
    pub repository_url: String,
    pub upstream_owner: String,
    pub integration_mode: IntegrationMode,

    pub pinned_ref: String,
    pub commit_sha: String,
    pub source_archive_hash: ContentHash,
    pub artifact_hashes: Vec<ContentHash>,

    pub license_spdx: Vec<String>,
    pub notice_required: bool,
    pub attribution_requirements: Vec<AttributionRequirement>,
    pub legal_review: ReviewState,

    pub languages: Vec<Language>,
    pub package_ecosystems: Vec<PackageEcosystem>,
    pub transitive_components: Vec<ThirdPartyComponentId>,

    pub capabilities: CapabilitySet,
    pub required_permissions: PermissionSet,
    pub secret_access: SecretAccessPolicy,
    pub network_policy: NetworkPolicy,
    pub filesystem_policy: FilesystemPolicy,

    pub security_policy_present: bool,
    pub vulnerability_state: VulnerabilityState,
    pub security_review: ReviewState,
    pub quality_review: ReviewState,
    pub numerical_conformance: Option<ConformanceReportId>,

    pub owner: TeamId,
    pub update_policy: UpdatePolicy,
    pub rollback_plan: RollbackPlan,
    pub approved_at: Option<OffsetDateTime>,
}
```

No external code enters a production release without this manifest.

---

# Part IV — TradingView Ecosystem Adoption

## 7. Lightweight Charts — Adopt as the Standard Price-Chart Backend

### Decision

**Adopt with a PRISMATIK abstraction layer.**

Use it for:

- Candlestick, bar, line, and area charts.
- Volume panels.
- Price and time scales.
- Crosshairs.
- Markers and price lines.
- Standard indicators.
- Custom primitives.
- Responsive desktop and web charting.

It should power ordinary instrument charts while PRISMATIK retains a custom GPU layer for advanced visualizations.

### Required Architecture

```rust
pub enum ChartBackendKind {
    LightweightCharts,
    PrismatikWgpu,
    AccessibleSvg,
    ExportRenderer,
}

pub trait ChartBackend {
    fn capabilities(&self) -> ChartCapabilities;
    fn mount(&mut self, target: RenderTarget) -> Result<(), ChartError>;
    fn apply(&mut self, command: ChartCommand) -> Result<(), ChartError>;
    fn snapshot(&self, request: SnapshotRequest) -> Result<ChartSnapshot, ChartError>;
    fn dispose(&mut self) -> Result<(), ChartError>;
}
```

TypeScript mirror:

```ts
export interface ChartBackend {
  readonly kind: ChartBackendKind;
  readonly capabilities: ChartCapabilities;

  mount(target: HTMLElement): Promise<void>;
  apply(command: ChartCommand): Promise<void>;
  snapshot(request: SnapshotRequest): Promise<ChartSnapshot>;
  dispose(): Promise<void>;
}
```

PRISMATIK must not let TradingView-specific types leak into domain models, strategies, alert rules, saved chart state, reports, AI chart tools, or exports. The workspace stores `ChartDocument`, not a vendor configuration object.

All required notices and attribution must be included in the release process and visible product surface where required.

## 8. Advanced Charts and Trading Platform — Do Not Make Them the Foundation

### Decision

**Do not use them as the core of the commercial/private desktop product.**

Reasons:

- Proprietary licensing.
- Usage and distribution constraints.
- Environment and attribution conditions.
- Product lock-in.
- Reduced control over differentiation.
- PRISMATIK still needs its own data source.

They may be evaluated for a separate licensed public portal, a public demo, comparative UX research, or enterprise customers with their own license.

## 9. TradingView Widgets — Optional Public-Web Surface Only

Widgets may be used on public marketing, education, or symbol-preview pages. They should not be canonical data sources or trusted desktop components.

## 10. Official Lightweight Charts Plugins — Adopt Selectively

Use official examples to inform custom series, drawing primitives, visible-range utilities, markers, watermarks, interaction patterns, and plugin architecture.

Every selected example still requires license verification, version pinning, tests, a native Svelte wrapper, theme integration, accessibility work, and performance testing.

## 11. `lightweight-charts-indicators` — Quarantine and Mine Carefully

The repository is potentially useful as a catalog of standard and community indicators, candlestick patterns, and drawing primitives.

The critical concern is provenance: community indicators may have been ported from Pine scripts whose individual licenses and authorship differ from the repository-level license.

Do not bulk copy the catalog. For each selected indicator:

1. Identify the mathematical definition.
2. Identify original sources and licenses.
3. Independently implement it in Rust.
4. Generate deterministic fixtures.
5. Compare against at least two independent implementations.
6. Document formula variations.
7. Register provenance.

## 12. OakScriptJS — Optional Compatibility Sandbox

Use only as a Pine-like compatibility experiment, formula reference, sandboxed JavaScript runtime, or migration assistant.

Do not make it the canonical strategy language, trusted execution runtime, direct live-order environment, or authority for risk and accounting.

## 13. TradingView UDF Samples — Contract Reference Only

Older UDF examples can help explain symbol resolution, history endpoints, and data-feed contracts. They should not become production code.

PRISMATIK may expose a read-only UDF-compatible endpoint if provider redistribution rights permit it.

---

# Part V — Visualization Stack Beyond TradingView

## 14. Dual-Layer Chart Architecture

### Layer 1 — Standard Financial Charts

**TradingView Lightweight Charts**

Use for price history, volume, indicators, event markers, drawing tools, multi-pane views, synchronized crosshairs, and replay.

### Layer 2 — PRISMATIK GPU Visualization

**Rust + wgpu + WGSL**

Use for:

- DOM and liquidity heatmaps.
- Footprint charts.
- Dense options-flow clusters.
- Volatility and Greeks surfaces.
- Monte Carlo path fields.
- Correlation matrices.
- Market topology.
- Institutional relationship graphs.
- TSFM distributions and embeddings.
- Millions of points and layered animated market maps.

Do not recreate ordinary chart primitives in wgpu unless benchmarks prove the standard backend is insufficient.

## 15. Perspective — Adopt for Streaming Analytical Workspaces

Perspective is an excellent fit for:

- Options-flow tables.
- Scanner results.
- Filing tables.
- Institutional holdings.
- Portfolio drilldowns.
- Streaming pivots.
- Grouped aggregations.
- User-configurable reports.
- Arrow-backed analytical views.
- DuckDB and ClickHouse-connected views.

Build a Svelte wrapper:

```ts
export interface PerspectivePanelConfig {
  source: PerspectiveSource;
  columns: string[];
  groupBy: string[];
  splitBy: string[];
  filters: FilterExpression[];
  aggregates: Record<string, AggregateFunction>;
  sort: SortExpression[];
  plugin: PerspectivePlugin;
}
```

Do not expose unrestricted SQL to AI tools or ordinary users. Query plans must pass authorization and resource budgets.

## 16. Apache ECharts — Adopt for General Visual Analytics

Use ECharts for treemaps, sunbursts, Sankey flows, category rotation, portfolio allocation, event timelines, macro dashboards, sector maps, and non-critical 3D views.

Do not use it as the authoritative high-frequency price chart.

## 17. KLineChart — Prototype as a Bake-Off Candidate

Evaluate against Lightweight Charts for built-in trading interaction, drawing tools, mobile behavior, customization, bundle size, accessibility, Svelte integration, and maintenance.

---

# Part VI — Technical Indicator and Pattern Engine

## 18. Canonical Rust Indicator Kernel

Create `prismatik-indicator-core`.

It must be:

- Rust-native.
- Deterministic.
- Streaming-capable.
- Batch-capable.
- Point-in-time safe.
- Reproducible.
- No-network.
- No-filesystem by default.
- Serializable.
- Numerically tested.
- WebAssembly-compatible for selected client workloads.

## 19. YATA — Strong Direct Candidate

YATA provides common technical-analysis methods and a custom-indicator interface under a permissive license.

Recommended approach:

- Pin a version.
- Wrap it behind PRISMATIK traits.
- Create golden conformance fixtures.
- Replace or fork individual calculations where requirements differ.
- Never expose YATA types publicly.

## 20. Rust Indicator Trait

```rust
pub trait Indicator: Send + Sync {
    fn descriptor(&self) -> IndicatorDescriptor;
    fn warmup_period(&self) -> usize;
    fn reset(&mut self);

    fn update(
        &mut self,
        input: &IndicatorInput,
    ) -> Result<IndicatorOutput, IndicatorError>;

    fn snapshot(&self) -> IndicatorState;
    fn restore(&mut self, state: IndicatorState) -> Result<(), IndicatorError>;
}
```

## 21. Indicator Provenance

```rust
pub struct IndicatorDescriptor {
    pub id: IndicatorId,
    pub name: String,
    pub semantic_version: SemanticVersion,
    pub formula_reference: Vec<SourceReference>,
    pub implementation_origin: ImplementationOrigin,
    pub parameters: Vec<ParameterDescriptor>,
    pub input_schema: SchemaId,
    pub output_schema: SchemaId,
    pub numerical_tolerance: NumericalTolerance,
    pub warmup_policy: WarmupPolicy,
    pub missing_data_policy: MissingDataPolicy,
}
```

## 22. Pattern Detection

Candlestick and price patterns must be deterministic rules with context-aware filters, explicit timeframes, quality dimensions, and separate backtestability. A pattern name is not a trade signal.

---

# Part VII — Quantitative Finance Libraries

## 23. RustQuant — Adopt Selectively Behind `prismatik-quant-kernel`

RustQuant is useful for option and instrument pricing, stochastic processes, numerical methods, distributions, optimization, term structures, calendars, schedules, and risk/reward calculations.

Adopt selected modules only after numerical review, stability tests, benchmarking, API maturity assessment, and independent conformance against QuantLib and analytic results.

Rust-native does not automatically mean production-ready.

## 24. QuantLib — Validation Oracle and Optional Pricing Sidecar

Use QuantLib for:

- Independent option-pricing verification.
- Calendar and day-count comparison.
- Volatility and yield-curve model comparison.
- Greeks conformance.
- Exotic-instrument research.
- Golden fixtures.

Possible modes:

- CI-only C++ validation executable.
- Isolated pricing service.
- Offline research bridge.
- Selective FFI when justified.

PRISMATIK should preserve a Rust-native authoritative path for core instruments while using QuantLib to find implementation errors.

## 25. `hftbacktest` — Market Microstructure Lab

Use as a simulation reference, queue-position comparison, latency and partial-fill lab, source of exchange microstructure scenarios, and optional isolated worker.

Do not merge it into the primary event engine without dataset compatibility, reproducibility, execution-model, numeric, timing, and security reviews.

---

# Part VIII — Research and Backtesting Ecosystem

## 26. QuantConnect LEAN — External Validation Engine

Use LEAN for strategy-result comparison, corporate-action behavior, brokerage-model comparison, event-driven architecture study, data import/export testing, and paper/live parity research.

It must never become a second source of truth for portfolio accounting, risk, canonical strategy state, orders, or audit.

A PRISMATIK `StrategyIR` may compile to an optional LEAN experiment package; results return as external evidence.

## 27. Microsoft Qlib — AI Research Sidecar

Use Qlib for factor research, dataset experiments, supervised learning, market-dynamics models, reinforcement learning, research automation, and model comparisons.

Guardrails:

- Run jobs in isolated containers.
- Supply signed dataset snapshots.
- Treat outputs as untrusted until imported.
- Never expose broker credentials.
- No direct live execution.
- Every model enters PRISMATIK model governance.
- Translate features into point-in-time-correct PRISMATIK feature views.
- Require training cutoff and contamination review.

## 28. NautilusTrader — Architectural Reference and Optional Sidecar

Use it for event-driven runtime comparisons, clock/replay semantics, order-state-machine study, adapter architecture, and performance design.

Avoid copying the architecture wholesale. PRISMATIK has additional requirements around evidence lineage, filings, TSFM governance, plugin trust, enterprise security, and post-quantum readiness.

## 29. Why PRISMATIK Should Not Replace Its Core With an Existing Engine

Existing engines specialize in execution speed, bots, research, asset pricing, connectors, or notebooks.

PRISMATIK’s moat is the governed fusion of market evidence, options intelligence, filings, institutional activity, quant validation, probabilistic forecasting, risk, AI, explainability, and enterprise security.

---

# Part IX — Crypto Connector Ecosystem

## 30. CoinGecko Remains the Primary Crypto Intelligence Provider

CoinGecko remains the canonical broad aggregation provider for asset identity, market snapshots, rankings, categories, exchanges, history, and global market state.

Exchange-native tools complement it rather than replace it.

## 31. CCXT — Isolated Connector Gateway

```text
PRISMATIK Rust Core
    │
    │ signed, typed gRPC/IPC
    ▼
CCXT Connector Worker
    │
    ├── Public market-data methods
    └── Optional private methods behind separate privilege profile
```

Security rules:

- Private keys never enter Svelte.
- Workers receive credentials only for assigned venues/accounts.
- Public and private workers are separate.
- Egress is allowlisted.
- Nonce and clock synchronization are monitored.
- Every order maps to a PRISMATIK idempotency key.
- State is reconciled after uncertainty.
- Workers cannot bypass risk evaluation.
- Workers can be independently disabled.

## 32. Hummingbot — Connector and Strategy Research Sidecar

Use Hummingbot for connector research, market-making comparison, exchange testing, paper simulations, and external strategy interoperability.

Do not expose it as the live-execution authority.

## 33. Native Rust Connectors Remain the Long-Term Goal

For high-value venues, build audited native Rust adapters using official APIs.

CCXT is the breadth accelerator; native Rust is the assurance path.

---

# Part X — Licensing and Commercial Product Boundaries

## 34. License Classes

### Green — Generally Compatible, Still Review Required

- MIT.
- Apache-2.0.
- BSD-2-Clause.
- BSD-3-Clause.
- ISC.

### Amber — Architecture and Distribution Review Required

- LGPL.
- MPL-2.0.
- EPL.
- Source-available licenses.
- Dual licenses.
- Attribution, network-use, or trademark provisions.
- Components with mixed-license subcomponents.

### Red — Do Not Embed by Default

- AGPL.
- GPL in linked proprietary product paths.
- Commons Clause.
- Non-commercial.
- No-derivatives.
- Unlicensed source.
- Copied scripts with no provenance.

This is an engineering intake policy, not legal advice. Counsel makes the final determination.

## 35. Projects That Should Stay Outside the Proprietary Core

### OpenBB

Potentially useful as an external user-deployed platform or integration endpoint, but its AGPL license requires careful legal treatment. Do not embed or copy it casually.

### vectorbt

Its licensing introduces commercial constraints beyond ordinary permissive terms. Use published ideas, formulas, or a user-operated environment—not an embedded dependency without legal approval.

### GPL Trading Bots and Backtesters

Keep them outside the proprietary distribution unless legal review approves the architecture and distribution model.

## 36. Pine Script and Community Indicator Provenance

Pine scripts may have explicit licenses, platform publication rules, author restrictions, no license, copied lineage, or public formulas expressed through unique source code.

```rust
pub struct IndicatorProvenance {
    pub mathematical_sources: Vec<SourceReference>,
    pub code_sources: Vec<CodeSource>,
    pub authors: Vec<AttributionParty>,
    pub licenses: Vec<SpdxExpression>,
    pub clean_room_reimplementation: bool,
    pub conformance_only_sources: Vec<CodeSource>,
    pub approved_for_distribution: bool,
}
```

---

# Part XI — Secure Intake Pipeline

## 37. Repository Intake Workflow

```text
Candidate Discovered
    ↓
Metadata and License Collection
    ↓
Maintainer and Activity Review
    ↓
Source Archive Pinned by Commit
    ↓
Quarantine Build
    ↓
SBOM Generation
    ↓
Malware / Secret / Dependency / License Scan
    ↓
Static Analysis and Tests
    ↓
Capability and Permission Analysis
    ↓
Numerical / Behavioral Conformance
    ↓
Architecture Review
    ↓
Legal Review
    ↓
Decision Record
    ↓
Signed Internal Artifact
    ↓
Staged Release
    ↓
Continuous Upstream Monitoring
```

## 38. Required Automated Checks

- OpenSSF Scorecard.
- OSV-Scanner.
- GitHub advisory review.
- `cargo audit`.
- `cargo deny`.
- npm/pnpm audit.
- License scanning.
- SBOM generation.
- Secret scanning.
- Malware scanning.
- SAST.
- Binary and install-script inspection.
- Reproducibility checks.
- Artifact signature verification.
- Parser and adapter fuzzing.
- Network-call inventory.
- Filesystem-call inventory.
- Unsafe Rust inventory.

## 39. No Unpinned Upstream Execution

Forbidden:

- Git dependencies pointed at a branch.
- Production installs from `main` or `master`.
- Runtime code downloads.
- Unverified plugin registries.
- Unverified GitHub Actions artifacts.
- AI agents adding dependencies without manifests.
- Blind `curl | sh`.
- Pulling community Pine scripts directly into releases.

---

# Part XII — PRISMATIK Chart and Visualization Contracts

## 40. Chart Document

```rust
pub struct ChartDocument {
    pub id: ChartDocumentId,
    pub asset_context: AssetContext,
    pub time_range: TimeRange,
    pub interval: BarInterval,
    pub adjustment_policy: AdjustmentPolicy,
    pub panes: Vec<ChartPane>,
    pub drawings: Vec<DrawingObject>,
    pub events: Vec<EventOverlay>,
    pub annotations: Vec<Annotation>,
    pub links: Vec<PanelLink>,
    pub theme: ThemeReference,
    pub data_provenance: ProvenanceBundle,
    pub version: AggregateVersion,
}
```

## 41. Plot Primitives

```rust
pub enum PlotPrimitive {
    Candlestick(CandlestickPlot),
    Ohlc(OhlcPlot),
    Line(LinePlot),
    Area(AreaPlot),
    Histogram(HistogramPlot),
    Scatter(ScatterPlot),
    Band(BandPlot),
    Heatmap(HeatmapPlot),
    Profile(VolumeProfilePlot),
    Footprint(FootprintPlot),
    Surface(SurfacePlot),
    Network(NetworkPlot),
    Distribution(DistributionPlot),
}
```

## 42. Chart Commands

```rust
pub enum ChartCommand {
    SetSeries(SeriesData),
    AppendPoints(Vec<DataPoint>),
    ReplaceRange(SeriesRangePatch),
    AddIndicator(IndicatorInstance),
    RemoveIndicator(IndicatorInstanceId),
    AddDrawing(DrawingObject),
    UpdateDrawing(DrawingObject),
    AddEventOverlay(EventOverlay),
    SetVisibleRange(TimeRange),
    SetCrosshair(CrosshairState),
    SetTheme(ThemeReference),
    CaptureSnapshot(SnapshotRequest),
}
```

## 43. AI Chart Tool

```json
{
  "tool": "chart.compose",
  "input": {
    "asset_id": "asset_...",
    "interval": "1d",
    "range": "1y",
    "overlays": [
      {"type": "event", "event_types": ["earnings", "form4"]},
      {"type": "indicator", "indicator_id": "rsi", "parameters": {"period": 14}}
    ]
  }
}
```

The AI never executes arbitrary chart JavaScript.

---

# Part XIII — TSFM Integration With the Visualization Ecosystem

The v0.3 TSFM architecture remains authoritative.

## 44. New Visual Surfaces

### Forecast Fan

Render median path, quantile bands, sample density, calibration state, out-of-domain warnings, model/tokenizer versions, and data cutoff.

### Embedding Analog Map

Use TSFM embeddings with regime/event filters, similarity distances, bias indicators, and counterfactual sample removal.

### Simulation Comparison

Compare historical bootstrap, parametric, regime-switching, TSFM, ensemble, and realized paths.

No TSFM output receives privileged visual emphasis unless selected explicitly.

## 45. External Model Repositories

Open-source forecasting models can be imported only as signed model artifacts through the model registry.

Required:

- Model and weight licenses.
- Training-data disclosure.
- Pretraining cutoff.
- Contamination review.
- Input/output schemas.
- Runtime hash.
- Calibration report.
- Drift profile.
- Benchmark against simpler baselines.
- Reproducibility manifest.

---

# Part XIV — Adoption Matrix

## 46. Recommended Disposition

| Repository / Project | Primary Value | License Posture | Recommended Mode | Decision |
|---|---|---:|---|---|
| `tradingview/awesome-tradingview` | Discovery catalog | Catalog | Discovery only | Monitor and triage |
| `tradingview/lightweight-charts` | Standard financial charting | Apache-2.0 + notices | Wrapped dependency | **Adopt** |
| TradingView Advanced Charts | Advanced charting | Proprietary | Separately licensed integration | Do not make core |
| TradingView Widgets | Public embeds | Vendor terms | Public web only | Optional |
| Official LWC plugins/examples | Chart primitives | Verify per repository | Selective direct/fork | Adopt selectively |
| `deepentropy/lightweight-charts-indicators` | Indicator catalog | Provenance review needed | Quarantine/reference | Do not bulk ingest |
| `deepentropy/oakscriptJS` | Pine-like compatibility | MIT | Sandboxed compatibility | Prototype |
| `perspective-dev/perspective` | Streaming grid/pivot | Apache-2.0 | Wrapped dependency | **Adopt** |
| `apache/echarts` | General visualization | Apache-2.0 | Wrapped dependency | **Adopt** |
| KLineChart | Chart alternative | Permissive | Prototype bake-off | Evaluate |
| `amv-dev/yata` | Rust indicators | Apache-2.0 | Wrapped dependency | **Adopt after conformance** |
| `avhz/RustQuant` | Rust quant finance | MIT/Apache-2.0 | Selective wrapped dependency | Evaluate/adopt modules |
| QuantLib | Mature quant pricing | BSD-3-Clause | Validation oracle/sidecar | **Adopt for conformance** |
| QuantConnect LEAN | Backtest/live engine | Apache-2.0 | External validation engine | Optional bridge |
| Microsoft Qlib | AI quant research | MIT | Isolated research sidecar | **Integrate** |
| NautilusTrader | Rust trading engine | Review LGPL implications | Reference/optional sidecar | Evaluate carefully |
| `ccxt/ccxt` | Exchange breadth | MIT | Isolated connector worker | **Integrate** |
| Hummingbot | Crypto connectors/market making | Apache-2.0 | Sidecar/reference | Integrate selectively |
| `nkaz001/hftbacktest` | Microstructure simulation | MIT | Research sidecar/oracle | **Integrate for research** |
| OpenBB | Financial data platform | AGPL | User-run external integration | Do not embed |
| vectorbt | High-speed research | Commercial-use constraints | External/reference only | Do not embed |
| Random Pine repositories | Indicator ideas | Mixed/unknown | Reject unless proven | Default reject |

---

# Part XV — Phased Implementation

## Phase 0.4A — Registry and Governance Foundation

Deliver:

- `prismatik-oss-registry`.
- Manifest schema.
- License policy.
- Integration-mode policy.
- Quarantine environment.
- ADR template.
- SBOM and vulnerability gates.
- Upstream-monitoring job.
- Initial component registry.

Exit criteria:

- No component can be approved without a signed manifest.
- Every shipped dependency appears in SBOM and notices.
- Branch-based production dependencies fail CI.

## Phase 0.4B — Chart Kernel and Lightweight Charts

Deliver:

- `ChartDocument`.
- `ChartBackend`.
- Svelte wrapper.
- Theme bridge.
- Provenance overlays.
- Snapshots.
- Crosshair synchronization.
- Event overlays.
- Attribution compliance.
- Performance suite.

Exit criteria:

- The backend can be replaced without domain changes.
- Million-point stress behavior is documented.
- Saved charts contain no vendor-specific types.

## Phase 0.4C — Indicator Kernel

Deliver:

- Rust indicator trait.
- YATA adapter.
- First 30 core indicators.
- Deterministic fixtures.
- WASM build.
- Pine/TradingView conformance comparisons.
- Provenance registry.
- Indicator editor schema.

Core first set:

- SMA, EMA, WMA.
- VWAP and anchored VWAP.
- RSI, MACD, ATR, ADX.
- Bollinger Bands, Keltner, Donchian.
- Stochastic, ROC, OBV, MFI, CCI.
- Supertrend.
- Ichimoku components.
- Realized volatility.
- Rolling beta/correlation.
- Volume-profile primitives.

## Phase 0.4D — Analytical Workspace

Deliver:

- Perspective Svelte wrapper.
- Streaming options-flow table.
- Scanner grid.
- Institutional holdings pivot.
- Filing explorer.
- Arrow streaming.
- DuckDB adapter.
- Saved views.
- Governed export.

## Phase 0.4E — General Visualization

Deliver:

- ECharts wrapper.
- Category rotation.
- Sector treemap.
- Macro dashboard.
- Institutional Sankey.
- Portfolio sunburst.
- Event timeline.
- Accessible fallbacks.

## Phase 0.4F — Quant and Conformance Lab

Deliver:

- RustQuant adapter.
- QuantLib validation executable.
- Option-pricing golden corpus.
- Greeks comparison.
- Stochastic-process tests.
- Calendar/day-count tests.
- Numeric tolerance policy.
- Conformance dashboard.

## Phase 0.4G — Research Bridges

Deliver:

- Signed dataset export.
- LEAN experiment adapter.
- Qlib job adapter.
- hftbacktest job adapter.
- Result import schema.
- Untrusted-result quarantine.
- Manifest extension.
- Research-worker sandbox.

## Phase 0.4H — Crypto Connector Gateway

Deliver:

- Public CCXT worker.
- Separate private CCXT worker.
- Capability registry.
- Venue health.
- Rate/nonce management.
- Typed reconciliation.
- Paper-order flow.
- Hummingbot research bridge.
- First native Rust venue adapter.

## Phase 0.4I — Pine Compatibility Research

Deliver:

- Supported Pine subset definition.
- Parser feasibility study.
- Clean-room compiler plan.
- OakScriptJS sandbox prototype.
- Translation assistant.
- Unsupported-construct diagnostics.
- No-network execution.
- CPU/memory/time limits.

## Phase 0.4J — GPU Advanced Visualization

Deliver:

- wgpu chart surface.
- Options-flow heatmap.
- Monte Carlo field.
- Volatility surface.
- Correlation map.
- TSFM forecast fan.
- Device-loss recovery.
- Reduced-capability fallback.
- GPU telemetry.

---

# Part XVI — ADR Backlog

Create at minimum:

- Lightweight Charts as standard chart backend.
- Chart backend abstraction.
- wgpu advanced visualization boundary.
- Perspective analytical workspace.
- ECharts general visualization.
- Canonical Rust indicator kernel.
- YATA adoption.
- RustQuant scope.
- QuantLib conformance role.
- LEAN external validation role.
- Qlib research sidecar.
- CCXT connector isolation.
- Hummingbot integration boundary.
- Pine compatibility and clean-room policy.
- Third-party component signing.
- Copyleft and source-available policy.
- External model artifact intake.
- Upstream monitoring and emergency revocation.

---

# Part XVII — Immediate Engineering Backlog

## First 30 Days

1. Create OSS component registry schema.
2. Register all current dependencies.
3. Add license and SBOM gates.
4. Prototype Lightweight Charts in Svelte/Tauri.
5. Implement `ChartDocument` and `ChartBackend`.
6. Prototype Perspective against synthetic options-flow data.
7. Build the first Rust indicator conformance harness.
8. Compare YATA, RustQuant, and analytic fixtures.
9. Build a QuantLib CI comparison container.
10. Create a CCXT public-data worker with no credentials.
11. Create a Qlib signed-dataset experiment.
12. Build an `awesome-tradingview` change monitor.
13. Produce a repository candidate dashboard.
14. Define forbidden-license policies.
15. Define attribution UI and notice generation.

## First 90 Days

- Standard chart workspace complete.
- 30 core indicators.
- Perspective flow/scanner workspaces.
- ECharts macro and portfolio views.
- Quant conformance lab.
- CCXT public market-data connector.
- Qlib research bridge.
- LEAN comparison prototype.
- hftbacktest simulation experiment.
- Signed third-party artifact pipeline.
- Upstream drift alerts.
- Pine compatibility scope.

---

# Part XVIII — Final Target Stack

```text
EXPERIENCE
  Svelte 5 / SvelteKit
  Tauri 2
  PRISMATIK Design System
  Lightweight Charts
  Perspective
  Apache ECharts
  PRISMATIK wgpu Renderer

TRUSTED CORE
  Rust
  StrategyIR
  Deterministic Runtime
  Risk Policy
  Portfolio Accounting
  Order Intent and Execution Governance
  Evidence / Lineage / Audit
  Cryptographic and Plugin Policy

INDICATORS AND QUANT
  prismatik-indicator-core
  YATA adapter
  prismatik-quant-kernel
  selected RustQuant modules
  QuantLib conformance worker

RESEARCH
  PRISMATIK Backtest Engine
  PRISMATIK Monte Carlo Lab
  PRISMATIK TSFM Runtime
  Qlib sidecar
  LEAN validation bridge
  hftbacktest microstructure lab

CRYPTO
  CoinGecko canonical aggregation
  native Rust venue adapters
  CCXT isolated breadth gateway
  Hummingbot research/connector bridge

DATA
  PostgreSQL
  ClickHouse
  DuckDB
  SQLite
  Apache Arrow
  Parquet
  NATS JetStream when needed

SECURITY AND SUPPLY CHAIN
  signed component manifests
  SBOM
  SLSA-style provenance
  artifact signatures
  capability sandboxing
  license gates
  vulnerability monitoring
  PQC-aware signing and crypto agility
```

---

# Closing Direction

The objective is not to reproduce TradingView or assemble dozens of GitHub projects into one interface.

The objective is to use the strongest open-source building blocks to accelerate commodity capabilities while Mythos Systems owns the parts that make PRISMATIK defensible:

- The canonical evidence graph.
- Institutional and event intelligence.
- The options opportunity engine.
- Quantitative validation discipline.
- The TSFM and model-governance plane.
- Deterministic risk and execution boundaries.
- The security model.
- The visual synthesis of uncertainty.
- The unified product experience.

Every adopted capability must be translated through PRISMATIK contracts, themed by the PRISMATIK design system, governed by the same trust policy, and audited through the same evidence and reproducibility framework.

---

# Verified Source Catalog

- TradingView Awesome TradingView: https://github.com/tradingview/awesome-tradingview
- TradingView Lightweight Charts: https://github.com/tradingview/lightweight-charts
- TradingView charting solutions: https://www.tradingview.com/free-charting-libraries/
- Perspective: https://github.com/perspective-dev/perspective
- Apache ECharts: https://github.com/apache/echarts
- YATA: https://github.com/amv-dev/yata
- RustQuant: https://github.com/avhz/RustQuant
- QuantLib: https://github.com/lballabio/QuantLib
- QuantConnect LEAN: https://github.com/QuantConnect/Lean
- Microsoft Qlib: https://github.com/microsoft/qlib
- NautilusTrader: https://github.com/nautechsystems/nautilus_trader
- CCXT: https://github.com/ccxt/ccxt
- Hummingbot: https://github.com/hummingbot/hummingbot
- hftbacktest: https://github.com/nkaz001/hftbacktest
- OpenBB: https://github.com/OpenBB-finance/OpenBB
- vectorbt: https://github.com/polakowo/vectorbt
- OakScriptJS: https://github.com/deepentropy/oakscriptJS
- Lightweight Charts Indicators: https://github.com/deepentropy/lightweight-charts-indicators
