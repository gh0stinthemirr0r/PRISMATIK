# P0-SS-06 floor: generate a CycloneDX SBOM into artifacts/sbom/ when
# `cargo cyclonedx` is available; otherwise ensure the output directory + README
# exist and print install instructions. Does not claim production release signing.
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root

$OutDir = "artifacts/sbom"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$Readme = Join-Path $OutDir "README.md"
if (-not (Test-Path $Readme)) {
    @"
# SBOM output directory (P0-SS-06 floor)

Generated CycloneDX SBOMs land here when ``scripts/generate-sbom.sh`` (or
``.ps1``) can run ``cargo cyclonedx``.

SBOMs are **compliance / inventory** artifacts, not a supply-chain defense.
Signing and provenance verification live under ``prismatik-security::updater``
and author release-ops (cosign / SLSA). See ``DOCS/waves/P0_SS_06_Release_Signing.md``.
"@ | Set-Content -Encoding utf8 $Readme
}

function Test-CargoCyclonedx {
    if (Get-Command cargo-cyclonedx -ErrorAction SilentlyContinue) { return $true }
    cargo cyclonedx --help 2>$null | Out-Null
    return ($LASTEXITCODE -eq 0)
}

if (Test-CargoCyclonedx) {
    Write-Host "P0-SS-06: running cargo cyclonedx → $OutDir"
    cargo cyclonedx --manifest-path Cargo.toml --format json --output-cdx
    if ($LASTEXITCODE -ne 0) {
        cargo cyclonedx --manifest-path Cargo.toml --format json
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
    # cargo-cyclonedx writes beside each manifest, so a workspace run scatters
    # one *.cdx.json per member crate. Sweep the root AND every member directory,
    # or the generated files are left stranded in the tree (and get committed).
    $Scattered = @(Get-ChildItem -ErrorAction SilentlyContinue -Path "bom.json")
    $Scattered += @(Get-ChildItem -ErrorAction SilentlyContinue -Recurse -Filter "*.cdx.json" `
        -Path "." | Where-Object { $_.FullName -notlike "*\artifacts\sbom\*" -and $_.FullName -notlike "*\target\*" })
    foreach ($item in $Scattered) {
        Move-Item -Force $item.FullName (Join-Path $OutDir $item.Name)
        Write-Host "moved $($item.Name) → $OutDir/"
    }
    $TargetBom = "target/cyclonedx/bom.json"
    if (Test-Path $TargetBom) {
        Copy-Item -Force $TargetBom (Join-Path $OutDir "bom.json")
        Write-Host "copied $TargetBom → $OutDir/bom.json"
    }
    Write-Host "P0-SS-06: SBOM generation finished (see $OutDir/)."
    exit 0
}

Write-Host @"
P0-SS-06: cargo cyclonedx not installed — placeholder path ready.

Install (author / release host):
  cargo install cargo-cyclonedx

Then re-run:
  pwsh scripts/generate-sbom.ps1

Output directory (committed README, generated files local):
  artifacts/sbom/

No production cosign keys or SLSA attestations are produced by this script.
See scripts/cosign-verify-example.md and DOCS/waves/P0_SS_06_Release_Signing.md.
"@
exit 0
