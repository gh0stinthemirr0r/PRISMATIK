# PRISMATIK Data Schemas Specification

**Document:** `spec/DATA_SCHEMAS.md`
**Status:** NORMATIVE — RFC 2119 keywords apply
**Companion to:** `spec/CRATE_ARCHITECTURE.md` §3 (domain crates), `PRISMATIK_Unified_Solution_Architecture_v1.0.md` §15 (data plane)
**Date:** 2026-07-26

---

## 0. Purpose

This document specifies the **exact on-disk shape** of every persistent structure in PRISMATIK: Parquet schemas for the six data layers, LanceDB table schemas for embeddings and analog search, and the SQLite operational schema with numbered forward-only migrations. An engineer implementing storage MUST conform to these schemas exactly; deviations require an ADR.

**The cardinal rule (v1.0 §15.1):** raw is append-only and never rewritten, including when the provider was wrong. A provider correction creates a new raw record with its own `retrieved_at` and a supersession link. This is what makes "what did we know at 09:31" answerable.

---

## 1. The Six Data Layers

| Layer | Content | Mutable | Store | Retention |
|---|---|:---:|---|---|
| **Raw** | Verbatim provider payload + request metadata | **never** | Parquet, gzip, partitioned by provider/date | Per entitlement, default 7 years |
| **Normalized** | Canonical types, no interpretation | never | Parquet | Recomputable, 2 years hot |
| **Curated** | Quality-scored, deduplicated, conflict-resolved | superseded only | Parquet + DuckDB views | 7 years |
| **Feature** | Point-in-time-correct derived values | versioned | Parquet offline, SQLite online | Recomputable, 1 year hot |
| **Intelligence** | Model output, scores, forecasts, embeddings | versioned | Parquet + LanceDB | 3 years |
| **Presentation** | User-scoped materialized views | disposable | SQLite | Cache, evictable |

---

## 2. Parquet Schemas (Raw, Normalized, Curated, Feature, Intelligence)

### 2.1 Common Conventions

- **Encoding:** Parquet with `SNAPPY` compression for hot layers, `ZSTD(19)` for cold archives.
- **Partitioning:** `provider=<id>/<year>/<month>/` for raw; `<asset_class>/<year>/` for normalized onward.
- **Time semantics:** every timestamp is `timestamp_micros` UTC. `event_time` is the provider's own timestamp for the event; `retrieved_at` is when we fetched it. Freshness is measured against `event_time`, NOT `retrieved_at`.
- **Asset identity:** `asset_id` (BLAKE3 of canonical record) is the join key. External identifiers are attributes, never keys.
- **Decimal discipline:** monetary quantities are `DECIMAL(128, 18)` encoded as `FIXED_LEN_BYTE_ARRAY(16)` per the Parquet logical type. At the Rust boundary these become `rust_decimal::Decimal`. NEVER float.
- **Provenance:** every record carries `provider`, `retrieved_at`, `evidence_hash` (BLAKE3 of canonical record content at write time — tamper detection on read).

### 2.2 Raw Layer Schemas

Raw is verbatim. The schema is intentionally generic; provider-specific fields live in a `payload BYTES` column containing the original JSON/BSON/MessagePack blob, base64'd.

#### `raw_market_data` (one row per fetched observation)

| Column | Type | Notes |
|---|---|---|
| `request_id` | `BYTE_ARRAY` (UUID) | Unique per HTTP request |
| `provider` | `INT16` | `ProviderId` |
| `endpoint` | `BYTE_ARRAY` | Provider's endpoint path |
| `asset_id` | `BYTE_ARRAY(32)` | Resolved at ingest time |
| `event_time` | `INT64` (timestamp_micros UTC) | Provider's own timestamp |
| `retrieved_at` | `INT64` (timestamp_micros UTC) | Our fetch time |
| `request_params` | `BYTE_ARRAY` (JSON) | The params we sent |
| `response_status` | `INT32` | HTTP status |
| `response_headers` | `BYTE_ARRAY` (JSON) | Selective: rate-limit, etag, cache-control |
| `payload` | `BYTE_ARRAY` | Verbatim response body |
| `payload_format` | `BYTE_ARRAY` | `json` / `msgpack` / `csv` / `parquet` |
| `evidence_hash` | `BYTE_ARRAY(32)` | BLAKE3 of canonical record |
| `quality` | `INT8` | Initial quality score (curated layer refines) |
| `supersedes` | `BYTE_ARRAY` (UUID, optional) | Request ID this corrects |

**Append-only enforcement:** the writer uses Parquet's `_metadata` file with a `commit_hook` that refuses overwrites. A new write with an existing `(request_id, retrieved_at)` pair MUST fail at the writer level.

#### `raw_filing` (one row per fetched SEC/EDGAR filing)

| Column | Type | Notes |
|---|---|---|
| `filing_id` | `BYTE_ARRAY` (UUID) | |
| `provider` | `INT16` | Usually `SEC_EDGAR` |
| `cik` | `INT64` | |
| `form_type` | `BYTE_ARRAY` | `10-K`, `13F`, `8-K`, etc. |
| `accession_no` | `BYTE_ARRAY` | EDGAR accession number |
| `filing_date` | `INT32` (date) | The filing date — observability hinge |
| `period_end` | `INT32` (date, optional) | For periodic filings |
| `retrieved_at` | `INT64` (timestamp_micros UTC) | |
| `payload` | `BYTE_ARRAY` | Full filing document |
| `payload_format` | `BYTE_ARRAY` | `xbrl` / `html` / `xml` |
| `evidence_hash` | `BYTE_ARRAY(32)` | |

#### `raw_flow_print` (one row per options flow print)

| Column | Type | Notes |
|---|---|---|
| `print_id` | `BYTE_ARRAY` (UUID) | |
| `provider` | `INT16` | Usually `UNUSUAL_WHALES` |
| `contract_asset_id` | `BYTE_ARRAY(32)` | |
| `event_time` | `INT64` (timestamp_micros UTC) | Print time |
| `retrieved_at` | `INT64` (timestamp_micros UTC) | |
| `payload` | `BYTE_ARRAY` | Verbatim JSON |
| `evidence_hash` | `BYTE_ARRAY(32)` | |

### 2.3 Normalized Layer Schemas

Normalized records carry canonical types with no provider-specific shape.

#### `normalized_bar` (OHLCV)

| Column | Type | Notes |
|---|---|---|
| `asset_id` | `BYTE_ARRAY(32)` | |
| `bar_kind` | `BYTE_ARRAY` | `time` / `tick` / `volume` / `dollar` |
| `bar_interval` | `INT64` | Seconds for time; threshold for others |
| `bar_start` | `INT64` (timestamp_micros UTC) | |
| `bar_end` | `INT64` (timestamp_micros UTC) | |
| `open` | `DECIMAL(128, 18)` | |
| `high` | `DECIMAL(128, 18)` | |
| `low` | `DECIMAL(128, 18)` | |
| `close` | `DECIMAL(128, 18)` | |
| `volume` | `DECIMAL(128, 18)` | |
| `notional` | `DECIMAL(128, 18)` | Volume × price; nullable |
| `trade_count` | `INT64` | Nullable |
| `provider` | `INT16` | The provider actually used (NOT the configured chain) |
| `event_time` | `INT64` (timestamp_micros UTC) | Same as `bar_end` for bars |
| `retrieved_at` | `INT64` (timestamp_micros UTC) | |
| `evidence_hash` | `BYTE_ARRAY(32)` | |
| `quality` | `INT8` | Curated-layer-adjusted |

**Critical rule (v1.2 §3.5):** `provider` records the provider ACTUALLY used per request, NOT the configured chain. A backtest that silently mixed fallback-provider bars with primary-provider bars is not reproducible.

#### `normalized_quote`

| Column | Type | Notes |
|---|---|---|
| `asset_id` | `BYTE_ARRAY(32)` | |
| `event_time` | `INT64` (timestamp_micros UTC) | |
| `bid` | `DECIMAL(128, 18)` | |
| `ask` | `DECIMAL(128, 18)` | |
| `bid_size` | `DECIMAL(128, 18)` | |
| `ask_size` | `DECIMAL(128, 18)` | |
| `mid` | `DECIMAL(128, 18)` | Derived |
| `spread_bps` | `DECIMAL(128, 18)` | Derived |
| `provider` | `INT16` | |
| `retrieved_at` | `INT64` (timestamp_micros UTC) | |
| `evidence_hash` | `BYTE_ARRAY(32)` | |

#### `normalized_option_quote`

| Column | Type | Notes |
|---|---|---|
| `contract_asset_id` | `BYTE_ARRAY(32)` | |
| `event_time` | `INT64` (timestamp_micros UTC) | |
| `bid`, `ask`, `bid_size`, `ask_size`, `mid`, `spread_bps` | as above | |
| `iv` | `DECIMAL(128, 18)` | Implied vol, optional |
| `delta`, `gamma`, `theta`, `vega`, `rho` | `DECIMAL(128, 18)` | Optional |
| `open_interest` | `INT64` | |
| `volume` | `INT64` | |
| `provider` | `INT16` | |
| `retrieved_at` | `INT64` | |
| `evidence_hash` | `BYTE_ARRAY(32)` | |

#### `normalized_filing_metric` (one row per company-fact)

| Column | Type | Notes |
|---|---|---|
| `cik` | `INT64` | |
| `asset_id` | `BYTE_ARRAY(32)` | Resolved at ingest |
| `concept` | `BYTE_ARRAY` | XBRL concept (e.g. `Revenues`) |
| `value` | `DECIMAL(128, 18)` | |
| `unit` | `BYTE_ARRAY` | `USD` / `shares` / etc. |
| `period_start` | `INT32` (date) | |
| `period_end` | `INT32` (date) | |
| `filing_date` | `INT32` (date) | When it became public |
| `form_type` | `BYTE_ARRAY` | |
| `evidence_hash` | `BYTE_ARRAY(32)` | |

**Critical for `FeatureView.observation_delay`:** the joinable `as_of` for a `normalized_filing_metric` MUST be `filing_date`, NOT `period_end`. A naive `event_time <= as_of` join using `period_end` produces look-ahead because the metric was not public until `filing_date`.

#### `normalized_institutional_holding` (13F)

| Column | Type | Notes |
|---|---|---|
| `filer_cik` | `INT64` | |
| `filer_asset_id` | `BYTE_ARRAY(32)` | |
| `held_asset_id` | `BYTE_ARRAY(32)` | |
| `shares` | `DECIMAL(128, 18)` | |
| `value_usd` | `DECIMAL(128, 18)` | |
| `period_end` | `INT32` (date) | Quarter end |
| `filing_date` | `INT32` (date) | Filed ~45 days after period end |
| `evidence_hash` | `BYTE_ARRAY(32)` | |

**Critical rule:** 13F holdings are NOT observable before their filing date, only their period end. Wave 2 DoD criterion 4 verifies this with a targeted test.

### 2.4 Curated Layer Schemas

Curated tables add quality scores, dedup markers, and conflict-resolution metadata. They SUPERSEDE normalized records rather than mutating them.

#### `curated_bar` (extends `normalized_bar`)

| Additional Column | Type | Notes |
|---|---|---|
| `dedup_key` | `BYTE_ARRAY` | Hash of `(asset_id, bar_start, bar_kind)` |
| `quality_score` | `INT8` | 0–100; curated-layer-adjusted |
| `quality_reasons` | `BYTE_ARRAY` (JSON array of strings) | `["gap_fill"]` / `["stale_warn"]` |
| `supersession_of` | `BYTE_ARRAY(32)` (optional) | Hash of superseded normalized record |
| `agreement_providers` | `BYTE_ARRAY` (JSON array of int16) | When N-of-M agreement checked |
| `divergence_flag` | `BOOLEAN` | Set when N-of-M failed |
| `lineage_transform_id` | `BYTE_ARRAY` | |

### 2.5 Feature Layer Schemas

#### `feature_value` (long-form: one row per feature observation)

| Column | Type | Notes |
|---|---|---|
| `feature_view_id` | `BYTE_ARRAY` | |
| `feature_view_version` | `BYTE_ARRAY` | Semantic version |
| `asset_id` | `BYTE_ARRAY(32)` | |
| `as_of` | `INT64` (timestamp_micros UTC) | The point in time |
| `event_time` | `INT64` (timestamp_micros UTC) | When the underlying event happened |
| `observation_delay_micros` | `INT64` | Publication lag encoded |
| `feature_name` | `BYTE_ARRAY` | |
| `value_float` | `DOUBLE` (optional) | |
| `value_decimal` | `DECIMAL(128, 18)` (optional) | |
| `value_int` | `INT64` (optional) | |
| `value_str` | `BYTE_ARRAY` (optional) | |
| `lineage_transform_id` | `BYTE_ARRAY` | |
| `dataset_version` | `BYTE_ARRAY` | Lance dataset version pin |

**Property test (Wave 2 DoD criterion 3):** for random view, entity, instant: `event_time + observation_delay_micros <= as_of` for every returned row. 10k iterations.

### 2.6 Intelligence Layer Schemas

#### `forecast` (one row per calibrated forecast point)

| Column | Type | Notes |
|---|---|---|
| `forecast_id` | `BYTE_ARRAY` (UUID) | |
| `asset_id` | `BYTE_ARRAY(32)` | |
| `model_id` | `BYTE_ARRAY` | |
| `model_digest` | `BYTE_ARRAY(32)` | Pinned weights |
| `issued_at` | `INT64` (timestamp_micros UTC) | When the forecast was made |
| `target_at` | `INT64` (timestamp_micros UTC) | The horizon |
| `quantile_05`, `q_25`, `q_50`, `q_75`, `q_95` | `DECIMAL(128, 18)` | |
| `mean`, `stddev` | `DECIMAL(128, 18)` | |
| `calibration_record_id` | `BYTE_ARRAY` | |
| `per_regime_coverage` | `BYTE_ARRAY` (JSON map) | |
| `drift_status` | `BYTE_ARRAY` | `none` / `annotate` / `widen` / `suppress` |
| `evidence_hash` | `BYTE_ARRAY(32)` | |

**Invariant I2 enforced structurally:** there is NO constructor for `forecast` that omits `calibration_record_id`. A forecast without a calibration record cannot exist as a type.

---

## 3. Calendar Artifact Schema (Parquet)

The signed artifact at `artifacts/calendars/<venue>/<version>.parquet`:

| Column | Type | Notes |
|---|---|---|
| `venue` | `BYTE_ARRAY` | NYSE / NASDAQ / ARCA / BATS / etc. |
| `session_date` | `INT32` (date) | |
| `session_kind` | `BYTE_ARRAY` | `regular` / `pre_market` / `post_market` / `holiday` |
| `open_utc` | `INT64` (timestamp_micros UTC) | |
| `close_utc` | `INT64` (timestamp_micros UTC) | |
| `early_close` | `BOOLEAN` | |
| `breaks_json` | `BYTE_ARRAY` (JSON array of `[start_utc, end_utc]`) | Asian lunch, futures processing |
| `interruptions_json` | `BYTE_ARRAY` (JSON array) | Unplanned halts |
| `generator_version` | `BYTE_ARRAY` | Calendar generator script version |
| `cross_validation_hash` | `BYTE_ARRAY(32)` | BLAKE3 of QuantLib comparison report |

**Cross-validation:** the generator script runs QuantLib for every session and FAILS THE BUILD on any disagreement. Two independent sources disagreeing about a session is a data-quality event, not a coin flip.

The artifact is signed (`DualSignature`) and content-addressed (`ArtifactRef` with BLAKE3). The runtime reads ONLY the pinned artifact. A calendar upgrade is an explicit, reviewed, invalidating event propagated through the lineage model exactly like a tokenizer codebook change.

---

## 4. SQLite Operational Schema (with Migrations)

Migrations are forward-only, idempotent, numbered `M{N:04}_{name}.sql`. They live in `prismatik-storage/migrations/`. Down-migrations are forbidden; a migration that fails partway MUST be detected at startup and the database quarantined.

### M0001_init.sql — initial schema

```sql
-- Operational state, task graph, presentation cache (the desktop spine)

-- Run tracking (Determinism Kernel records)
CREATE TABLE runs (
    run_id BLOB PRIMARY KEY,
    root_seed INTEGER NOT NULL,
    started_at INTEGER NOT NULL,  -- microseconds UTC
    finished_at INTEGER,
    pinned_artifact_set_json TEXT NOT NULL,
    manifest_blob BLOB,           -- final signed manifest, NULL until run completes
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed', 'aborted'))
);
CREATE INDEX idx_runs_started ON runs(started_at);

-- Task graph (in-process Tokio orchestration)
CREATE TABLE tasks (
    task_id BLOB PRIMARY KEY,
    parent_task_id BLOB,
    task_kind TEXT NOT NULL,
    trigger_json TEXT NOT NULL,
    payload_json TEXT,
    state TEXT NOT NULL CHECK (state IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    completed_at INTEGER,
    last_error TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (parent_task_id) REFERENCES tasks(task_id)
);
CREATE INDEX idx_tasks_state ON tasks(state);
CREATE INDEX idx_tasks_parent ON tasks(parent_task_id);

-- Audit ledger (desktop backend; Merkle over this table)
CREATE TABLE audit_entries (
    position INTEGER PRIMARY KEY AUTOINCREMENT,
    occurred_at INTEGER NOT NULL,
    actor_kind TEXT NOT NULL,
    actor_id BLOB,
    action TEXT NOT NULL,
    subject_kind TEXT,
    subject_id BLOB,
    outcome TEXT NOT NULL CHECK (outcome IN ('allowed', 'denied', 'failed')),
    prev_hash BLOB NOT NULL,
    detail_json TEXT NOT NULL,         -- RedactedJson
    entry_hash BLOB NOT NULL,          -- BLAKE3 of canonical(entry)
    FOREIGN KEY (position) REFERENCES audit_entries(position)
);
CREATE INDEX idx_audit_action ON audit_entries(action);
CREATE INDEX idx_audit_occurred ON audit_entries(occurred_at);

-- Signed tree heads (Merkle checkpoints)
CREATE TABLE audit_tree_heads (
    tree_size INTEGER PRIMARY KEY,
    root_hash BLOB NOT NULL,
    timestamp INTEGER NOT NULL,
    signature_blob BLOB NOT NULL       -- DualSignature
);

-- Symbology snapshot pinning
CREATE TABLE pinned_artifacts (
    artifact_id BLOB PRIMARY KEY,
    kind TEXT NOT NULL,
    version TEXT NOT NULL,
    content_hash BLOB NOT NULL,
    signature_blob BLOB NOT NULL,
    pinned_at INTEGER NOT NULL,
    pinned_by_actor_id BLOB,
    bytes_path TEXT NOT NULL            -- path within artifacts/ tree
);

-- Credential references (NEVER values; OS keychain holds values)
CREATE TABLE credential_refs (
    credential_id BLOB PRIMARY KEY,
    purpose TEXT NOT NULL,
    keychain_key TEXT NOT NULL,
    fingerprint BLOB NOT NULL,          -- BLAKE3 of value, for rotation detection
    added_at INTEGER NOT NULL,
    rotated_at INTEGER
);

-- User preferences (presentation layer)
CREATE TABLE preferences (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Layout persistence (presentation layer)
CREATE TABLE workspace_layouts (
    layout_id BLOB PRIMARY KEY,
    name TEXT NOT NULL,
    layout_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
```

### M0002_evidence.sql — evidence/lineage (Wave 1)

```sql
CREATE TABLE evidence_refs (
    record_id BLOB PRIMARY KEY,
    layer TEXT NOT NULL,
    content_hash BLOB NOT NULL,
    provider INTEGER NOT NULL,
    retrieved_at INTEGER NOT NULL,
    event_time INTEGER NOT NULL,
    quality INTEGER NOT NULL,
    bytes_path TEXT NOT NULL             -- Parquet file + row offset
);
CREATE INDEX idx_evidence_provider ON evidence_refs(provider);
CREATE INDEX idx_evidence_event_time ON evidence_refs(event_time);

CREATE TABLE lineage (
    record_id BLOB PRIMARY KEY,
    upstream_record_ids_json TEXT NOT NULL,  -- array of record_id
    transform_id TEXT NOT NULL,
    transform_version TEXT NOT NULL,
    pinned_artifact_set_json TEXT NOT NULL,
    invalidation_hash BLOB NOT NULL          -- includes pinned set
);
CREATE INDEX idx_lineage_transform ON lineage(transform_id);
```

### M0003_feature_views.sql — feature store (Wave 2)

```sql
CREATE TABLE feature_views (
    view_id BLOB PRIMARY KEY,
    version TEXT NOT NULL,
    entity_kind TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    observation_delay_micros INTEGER NOT NULL,
    online_store TEXT NOT NULL,
    offline_store TEXT NOT NULL,
    lineage_transform_id TEXT NOT NULL,
    UNIQUE (view_id, version)
);
```

### M0004_portfolio.sql — portfolio/journal (Wave 4)

```sql
-- Portfolio projections (rebuilt from audit ledger)
CREATE TABLE positions (
    asset_id BLOB PRIMARY KEY,
    currency TEXT NOT NULL,
    side TEXT NOT NULL,
    quantity TEXT NOT NULL,              -- decimal as TEXT
    avg_cost TEXT NOT NULL,
    avg_cost_source TEXT NOT NULL CHECK (avg_cost_source IN ('broker', 'wallet')),
    market_price TEXT NOT NULL,
    market_value TEXT NOT NULL,
    unrealized_pnl TEXT NOT NULL,
    realized_pnl TEXT NOT NULL,
    multiplier TEXT NOT NULL,
    risk_json TEXT,
    updated_at INTEGER NOT NULL,
    ledger_position INTEGER NOT NULL,    -- last audit position applied
    reconciliation_status TEXT NOT NULL CHECK (reconciliation_status IN ('reconciled', 'drift', 'quarantined'))
);

CREATE TABLE lots (
    lot_id BLOB PRIMARY KEY,
    asset_id BLOB NOT NULL,
    opened_at INTEGER NOT NULL,
    opened_audit_position INTEGER NOT NULL,
    quantity TEXT NOT NULL,
    cost_basis TEXT NOT NULL,
    realized_quantity TEXT NOT NULL DEFAULT '0',
    realized_pnl TEXT NOT NULL DEFAULT '0',
    FOREIGN KEY (asset_id) REFERENCES positions(asset_id)
);

-- Journal
CREATE TABLE journal_entries (
    entry_id BLOB PRIMARY KEY,
    asset_id BLOB,
    thesis_text TEXT NOT NULL,
    thesis_evidence_json TEXT NOT NULL,   -- EvidenceRef array
    outcome_tag TEXT,
    outcome_at INTEGER,
    created_at INTEGER NOT NULL,
    memory_layer INTEGER NOT NULL DEFAULT 1,  -- 1/2/3
    compressed_summary TEXT,              -- for layer 2/3 records
    hit_rate_stats_json TEXT              -- for layer 3
);
```

### M0005_orders.sql — order management (Wave 5)

```sql
CREATE TABLE order_intents (
    intent_id BLOB PRIMARY KEY,
    idempotency_key BLOB NOT NULL UNIQUE,
    asset_id BLOB NOT NULL,
    side TEXT NOT NULL,
    quantity TEXT NOT NULL,
    order_type TEXT NOT NULL,
    limit_price TEXT,
    time_in_force TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    risk_decision_json TEXT NOT NULL,     -- full 16-check results
    risk_approved_at INTEGER,
    broker_order_id BLOB,
    submission_state TEXT NOT NULL CHECK (submission_state IN ('draft', 'risk_evaluating', 'risk_approved', 'submitted', 'accepted', 'rejected', 'unknown', 'quarantined', 'filled', 'cancelled')),
    last_state_change_at INTEGER NOT NULL,
    quarantine_reason TEXT
);
CREATE INDEX idx_intents_state ON order_intents(submission_state);
CREATE INDEX idx_intents_idem ON order_intents(idempotency_key);

CREATE TABLE order_fills (
    fill_id BLOB PRIMARY KEY,
    intent_id BLOB NOT NULL,
    broker_fill_id BLOB,
    filled_at INTEGER NOT NULL,
    quantity TEXT NOT NULL,
    price TEXT NOT NULL,
    fee TEXT NOT NULL,
    fee_currency TEXT,
    liquidity TEXT CHECK (liquidity IN ('maker', 'taker', 'unknown')),
    audit_position INTEGER NOT NULL,
    FOREIGN KEY (intent_id) REFERENCES order_intents(intent_id)
);
```

### M0006_reconciliation.sql — reconciliation state (Wave 4/5)

```sql
CREATE TABLE reconciliation_log (
    recon_id BLOB PRIMARY KEY,
    asset_id BLOB NOT NULL,
    initiated_at INTEGER NOT NULL,
    completed_at INTEGER,
    divergence_kind TEXT,                  -- 'none' / 'fill_missing_locally' / 'fill_missing_broker' / 'quantity_drift' / 'cost_drift' / 'irreconcilable'
    local_state_json TEXT,
    broker_state_json TEXT,
    resolution_json TEXT,                  -- how it was healed (or NULL if quarantined)
    audit_position INTEGER NOT NULL
);
```

### M0007_ai_router.sql — AI provider state (Wave 1)

```sql
CREATE TABLE inference_providers (
    provider_id BLOB PRIMARY KEY,
    kind TEXT NOT NULL,
    auth_kind TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    model_id TEXT NOT NULL,
    model_digest BLOB,
    capabilities_json TEXT NOT NULL,
    determinism_json TEXT NOT NULL,
    egress TEXT NOT NULL,
    cost_model_json TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    added_at INTEGER NOT NULL
);

-- Recorded effects (replay returns the recorded response, never re-invokes)
CREATE TABLE inference_effects (
    effect_id BLOB PRIMARY KEY,
    run_id BLOB NOT NULL,
    provider_id BLOB NOT NULL,
    model_id TEXT NOT NULL,
    model_digest BLOB NOT NULL,
    prompt_hash BLOB NOT NULL,
    request_params_json TEXT NOT NULL,
    recorded_response_blob BLOB NOT NULL,
    recorded_at INTEGER NOT NULL,
    FOREIGN KEY (run_id) REFERENCES runs(run_id)
);
CREATE INDEX idx_inference_replay ON inference_effects(run_id, prompt_hash);
```

### M0008_oss_registry.sql — third-party component manifests (Wave 0)

```sql
CREATE TABLE third_party_components (
    component_id BLOB PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    license_spdx TEXT NOT NULL,
    license_class TEXT NOT NULL,
    integration_mode TEXT NOT NULL,
    execution_allowed INTEGER NOT NULL DEFAULT 0,
    commit_sha TEXT,
    source_archive_hash BLOB,
    grant_path TEXT,                      -- path to grant PDF if required
    manifest_json TEXT NOT NULL,
    ingested_at INTEGER NOT NULL
);
```

---

## 5. LanceDB Table Schemas

### 5.1 `analog_embeddings` (Wave 5.5)

| Column | Type | Notes |
|---|---|---|
| `id` | `BLOB` | UUID |
| `asset_id` | `BLOB(32)` | |
| `event_time` | `INT64` (timestamp_micros UTC) | |
| `embedding` | `FLOAT_ARRAY` | TSFM hidden state |
| `embedding_dim` | `INT32` | |
| `modality` | `STRING` | `ohlcv` / `orderflow` / `news` |
| `codebook_artifact_id` | `BLOB` | Tokenizer binding (Kronos 2k/base, etc.) |
| `codebook_digest` | `BLOB(32)` | |
| `model_digest` | `BLOB(32)` | Model that produced the embedding |
| `dataset_version` | `INT64` | Lance automatic versioning — pinned by manifest |

**Index:** IVF-PQ. **Vector dimension** declared at table creation; embedding column is fixed-dim per modality.

### 5.2 `evidence_index` (filings/news text embeddings)

| Column | Type | Notes |
|---|---|---|
| `id` | `BLOB` | |
| `evidence_record_id` | `BLOB` | FK to SQLite `evidence_refs` |
| `kind` | `STRING` | `filing` / `news` / `transcript` |
| `asset_id` | `BLOB(32)` | |
| `event_time` | `INT64` | |
| `embedding` | `FLOAT_ARRAY` | Sentence/doc embedding |
| `chunk_text` | `STRING` | The embedded passage |
| `chunk_index` | `INT32` | Position within source |

### 5.3 `regime_prototypes` (labelled regime centroids)

| Column | Type | Notes |
|---|---|---|
| `regime_label` | `STRING` | `narrow` / `standard` / `wide` / `volatile` / `crisis` |
| `centroid` | `FLOAT_ARRAY` | Mean embedding |
| `sample_count` | `INT64` | |
| `last_updated` | `INT64` | |
| `calibration_record_id` | `BLOB` | Coverage per regime |

---

## 6. DuckDB Views (over Parquet curated layer)

DuckDB reads Parquet directly; views are defined in `prismatik-storage/duckdb_views/`.

```sql
-- v_bars_daily: convenience view for daily bars
CREATE OR REPLACE VIEW v_bars_daily AS
SELECT asset_id, bar_start, open, high, low, close, volume, notional
FROM read_parquet('curated/bars/daily/**/*.parquet')
WHERE bar_kind = 'time' AND bar_interval = 86400;

-- v_latest_quote: latest quote per asset
CREATE OR REPLACE VIEW v_latest_quote AS
SELECT DISTINCT ON (asset_id) *
FROM read_parquet('normalized/quotes/**/*.parquet')
ORDER BY asset_id, event_time DESC;

-- v_filings_observable_at: filing metrics observable as of a given date
-- The CRITICAL pattern: observation = filing_date, not period_end
CREATE OR REPLACE VIEW v_filings_observable_at AS
SELECT
    asset_id, concept, value, unit,
    period_start, period_end,
    filing_date AS observable_at   -- the load-bearing alias
FROM read_parquet('normalized/filings/**/*.parquet');
```

---

## 7. Manifest Bundle Layout

A research bundle tarball (v1.0 §14.4) has this layout:

```
<run_id>/
├── manifest.json                 # the signed reproducibility manifest
├── manifest.sig                  # dual signature (Ed25519 + ML-DSA)
├── pinned_artifacts.json         # artifact refs with hashes (NOT bytes)
├── metrics.json                  # backtest/simulation metrics
├── audit_inclusion.json          # inclusion proof against audit tree head
└── README.md                     # human-readable summary
```

The bundle does NOT contain artifact bytes — it references them by hash. Verification requires the verifier to have the artifacts locally (or fetch them by hash from a content-addressed store). Normative format in `spec/MANIFEST_SCHEMA.md`.

---

## 8. Profile-Scoped Storage Rules

| Store | Personal Desktop | Team Cloud | Enterprise On-Prem | Disconnected Research |
|---|:---:|:---:|:---:|:---:|
| SQLite (operational) | ✓ on local disk | ✓ on persistent volume | ✓ on persistent volume | ✓ on local disk |
| Parquet (raw + curated) | ✓ local disk | ✓ S3-compatible | ✓ S3-compatible | ✓ local disk, pre-staged |
| DuckDB (analytical views) | ✓ in-process | ✓ in-process | ✓ in-process | ✓ in-process |
| LanceDB (embeddings) | ✓ local | ✓ on shared volume | ✓ on shared volume | ✓ local, pre-staged |
| PostgreSQL | — | ✓ | ✓ | optional |
| ClickHouse | — | ✓ (Wave 3+) | ✓ | optional |
| NATS JetStream | — | ✓ | ✓ | — |
| S3-compatible object store | — | ✓ | ✓ | — |

**A desktop install MUST NOT ship with Postgres, ClickHouse, or NATS dependencies.** Shipping six engines to a single-user desktop is a defect.

---

*Author: Aaron Stovall · Mythos Systems · 2026-07-26 · Version 1.0*
