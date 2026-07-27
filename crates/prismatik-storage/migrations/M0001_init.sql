-- M0001_init.sql — operational spine (DATA_SCHEMAS §4)

CREATE TABLE IF NOT EXISTS runs (
    run_id BLOB PRIMARY KEY,
    root_seed INTEGER NOT NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    pinned_artifact_set_json TEXT NOT NULL,
    manifest_blob BLOB,
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed', 'aborted'))
);
CREATE INDEX IF NOT EXISTS idx_runs_started ON runs(started_at);

CREATE TABLE IF NOT EXISTS tasks (
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
CREATE INDEX IF NOT EXISTS idx_tasks_state ON tasks(state);
CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_task_id);

CREATE TABLE IF NOT EXISTS preferences (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS workspace_layouts (
    layout_id BLOB PRIMARY KEY,
    name TEXT NOT NULL,
    layout_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at INTEGER NOT NULL
);
