# P0-SS-05: every workspace member must appear in prismatik-oss-registry.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
cargo test -p prismatik-oss-registry registry_coverage -- --nocapture
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
