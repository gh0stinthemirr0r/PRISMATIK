# PRISMATIK Specifications — Index

**Document:** `spec/SPEC_INDEX.md`
**Purpose:** Navigation index for the PRISMATIK specification corpus
**Date:** 2026-07-26

This directory contains the **normative engineering specifications** that turn the architecture vision into compilable contracts. An engineer (human or AI agent) implementing PRISMATIK MUST conform to these specifications; deviations require an ADR.

## Companion Documents

These specs sit alongside the strategic and architecture corpus:

| Document | Location | Purpose |
|---|---|---|
| Strategic vision + wave framework | `../PRISMATIK_Enterprise_Overview_Vision_and_Wave_Plan.md` | What we're building and in what order |
| Wave-by-wave delivery plan | `../waves/` (8 files + README) | Per-wave work items, DoD, risks |
| Architecture of record | `../../PRISMATIK_Unified_Solution_Architecture_v1.0.md` | Normative architecture (2887 lines) |
| Reference corpus analysis | `../PRISMATIK_v1.1_*.md`, `../PRISMATIK_v1.2_*.md` | Pattern sources and AI stack |
| README | `../../README.md` | Existing project README |

---

## Specification Catalog

### Tier 1 — Critical (without these, agents diverge immediately)

| Spec | File | What it specifies |
|---|---|---|
| **Crate Architecture** | [CRATE_ARCHITECTURE.md](CRATE_ARCHITECTURE.md) | Every crate's public API, dependencies, error types, configuration. The normative workspace layout. |
| **Data Schemas** | [DATA_SCHEMAS.md](DATA_SCHEMAS.md) | Parquet schemas for 6 layers, LanceDB tables, SQLite schema with numbered forward-only migrations. |
| **IPC Contracts** | [IPC_CONTRACTS.md](IPC_CONTRACTS.md) | Every Tauri command and event: name, params, returns, capability tier, error variants. |
| **Strategy IR** | [STRATEGY_IR.md](STRATEGY_IR.md) | The JSON IR schema, AST node types, three authoring modes' codegen targets, validation rules. |
| **Manifest Schema** | [MANIFEST_SCHEMA.md](MANIFEST_SCHEMA.md) | Normative reproducibility manifest format, dual-signature envelope, standalone verifier contract. |

### Tier 2 — High (without these, agents make inconsistent engineering choices)

| Spec | File | What it specifies |
|---|---|---|
| **Provider Adapters** | [PROVIDER_ADAPTERS.md](PROVIDER_ADAPTERS.md) | Per-provider: endpoint catalog, auth, rate limits, error mapping, cassette format. |
| **Testing** | [TESTING.md](TESTING.md) | Property test catalog, DST corpus, conformance fixtures, golden vectors, adversarial suites. |
| **CI Workflows** | [CI_WORKFLOWS.md](CI_WORKFLOWS.md) | GitHub Actions, matrix dimensions, gate definitions, failure handling, release pipeline. |
| **Security Threat Model** | [SECURITY_THREAT_MODEL.md](SECURITY_THREAT_MODEL.md) | STRIDE per trust boundary, attack surface inventory, control mapping, adversarial eval cadence. |

### Tier 3 — Medium (improves quality, can be filled iteratively)

| Spec | File | What it specifies |
|---|---|---|
| **Design System** | [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md) | Design tokens, component library inventory, evidence-drawer UX, chart theming. |
| **Observability** | [OBSERVABILITY.md](OBSERVABILITY.md) | OTel span catalog, attribute taxonomy, SLO definitions, alert rules, dashboards. |
| **AI Router Internals** | [AI_ROUTER_INTERNALS.md](AI_ROUTER_INTERNALS.md) | Provider dispatch, capability routing, recorded-effect replay, MCP registration, AnalystScope. |

---

## How to Use These Specs

### For an AI agent building PRISMATIK

1. **Read in dependency order.** Start with `CRATE_ARCHITECTURE.md` (everything depends on it), then `DATA_SCHEMAS.md` and `IPC_CONTRACTS.md` (the surfaces you'll touch most), then the relevant Tier 1 spec for your task.
2. **Conform, don't invent.** These specs are normative. If a contract is undefined here, it's an open question — surface it as an ADR rather than guessing.
3. **Check the wave file.** Before implementing, check `../waves/Wave_N_*.md` for your work item's ID, DoD criteria, and risks.
4. **Run the tests.** Every spec defines its tests in `TESTING.md` and CI gates in `CI_WORKFLOWS.md`. A passing test is the verification, not assertion.

### For an engineering team

1. **Onboard with this index, then the wave README, then the Enterprise Overview.**
2. **Pick a wave and own it.** Each wave is self-contained for sequencing.
3. **Use the spec as the contract.** Code review verifies conformance to these specs.
4. **Update the spec when the architecture evolves.** A spec change requires an ADR.

### For program management

1. **Wave files are the work plan.** Work items have stable IDs across the issue tracker.
2. **DoD criteria are gateable.** Each criterion names its verification artifact.
3. **Risk register is in the Enterprise Overview Part VI.**

---

## Specification Status

| Spec | Status | Stable as of |
|---|---|---|
| CRATE_ARCHITECTURE | Normative v1.0 | 2026-07-26 |
| DATA_SCHEMAS | Normative v1.0 | 2026-07-26 |
| IPC_CONTRACTS | Normative v1.0 | 2026-07-26 |
| STRATEGY_IR | Normative v1.0 | 2026-07-26 |
| MANIFEST_SCHEMA | Normative v1.0 (Apache-2.0 published) | 2026-07-26 |
| PROVIDER_ADAPTERS | Normative v1.0 | 2026-07-26 |
| TESTING | Normative v1.0 | 2026-07-26 |
| CI_WORKFLOWS | Normative v1.0 | 2026-07-26 |
| SECURITY_THREAT_MODEL | Normative v1.0 | 2026-07-26 |
| DESIGN_SYSTEM | Normative for tokens/components; recommended for style | 2026-07-26 |
| OBSERVABILITY | Normative v1.0 | 2026-07-26 |
| AI_ROUTER_INTERNALS | Normative v1.0 | 2026-07-26 |

---

## Cross-Reference Map

```
CRATE_ARCHITECTURE.md ─┬─→ DATA_SCHEMAS.md (storage shapes)
                       ├─→ IPC_CONTRACTS.md (frontend wiring)
                       ├─→ STRATEGY_IR.md (Strategy crate detail)
                       ├─→ MANIFEST_SCHEMA.md (Manifest crate detail)
                       ├─→ PROVIDER_ADAPTERS.md (Provider implementations)
                       ├─→ AI_ROUTER_INTERNALS.md (AI tools detail)
                       ├─→ OBSERVABILITY.md (telemetry attributes)
                       └─→ SECURITY_THREAT_MODEL.md (per-crate threats)

IPC_CONTRACTS.md ──────┬─→ CRATE_ARCHITECTURE.md (crate APIs)
                       ├─→ DATA_SCHEMAS.md (returned shapes)
                       ├─→ STRATEGY_IR.md (compile/validate commands)
                       ├─→ DESIGN_SYSTEM.md (consuming components)
                       └─→ OBSERVABILITY.md (command spans)

waves/ ─────────────────→ Enterprise_Overview (strategic context)
                       └─→ spec/* (implementation contracts)
```

---

## What These Specs Deliberately Do Not Cover

These items are deferred per the wave framework's "Explicitly Not Now" list (see Enterprise Overview Part VIII):

- Causal market graph (post-Wave 7 research)
- Market digital twin (post-Wave 7 research)
- Federated strategy learning (Wave 6+, demand-driven)
- Confidential computing / TEE broker integration (2027+ pilot)
- Multi-agent research council (post-Wave 4)
- Pine Script compatibility (post-Wave 7, if ever)
- TradingView Advanced Charts (only if customer requires)
- NautilusTrader adoption (reference only, permanently)
- Temporal orchestration below Wave 6
- Additional TSFM families
- Mobile client

When a deferred item is moved into scope, a new spec is added here.

---

## Updating These Specs

1. **Spec changes require an ADR.** The ADR names the spec, the change, the rationale, and the migration path.
2. **Backward compatibility is the default.** Within v1.x, additions are okay; removals/renames require v2.
3. **Test the spec.** Every normative statement MUST have a corresponding test in `TESTING.md`. Untested specs rot.
4. **Version the spec.** Each spec carries a version and a stable-as-of date. Update both on substantive change.

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
