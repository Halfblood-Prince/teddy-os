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
$espRoot = Join-Path $repoRoot "build\staging\esp"
$isoRoot = Join-Path $repoRoot "build\staging\iso"
$distDir = Join-Path $repoRoot "build\dist"
$espImage = Join-Path $repoRoot "build\staging\efiboot.img"
$isoPath = Join-Path $distDir "teddy-os-$Profile.iso"

if (-not (Test-Path $espRoot)) {
    throw "EFI staging tree not found at $espRoot. Run scripts/build.ps1 first."
}

Require-Command mformat
Require-Command mmd
Require-Command mcopy
Require-Command xorriso

New-Item -ItemType Directory -Force -Path $distDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $isoRoot "EFI") | Out-Null

$stream = [System.IO.File]::Create($espImage)
try {
    $stream.SetLength(64MB)
}
finally {
    $stream.Dispose()
}

& mformat -i $espImage -F ::
if ($LASTEXITCODE -ne 0) {
    throw "mformat failed."
}

& mmd -i $espImage ::/EFI
& mmd -i $espImage ::/EFI/BOOT
if ($LASTEXITCODE -ne 0) {
    throw "mmd failed."
}

& mcopy -i $espImage -s "$espRoot\*" ::/
if ($LASTEXITCODE -ne 0) {
    throw "mcopy failed."
}

Copy-Item $espImage (Join-Path $isoRoot "EFI\efiboot.img") -Force

& xorriso -as mkisofs `
    -R `
    -J `
    -volid "TEDDYOS" `
    -eltorito-alt-boot `
    -e EFI/efiboot.img `
    -no-emul-boot `
    -isohybrid-gpt-basdat `
    -o $isoPath `
    $isoRoot

if ($LASTEXITCODE -ne 0) {
    throw "xorriso failed."
}

Write-Host "ISO created at $isoPath" -ForegroundColor Green

