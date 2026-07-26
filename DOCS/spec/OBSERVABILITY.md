# PRISMATIK Observability Specification

**Document:** `spec/OBSERVABILITY.md`
**Status:** NORMATIVE — RFC 2119 keywords apply
**Companion to:** `spec/CI_WORKFLOWS.md`, `PRISMATIK_Unified_Solution_Architecture_v1.0.md` §26 (observability plane)
**Date:** 2026-07-26

---

## 0. Purpose

This document specifies the **OpenTelemetry span catalog, attribute taxonomy, SLO definitions, alert rules, and dashboard inventory** for PRISMATIK. Observability is the difference between "the system is slow" and "the system is slow because provider X is rate-limiting and the fallback is also degraded, with the divergence flagged at 14:32 UTC."

The cardinal rule (v1.0 §26): tracing attributes MUST include determinism telemetry (`run_id`, `pinned_artifact_set` digest, `clock_kind`, `entropy_stream_path`) so a failing DST seed is reproducible on a developer machine. Observability isn't just for production; it's the bridge between production failures and local reproduction.

---

## 1. Stack

- **Tracing:** OpenTelemetry (OTel) SDK in `prismatik-observability`.
- **Metrics:** OTel metrics + Prometheus scrape for Team Cloud/Enterprise.
- **Logs:** `tracing` crate (Rust) with structured events; bridge to OTel logs.
- **Exporters:** OTLP to local collector (desktop); OTLP to central collector (cloud/enterprise).
- **Dashboards:** Grafana 13 (per QuantDinger's separate-compose pattern, v1.2 §3.4).
- **Alerts:** Alertmanager.

---

## 2. Span Catalog

Every IPC command, provider call, plugin invocation, and significant internal operation is a span. Naming convention: `<crate>.<operation>`.

### 2.1 IPC Spans

| Span name | Trigger | Attributes |
|---|---|---|
| `ipc.command.<name>` | Tauri command invoked | `command`, `capability_tier`, `actor_id`, `correlation_id` |
| `ipc.event.<name>` | Event emitted | `event_type`, `schema_version`, `correlation_id` |

### 2.2 Provider Spans

| Span name | Trigger | Attributes |
|---|---|---|
| `provider.<id>.<endpoint>` | Provider call | `provider_id`, `endpoint`, `cost_units`, `entitled`, `admission_decision`, `rate_limit_remaining`, `latency_ms` |
| `provider.<id>.health` | Health check | `provider_id`, `status`, `consecutive_failures` |
| `provider_chain.failover` | Chain failover | `capability`, `primary`, `fallback`, `trigger` |

### 2.3 Determinism Spans (the load-bearing ones)

| Span name | Trigger | Attributes |
|---|---|---|
| `determinism.run` | Run started | `run_id`, `root_seed`, `clock_kind`, `clock_start`, `clock_end`, `pinned_artifact_digest`, `trace_digest` |
| `determinism.entropy_split` | Stream split | `parent_path`, `label`, `derived_seed` |
| `determinism.artifact_load` | Pinned artifact load | `artifact_id`, `kind`, `version`, `content_hash`, `signature_valid` |

**The `trace_digest` attribute is critical.** A failing DST seed reports its trace digest; a developer reproduces by re-running with the same seed and diffing traces.

### 2.4 AI Router Spans

| Span name | Trigger | Attributes |
|---|---|---|
| `ai_router.dispatch` | Inference call routed | `provider_id`, `model_id`, `model_digest`, `tier` (T0/T1/T2/T3), `egress_kind`, `recorded_effect_id` |
| `ai_router.replay` | Recorded effect replayed | `effect_id`, `prompt_hash`, `model_digest_match` |
| `ai_tool.invoke` | Controlled tool invoked | `tool_name`, `tool_class`, `actor_kind` (user/agent) |
| `ai.analyst.<role>` | Per-analyst invocation | `role`, `asset_id`, `evidence_scope_disjoint` |

### 2.5 Risk and Execution Spans

| Span name | Trigger | Attributes |
|---|---|---|
| `risk.evaluate_intent` | Order intent evaluated | `intent_id`, `checks_run`, `checks_passed`, `checks_failed`, `overall`, `portfolio_snapshot_hash` |
| `risk.check.<id>` | Single check | `check_id`, `severity`, `result`, `reason` |
| `execution.submit` | Order submitted | `intent_id`, `broker`, `idempotency_key`, `result_kind` (Accepted/Rejected/Unknown) |
| `execution.reconcile` | Reconciliation ran | `asset_id`, `divergence_kind`, `resolution` |

### 2.6 Audit Spans

| Span name | Trigger | Attributes |
|---|---|---|
| `audit.append` | Audit entry appended | `position`, `entry_hash`, `prev_hash`, `actor`, `action`, `outcome` |
| `audit.verify_all` | Startup verification | `entries_verified`, `tamper_detected` |
| `audit.tree_advance` | Tree head moved | `tree_size`, `root_hash` |

### 2.7 Plugin Spans

| Span name | Trigger | Attributes |
|---|---|---|
| `plugin.install` | Plugin install | `plugin_id`, `version`, `capabilities_requested`, `capabilities_granted`, `capability_diff` |
| `plugin.invoke` | Plugin invocation | `plugin_id`, `entry`, `fuel_used`, `wall_nanos`, `memory_peak`, `result_kind` |
| `plugin.capability_diff_blocked` | Capability diff gate fired | `plugin_id`, `attempted`, `baseline` |

### 2.8 Storage Spans

| Span name | Trigger | Attributes |
|---|---|---|
| `storage.parquet_write` | Parquet write | `layer`, `rows`, `bytes`, `path` |
| `storage.lance_query` | Lance query | `table`, `vector_count`, `latency_ms` |
| `storage.sqlite_migration` | Migration applied | `version`, `idempotent` |

---

## 3. Attribute Taxonomy

### 3.1 Standard Attributes (every span)

| Attribute | Type | Notes |
|---|---|---|
| `trace_id` | string | OTel-generated |
| `span_id` | string | OTel-generated |
| `parent_span_id` | string | OTel-generated |
| `correlation_id` | string | Per v1.0 §13.3 — follows a user action through to every derived effect |
| `causation_id` | string | Optional; the causing event |
| `run_id` | string | If within a deterministic run |
| `service.name` | string | `prismatik-core` / `prismatik-ingest` / etc. |
| `service.version` | string | PRISMATIK semantic version |
| `service.build_hash` | string | |
| `deployment.profile` | string | `desktop` / `cloud` / `enterprise` |

### 3.2 Determinism Attributes (load-bearing)

| Attribute | Type | Notes |
|---|---|---|
| `determinism.clock_kind` | enum | `system` / `simulated` / `frozen` |
| `determinism.root_seed` | int | Run root seed |
| `determinism.pinned_artifact_digest` | string | BLAKE3 of PinnedArtifactSet |
| `determinism.entropy_stream_path` | string | For entropy splits |
| `determinism.trace_digest` | string | Hash of TRACE log; for DST diffing |

### 3.3 Provider Attributes

| Attribute | Type | Notes |
|---|---|---|
| `provider.id` | int | ProviderId |
| `provider.endpoint` | string | |
| `provider.cost_units` | int | |
| `provider.rate_limit_remaining` | float | 0.0–1.0 |
| `provider.admission_decision` | enum | Admit/Defer/BudgetExhausted/NotEntitled |

### 3.4 Risk Attributes

| Attribute | Type | Notes |
|---|---|---|
| `risk.intent_id` | string | |
| `risk.check_id` | string | |
| `risk.severity` | enum | HardDeny/SoftWarn |
| `risk.result` | enum | Pass/Fail/NotApplicable |
| `risk.portfolio_snapshot_hash` | string | |

### 3.5 AI Attributes

| Attribute | Type | Notes |
|---|---|---|
| `ai.provider_id` | string | |
| `ai.model_id` | string | |
| `ai.model_digest` | string | Pinned weights |
| `ai.tier` | enum | T0/T1/T2/T3 |
| `ai.tool_class` | enum | ReadOnly/RiskReducing/RiskIncreasing |
| `ai.recorded_effect_id` | string | For replay |

---

## 4. SLO Definitions (Team Cloud + Enterprise)

Desktop has no SLOs (single user, local). Team Cloud and Enterprise define SLOs per service.

### 4.1 Trusted Core SLOs

| SLO | Target | Window |
|---|---|---|
| Availability | 99.9% | 30 days |
| IPC command p95 latency | <100ms | 30 days |
| IPC command p99 latency | <500ms | 30 days |
| Audit append p99 latency | <1ms | 30 days |
| Cold start to interactive | <2.0s | per release |

### 4.2 Provider SLOs (per provider)

| SLO | Target | Window |
|---|---|---|
| Provider availability (upstream) | tracked, no target (we don't control) | 30 days |
| Provider failover rate | <5% of requests | 30 days |
| Provider disagreement rate | <0.1% of N-of-M reads | 30 days |

### 4.3 DST SLO

| SLO | Target | Window |
|---|---|---|
| DST pass rate | 100% of seed corpus | each release |

A single DST failure blocks release (per `spec/CI_WORKFLOWS.md`).

---

## 5. Alert Rules

Alertmanager rules in `ops/alertmanager/rules.yml`. Severity levels: `critical` (page), `warning` (ticket), `info` (log).

### 5.1 Critical Alerts

| Alert | Condition |
|---|---|
| `AuditTamperDetected` | `audit.verify_all` reports `tamper_detected = true` |
| `AuditAppendLatencyHigh` | p99 > 5ms for 5 min (audit on sensitive path) |
| `LiveExecutionHalted` | Session halted in live mode |
| `RiskGateBypassAttempted` | Code path attempted to construct `OrderIntent` outside risk kernel (would be compile-time, but runtime detection as defense-in-depth) |
| `ManifestSignatureInvalid` | Any manifest fails signature verification on load |
| `ArtifactHashMismatch` | Loaded artifact hash doesn't match pinned hash |
| `Unknown` | Order submission returned `Unknown` and reconciliation failed |

### 5.2 Warning Alerts

| Alert | Condition |
|---|---|
| `ProviderDegraded` | Provider health `Degraded` for >5 min |
| `ProviderChainFailoverRate` | >10% of requests failing over for >1 hour |
| `CalibrationDriftDetected` | `drift_detected` event with action `widen_intervals` |
| `PretrainingContaminationDenied` | A model load was denied for contamination (someone tried) |
| `CapabilityDiffBlocked` | Plugin capability diff gate fired (someone tried) |
| `CoverageBelowNominal` | Realized coverage < nominal - sampling error for >24h |

### 5.3 Info Alerts (Logged Only)

| Alert | Condition |
|---|---|
| `RateBudgetExhausted` | Provider budget exhausted (expected, not paging) |
| `StaleDataDetected` | Stale data detected (UI handles; logged for diagnostics) |

---

## 6. Dashboard Inventory (Grafana)

### 6.1 System Overview

- Run rate by service
- Error rate by service
- IPC command latency (p50/p95/p99)
- Audit append latency
- Active sessions

### 6.2 Determinism Health

- DST pass rate over time
- Trace digest divergence count
- Pinned artifact rotation frequency
- Entropy stream consumption

### 6.3 Provider Health

- Per-provider availability
- Per-provider latency p95
- Failover rate
- Rate budget utilization per provider
- Cost accumulated per provider

### 6.4 Risk and Execution

- Risk check pass/fail rate per check
- Order submission result distribution (Accepted/Rejected/Unknown)
- Reconciliation divergence kinds
- Quarantined instruments count

### 6.5 AI Plane

- Inference calls per tier (T0/T1/T2/T3)
- Inference latency per provider
- Tool invocation by class
- Recorded-effect replay hit rate
- Drift status by model

### 6.6 Calibration

- Realized vs nominal coverage per model
- Per-regime coverage
- Interval width over time
- Drift actions taken

### 6.7 Audit Chain

- Tree size growth
- Inclusion proof verification rate
- Tamper detection events
- WORM mirror sync status (cloud/enterprise)

---

## 7. The Determinism Telemetry Bridge

The load-bearing observability feature: a production failure with a `run_id` and `trace_digest` is reproducible locally.

```rust
// When a run fails in production
tracing::error!(
    run_id = %run.id,
    trace_digest = %trace.digest(),
    pinned_artifact_digest = %pinned.digest(),
    root_seed = run.root_seed,
    "run failed; reproduce locally with: prismatik replay --run-id {} --seed {}",
    run.id, run.root_seed
);
```

The developer runs `prismatik replay --run-id <id> --seed <seed>` locally; the DST suite re-executes with the same seed and pinned artifact set. If the local trace digest differs from production, the divergence points to the bug.

---

## 8. Privacy and Observability

### 8.1 Local-Only by Default

Desktop profile exports OTel to a local collector ONLY. No telemetry leaves the machine by default. Telemetry data MAY contain user behavior patterns; treating it as sensitive.

### 8.2 Opt-In Cloud Telemetry

Team Cloud and Enterprise MAY opt-in to centralized telemetry. The opt-in is explicit per deployment; the data flows are documented.

### 8.3 Redaction

`redact_secrets` (per QuantDinger pattern) runs on all log/spans/exports. Sensitive patterns scrubbed: `api_key`, `secret`, `password`, etc. (20+ patterns).

---

## 9. Cross-References

| Topic | Document |
|---|---|
| CI cadence for observability | `spec/CI_WORKFLOWS.md` |
| Span-emitting crates | `spec/CRATE_ARCHITECTURE.md` |
| Threat model (observability surface) | `spec/SECURITY_THREAT_MODEL.md` |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
