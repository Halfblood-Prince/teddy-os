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

Write-Host "VMware helper expects the VMX to already reference $isoPath" -ForegroundColor Yellow
& vmrun start $resolvedVmx $mode
if ($LASTEXITCODE -ne 0) {
    throw "vmrun failed to start the VM."
}

Write-Host "VMware VM started using $resolvedVmx" -ForegroundColor Green
