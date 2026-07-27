$ErrorActionPreference = "Stop"

$generatedDir = "packages/api-client/src/generated"
$marker = Join-Path $generatedDir ".gitkeep"

if (-not (Test-Path $marker)) {
    Write-Error "Missing generated bindings marker: $marker"
    exit 1
}

$changes = git status --porcelain -- $generatedDir
if ($changes) {
    Write-Error "Generated API bindings differ from the committed tree. Regenerate them with tauri-specta and commit the result."
    exit 1
}

Write-Output "Generated API bindings are clean."
