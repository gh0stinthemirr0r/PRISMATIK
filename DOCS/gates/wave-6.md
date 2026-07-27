# Wave 6 Gate — Turbo Floor vs Author-Ops

**Policy:** [`TURBO_GATE_POLICY.md`](../waves/TURBO_GATE_POLICY.md) · **Workbook:** [`Wave_6_Workbook.md`](../waves/Wave_6_Workbook.md)  
**Verdict:** **OPEN (turbo floors).** No claim of K8s / marketplace production deploy.

## Exit criteria / work items

| ID / Criterion | Class | Status |
|---|---|---|
| P8-DP-03 event envelope | turbo-floor | floor |
| P8-DP-05 OpenLineage RunEvent stub | turbo-floor | floor (`prismatik-events::openlineage`) |
| P8-OD-03 metric/SLO / AlertRule stubs | turbo-floor | floor |
| P8-QM-01..02 fine-tune types via TSFM | turbo-floor | floor |
| ClickHouse cloud profile types | turbo-floor | floor |
| P8-SS-01..04 SessionPolicy/RBAC/TenantScopedKey | turbo-floor | floor partial |
| P8-DP-01..02 Postgres / RLS | author-ops residual | deferred types |
| P8-DP-04 S3 object store | turbo-floor | floor (`ObjectStore` / `InMemoryObjectStore`) |
| P8-OD-04 Temporal evaluation | turbo-floor | floor/docs (`DOCS/adr/0015-temporal-evaluation.md`) |
| OIDC / Vault | author-ops residual | open |
| P8-OD-01..03 K8s/Terraform | author-ops residual | deferred |
| Native wgpu | author-ops residual | deferred |
| OpenLineage emission sink (live) | author-ops residual | stub only; emit later |
