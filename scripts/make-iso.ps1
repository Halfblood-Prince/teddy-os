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

function Reset-Directory {
    param([string]$Path)

    if (Test-Path $Path) {
        Remove-Item -Recurse -Force $Path
    }
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Write-TextFile {
    param(
        [string]$Path,
        [string]$Content
    )

    [System.IO.File]::WriteAllText($Path, $Content + [Environment]::NewLine, [System.Text.UTF8Encoding]::new($false))
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$buildRoot = Join-Path $repoRoot "build"
$stagingRoot = Join-Path $buildRoot "staging"
$espRoot = Join-Path $stagingRoot "esp"
$isoRoot = Join-Path $stagingRoot "iso"
$distDir = Join-Path $buildRoot "dist"
$espImage = Join-Path $stagingRoot "efiboot.img"
$isoPath = Join-Path $distDir "teddy-os-$Profile.iso"

if (-not (Test-Path $espRoot)) {
    throw "EFI staging tree not found at $espRoot. Run scripts/build.ps1 first."
}

Require-Command mformat
Require-Command mmd
Require-Command mcopy
Require-Command xorriso

New-Item -ItemType Directory -Force -Path $distDir | Out-Null
Reset-Directory $isoRoot
$isoEfiDir = Join-Path $isoRoot "EFI"
New-Item -ItemType Directory -Force -Path $isoEfiDir | Out-Null

if (Test-Path $espImage) {
    Remove-Item -Force $espImage
}
if (Test-Path $isoPath) {
    Remove-Item -Force $isoPath
}

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

$espSource = Join-Path $espRoot "*"
& mcopy -i $espImage -s $espSource ::/
if ($LASTEXITCODE -ne 0) {
    throw "mcopy failed."
}

Copy-Item $espImage (Join-Path $isoEfiDir "efiboot.img") -Force

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

if (-not (Test-Path $isoPath)) {
    throw "ISO file was not produced at $isoPath"
}

$isoHash = (Get-FileHash -Algorithm SHA256 $isoPath).Hash.ToLowerInvariant()
Write-TextFile -Path "$isoPath.sha256" -Content "$isoHash *$(Split-Path -Leaf $isoPath)"

Write-Host "ISO created at $isoPath" -ForegroundColor Green
Write-Host "SHA256 written to $isoPath.sha256" -ForegroundColor Green
