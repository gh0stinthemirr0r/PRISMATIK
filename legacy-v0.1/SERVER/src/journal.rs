//! Append-only JSONL session and audit journal.
//! Author: Aaron Stovall · Version 0.1.0 · 2026-07-07
//!
//! Every bar close, decision, fill, order, halt, and lifecycle event is one JSON
//! line with a UTC timestamp, so any session is reconstructable for audit and,
//! for live trading, forms the required order audit trail. Never contains
//! credentials.

use serde_json::{json, Value};
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Journal {
    path: PathBuf,
    lock: Mutex<()>,
}

impl Journal {
    pub fn open(session_id: &str) -> std::io::Result<Self> {
        let root = std::env::var("PRISMATIK_JOURNAL_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                PathBuf::from(home).join(".prismatik").join("journal")
            });
        create_dir_all(&root)?;
        let safe: String = session_id.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
            .collect();
        let path = root.join(format!("{safe}.jsonl"));
        tracing::info!(path = %path.display(), "journal_open");
        Ok(Self { path, lock: Mutex::new(()) })
    }

    pub fn write(&self, event: &str, mut fields: Value) {
        let record = {
            let obj = fields.as_object_mut();
            let mut base = json!({
                "ts": chrono::Utc::now().to_rfc3339(),
                "event": event,
            });
            if let (Some(dst), Some(src)) = (base.as_object_mut(), obj) {
                for (k, v) in src.iter() { dst.insert(k.clone(), v.clone()); }
            }
            base
        };
        let _guard = self.lock.lock().expect("journal lock");
        if let Ok(mut fh) = OpenOptions::new().create(true).append(true).open(&self.path) {
            let _ = writeln!(fh, "{}", record);
        }
    }

    pub fn read_tail(&self, max: usize) -> Vec<Value> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => {
                let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
                lines.iter().rev().take(max).rev()
                    .filter_map(|l| serde_json::from_str(l).ok())
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }
}
