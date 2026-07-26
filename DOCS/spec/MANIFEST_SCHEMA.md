# PRISMATIK Reproducibility Manifest Schema

**Document:** `spec/MANIFEST_SCHEMA.md`
**Status:** NORMATIVE — RFC 2119 keywords apply. This schema is published under Apache-2.0 (per v1.0 §8 amendment 3) so third parties can verify PRISMATIK research bundles without installing PRISMATIK.
**Companion to:** `spec/CRATE_ARCHITECTURE.md` §2.3 (`prismatik-manifest`), `PRISMATIK_Unified_Solution_Architecture_v1.0.md` §12 (Determinism Kernel), §14.4 (verifiable research artifacts)
**Date:** 2026-07-26

---

## 0. Purpose

The reproducibility manifest is the **single artifact that makes a PRISMATIK research result third-party-verifiable**. It captures every input whose change would alter the result: the deterministic seed, the pinned artifact set (calendar, codebook, model weights, indicator kernel version), the dataset versions, the transform lineage, and the dual signature.

The central claim (v1.0 §1, §14.4): a manifest produced in July 2026 can be re-executed byte-for-byte in July 2029 on a different machine, with every input traceable to a provider endpoint and retrieval timestamp. **This is the product.** Everything else is a delivery vehicle for it.

This document specifies:
- The normative JSON schema (v1.0.0).
- Canonical serialization rules (byte-stable across machines).
- The dual-signature envelope (Ed25519 + ML-DSA).
- The standalone verifier contract.
- Bundle tarball format.

---

## 1. Top-Level Schema

```json
{
  "$schema": "https://prismatik.example/schemas/manifest-v1.json",
  "schema_version": "1.0.0",
  "manifest_id": "01HZX...",
  "run_id": "06d2e7e0-...",
  "kind": "backtest" | "walk_forward" | "monte_carlo" | "tsfm_forecast" | "calibration_fit" | "live_session" | "data_ingest",
  "produced_at": "2026-07-26T14:32:00.123456Z",
  "producer": {
    "prismatik_version": "1.0.0",
    "build_hash": "abc123...",
    "build_attestation": "cosign:...",
    "profile": "desktop" | "cloud" | "enterprise"
  },
  "determinism": { "...": "see §2" },
  "pinned_artifacts": { "...": "see §3" },
  "datasets": { "...": "see §4" },
  "lineage": { "...": "see §5" },
  "inputs": { "...": "see §6 (kind-specific)" },
  "metrics": { "...": "see §7 (kind-specific)" },
  "audit": { "...": "see §8" },
  "signature": { "...": "see §9 (dual)" },
  "extensions": {}
}
```

### Required Fields

Every manifest MUST carry: `schema_version`, `manifest_id`, `run_id`, `kind`, `produced_at`, `producer`, `determinism`, `pinned_artifacts`, `lineage`, `audit`, `signature`. `datasets` is required for kinds that consume data. `inputs` and `metrics` are kind-specific (see §6, §7).

---

## 2. Determinism Block

```json
"determinism": {
  "root_seed": 8675309,
  "clock_kind": "simulated" | "system" | "frozen",
  "clock_start": "2018-01-01T00:00:00Z",
  "clock_end": "2026-06-30T00:00:00Z",
  "clock_tick_step_micros": 1000000,
  "entropy_streams": [
    { "label": "backtest.engine", "seed": 12345 },
    { "label": "backtest.fill_model.slippage", "seed": 23456 },
    { "label": "simulation.path_0", "seed": 34567 },
    { "label": "simulation.path_1", "seed": 34568 }
  ],
  "thread_count": 4,
  "rayon_parallel": true,
  "libc_overrides": ["madsim::rand", "madsim::time"],
  "trace_digest": "blake3:abc..."
}
```

**The `entropy_streams` list is the load-bearing record.** Each labelled stream's seed is derived from the root seed deterministically; replay reconstructs the same streams in any order. This is what makes parallel Monte Carlo reproducible under Rayon, where completion order is not deterministic but stream identity is (v1.0 §12.2).

`trace_digest` is the BLAKE3 of the captured TRACE-level log of the run (per S2/DST methodology). A re-execution that produces a different trace digest is a determinism leak.

**`thread_count` is required, not optional.** Wave 3 DoD criterion 7 verifies byte-identical results across thread counts (1, 4, 16). The manifest records the count so reproduction can verify it on the same count and test divergence on different counts.

---

## 3. Pinned Artifacts Block

Every external artifact whose change would alter the result. If it is not in here, it cannot influence a run.

```json
"pinned_artifacts": {
  "calendar": {
    "artifact_id": "cal-nyse-2026-07-26",
    "kind": "calendar",
    "version": "2026.07.26",
    "content_hash": "blake3:abc123...",
    "signature": { "...": "see §9 sub-envelope" }
  },
  "symbology_snapshot": {
    "artifact_id": "sym-2026-07-26",
    "kind": "symbology_snapshot",
    "version": "2026.07.26",
    "content_hash": "blake3:def456...",
    "signature": { "..." : "..." }
  },
  "corporate_actions": {
    "artifact_id": "ca-2026-07-26",
    "kind": "corporate_action_ledger",
    "version": "2026.07.26",
    "content_hash": "blake3:ghi789...",
    "signature": { "..." : "..." }
  },
  "codebooks": [
    {
      "artifact_id": "kronos-tokenizer-base-1.0",
      "kind": "tokenizer_codebook",
      "version": "1.0.0",
      "content_hash": "blake3:jkl012...",
      "signature": { "..." : "..." }
    }
  ],
  "models": [
    {
      "artifact_id": "kronos-base-aaai2026",
      "kind": "model_weights",
      "version": "1.0.0",
      "content_hash": "blake3:mno345...",
      "pretraining_cutoff": "2025-12-31",
      "license_spdx": "MIT",
      "license_class": "permissive_commercial",
      "signature": { "..." : "..." }
    }
  ],
  "indicator_kernel_version": "1.0.0",
  "plugin_modules": [
    {
      "artifact_id": "plugin-custom-vol-0.1",
      "kind": "plugin_module",
      "version": "0.1.0",
      "content_hash": "blake3:pqr678...",
      "capabilities_granted": { "..." : "see spec/CRATE_ARCHITECTURE.md §4.1" },
      "signature": { "..." : "..." }
    }
  ]
}
```

### Hash Verification on Load

When the runtime loads an artifact during a re-execution, it computes BLAKE3 of the artifact bytes and compares to `content_hash`. **A mismatch is a security event, not an I/O error.** The verifier refuses to proceed; the audit ledger records `ArtifactHashMismatch`.

### Pretraining Cutoff Enforcement

For `model_weights` artifacts, `pretraining_cutoff` is REQUIRED. The backtest runtime hard-denies any model whose cutoff is at or after the backtest start (Wave 3 DoD criterion 3):

```rust
if entry.pretraining_cutoff >= backtest_window.start {
    return Err(RegistryError::PretrainingContamination { ... });
}
```

This is silent-contamination prevention: a model pretrained on data that overlaps the test window contaminates the result.

---

## 4. Datasets Block

For data-consuming kinds, every dataset version read.

```json
"datasets": {
  "raw": [
    {
      "dataset_version": "lancedb:raw_market_data:v42",
      "time_range": ["2018-01-01T00:00:00Z", "2026-06-30T00:00:00Z"],
      "row_count": 12345678,
      "content_hash": "blake3:..."
    }
  ],
  "curated": [
    {
      "dataset_version": "lancedb:curated_bar:daily:v17",
      "time_range": ["2018-01-01T00:00:00Z", "2026-06-30T00:00:00Z"],
      "row_count": 2345678,
      "content_hash": "blake3:..."
    }
  ],
  "features": [
    {
      "view_id": "06d2e7e0-...",
      "view_version": "1.2.0",
      "materialized_at": "2026-07-15T00:00:00Z",
      "dataset_version": "lancedb:feature_value:rsi_14:v3",
      "row_count": 345678,
      "content_hash": "blake3:..."
    }
  ]
}
```

**LanceDB's automatic versioning is what makes this a pin rather than a hope.** A future reproduction reads the rows that existed at manifest time, not the rows that exist now.

---

## 5. Lineage Block

```json
"lineage": {
  "transform_id": "backtest.event_loop",
  "transform_version": "1.0.0",
  "strategy_ir_hash": "blake3:...",           // for backtest/walk_forward
  "execution_assumptions_hash": "blake3:...",  // fill models, slippage, commission
  "upstream_record_ids": ["abc...", "def..."],
  "invalidation_hash": "blake3:..."            // includes pinned set per spec/CRATE_ARCHITECTURE.md §13.2
}
```

**The invalidation hash MUST include the `PinnedArtifactSet`.** Without this, calendar/codebook/model changes silently pass through lineage checks. The corrected definition (v1.0 §13.2):

```rust
pub fn invalidation_hash(&self, pinned: &PinnedArtifactSet) -> ContentHash {
    let mut h = blake3::Hasher::new();
    let mut ids: Vec<_> = self.upstream_record_ids.iter().collect();
    ids.sort_unstable();
    for id in ids { h.update(id.as_bytes()); }
    h.update(self.transform_id.as_bytes());
    h.update(self.transform_version.to_string().as_bytes());
    h.update(&pinned.stable_digest());
    ContentHash(h.finalize().into())
}
```

---

## 6. Kind-Specific Inputs

### 6.1 `backtest`

```json
"inputs": {
  "universe": ["asset_id_1", "asset_id_2"],
  "period": ["2018-01-01T00:00:00Z", "2026-06-30T00:00:00Z"],
  "starting_capital": "100000",
  "execution_assumptions": {
    "fill_model": "next_open",
    "slippage_model": { "kind": "linear", "bps_per_dollar_volume": "0.0005" },
    "commission_model": { "kind": "per_share", "per_share": "0.005", "min_per_order": "1.00" }
  },
  "session_filter": "regular_only",
  "bar_kind": "time",
  "bar_interval_seconds": 86400,
  "walk_forward_config": {
    "windows": 10,
    "train_size": 0.8,
    "purge_window_bars": 5,
    "embargo_window_bars": 5
  }
}
```

### 6.2 `monte_carlo`

```json
"inputs": {
  "path_count": 1000000,
  "horizon_bars": 252,
  "sources": [
    { "kind": "bootstrap", "source_dataset_version": "...", "weight": "0.5" },
    { "kind": "parametric", "params": { "drift": "0.05", "vol": "0.15" }, "weight": "0.3" },
    { "kind": "tsfm_generator", "model_artifact_id": "kronos-base-aaai2026", "weight": "0.2" }
  ]
}
```

### 6.3 `tsfm_forecast`

```json
"inputs": {
  "asset_id": "...",
  "model_artifact_id": "kronos-base-aaai2026",
  "tokenizer_artifact_id": "kronos-tokenizer-base-1.0",
  "context_window_bars": 400,
  "horizon_bars": 120,
  "temperature": "0.7",
  "top_p": "0.95",
  "sample_count": 100,
  "calibration_record_id": "..."
}
```

The forecast's calibration record is REQUIRED here (Invariant I2). A forecast without calibration cannot exist as a type.

### 6.4 `live_session`

```json
"inputs": {
  "session_id": "...",
  "strategy_ir_hash": "...",
  "broker_adapter": "alpaca",
  "broker_account_fingerprint": "blake3:...",  // NOT the account ID
  "session_start": "2026-07-26T14:32:00Z",
  "session_end": "2026-07-26T21:00:00Z",
  "live_confirm_hash": "blake3:..."  // hash of the live_confirm phrase
}
```

**Never record secrets.** The broker account is recorded by fingerprint (BLAKE3 of account ID), not by value. The live_confirm is recorded by hash, not by phrase.

---

## 7. Kind-Specific Metrics

### 7.1 `backtest`

```json
"metrics": {
  "total_return": "0.423",
  "annual_return": "0.085",
  "sharpe": "1.42",
  "sortino": "1.85",
  "max_drawdown": "-0.187",
  "calmar": "0.45",
  "win_rate": "0.56",
  "profit_factor": "1.78",
  "trade_count": 234,
  "excess_return_vs_buy_hold": "0.123",          // MANDATORY per Wave 4
  "buy_hold_return_same_period": "0.300",
  "deflated_sharpe": "1.18",                      // multiple-testing-adjusted
  "configs_searched": 47,                          // multiple-testing disclosure
  "configurations": [
    { "params": { "sma_fast": 50, "sma_slow": 200 }, "sharpe": "1.42" },
    ...
  ]
}
```

**`excess_return_vs_buy_hold` and `configs_searched` are non-optional.** A strategy result that omits them MUST fail validation, not render (per v1.2 §3.11). Multiple-testing bias is how these products quietly lie.

### 7.2 `monte_carlo`

```json
"metrics": {
  "terminal_value_mean": "142300",
  "terminal_value_p05": "78900",
  "terminal_value_p95": "234500",
  "drawdown_p95": "-0.342",
  "probability_of_ruin": "0.023",
  "coverage_realized": "0.91",
  "coverage_nominal": "0.90",
  "per_regime_coverage": { "narrow": "0.92", "volatile": "0.78" }
}
```

### 7.3 `tsfm_forecast`

```json
"metrics": {
  "pinball_loss": "0.024",
  "crps": "0.031",
  "coverage_realized": "0.91",
  "coverage_nominal": "0.90",
  "mean_interval_width": "0.045",
  "per_regime_coverage": { "narrow": "0.92", "volatile": "0.78" }
}
```

---

## 8. Audit Block

```json
"audit": {
  "audit_tree_size_at_run_start": 1234567,
  "audit_tree_size_at_run_end": 1234589,
  "audit_root_hash_at_run_end": "blake3:...",
  "inclusion_proof": {
    "leaf_position": 1234570,
    "hash_path": ["blake3:...", "blake3:...", ...],
    "verified_against_tree_size": 1234589
  },
  "signed_tree_head": { "..." : "see §9 sub-envelope" }
}
```

The audit inclusion proof demonstrates that the manifest's run records are part of the Merkle tree. A third party with the signed tree head can verify inclusion without trusting the manifest producer.

---

## 9. Signature Block (Dual Signature)

```json
"signature": {
  "scheme": "dual-ed25519-ml-dsa-65",
  "canonical_serialization": "blake3:...",         // hash of canonical bytes (see §10)
  "ed25519": {
    "public_key": "base64:...",
    "signature": "base64:...",
    "signing_identity": {
      "kind": "operator" | "ci" | "service",
      "id": "aaron@mythos.systems",
      "signed_at": "2026-07-26T14:32:01Z"
    }
  },
  "ml_dsa": {
    "public_key": "base64:...",
    "signature": "base64:...",
    "signing_identity": { "...": "..." }
  }
}
```

### Why Dual Signature

- **Ed25519** — fast, widely supported, classical-security.
- **ML-DSA-65** (FIPS 204) — post-quantum-resistant. Forward security against "harvest now, decrypt later" threats.

**Both signatures are required.** A single valid signature fails verification (Wave 0 DoD criterion 6). The dual-scheme approach future-proofs against quantum adversaries while remaining practical today.

### Wave 0 Conditional Shipping

Per Wave 0 risks: if ML-DSA Rust implementation maturity is uneven, ship Ed25519-only with the dual-signature *format* in place and the `ml_dsa` field present but `signature: null`. The retrofit is field population, not schema migration. Recorded as an explicit exception in the PQC register.

---

## 10. Canonical Serialization

For byte-stable signatures, the manifest is canonicalized before signing:

1. **Sort object keys** lexicographically (recursive).
2. **Sort array elements** by their canonical bytes (where order doesn't carry meaning; for `entropy_streams`, `models`, `datasets`, the order IS significant and preserved).
3. **UTF-8 encode** all strings, NFC-normalized.
4. **No whitespace** in serialization (compact JSON).
5. **BLAKE3** of the canonical bytes is the signed digest.

```rust
pub fn canonical_bytes(manifest: &Manifest) -> Vec<u8> {
    let mut value = serde_json::to_value(manifest).unwrap();
    canonicalize_in_place(&mut value);  // recursive key sort, NFC normalize
    serde_json::to_vec(&value).unwrap()  // compact, no whitespace
}
```

**Re-execution produces byte-identical canonical bytes.** If it doesn't, the manifest is not reproducible and the verification fails.

---

## 11. Bundle Tarball Format

A research bundle (v1.0 §14.4) is a tarball:

```
<run_id>/
├── manifest.json                 # canonical JSON of this schema
├── manifest.sig                  # dual signature envelope
├── pinned_artifacts.json         # artifact refs with hashes (NOT bytes)
├── metrics.json                  # kind-specific metrics
├── audit_inclusion.json          # inclusion proof
├── README.md                     # human-readable summary
└── (optional) trace.log.gz       # compressed TRACE-level log; trace_digest matches
```

The bundle does NOT contain artifact bytes — only hashes. Verification requires the verifier to have artifacts locally (or fetch from a content-addressed store).

---

## 12. Standalone Verifier Contract

`prismatik-cli verify <bundle.tar>` MUST:

1. **NOT link any application code** — only `prismatik-manifest::verify`, plus crypto primitives.
2. **NOT require a PRISMATIK installation.**
3. **NOT make network calls.**
4. Exit 0 if and only if ALL of:
   - Manifest schema version is supported (`1.0.x`).
   - Every pinned artifact's `content_hash` matches its locally-available bytes (or, if not available, the artifact is reported as `not_verifiable_content` and verification exits 1 with that reason).
   - Dual signature verifies: `ed25519` AND `ml_dsa` (unless `ml_dsa.signature` is null AND the verifier is in `--allow-pq-pending` mode for transitional Wave 0 bundles).
   - Audit inclusion proof is valid against the manifest's signed tree head.
   - Canonical serialization regenerates the recorded `canonical_serialization` digest.
5. Print a human-readable report on success or failure, with per-check pass/fail.
6. Print machine-readable JSON with `--json` flag for CI integration.

```rust
pub struct VerificationReport {
    pub schema_version_ok: bool,
    pub artifact_hashes_ok: Vec<(ArtifactId, bool)>,
    pub signature_ed25519_ok: bool,
    pub signature_ml_dsa_ok: bool,
    pub audit_inclusion_ok: bool,
    pub canonical_serialization_ok: bool,
    pub overall: bool,
    pub verified_at: OffsetDateTime,
    pub verifier_version: String,
}
```

**The standalone verifier is the public trust surface.** Its binary MUST be reproducibly built, cosign-signed, and the build attestation published. A third party can build it themselves from the Apache-2.0 source and confirm.

---

## 13. Schema Versioning and Compatibility

Within v1.x.y:
- Additive field additions are backward-compatible.
- Adding a required field requires a minor version bump (v1.1.0) and a migration tool.
- Removing or repurposing a field requires v2.0.

The verifier supports a range `[MIN_SUPPORTED, CURRENT]`. Bundles outside the range fail with `IpcError::PreconditionsFailed { code: "manifest_version_out_of_range" }`.

The schema is published at `https://prismatik.example/schemas/manifest-v1.json` and in `packages/schemas/manifest-v1.json` (Apache-2.0).

---

## 14. Extensions Block

```json
"extensions": {
  "org.example.custom_metric": { "...": "..." }
}
```

Extensions are namespaced by reverse-DNS. The core verifier IGNORES unknown extensions (forward-compatible). Enterprise plugins MAY define their own extension schemas. The schema-publishing surface (`packages/schemas/`) is where extension schemas are published.

---

## 15. Cross-References

| Topic | Document |
|---|---|
| Determinism Kernel | `spec/CRATE_ARCHITECTURE.md` §1.1 |
| Manifest crate API | `spec/CRATE_ARCHITECTURE.md` §2.3 |
| Audit ledger (inclusion proofs) | `spec/CRATE_ARCHITECTURE.md` §2.2 |
| Strategy IR hash reference | `spec/STRATEGY_IR.md` |
| Data versioning (Lance) | `spec/DATA_SCHEMAS.md` §5 |
| CI verification gates | `spec/CI_WORKFLOWS.md` |
| Threat model (manifest tampering) | `spec/SECURITY_THREAT_MODEL.md` |
| Testing corpus | `spec/TESTING.md` |

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
