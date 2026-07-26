# PRISMATIK StrategyIR Specification

**Document:** `spec/STRATEGY_IR.md`
**Status:** NORMATIVE — RFC 2119 keywords apply
**Companion to:** `spec/CRATE_ARCHITECTURE.md` §3.7 (`prismatik-strategy`), `PRISMATIK_Unified_Solution_Architecture_v1.0.md` §18 (strategy plane)
**Date:** 2026-07-26

---

## 0. Purpose

This document specifies the canonical **`StrategyIR`** — the JSON-serializable intermediate representation that every strategy authoring mode produces and the runtime accepts. Three authoring modes (visual builder, DSL, Rust SDK) plus a research mode (Python emitter) ALL converge on this format. **The runtime accepts nothing else.**

The design intent (v1.0 §18): a strategy authored in any mode executes identically because the IR is the single source of truth. Wave 3 DoD criterion 10 verifies this with a three-way comparison test.

**Non-Turing-complete by construction.** The IR has no loops, no recursion, no unbounded computation. Every expression terminates. This is deliberate: a strategy language with unbounded loops needs fuel metering, a timeout, and a story about what a half-executed strategy means for portfolio state. A language where every expression terminates by construction needs none of those.

---

## 1. Top-Level Schema

```json
{
  "$schema": "https://prismatik.example/schemas/strategy-ir-v1.json",
  "schema_version": "1.0.0",
  "strategy_id": "06d2e7e0-...-...",
  "name": "Crypto momentum with vol filter",
  "description": "Long BTC when 50d MA > 200d MA and realized vol < 60%",
  "capabilities": {
    "can_access_network": false,
    "can_access_filesystem": false,
    "can_invoke_ai": false,
    "max_compute_per_bar_ms": 50
  },
  "universe": { "...": "see §2" },
  "inputs": { "...": "see §3" },
  "schedule": { "...": "see §4" },
  "rules": { "...": "see §5" },
  "risk_overrides": [],
  "metadata": {
    "author": "aaron@mythos.systems",
    "created_at": "2026-07-26T14:32:00Z",
    "source_mode": "dsl",
    "source_hash": "blake3:..."
  }
}
```

### Required Fields

| Field | Type | Notes |
|---|---|---|
| `schema_version` | semver string | Currently `"1.0.0"`. |
| `strategy_id` | UUID | Generated at first save; stable across versions. |
| `name` | string | Human-readable. |
| `capabilities` | object | Capability declaration; enforced by runtime. |
| `universe` | object | What assets the strategy trades. |
| `inputs` | object | What data the strategy consumes. |
| `schedule` | object | When the strategy is invoked. |
| `rules` | object | The decision logic. |

`description`, `risk_overrides`, `metadata` are optional but recommended.

---

## 2. Universe Section

The universe declares what assets the strategy trades. Resolution is bitemporal via `SymbologyResolver` — the strategy never sees tickers directly.

```json
"universe": {
  "kind": "static" | "dynamic_query" | "parametric",
  "static_members": ["asset_id_1", "asset_id_2"],
  "dynamic_query": {
    "datafusion_plan": "<base64-encoded DataFusion LogicalPlan>",
    "rebalance_frequency": "daily" | "weekly" | "monthly" | "event_driven"
  },
  "parametric": {
    "parameter_name": "universe_param",
    "default": ["asset_id_1"]
  }
}
```

**Variants:**
- `static`: fixed list of `AssetId`s. Always resolves the same.
- `dynamic_query`: a DataFusion logical plan (e.g. "top 100 by 30d dollar volume on NASDAQ, as of the simulated clock"). Re-evaluated on `rebalance_frequency`.
- `parametric`: parameterizable at instantiation; used for parameter sweeps in walk-forward.

**Capabilities inferred:** a `dynamic_query` universe implies `can_access_data` capability. The runtime verifies the plan contains only point-in-time-correct rewrites.

---

## 3. Inputs Section

What the strategy consumes. Each input is a `FeatureView` reference or an indicator specification.

```json
"inputs": {
  "features": [
    {
      "view_id": "06d2e7e0-...",
      "version": "1.2.0",
      "alias": "rsi_14",
      "entity_scope": "asset"
    }
  ],
  "indicators": [
    {
      "kind": "sma",
      "params": { "period": 50 },
      "input": "bar.close",
      "alias": "sma_50"
    },
    {
      "kind": "sma",
      "params": { "period": 200 },
      "input": "bar.close",
      "alias": "sma_200"
    }
  ],
  "bars": {
    "asset_id_field": "asset_id",
    "bar_kind": "time",
    "bar_interval_seconds": 86400,
    "fields_used": ["open", "high", "low", "close", "volume"]
  }
}
```

### Indicator Catalog (closed set, extensible via plugins)

The built-in indicator set is closed and enumerated. Plugins add to the set via the WASM plugin host (Wave 3+). Each indicator declares warmup length.

| `kind` | params | warmup |
|---|---|---|
| `sma` | `{ period: int }` | `period - 1` |
| `ema` | `{ period: int }` | `period - 1` |
| `rsi` | `{ period: int }` | `period` |
| `macd` | `{ fast: int, slow: int, signal: int }` | `slow + signal - 1` |
| `bollinger` | `{ period: int, stddev: float }` | `period - 1` |
| `atr` | `{ period: int }` | `period` |
| `adx` | `{ period: int }` | `period * 2` |
| `obv` | `{}` | 0 |
| `realized_vol` | `{ window: int, returns_type: "log" | "arith" }` | `window` |
| ... (full YATA-backed set) | | |

### Warmup Enforcement

The runtime tracks warmup state per indicator. A strategy reading `sma_200` on bar 199 receives `NaN`, NOT a partial calculation. This is enforced structurally; there is no "best-effort" mode.

---

## 4. Schedule Section

When the strategy is invoked. Three kinds:

```json
"schedule": {
  "kind": "on_bar_close",
  "bar_kind": "time",
  "bar_interval_seconds": 86400,
  "session_filter": "regular_only"
}
```

```json
"schedule": {
  "kind": "on_event",
  "event_types": ["filing-received", "flow-print-received"],
  "filter": { "form_types": ["8-K"], "min_premium_usd": 100000 }
}
```

```json
"schedule": {
  "kind": "time_triggered",
  "cron": "0 16 * * 1-5",  // 4pm ET weekdays
  "timezone": "America/New_York",
  "session_filter": "regular_only"
}
```

**`session_filter`** enforces calendar awareness: `regular_only`, `regular_and_extended`, `any`, or a custom predicate. The runtime uses the pinned `SessionCalendar` artifact, never a live library.

---

## 5. Rules Section (the decision logic)

The rules section is a **declarative rule tree**, not a sequence of statements. Three node kinds: `condition`, `action`, `control`. Every rule tree terminates in `action` nodes.

### 5.1 Action Nodes

```json
{
  "node_kind": "action",
  "action": "open_long" | "open_short" | "close_long" | "close_short" | "reduce" | "increase" | "rebalance_to" | "halt_strategy" | "log_only",
  "params": {
    "asset_ref": "universe_member" | "parametric:asset_name" | { "asset_id": "..." },
    "size": { "...": "see §5.3" },
    "order_type": "market" | "limit" | "stop" | "stop_limit",
    "limit_price_ref": "expr:...",
    "time_in_force": "day" | "gtc" | "ioc",
    "risk_overrides": []
  }
}
```

`halt_strategy` is risk-reducing: it stops new entries but does NOT close existing positions. `log_only` produces no orders but writes an audit entry; used for diagnostic strategies.

### 5.2 Condition Nodes

```json
{
  "node_kind": "condition",
  "predicate": { "...": "see §6 for predicate language" },
  "then": { "...": "child node" },
  "else": { "...": "child node, optional" }
}
```

### 5.3 Size Expressions

Size can be:
```json
"size": {
  "kind": "fixed_notional",
  "notional": "5000"  // decimal string
}
```
```json
"size": {
  "kind": "fixed_quantity",
  "quantity": "100"
}
```
```json
"size": {
  "kind": "percent_of_portfolio",
  "percent": "0.05"
}
```
```json
"size": {
  "kind": "volatility_targeted",
  "target_vol_annual": "0.15",
  "lookback_window": 60,
  "vol_estimator": "realized_vol",
  "max_leverage": "1.0"
}
```
```json
"size": {
  "kind": "expression",
  "expr": "size_param * confidence_score"
}
```

The runtime computes size using the deterministic `DeterminismContext`. Decimal arithmetic only.

---

## 6. Predicate Language

Predicates are pure expressions over the strategy's input bindings. The language is **strongly typed, non-Turing-complete, side-effect-free**.

### 6.1 Literals and References

```
42                          // int
3.14                        // float (but use decimals for money)
"BTC"                       // string
true | false                // bool
null                        // null
sma_50                      // input reference (resolved at runtime)
bar.close                   // bar field access
portfolio.equity            // portfolio field
clock.now                  // simulated clock (deterministic)
entropy.next_f64           // seeded RNG (deterministic, recorded)
```

### 6.2 Operators

```
// Arithmetic
a + b | a - b | a * b | a / b
a div b   // integer division
a mod b

// Comparison
a == b | a != b | a < b | a <= b | a > b | a >= b

// Logical
a and b | a or b | not a

// Coalesce and default
a ?? b     // null coalesce

// Conditional
cond ? then : else
```

### 6.3 Built-in Functions (closed set)

| Function | Type | Notes |
|---|---|---|
| `isnan(x)` | float → bool | NaN check |
| `coalesce(a, b)` | (T?, T) → T | Null coalesce |
| `crosses_above(a, b)` | (series, series) → bool | Crossover (a was below b, now above) |
| `crosses_below(a, b)` | (series, series) → bool | Crossover (a was above b, now below) |
| `highest(x, n)` | (series, int) → value | Rolling max over last n |
| `lowest(x, n)` | (series, int) → value | Rolling min |
| `rank(x, universe)` | (series, [asset]) → float | Cross-sectional rank |
| `zscore(x, lookback)` | (series, int) → float | Rolling z-score |
| `covariance(a, b, lookback)` | (series, series, int) → float | |
| `correlation(a, b, lookback)` | (series, series, int) → float | |
| `decay(x, halflife)` | (series, int) → float | EWM with given halflife |

**No user-defined functions.** Plugins extend the function set via WASM (declared in capabilities, capability-checked).

### 6.4 Example Predicate

```json
"predicate": {
  "expr": "sma_50 > sma_200 and realized_vol_60 < 0.60 and crosses_above(sma_50, sma_200)"
}
```

---

## 7. Risk Overrides

A strategy MAY declare per-strategy risk overrides, but they are:
- Always MORE restrictive than the global policy (never looser).
- Always recorded in the audit ledger at strategy load.
- Always surfaced in the UI before any order is submitted.

```json
"risk_overrides": [
  {
    "check_id": "max_position",
    "strategy_value": { "notional_usd": "10000" },
    "global_value": { "notional_usd": "25000" }
  }
]
```

If `strategy_value` is looser than `global_value`, the strategy fails validation at compile time.

---

## 8. Capabilities and Validation

### 8.1 Capability Inference

Capabilities are inferred from the IR by the compiler:

| IR construct | Implied capability |
|---|---|
| `dynamic_query` universe | `can_access_data` |
| Network-referenced input | `can_access_network` (RARE — usually forbidden) |
| `can_invoke_ai` invocation | `can_invoke_ai` (currently forbidden in IR; AI tools are separate) |
| Plugin-provided indicator | `can_invoke_plugin:plugin_id` |

### 8.2 Validation Steps (compile-time)

1. **Schema validation** — JSON Schema compliance.
2. **Type checking** — every expression is well-typed; indicator outputs match predicate input types.
3. **Capability check** — declared capabilities ≥ inferred capabilities.
4. **Determinism lint** — no `clock.now_wall` (forbidden; only `clock.now` simulated), no `Math.random()`.
5. **Warmup feasibility** — every indicator reference can complete warmup within the schedule's first invocation; if not, fail with explicit minimum-history requirement.
6. **Point-in-time check** — every `dynamic_query` plan passes the DataFusion point-in-time rewrite rule (see `spec/CRATE_ARCHITECTURE.md` §3.7).
7. **Risk-override sanity** — overrides are stricter, not looser.
8. **Dry-run on fixtures** — runs against golden fixture data; must not panic.

A failure at any step blocks `save_strategy`. The three-stage authoring-contract triple (from QuantDinger, v1.2 §3.4):
```
get_strategy_authoring_contract  →  returns this schema + indicator catalog + capability envelope
        ↓
validate_strategy                →  runs the 8 validation steps; returns diagnostics with spans
        ↓
save_strategy                    →  persists as UNSIGNED DRAFT, never live
```

---

## 9. Three Authoring Modes → IR

### 9.1 Visual Builder

The visual builder emits IR via codegen. Every node in the visual graph corresponds 1:1 to an IR node. The builder CANNOT express anything not in the IR.

```rust
// pseudocode for the visual-builder codegen
fn visual_to_ir(graph: VisualGraph) -> StrategyIR {
    let rules = graph.root_node.to_ir_node();
    StrategyIR {
        schema_version: "1.0.0".into(),
        // ... direct mapping
        rules,
        metadata: Metadata { source_mode: "visual".into(), .. },
    }
}
```

### 9.2 DSL

The DSL parser (lexer on `logos`, recursive descent, type checker) produces IR. The DSL is a thin syntactic sugar over the IR.

Example DSL:
```
strategy "Crypto momentum with vol filter"
  description "Long BTC when 50d MA > 200d MA and realized vol < 60%"

  universe static [BTC, ETH]

  input sma_50 = sma(bar.close, 50)
  input sma_200 = sma(bar.close, 200)
  input realized_vol_60 = realized_vol(bar.close, 60)

  schedule on_bar_close daily

  rule main:
    when sma_50 > sma_200
         and realized_vol_60 < 0.60
         and crosses_above(sma_50, sma_200):
      open_long size = percent_of_portfolio(0.05)
      risk_override max_position = notional_usd(10000)
```

Compiles to the IR in §1. The DSL type-checks against the IR schema; type errors block compilation.

### 9.3 Rust SDK

The Rust SDK constructs IR programmatically via a builder:

```rust
use prismatik_strategy::ir::*;

let ir = StrategyIR::builder("Crypto momentum with vol filter")
    .description("Long BTC when 50d MA > 200d MA and realized vol < 60%")
    .universe(Universe::static_members(vec![btc_id, eth_id]))
    .input(Indicator::sma("close", 50).alias("sma_50"))
    .input(Indicator::sma("close", 200).alias("sma_200"))
    .input(Indicator::realized_vol("close", 60).alias("realized_vol_60"))
    .schedule(Schedule::on_bar_close_daily())
    .rules(
        Conditional::new(
            Predicate::all_of(vec![
                "sma_50 > sma_200".parse()?,
                "realized_vol_60 < 0.60".parse()?,
                Predicate::crosses_above("sma_50", "sma_200"),
            ])
        )
        .then(Action::open_long()
            .size(Size::percent_of_portfolio(0.05)?)
            .risk_override(RiskOverride::max_position_notional(10000)?)
        )
    )
    .build()?;
```

This produces the same IR JSON as the other modes. Wave 3 DoD criterion 10 verifies byte-identical IR across the three modes for the same logical strategy.

### 9.4 Python Research Emitter

Python is a research mode; it EMITS IR (in the qlib-research sidecar), never executes inside a backtest. The emitter writes the same JSON.

```python
from prismatik_research import StrategyIRBuilder

ir = (
    StrategyIRBuilder("Crypto momentum with vol filter")
    .description("Long BTC when 50d MA > 200d MA and realized vol < 60%")
    .universe_static(["BTC", "ETH"])
    .input_sma("close", 50, alias="sma_50")
    .input_sma("close", 200, alias="sma_200")
    .input_realized_vol("close", 60, alias="realized_vol_60")
    .schedule_on_bar_close_daily()
    .when("sma_50 > sma_200 and realized_vol_60 < 0.60 and crosses_above(sma_50, sma_200)")
    .then_open_long(percent_of_portfolio=0.05)
    .build()
)
ir.save("/path/to/strategy.json")
```

The JSON is then loaded by the Rust runtime via `compile_strategy { source: Python { ir_json } }`. The runtime does not know or care that it came from Python.

---

## 10. Runtime Execution

### 10.1 Execution Context

The runtime constructs a `StrategyContext` wrapping `DeterminismContext`:

```rust
pub struct StrategyContext<'a> {
    pub clock: &'a dyn Clock,
    pub entropy: &'a mut dyn Entropy,
    pub pinned: &'a PinnedArtifactSet,
    pub session_calendar: &'a dyn SessionCalendar,
    pub symbology: &'a dyn SymbologyResolver,
    // data access scoped by capabilities
    pub data: ScopedDataAccess<'a>,
}
```

### 10.2 Per-Bar Invocation

```rust
#[async_trait]
pub trait Strategy: Send + Sync {
    fn ir(&self) -> &StrategyIR;
    fn capabilities(&self) -> &StrategyCapabilities;
    async fn on_bar(&mut self, ctx: &mut StrategyContext, bar: &Bar) -> Result<Vec<OrderIntent>, RuntimeError>;
    async fn on_event(&mut self, ctx: &mut StrategyContext, event: &MarketEvent) -> Result<Vec<OrderIntent>, RuntimeError>;
}
```

Per bar:
1. Runtime resolves universe (if dynamic_query).
2. Runtime materializes inputs (feature views, indicators with warmup tracked).
3. Runtime invokes `on_bar(ctx, bar)` (or `on_event`).
4. Strategy emits zero or more `OrderIntent`s.
5. Each intent flows through risk checks → if approved → OMS.

### 10.3 Recorded Effects

Every `entropy.next_f64` call, every plugin invocation is a recorded effect in the run's manifest. Replay returns the recorded values.

---

## 11. Versioning

`schema_version` follows semver. Within v1.x, additions are backward-compatible. Removing fields or changing semantics requires v2.0 with a migration tool.

The strategy IR schema is published under Apache-2.0 in `packages/schemas/strategy-ir-v1.json` (per v1.0 §8 amendment 3), so third parties can verify strategies and emit compatible IR without PRISMATIK.

---

## 12. Cross-References

| Topic | Document |
|---|---|
| Strategy crate API | `spec/CRATE_ARCHITECTURE.md` §3.7 |
| Backtest engine | `spec/CRATE_ARCHITECTURE.md` §3.8 |
| Determinism contract | `spec/CRATE_ARCHITECTURE.md` §1.1 |
| IPC for compile/validate/save | `spec/IPC_CONTRACTS.md` §2.6 |
| Manifest that records the strategy | `spec/MANIFEST_SCHEMA.md` |
| Testing fixtures | `spec/TESTING.md` |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
