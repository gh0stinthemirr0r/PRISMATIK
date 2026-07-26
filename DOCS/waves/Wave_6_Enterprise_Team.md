# Wave 6 — Enterprise and Team

**Maps to v1.0 phases:** P8 (Enterprise, On-Premises, and Team)
**Business outcome:** second business. Multi-user deployment, tenancy, air-gapped operation, organizational model governance.
**Duration:** ~84 days.
**Date:** 2026-07-26

---

## Objective

Multi-user deployment, tenant isolation, air-gapped operation, organizational model governance, and the infrastructure for enterprise customers. This wave adds the Team Cloud and Enterprise On-Prem deployment profiles alongside the Personal Desktop profile shipped in earlier waves.

## Entry Criteria

Wave 5 exit gate **plus a signed enterprise customer or a credible pipeline. Do not build this speculatively.** Eighty-four days of enterprise infrastructure with no enterprise customer is the most expensive mistake available in this plan.

## Work Items

| ID | Track | Work item | Days | Conf |
|---|---|---|---:|:---:|
| P8-DP-01 | DP | PostgreSQL backend, migrations, connection management | 6 | M |
| P8-DP-02 | DP | Multi-tenant isolation model, row-level security, tenant-scoped keys | 8 | L |
| P8-DP-03 | DP | NATS JetStream event transport with the shared envelope | 5 | M |
| P8-DP-04 | DP | S3-compatible object storage for artifacts and bundles | 4 | M |
| P8-DP-05 | DP | OpenLineage projection and emission sink | 3 | M |
| P8-SS-01 | SS | OIDC and passkey authentication, session policy | 6 | M |
| P8-SS-02 | SS | RBAC and ABAC policy engine, approval workflows | 8 | L |
| P8-SS-03 | SS | Vault, KMS, and HSM integration for server keys | 6 | L |
| P8-SS-04 | SS | Air-gapped build: vendored crates, private registry, offline artifact staging | 6 | L |
| P8-OD-01 | OD | Container images, Kubernetes manifests, Terraform modules | 8 | M |
| P8-OD-02 | OD | Admin console: providers, entitlements, health, cost, audit export | 8 | M |
| P8-OD-03 | OD | Enterprise observability: metrics, SLOs, alerting, runbooks | 5 | M |
| P8-QM-01 | QM | Organization-specific TSFM fine-tuning through the governance pipeline | 6 | L |
| P8-QM-02 | QM | Fine-tune manifests: base model, dataset hash, differential privacy posture | 4 | L |
| P8-OD-04 | OD | Temporal evaluation ADR and, if adopted, durable workflow migration | 5 | L |

## Why This Wave Is Gated on a Customer

Every item in this wave is enterprise infrastructure. None of it improves the Personal Desktop product. The Personal Desktop customer does not need PostgreSQL multi-tenancy, NATS JetStream, OIDC, RBAC/ABAC, Vault/KMS/HSM, container images, Kubernetes, Terraform, or admin consoles. Building all of it speculatively is a 84-day bet that an enterprise customer will materialize and want exactly this set of capabilities.

The wave framework treats this as a binary go/no-go at the gate. **No signed customer or credible pipeline = wave does not open.** The waves below Wave 6 are a complete retail product; Wave 6 is a different business with a different buyer.

## The Three Profiles After Wave 6

| Component | Personal Desktop | Team Cloud | Enterprise On-Prem | Disconnected Research |
|---|:---:|:---:|:---:|:---:|
| SQLite (operational state, task graph) | ✓ | ✓ | ✓ | ✓ |
| DuckDB + DataFusion (analytics) | ✓ | ✓ | ✓ | ✓ |
| Parquet on local disk (raw + curated) | ✓ | ✓ | ✓ | ✓ |
| LanceDB (embeddings, analog index) | ✓ | ✓ | ✓ | ✓ |
| PostgreSQL (multi-user, RBAC, tenancy) | — | ✓ | ✓ | optional |
| ClickHouse (tick and flow scale) | — | ✓ | ✓ | optional |
| NATS JetStream (durable events) | — | ✓ | ✓ | — |
| S3-compatible object store | — | ✓ | ✓ | — |
| Sidecars (CCXT, QuantLib, Qlib, MAPIE) | opt-in | ✓ | ✓ | pre-staged |
| TSFM runtime | ONNX INT8 CPU | GPU service | GPU or quantized CPU | pre-staged signed artifacts |
| LM Studio (local AI inference) | ✓ | optional | optional | ✓ (air-gapped) |

**Shipping six engines to a single-user desktop is a defect.** A desktop install ships SQLite, DuckDB, Parquet, and LanceDB only.

## Compliance Posture at This Wave

By Wave 6, the compliance plane must be full-spectrum. The Wave 1 transparency-only scope expands to cover:

- **EU AI Act high-risk obligations (Annex III)** — deferred to **December 2, 2027** by the Digital Omnibus (May/June 2026). Transparency obligations still bit August 2, 2026. Full high-risk compliance plane lands here if PRISMATIK is classified Annex III (debatable — credit scoring is listed, but retail trading research tools may not be). Fines up to €35M or 7% global turnover.
- **SR 26-2 / OCC Bulletin 2026-13** (April 2026) — replaced SR 11-7. Modernizes for AI/ML; concentrates validation on high-materiality models. **Critical carve-out: GenAI/agentic AI explicitly out of scope for now**; GenAI-specific guidance promised via future RFI. PRISMATIK's model registry already satisfies most MRM requirements — say so explicitly in enterprise sales, because "we already have a compliant model inventory" is a concrete enterprise sales asset.
- **SEC Predictive Data Analytics final rule** (Release 34-97990, June 2025) — broker-dealers/RIAs using "covered technology" in "investor interactions" must identify, eliminate, or neutralize conflicts of interest. Build conflict-elimination documentation into the design from Wave 1; expand for enterprise customers here.

## Definition of Done

| # | Criterion | Verified by |
|---|---|---|
| 1 | Two tenants cannot observe each other's data under any query path | Red-team pass |
| 2 | An air-gapped install completes from staged artifacts with no network | Install test on disconnected host |
| 3 | Audit export produces a verifiable bundle (manifest + refs + metrics + inclusion proof + dual signature) | Standalone verifier accepts the bundle with no PRISMATIK install |
| 4 | Fine-tuned models pass the same registration gates as pretrained ones, including license class and contamination review | Registration negative tests |
| 5 | An enterprise customer can deploy via Terraform with no Mythos Systems engineering involvement | Customer-led deployment, recorded |
| 6 | RBAC policy correctly denies cross-tenant queries | Property test, 1k cases |
| 7 | OIDC + passkey auth enforced; no password fallback for enterprise profile | Auth audit |

## Risks

| Risk | Response |
|---|---|
| Built speculatively with no customer | **Hard gate: do not open this wave without a signed customer or credible pipeline.** |
| Enterprise customer wants a capability the wave doesn't include (e.g., custom data adapter, specific broker integration) | Scope to the customer's specific requirements; do not build the full 84 days speculatively. Treat the work item list as a menu, not a checklist. |
| EU AI Act high-risk classification applies and full compliance plane is required earlier | The model registry, calibration records, audit ledger, and signed reproducibility manifests already satisfy most high-risk obligations. Compliance plane expands documentation and human-oversight surfaces, not core architecture. |
| SR 26-2 GenAI-specific follow-on guidance lands mid-wave | Monitor OCC bulletins; the explicit GenAI carve-out in SR 26-2 means new guidance is coming via RFI before rulemaking. Quarterly compliance review. |
| Multi-tenant isolation has subtle cross-tenant leak paths | Red-team pass is mandatory at the gate. Row-level security tested by property test (1k cases). Tenant-scoped keys enforced at the storage layer, not just the query layer. |

## Commercial Metrics

| Metric | Wave 6 target |
|---|---|
| Enterprise customers | 1+ signed (entry criteria) → 3+ within 12 months |
| Revenue | Annual contracts + services |
| Retention | Contractual (annual) |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26*
