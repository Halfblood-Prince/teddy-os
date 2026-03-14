param(
    [string]$IsoPath = "build/dist/teddy-os-debug.iso"
)

$ErrorActionPreference = "Stop"

function Require-Command {
    param([string]$Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command '$Name' was not found on PATH."
    }
}

if (-not (Test-Path $IsoPath)) {
    throw "ISO not found at $IsoPath"
}

Require-Command xorriso

$resolvedIso = (Resolve-Path $IsoPath).Path

Write-Host "Inspecting ISO: $resolvedIso" -ForegroundColor Cyan
Write-Host ""
Write-Host "[El Torito boot catalog]" -ForegroundColor Yellow
& xorriso -indev $resolvedIso -report_el_torito plain
if ($LASTEXITCODE -ne 0) {
    throw "Failed to read El Torito data from $resolvedIso"
}

Write-Host ""
Write-Host "[EFI boot files]" -ForegroundColor Yellow
& xorriso -indev $resolvedIso -find /EFI/BOOT -maxdepth 2 -type f -exec lsdl
if ($LASTEXITCODE -ne 0) {
    throw "Failed to list EFI boot files from $resolvedIso"
}

Write-Host ""
Write-Host "If you see an EFI El Torito entry and /EFI/BOOT/BOOTX64.EFI, the ISO is UEFI-bootable." -ForegroundColor Green
Write-Host "If VMware still shows PXE text, the VM firmware is likely set to BIOS or CD/DVD is disconnected." -ForegroundColor Green
