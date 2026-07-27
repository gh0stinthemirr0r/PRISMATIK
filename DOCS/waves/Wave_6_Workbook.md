# Wave 6 Workbook — Enterprise & Team

**Updated:** 2026-07-26  
**Status:** **OPEN (turbo floors)** — see `TURBO_GATE_POLICY.md`

| ID | Status | Notes |
|---|---|---|
| P8-DP-01..02 | floor | `PostgresProfile` + `RowLevelSecurityContext` in `prismatik-storage` (no live backend) |
| P8-DP-03 | floor | `EventEnvelope` + `JetStreamEnvelope` in `prismatik-events` |
| P8-DP-04 | floor | S3-compatible `ObjectStore` + `InMemoryObjectStore` + `PathKey` + `put_blake3` in `prismatik-storage` (no AWS SDK) |
| P8-DP-05 | floor | OpenLineage `RunEvent` + `OpenLineageEmitter` (`InMemoryOpenLineageSink` / `HttpOpenLineageSink` fails closed) |
| P8-SS-01..04 | floor partial | SessionPolicy + RBAC + TenantScopedKey + OIDC/Vault **type floors** (`OidcProviderConfig` / `validate_claims_offline`, `VaultSecretRef` / `try_resolve_secret` → `NotLinked`) in `prismatik-security`; live IdP/Vault/KMS/HSM still author-ops |
| P8-OD-01..02 | **floor scaffold** | `deploy/k8s/` + `deploy/terraform/` stubs + author-ops README |
| P8-OD-03 | floor | `prismatik-observability` metric/SLO registries + enterprise AlertRule stubs |
| P8-OD-04 | floor/docs | `DOCS/adr/0015-temporal-evaluation.md` — defer Temporal until enterprise multi-tenant workflows |
| P8-QM-01..02 | floor | FineTuneManifest + dp_posture in `prismatik-tsfm`; TSFM families onboarded |
| ClickHouse cloud | floor | `ClickHouseCloudProfile` in `prismatik-storage` |
| Native wgpu | deferred | `IvSurfaceBackend::NativeWgpuDeferred` in `prismatik-renderer` |
