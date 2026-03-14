param(
    [Parameter(Mandatory = $true)]
    [string]$VmxPath,
    [ValidateSet("debug", "release")]
    [string]$Profile = "debug",
    [switch]$NoGui
)

$ErrorActionPreference = "Stop"

function Require-Command {
    param([string]$Name)

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command '$Name' was not found on PATH."
    }
}

function Set-VmxKey {
    param(
        [System.Collections.Generic.List[string]]$Lines,
        [string]$Key,
        [string]$Value
    )

    $pattern = '^{0}\s*=\s*".*"\s*$' -f [Regex]::Escape($Key)
    $replacement = '{0} = "{1}"' -f $Key, $Value

    for ($index = 0; $index -lt $Lines.Count; $index++) {
        if ($Lines[$index] -match $pattern) {
            $Lines[$index] = $replacement
            return
        }
    }

    $Lines.Add($replacement)
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$isoPath = Join-Path $repoRoot "build\dist\teddy-os-$Profile.iso"

if (-not (Test-Path $VmxPath)) {
    throw "VMware configuration file not found at $VmxPath"
}

if (-not (Test-Path $isoPath)) {
    throw "ISO not found at $isoPath. Run scripts/build.ps1 first."
}

Require-Command vmrun

$mode = if ($NoGui) { "nogui" } else { "gui" }
$resolvedVmx = (Resolve-Path $VmxPath).Path
$resolvedIso = (Resolve-Path $isoPath).Path

$vmxLines = [System.Collections.Generic.List[string]]::new()
$vmxLines.AddRange([System.IO.File]::ReadAllLines($resolvedVmx))

# Force a UEFI/CDROM boot path so the VM does not fall back to PXE network boot.
Set-VmxKey -Lines $vmxLines -Key "firmware" -Value "efi"
Set-VmxKey -Lines $vmxLines -Key "efi.secureBoot.enabled" -Value "FALSE"
Set-VmxKey -Lines $vmxLines -Key "cdrom0.present" -Value "TRUE"
Set-VmxKey -Lines $vmxLines -Key "cdrom0.startConnected" -Value "TRUE"
Set-VmxKey -Lines $vmxLines -Key "cdrom0.deviceType" -Value "cdrom-image"
Set-VmxKey -Lines $vmxLines -Key "cdrom0.fileName" -Value $resolvedIso
Set-VmxKey -Lines $vmxLines -Key "bios.bootOrder" -Value "cdrom,hdd"

[System.IO.File]::WriteAllLines($resolvedVmx, $vmxLines)

Write-Host "Updated VMX firmware and CD/DVD ISO path before launch:" -ForegroundColor Yellow
Write-Host "  firmware=efi" -ForegroundColor Yellow
Write-Host "  cdrom0.fileName=$resolvedIso" -ForegroundColor Yellow
& vmrun start $resolvedVmx $mode
if ($LASTEXITCODE -ne 0) {
    throw "vmrun failed to start the VM."
}

Write-Host "VMware VM started using $resolvedVmx" -ForegroundColor Green
