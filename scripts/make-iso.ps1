param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "debug"
)

$ErrorActionPreference = "Stop"

function Require-Command {
    param([string]$Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command '$Name' was not found on PATH."
    }
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$buildRoot = Join-Path $repoRoot "build"
$stagingRoot = Join-Path $buildRoot "staging"
$isoRoot = Join-Path $stagingRoot "iso"
$distDir = Join-Path $buildRoot "dist"
$bootImg = Join-Path $isoRoot "boot.img"
$isoPath = Join-Path $distDir "teddy-os-$Profile.iso"

if (-not (Test-Path $bootImg)) {
    throw "BIOS boot image not found at $bootImg. Run scripts/build.ps1 first."
}

Require-Command xorriso

New-Item -ItemType Directory -Force -Path $distDir | Out-Null
if (Test-Path $isoPath) {
    Remove-Item -Force $isoPath
}

& xorriso -as mkisofs `
    -V "TEDDYOS" `
    -b boot.img `
    -c boot.cat `
    -o $isoPath `
    $isoRoot

if ($LASTEXITCODE -ne 0) {
    throw "xorriso failed."
}

$isoHash = (Get-FileHash -Algorithm SHA256 $isoPath).Hash.ToLowerInvariant()
[System.IO.File]::WriteAllText(
    "$isoPath.sha256",
    "$isoHash *$(Split-Path -Leaf $isoPath)" + [Environment]::NewLine,
    [System.Text.UTF8Encoding]::new($false)
)

