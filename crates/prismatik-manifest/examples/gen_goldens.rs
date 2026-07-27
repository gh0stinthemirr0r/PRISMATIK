//! Generate committed golden manifest JSON files.

use prismatik_manifest::build_golden_pair;
use std::fs;
use std::path::PathBuf;
use time::macros::datetime;

fn main() {
    let at = datetime!(2026-07-26 14:32:00 UTC);
    let (ingest, dst) = build_golden_pair(at);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("manifest_data_ingest_v1.json"), ingest).unwrap();
    fs::write(root.join("manifest_dst_replay_v1.json"), dst).unwrap();
    println!("wrote goldens to {}", root.display());
}
