-- M0002_evidence.sql — evidence / lineage

CREATE TABLE IF NOT EXISTS evidence_refs (
    record_id BLOB PRIMARY KEY,
    layer TEXT NOT NULL,
    content_hash BLOB NOT NULL,
    provider INTEGER NOT NULL,
    retrieved_at INTEGER NOT NULL,
    event_time INTEGER NOT NULL,
    quality INTEGER NOT NULL,
    bytes_path TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_evidence_provider ON evidence_refs(provider);
CREATE INDEX IF NOT EXISTS idx_evidence_event_time ON evidence_refs(event_time);

CREATE TABLE IF NOT EXISTS lineage (
    record_id BLOB PRIMARY KEY,
    upstream_record_ids_json TEXT NOT NULL,
    transform_id TEXT NOT NULL,
    transform_version TEXT NOT NULL,
    pinned_artifact_set_json TEXT NOT NULL,
    invalidation_hash BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_lineage_transform ON lineage(transform_id);
