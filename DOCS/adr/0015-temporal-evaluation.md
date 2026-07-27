# ADR-0015 — Temporal evaluation for Wave 6 OD

- Status: Accepted (defer)
- Date: 2026-07-26
- Wave: P8-OD-04

## Context

v0.2 §7.1 and the enterprise wave plan call for evaluating **Temporal** once
PRISMATIK moves from a single-user desktop spine to multi-tenant, cross-service
workflows. Desktop and retail waves (0–5) already run an in-process Tokio task
graph (`prismatik-application::task_graph`) with SQLite-persisted state and a
portable `TaskRunner`-shaped boundary.

Temporal would add durable workflow history, timers, saga-style compensation,
and multi-worker scheduling. Those capabilities matter when many tenants share
long-running pipelines across services — not when one local process owns the
graph.

## Decision

1. **Defer adoption** of Temporal (or any external durable orchestrator) until
   enterprise multi-tenant workflows are a concrete product requirement —
   typically after Postgres/RLS tenancy, event transport, and container deploy
   floors are real, not stubs.
2. Keep the domain behind the existing in-process task graph and
   `TaskRunner`-compatible traits so a later Temporal worker can host the same
   activities without rewriting business logic.
3. Do **not** introduce Temporal SDKs, workers, or cluster dependencies into
   Wave 6 turbo floors. P8-OD-04 is satisfied by this ADR + workbook
   `floor/docs` mark.

## Alternatives considered

| Option | Verdict |
|---|---|
| **Adopt Temporal now (Wave 6)** | Rejected. Ops cost (cluster, persistence, versioning) exceeds benefit for single-tenant / desktop-local graphs. |
| **Apache Airflow / Prefect** | Rejected for the same phase. Batch DAG UIs fit offline analytics more than low-latency execution and reconciliation loops. |
| **Custom durable log on Postgres** | Deferred with Temporal. Reinvents history/replay; only revisit if Temporal licensing or air-gap constraints force it. |
| **Stay on in-process Tokio + SQLite** | **Chosen for Waves 0–6 floors.** Sufficient crash recovery for local-first; portable trait boundary preserved. |

## Consequences

- Wave 6 OD does not block on Temporal cluster provisioning or worker images.
- Enterprise multi-tenant durable workflows remain an explicit reopen trigger:
  when cross-service sagas span tenant-isolated services and must survive
  process death with shared history, reopen this ADR and decide adopt vs.
  alternative.
- Documentation and runbooks continue to describe the in-process graph as the
  normative orchestrator until a superseding ADR lands.
