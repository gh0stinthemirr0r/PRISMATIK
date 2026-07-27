$ErrorActionPreference = "Stop"

# Delegate to the canonical Python gate (Wave 0 DoD #1/#2).
# Spec: DOCS/spec/CI_WORKFLOWS.md §"Job: determinism-grep".
# The Python gate is comment-aware, supports an allowlist capped at 10 entries,
# and is exercised by the negative test at crates/prismatik-determinism/tests/grep_gate.rs.

python "$PSScriptRoot\determinism_grep.py" $args
exit $LASTEXITCODE
