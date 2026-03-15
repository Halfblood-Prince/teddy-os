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
New-Item -ItemType Directory -Force -Path (Join-Path $isoRoot "EFI\\BOOT") | Out-Null

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
& mmd -i $espImage ::/EFI
& mmd -i $espImage ::/EFI/BOOT
& mcopy -i $espImage -s (Join-Path $espRoot "*") ::/

Copy-Item $espImage (Join-Path $isoRoot "EFI\\efiboot.img") -Force
Copy-Item (Join-Path $espRoot "EFI\\BOOT\\BOOTX64.EFI") (Join-Path $isoRoot "EFI\\BOOT\\BOOTX64.EFI") -Force

& xorriso -as mkisofs `
    -R `
    -J `
    -volid "TEDDYOS" `
    -eltorito-catalog EFI/BOOT/boot.cat `
    -eltorito-alt-boot `
    -eltorito-platform efi `
    -e EFI/efiboot.img `
    -no-emul-boot `
    -boot-load-size 4 `
    -isohybrid-gpt-basdat `
    -o $isoPath `
    $isoRoot

if (-not (Test-Path $isoPath)) {
    throw "ISO file was not produced at $isoPath"
}

$isoHash = (Get-FileHash -Algorithm SHA256 $isoPath).Hash.ToLowerInvariant()
[System.IO.File]::WriteAllText(
    "$isoPath.sha256",
    "$isoHash *$(Split-Path -Leaf $isoPath)" + [Environment]::NewLine,
    [System.Text.UTF8Encoding]::new($false)
)

