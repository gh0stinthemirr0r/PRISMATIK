# PRISMATIK supply-chain (`cargo vet`)

P0-SS-04: dependency vetting for the Rust workspace.

## Layout

| File | Role |
|---|---|
| `config.toml` | Imports, package policy, **exemptions** (bootstrap backlog) |
| `audits.toml` | Local audits + custom criteria (`trusted-core`) |
| `imports.lock` | Locked peer audit snapshots (committed) |

## Peer imports

Configured in `config.toml`:

- **mozilla** — `https://raw.githubusercontent.com/mozilla/supply-chain/main/audits.toml`
- **google** — `https://raw.githubusercontent.com/google/supply-chain/main/audits.toml`
- **bytecode-alliance** — `https://raw.githubusercontent.com/bytecodealliance/wasmtime/main/supply-chain/audits.toml`

Refresh with `cargo vet` (network) or `cargo vet regenerate imports` as needed.
CI should run `cargo vet check --locked` so audits match `imports.lock`.

## Trusted-core policy

**Foundational crates (minimum):** `prismatik-determinism`, `prismatik-identity`,
`prismatik-audit`, `prismatik-manifest`.

**Criteria:** `[criteria.trusted-core]` in `audits.toml` (implies `safe-to-deploy`).

**CI scope:** exclude other workspace members and `is_dev_only(true)` so the
check covers production deps of those four crates only. Helper:

```bash
bash scripts/cargo-vet-trusted-core.sh
# or on Windows:
pwsh scripts/cargo-vet-trusted-core.ps1
```

As of init, trusted-core typically reports on the order of ~30 exemptions
(vs hundreds for the full workspace). Those exemptions are annotated with
`notes` containing `trusted-core` in `config.toml`.

## Exemptions policy

`cargo vet init` seeded exemptions so `cargo vet check` is green immediately.
That is **explicit debt**, not silent ignore:

1. Every exemption is listed in `config.toml` under `[[exemptions.*]]`.
2. Trusted-core exemptions carry `notes` marking them as P0 audit backlog.
3. Remaining workspace exemptions are bootstrap backlog; reduce via peer
   imports, `cargo vet trust` (publisher policy), or first-party `certify`.
4. Do **not** delete exemptions to force green without an audit path — prune
   only unused entries (`cargo vet prune`).

Priority reduction order: trusted-core exemptions first, then the rest of
`safe-to-deploy` for the full lockfile.

## Local commands

```bash
cargo install --locked cargo-vet --version 0.10.2   # once
cargo vet check --locked                            # full workspace
bash scripts/cargo-vet-trusted-core.sh              # trusted-core subgraph
cargo vet suggest                                   # audit backlog hints
cargo vet prune                                     # drop unused exemptions/imports
```

## CI

Job `cargo-vet` in `.github/workflows/ci.yml`:

1. Install `cargo-vet` 0.10.2
2. `cargo vet check --locked` (full workspace)
3. `bash scripts/cargo-vet-trusted-core.sh` (scoped policy)

See also `DOCS/spec/CI_WORKFLOWS.md` (`deny-and-audit` / cargo-vet notes).
