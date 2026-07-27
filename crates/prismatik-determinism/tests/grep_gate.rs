//! Determinism grep-gate negative test (Wave 0 DoD criterion 2).
//!
//! Spec: `DOCS/spec/CI_WORKFLOWS.md` §"Job: determinism-grep".
//!
//! A gate introduced after violations exist gets an allowlist, and an allowlist
//! is how gates die. This test proves the gate *fails* on a deliberate
//! violation, so a future regression cannot pass silently. It builds a tiny
//! fixture workspace under a temp dir, runs `scripts/determinism_grep.py`
//! against it, and asserts a non-zero exit plus a violation report.

#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Locate the workspace root from this crate's manifest dir.
///
/// `CARGO_MANIFEST_DIR` = `<workspace>/crates/prismatik-determinism`, so the
/// workspace root is two ancestors up.
fn workspace_root() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .ancestors()
        .nth(2)
        .map(PathBuf::from)
        .expect("crate sits inside <workspace>/crates/<name>")
}

/// Locate the gate script.
fn gate_script() -> PathBuf {
    workspace_root().join("scripts").join("determinism_grep.py")
}

/// Build a minimal fixture workspace under `root` with one clean crate and one
/// violating crate, returning the path.
fn build_fixture(root: &std::path::Path, violating: bool) -> PathBuf {
    let crates = root.join("crates");
    // A clean crate (must NOT trip the gate).
    let clean_src = crates.join("prismatik-clean").join("src");
    fs::create_dir_all(&clean_src).unwrap();
    fs::write(
        clean_src.join("lib.rs"),
        "// clean — uses DetMap, no ambient sources\n\
         use prismatik_determinism::DetMap;\n\
         pub struct Clean { _m: DetMap<u32, u32> }\n",
    )
    .unwrap();

    if violating {
        // A crate with a deliberate ambient-nondeterminism violation.
        let bad_src = crates.join("prismatik-bad").join("src");
        fs::create_dir_all(&bad_src).unwrap();
        fs::write(
            bad_src.join("lib.rs"),
            "use std::collections::HashMap;\n\
             pub struct Bad { _m: HashMap<u32, u32> }\n",
        )
        .unwrap();
    }
    root.to_path_buf()
}

/// Run the gate against `fixture_root`, returning (exit_code, combined_output).
fn run_gate(fixture_root: &std::path::Path) -> (i32, String) {
    let script = gate_script();
    assert!(
        script.exists(),
        "gate script missing at {}",
        script.display()
    );
    let output = Command::new("python3")
        .arg(&script)
        .args(["--root"])
        .arg(fixture_root)
        .args(["--no-allowlist"])
        .output()
        .expect("python3 is required to run the determinism gate");
    let code = output.status.code().unwrap_or(-1);
    let combined = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    (code, combined)
}

#[test]
fn grep_gate_fails_on_violation() {
    let tmp = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    // Use a directory fixture.
    let tmp = tmp.parent().unwrap().join("prismatik-grep-gate-fixture");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    // Scope the cleanup so the fixture survives the run above.
    let _guard = DropGuard(tmp.clone());

    build_fixture(&tmp, true);
    let (code, output) = run_gate(&tmp);
    assert_ne!(
        code, 0,
        "gate must FAIL on a deliberate HashMap violation; output:\n{output}"
    );
    assert!(
        output.contains("HashMap") && output.contains("prismatik-bad"),
        "gate output must name the violating pattern and file:\n{output}"
    );
}

#[test]
fn grep_gate_passes_when_clean() {
    let tmp = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    let tmp = tmp.parent().unwrap().join("prismatik-grep-gate-clean");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let _guard = DropGuard(tmp.clone());

    build_fixture(&tmp, false);
    let (code, output) = run_gate(&tmp);
    assert_eq!(
        code, 0,
        "gate must PASS when no violations exist; output:\n{output}"
    );
}

struct DropGuard(PathBuf);
impl Drop for DropGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
