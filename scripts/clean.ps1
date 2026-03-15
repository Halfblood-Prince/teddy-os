$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
foreach ($path in @("build", "dist", "target")) {
    $full = Join-Path $repoRoot $path
    if (Test-Path $full) {
        Remove-Item -Recurse -Force $full
    }
}
