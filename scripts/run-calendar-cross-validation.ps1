# Zero-tolerance calendar QuantLib cross-validation gate (P0-DK-14).
# Offline: uses committed fixture oracle — QuantLib is not required.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$Python = if ($env:PYTHON) { $env:PYTHON } elseif (Get-Command python -ErrorAction SilentlyContinue) { "python" } elseif (Get-Command python3 -ErrorAction SilentlyContinue) { "python3" } else { throw "python not found" }

& $Python scripts/build_calendar_artifact.py --validate
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo test -p prismatik-calendar cross_validation -- --nocapture
exit $LASTEXITCODE
