# Wave 0 — Foundation

**Maps to v1.0 phases:** P0 (Foundation, Governance, and the Determinism Kernel)
**Business outcome:** the non-negotiable floor — deterministic, evidence-chained, sandboxed shell. **Not a product.** Produces nothing a customer can see.
**Duration:** ~72 days total. **~24 days if scoped to `P0-FLOOR` under Option B** (the recommended path — see Enterprise Overview Part IV.1).
**Date:** 2026-07-26

---

## Objective

Establish the mechanisms whose absence would make every later wave more expensive: deterministic time and entropy, canonical identity, tamper-evident audit, signed reproducibility manifests, generated type bindings, sandboxed shell, and the CI gates that keep all of it true.

**Entry criteria.** None. This is the start of the project.

## Why This Wave Exists

v1.0 Architecture §12 documents seven doors through which nondeterminism enters a Rust application of this size: `Instant::now` and `SystemTime::now` anywhere in the process or transitive dependencies; `getrandom`/`getentropy` at the libc level; `HashMap`/`HashSet` iteration order (randomized for DoS resistance); task-scheduling order in a multi-threaded tokio runtime; floating-point reduction order under Rayon parallelism; network/disk timing affecting retry/timeout/cache eviction paths; and silently upgraded reference data (calendars, tokenizer codebooks). A seed in a manifest addresses door one partially and nothing else. The Determinism Kernel addresses all seven.

Most of this wave cannot be retrofitted cheaply, and the cost of retrofitting rises multiplicatively with every line of code written against a foundation that doesn't have it. **Wave 0 is the single largest risk in the plan** for a one-engineer commercial operation because it produces no customer-visible artifact — which is exactly why Option B exists (see "Sequencing" below).

## Work Items

`FLOOR` marks items in the Option B minimum. `DEFER` marks items moved to `P0-REMAINDER` under Option B.

| ID | Track | Work item | Days | Conf | Option B |
|---|---|---|---:|:---:|:---:|
| P0-OD-01 | OD | Cargo workspace, crate skeletons, `rust-toolchain.toml` pinned, `.editorconfig`, `rustfmt.toml` | 2 | H | FLOOR |
| P0-OD-02 | OD | `clippy.toml` with determinism `disallowed-methods` and `disallowed-types`, wired to `-D warnings` | 1 | H | FLOOR |
| P0-OD-03 | OD | GitHub Actions CI skeleton: fmt, clippy, test, matrix on Windows and Linux | 2 | H | FLOOR |
| P0-OD-04 | OD | Project workbook, ADR template and index, `CONTRIBUTING.md`, `SECURITY.md` | 2 | H | FLOOR |
| P0-DK-01 | DK | `prismatik-determinism`: `Clock` trait with `SystemClock`, `SimulatedClock`, `FrozenClock` | 3 | M | FLOOR |
| P0-DK-02 | DK | `Entropy` trait, splittable stream implementation, split-order-independence property test | 4 | M | FLOOR |
| P0-DK-03 | DK | `DetMap`, `DetSet`, `DeterminismContext`, `PinnedArtifactSet` types | 2 | H | FLOOR |
| P0-DK-04 | DK | `determinism-grep` CI gate plus a deliberate-violation test proving the gate fails | 2 | M | FLOOR |
| P0-DK-05 | DK | `prismatik-identity`: `AssetId`, `VenueId`, `ExternalIdentifier` enum, id factory backed by `Entropy` | 3 | M | FLOOR |
| P0-DK-06 | DK | Bitemporal symbology store: schema, `resolve_as_of`, identity chain, as-of property test | 6 | M | DEFER |
| P0-DK-07 | DK | `prismatik-audit`: Merkle append-only ledger, inclusion proof, consistency proof | 6 | M | DEFER |
| P0-DK-08 | DK | Audit startup verification plus `criterion` benchmark against the 1ms p99 append budget | 2 | M | DEFER |
| P0-DK-09 | DK | `prismatik-manifest`: schema v1.0 types, canonical serialization, builder | 4 | M | DEFER |
| P0-DK-10 | DK | Dual signature (Ed25519 + ML-DSA), sign and verify, cross-machine verification test | 5 | L | DEFER |
| P0-DK-11 | DK | `prismatik-cli verify` standalone verifier, no application dependency | 3 | M | DEFER |
| P0-DK-12 | DK | `ArtifactStore` trait, content-addressed storage, hash and signature verification on load | 3 | M | DEFER |
| P0-DK-13 | DK | `prismatik-calendar`: artifact format, `SessionCalendar` trait, reader | 3 | M | DEFER |
| P0-DK-14 | DK | Calendar artifact generator script with QuantLib cross-validation and zero-tolerance gate | 5 | L | DEFER |
| P0-SS-01 | SS | Tauri capability policy, default deny, strict CSP with no inline or eval — **Tauri patched ≥2.12 per CVE-2026-42184** | 3 | M | FLOOR |
| P0-SS-02 | SS | Key hierarchy: OS keychain integration, device root key, derived key types, `secrecy` and `zeroize` | 5 | M | DEFER |
| P0-SS-03 | SS | `deny.toml` license allowlist and advisory policy, blocking in CI | 2 | H | FLOOR |
| P0-SS-04 | SS | `cargo vet init`, import Mozilla, Google, Bytecode Alliance audit sets, trusted-core policy | 3 | M | DEFER |
| P0-SS-05 | SS | `prismatik-oss-registry`: manifest schema, license-class gate, coverage CI check | 4 | M | DEFER |
| P0-SS-06 | SS | Release signing with cosign, SBOM via `cargo cyclonedx`, SLSA provenance generation | 4 | L | DEFER |
| P0-SS-07 | SS | Updater with provenance **verification** and a test proving it refuses an invalid artifact | 4 | L | DEFER |
| P0-EX-01 | EX | Tauri 2 shell, window management, crash recovery, single instance | 3 | M | FLOOR |
| P0-EX-02 | EX | `tauri-specta` bindings pipeline, exact version pins, `bindings-drift` CI gate | 3 | M | FLOOR |
| P0-EX-03 | EX | Design tokens package, CSS variable theme, light and dark, contrast validation | 4 | M | FLOOR |
| P0-EX-04 | EX | Motion system primitives, reduced motion support, first five production components | 5 | M | FLOOR |
| P0-QM-01 | QM | Skeleton `dst_replay_suite` on a trivial pipeline, trace capture and digest comparison | 4 | L | DEFER |
| P0-QM-02 | QM | Golden manifest corpus harness plus first two committed manifests | 3 | M | DEFER |

**Floor subtotal: 24 days. Deferred subtotal: 48 days. Phase total: 72 days.**

## Sequencing Within Wave 0

The internal order matters because several items are blocking.

```text
Week 1-2   P0-OD-01 ─▶ P0-OD-02 ─▶ P0-OD-03 ─▶ P0-OD-04
                          │
Week 2-4                  ├─▶ P0-DK-01 ─▶ P0-DK-02 ─▶ P0-DK-03 ─▶ P0-DK-04
                          │                                          │
Week 4-5                  └─▶ P0-DK-05 ◀───────────────────────────┘
                                  │
Week 5-7   P0-EX-01 ─▶ P0-SS-01 ─▶ P0-EX-02 ─▶ P0-EX-03 ─▶ P0-EX-04
                                                    │
           ══════ OPTION B FLOOR COMPLETE (day 24) ═╪══════ proceed to Wave 1
                                                    │
Week 8-10  P0-DK-07 ─▶ P0-DK-08 ─▶ P0-DK-09 ─▶ P0-DK-10 ─▶ P0-DK-11
Week 11-12 P0-DK-12 ─▶ P0-DK-13 ─▶ P0-DK-14
Week 13-14 P0-SS-02 ─▶ P0-SS-04 ─▶ P0-SS-05 ─▶ P0-SS-06 ─▶ P0-SS-07
Week 15    P0-DK-06 ─▶ P0-QM-01 ─▶ P0-QM-02
```

`P0-DK-04`, the grep gate, **must land before any other crate is written.** Its entire value is preventing violations from accumulating, and a gate introduced after violations exist gets an allowlist, and an allowlist is how gates die.

## Definition of Done

A wave is complete when every line below is demonstrably true, verified by a named artifact rather than by assertion.

| # | Criterion | Verified by |
|---|---|---|
| 1 | No call to `Instant::now`, `SystemTime::now`, `thread_rng`, or a default-hasher map exists outside the determinism-crate allowlist | `determinism-grep` green; allowlist reviewed and under 10 entries |
| 2 | The grep gate demonstrably fails on an introduced violation | Committed negative test |
| 3 | Entropy split is order independent | `proptest` case, 10k iterations |
| 4 | A trivial pipeline replays byte identically across 64 seeds | `dst_replay_suite` green |
| 5 | A manifest signed on machine A verifies on machine B with no shared state | Manual cross-machine run, recorded in the workbook |
| 6 | Both signatures are required; a single valid signature fails verification | Committed negative test |
| 7 | The audit ledger detects a retroactive edit | Committed tamper test |
| 8 | Audit append meets 1ms p99 | `criterion` report committed |
| 9 | The updater refuses an artifact with invalid SLSA provenance | Committed negative test |
| 10 | Generated TypeScript bindings match the Rust source | `bindings-drift` green |
| 11 | Every workspace dependency appears in the component registry | `registry-coverage` green |
| 12 | Calendar generator produces zero disagreements against QuantLib for US equity venues | Generator report committed |
| 13 | The Tauri capability policy contains no `fs`, `http`, `process`, or `shell:execute` permission; Tauri ≥2.12 | Policy file review + version pin, recorded in ADR |
| 14 | ADRs 0020 through 0028 are written and accepted | ADR index |

**Under Option B**, criteria 1, 2, 3, 10, and 13 gate the floor. The remainder gate the opening of Wave 3.

## Why Most of This Cannot Be Deferred

The four items that cannot be deferred are the multiplicative-retrofit items:

- **`Clock` and `Entropy` traits plus `DeterminismContext`** — every call site written without them must be rewritten. This is the multiplicative one.
- **Determinism lints and the grep gate** — a gate added later has to be paid down against an existing violation set, which is how gates get disabled.
- **`AssetId` as the internal canonical identifier** — every table, every struct, every function signature. Changing the identity type later touches everything.
- **Tauri shell with capability policy and strict CSP** — loosening a policy later is easy. Tightening one after features depend on the looseness is not.

Everything else (the Merkle audit ledger, the manifest builder, dual signatures, calendar artifacts, the component registry, the golden manifest corpus, the full DST suite) is **additive rather than invasive**. Adding an audit ledger to a system that already has a clock trait is a bounded piece of work. Adding a clock trait to a system that does not have one is not.

## Risks

| Risk | Response |
|---|---|
| ML-DSA implementation maturity in Rust is uneven; `P0-DK-10` could run 2× | Time-box to 8 days. If it overruns, ship Ed25519 only with the dual-signature *format* in place and the ML-DSA field present but empty, so the retrofit is a field population rather than a schema migration. Record as an explicit exception in the PQC register. |
| Specta v2 RC breaks on a Tauri patch release | Exact pins, and a `cargo update` is a deliberate reviewed act. Fallback ADR-0031 already written. |
| Determinism lints produce excessive friction and get disabled | The allowlist is capped at 10 entries and every addition requires a one-line justification in the file. A cap makes pressure visible. |
| The 24-day floor slips to 40 and Option B loses its advantage | Weekly checkpoint against the floor item list. If day 30 arrives with the floor incomplete, cut `P0-EX-03` and `P0-EX-04` to a single unstyled component set and finish the styling during Wave 1. |

## Sequencing Options

This wave is the one where the Option A vs Option B choice (Enterprise Overview Part IV.1) is operative.

### Option A — Architecture Order
Run all 72 days before any other wave opens. Zero retrofit cost; every subsequent line of code written against the kernel from the start; reproducibility claim true from the first commit. Cost: ~128 days before anything demonstrable to a prospect. **Choose if funded, or if PRISMATIK is a long-horizon asset with no near-term revenue requirement.**

### Option B — Revenue-First with a Determinism Floor (RECOMMENDED)
Ship the 24-day floor; defer the 48-day remainder to interleave across Waves 1–2, funded by revenue. **First sellable artifact at ~80 days instead of ~128** — a 37% reduction in time to first revenue, at the cost of carrying an explicit, tracked, and time-boxed deferral.

**The condition, non-negotiable:** the deferred items are entered in the issue tracker on day one with their original identifiers, a stated deferral reason, and a hard deadline of "before Wave 3 begins." Wave 3 introduces the strategy runtime and the backtest engine, which is the first point where a missing manifest or a missing audit ledger becomes a correctness problem rather than a missing feature. **If `P0-REMAINDER` is not complete when Wave 3 opens, Wave 3 does not open.**

Choose Option B if the operation needs revenue or external validation before committing multiple years, which for a solo commercial operation is the usual case.

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26*
