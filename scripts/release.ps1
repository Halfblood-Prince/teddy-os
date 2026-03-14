param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [ValidateSet("debug", "release")]
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"

function Write-Utf8File {
    param(
        [string]$Path,
        [string]$Content
    )

    [System.IO.File]::WriteAllText($Path, $Content + [Environment]::NewLine, [System.Text.UTF8Encoding]::new($false))
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$distDir = Join-Path $repoRoot "build\dist"
$isoPath = Join-Path $distDir "teddy-os-$Profile.iso"
$isoChecksumPath = "$isoPath.sha256"
$manifestPath = Join-Path $distDir "release-$Version.json"

if (-not (Test-Path $isoPath)) {
    throw "ISO not found at $isoPath. Run scripts/build.ps1 -Profile $Profile first."
}

if (-not (Test-Path $isoChecksumPath)) {
    throw "Checksum file not found at $isoChecksumPath. Regenerate the ISO first."
}

$isoChecksum = ((Get-Content $isoChecksumPath | Select-Object -First 1) -split " ")[0]
$manifest = [ordered]@{
    version = $Version
    channel = if ($Profile -eq "release") { "stable" } else { "debug" }
    profile = $Profile
    artifacts = @(
        [ordered]@{
            name = (Split-Path -Leaf $isoPath)
            path = (Resolve-Path $isoPath).Path
            sha256 = $isoChecksum
        }
    )
}

$json = $manifest | ConvertTo-Json -Depth 6
Write-Utf8File -Path $manifestPath -Content $json

Write-Host "Release manifest written to $manifestPath" -ForegroundColor Green
