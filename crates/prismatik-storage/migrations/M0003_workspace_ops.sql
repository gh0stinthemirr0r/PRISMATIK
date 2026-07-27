-- M0003_workspace_ops.sql — watchlist, scanner, alerts, inference, supersession

CREATE TABLE IF NOT EXISTS watchlist_items (
    asset_key TEXT PRIMARY KEY,
    coingecko_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    added_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS scanner_filters (
    filter_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    filter_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS alert_rules (
    rule_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    rule_json TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    dedup_key TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS alert_events (
    event_id TEXT PRIMARY KEY,
    rule_id TEXT NOT NULL,
    dedup_key TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    state TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(rule_id, dedup_key)
);

CREATE TABLE IF NOT EXISTS inference_effects (
    effect_id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    model_digest TEXT NOT NULL,
    request_json TEXT NOT NULL,
    response_json TEXT NOT NULL,
    recorded_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS raw_supersessions (
    path TEXT PRIMARY KEY,
    superseded_by TEXT,
    created_at INTEGER NOT NULL
);
