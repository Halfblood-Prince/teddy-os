$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

foreach ($path in @(
    (Join-Path $repoRoot "build\staging"),
    (Join-Path $repoRoot "build\dist")
)) {
    if (Test-Path $path) {
        Remove-Item -Recurse -Force $path
        Write-Host "Removed $path" -ForegroundColor Green
    }
}
