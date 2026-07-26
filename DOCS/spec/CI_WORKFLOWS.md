# PRISMATIK CI Workflows Specification

**Document:** `spec/CI_WORKFLOWS.md`
**Status:** NORMATIVE — RFC 2119 keywords apply
**Companion to:** `spec/TESTING.md`, `spec/CRATE_ARCHITECTURE.md` §7
**Date:** 2026-07-26

---

## 0. Purpose

This document specifies the **concrete GitHub Actions workflows, gate definitions, matrix dimensions, failure handling, and release pipeline** for PRISMATIK. Every CI gate named in the architecture or wave DoD criteria MUST be defined here.

The cardinal rule: gates are blocking unless explicitly marked warning-only. A gate that warns but doesn't block is a gate that gets ignored, and an ignored gate is how defects reach production.

---

## 1. Workflow Overview

| Workflow | File | Trigger | Blocking? |
|---|---|---|---|
| `ci.yml` | Main CI | push, PR | Yes |
| `ci-windows.yml` | Windows matrix | push, PR | Yes |
| `dst.yml` | DST suite (256+ seeds) | push, PR (Wave 3+); scheduled | Yes |
| `chaos-weekly.yml` | Full chaos suite | Weekly schedule | Yes (release-blocking) |
| `adversarial.yml` | Full red-team | Weekly schedule | Yes (release-blocking) |
| `release.yml` | Release pipeline | tag push | Yes |
| `dependabot.yml` | Dependency bumps | Dependabot PRs | Yes |
| `coverage.yml` | Coverage report | nightly | Warning (release-blocking for core crates) |
| `benchmarks.yml` | Performance tracking | PR | Warning |

---

## 2. `ci.yml` — Main CI Workflow

### 2.1 Triggers

```yaml
on:
  push:
    branches: [main, release/*]
  pull_request:
    branches: [main, release/*]
```

### 2.2 Jobs

#### Job: `lint-and-format`

```yaml
lint-and-format:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
    - run: cargo fmt --all -- --check
    - run: cargo clippy --workspace --all-targets -- -D warnings
    - name: Workspace graph check
      run: python scripts/check_workspace_graph.py
    - name: Capability policy assertion
      run: cargo test --package prismatik-application capability_policy_assertion
```

#### Job: `determinism-grep`

The gate that prevents ambient-nondeterminism leaks (Wave 0 DoD criterion 1).

```yaml
determinism-grep:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Grep for forbidden patterns outside determinism crate
      run: |
        # Forbidden outside prismatik-determinism/src/:
        # - Instant::now
        # - SystemTime::now
        # - OffsetDateTime::now_utc
        # - thread_rng
        # - rand::random
        # - Uuid::new_v4
        # - HashMap/HashSet with default RandomState
        # Allowlist: <10 entries in prismatik-determinism/ALLOWLIST.txt
        python scripts/determinism_grep.py --allowlist prismatik-determinism/ALLOWLIST.txt
    - name: Negative test (gate must fail on violation)
      run: cargo test --package prismatik-determinism grep_gate_fails_on_violation
```

#### Job: `unit-and-property-tests`

```yaml
unit-and-property-tests:
  runs-on: ubuntu-latest
  strategy:
    matrix:
      rust: [stable, "1.96"]   # MSRV per v1.0
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@master
      with:
        toolchain: ${{ matrix.rust }}
    - uses: Swatinem/rust-cache@v2
    - run: cargo test --workspace --all-features
      env:
        PROPTEST_CASES: 1000   # 10000 for load-bearing in CI nightly
```

#### Job: `conformance`

```yaml
conformance:
  runs-on: ubuntu-latest
  services:
    quantlib-oracle:
      image: ghcr.io/mythos/prismatik-quantlib-oracle:latest
      ports: ['50051:50051']
  steps:
    - uses: actions/checkout@v4
    - run: cargo test --package prismatik-quant-kernel --test conformance -- --test-threads=4
    - name: Calendar cross-validation
      run: cargo test --package prismatik-calendar cross_validation
```

#### Job: `bindings-drift`

The gate that ensures tauri-specta-generated TypeScript matches Rust source.

```yaml
bindings-drift:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - run: cargo build --package prismatik-application --features generate-bindings
    - name: Check for diff in packages/api-client
      run: |
        git diff --exit-code packages/api-client/
```

#### Job: `registry-coverage`

```yaml
registry-coverage:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Every workspace dependency appears in component registry
      run: cargo test --package prismatik-oss-registry registry_coverage
```

#### Job: `deny-and-audit`

```yaml
deny-and-audit:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - run: cargo deny check --config deny.toml
    - name: Cargo audit
      run: cargo audit --deny warnings
    - name: Cargo vet (trusted core only)
      run: cargo vet check --trusted-core
```

#### Job: `golden-corpus`

```yaml
golden-corpus:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Re-execute golden manifests; assert byte-identical results
      run: cargo test --package prismatik-manifest golden_corpus_replay
```

#### Job: `integration-tests`

```yaml
integration-tests:
  runs-on: ubuntu-latest
  services:
    ccxt-gateway:
      image: ghcr.io/mythos/prismatik-ccxt-gateway:test
      ports: ['50052:50052']
  steps:
    - uses: actions/checkout@v4
    - name: Provider contract tests (network disabled)
      run: cargo test --package prismatik-market-data --test providers -- --features cassettes
      env:
        PRISMATIK_DISABLE_NETWORK: "1"
    - name: Multi-crate integration
      run: cargo test --package prismatik-application --test integration
```

---

## 3. `ci-windows.yml` — Windows Matrix

Per CVE-2026-42184 (Tauri IPC trust boundary on Windows), Windows testing is mandatory.

```yaml
ci-windows:
  runs-on: windows-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test --workspace --all-features
    - name: Tauri IPC trust boundary test
      run: cargo test --package prismatik-application tauri_ipc_trust_boundary_windows
```

---

## 4. `dst.yml` — Deterministic Simulation Testing

```yaml
dst:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Run DST over seed corpus
      run: |
        # Wave 0 floor: 64 seeds
        # Wave 3+ target: 256+ seeds
        # Wave 5+ target: 1000+ seeds
        cargo test --package prismatik-application dst_replay_suite -- --seed-file tests/dst/seeds.txt
```

**Verification method:** rerun each seed twice, diff TRACE-level logs byte-for-byte. A divergence is a build failure with the failing seed printed.

---

## 5. `chaos-weekly.yml` — Full Chaos Suite

```yaml
chaos-weekly:
  schedule:
    - cron: '0 8 * * 1'   # Monday 08:00 UTC
  runs-on: ubuntu-latest
  timeout-minutes: 120
  steps:
    - uses: actions/checkout@v4
    - name: Full chaos suite
      run: cargo test --package prismatik-application --test chaos --features full-chaos -- --include-ignored
```

Includes: network partitions, disk full, process kills mid-write, clock skew, OOM simulation.

---

## 6. `adversarial.yml` — Full Red-Team

```yaml
adversarial:
  schedule:
    - cron: '0 9 * * 1'   # Monday 09:00 UTC (after chaos)
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Inspect AI red-team scan
      run: |
        # AgentDojo-style prompt injection
        # OWASP ASI01 (Goal Hijack)
        # OWASP ASI02 (Tool Misuse)
        # TradeTrap faithfulness
        inspect eval evaluations/prismatik_redteam.toml --report
```

---

## 7. `release.yml` — Release Pipeline

### 7.1 Trigger

```yaml
on:
  push:
    tags: ['v*.*.*']
```

### 7.2 Build Matrix

```yaml
build:
  strategy:
    matrix:
      include:
        - os: ubuntu-latest
          target: x86_64-unknown-linux-gnu
        - os: windows-latest
          target: x86_64-pc-windows-msvc
        - os: macos-latest
          target: x86_64-apple-darwin
        - os: macos-latest
          target: aarch64-apple-darwin
  runs-on: ${{ matrix.os }}
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        targets: ${{ matrix.target }}
    - name: Build desktop app
      run: cargo build --release --target ${{ matrix.target }} --package apps-desktop
    - name: Build standalone verifier
      run: cargo build --release --target ${{ matrix.target }} --package prismatik-cli
    - name: Code sign (platform-specific)
      run: scripts/codesign.sh
      env:
        SIGNING_IDENTITY: ${{ secrets.SIGNING_IDENTITY }}
    - name: Generate SBOM
      run: cargo cyclonedx --manifest-path Cargo.toml --format json
    - name: SLSA provenance
      uses: slsa-framework/slsa-github-generator/.github/workflows/generator_container_slsa3.yml@v2.0.0
    - name: Cosign signing (keyless)
      run: cosign sign-blob --yes <artifact>
      env:
        COSIGN_EXPERIMENTAL: "1"
```

### 7.3 Release Artifact Verification (Updater)

The updater MUST verify artifacts before applying them:

```rust
// In prismatik-application updater
async fn apply_update(artifact: &UpdateArtifact) -> Result<(), UpdateError> {
    // 1. Verify SLSA provenance
    verify_slsa_provenance(&artifact.provenance)
        .map_err(|_| UpdateError::ProvenanceInvalid)?;

    // 2. Verify cosign signature (keyless via Rekor inclusion)
    verify_cosign_signature(&artifact.bytes, &artifact.signature)
        .map_err(|_| UpdateError::SignatureInvalid)?;

    // 3. Apply
    apply_artifact(&artifact.bytes).await
}
```

**Wave 0 DoD criterion 9:** the updater refuses an artifact with invalid SLSA provenance (committed negative test).

### 7.4 Release Gates

A release CANNOT be cut unless:
- All `ci.yml` gates green on the release branch.
- `dst.yml` green on 256+ seeds (Wave 3+).
- `chaos-weekly.yml` green for the past week.
- `adversarial.yml` green for the past week.
- `coverage.yml` shows core crates (`determinism`, `identity`, `audit`, `manifest`, `risk`) above 95%.
- All wave DoD criteria for the included waves passed.

---

## 8. `dependabot.yml` — Dependency

```yaml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 5
    groups:
      patch:
        update-types: ["patch"]
      minor:
        update-types: ["minor"]
  - package-ecosystem: "npm"
    directory: "/packages/ui"
    schedule:
      interval: "weekly"
```

**Critical pins (exact version, never caret):**
- `specta = "=2.0.0-rc.X"` (per v1.0 §6.3)
- `wasmtime = "=X.Y.Z"` (per v1.0 §6.4; bumping requires ADR)
- `rmcp = "=X.Y.Z"` (per v1.0 §6.5)
- `tauri = ">=2.12.0"` (per CVE-2026-42184)

A Dependabot PR bumping any of these MUST be reviewed manually before merge.

---

## 9. Failure Handling and Blockers

### 9.1 Blocking vs Warning

| Job | On failure |
|---|---|
| `lint-and-format` | Block |
| `determinism-grep` | Block |
| `unit-and-property-tests` | Block |
| `conformance` | Block |
| `bindings-drift` | Block |
| `registry-coverage` | Block |
| `deny-and-audit` | Block (security) |
| `golden-corpus` | Block (correctness) |
| `integration-tests` | Block |
| `dst` | Block (determinism) |
| `coverage` | Block for core crates (≥95%); warn for others |
| `benchmarks` | Warn (regression; tracked but not blocking) |

### 9.2 Quarantine

A flaky test MUST be quarantined within 24 hours of detection. Quarantine moves the test to `tests/quarantine/` with a reason file. A test in quarantine for more than 7 days MUST be either fixed or removed; indefinite quarantine is how test suites rot.

---

## 10. Workrun Artifacts

Every CI run produces:
- Test report (JUnit XML)
- Coverage report (lcov)
- DST trace digests (for golden corpus comparison)
- Benchmark history (CSV appended)
- Build artifacts (release pipeline only)

Stored for 90 days for diagnostic purposes.

---

## 11. Self-Hosted Runners (Wave 6+)

For enterprise customers running PRISMATIK on-prem, CI may run on self-hosted runners. The workflow definitions are identical; only the `runs-on:` differs.

For air-gapped operation (Disconnected Research profile), CI is replaced by a vendored offline build pipeline (`scripts/offline_build.sh`) that runs against pre-staged dependencies. This is documented in Wave 6.

---

## 12. Cross-References

| Topic | Document |
|---|---|
| Test catalog | `spec/TESTING.md` |
| Crate APIs | `spec/CRATE_ARCHITECTURE.md` |
| Threat model | `spec/SECURITY_THREAT_MODEL.md` |
| Release updater contract | `spec/MANIFEST_SCHEMA.md` §12 |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
