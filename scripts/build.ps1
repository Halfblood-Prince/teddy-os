param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "debug",
    [switch]$SkipIso
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
$targetDir = Join-Path $buildRoot "target"
$stagingRoot = Join-Path $buildRoot "staging"
$distDir = Join-Path $buildRoot "dist"
$espRoot = Join-Path $stagingRoot "esp"
$espBootDir = Join-Path $espRoot "EFI\\BOOT"

Require-Command cargo
Require-Command rustup

Push-Location $repoRoot
try {
    New-Item -ItemType Directory -Force -Path $buildRoot, $stagingRoot, $distDir, $espBootDir | Out-Null
    $env:CARGO_TARGET_DIR = $targetDir

    rustup target add x86_64-unknown-uefi | Out-Host

    $cargoArgs = @("build", "-p", "teddy-boot", "--target", "x86_64-unknown-uefi")
    if ($Profile -eq "release") {
        $cargoArgs += "--release"
    }

    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "UEFI boot app build failed."
    }

    $bootArtifact = Join-Path (Join-Path $targetDir "x86_64-unknown-uefi\\$Profile") "teddy-boot.efi"
    if (-not (Test-Path $bootArtifact)) {
        throw "Boot artifact not found at $bootArtifact"
    }

    Copy-Item $bootArtifact (Join-Path $espBootDir "BOOTX64.EFI") -Force

    if (-not $SkipIso) {
        & (Join-Path $PSScriptRoot "make-iso.ps1") -Profile $Profile
        if ($LASTEXITCODE -ne 0) {
            throw "ISO generation failed."
        }
    }
}
finally {
    Pop-Location
}

