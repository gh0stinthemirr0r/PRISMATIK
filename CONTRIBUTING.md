# Contributing to PRISMATIK

Thanks for helping. Turbo floors favor typed stubs + tests; production ops stay author-gated.

## Quick start

```bash
cargo test --workspace
cargo vet check --locked
pnpm --filter @prismatik/desktop check
```

## Rules of the road

1. **No secrets in git** — API tokens, Vault paths with live credentials, Machine B keys stay off-repo.
2. **Fail closed** — adapters that are not linked return typed `NotLinked` / `Stub` errors with unit tests.
3. **Provenance** — UI surfaces use `EvidenceChip` / stale markers; never invent live data in fixtures without labeling the provider.
4. **Supply chain** — new crates need `cargo vet` coverage before merge (`safe-to-deploy` for first-party deps).
5. **Docs honesty** — workbook rows are `floor` / `floor scaffold` / `author-ops`; do not mark production-ready without the residual checklist.

## Where to look

| Area | Path |
|---|---|
| Turbo policy | `DOCS/waves/TURBO_GATE_POLICY.md` |
| Wave trackers | `DOCS/waves/Wave_*_Workbook.md` |
| Gate reviews | `DOCS/gates/` |
| Architecture | `DOCS/spec/` |
| Desktop | `apps/desktop/` |

## PRs

- Small, wave-scoped diffs preferred.
- Include `cargo test -p <crate>` evidence for the crates you touch.
- Update the relevant wave workbook row when landing a floor.
