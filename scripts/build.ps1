param(
    [switch]$Release,
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
$profile = if ($Release) { "release" } else { "debug" }

Require-Command cargo
Require-Command rustup

Push-Location $repoRoot
try {
    rustup target add x86_64-unknown-uefi | Out-Host
    rustup component add rust-src llvm-tools-preview | Out-Host

    $cargoArgs = @("build", "-p", "teddy-bootloader", "--target", "x86_64-unknown-uefi")
    if ($Release) {
        $cargoArgs += "--release"
    }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Bootloader build failed."
    }

    $kernelArgs = @(
        "+nightly",
        "build",
        "-Z", "build-std=core,compiler_builtins",
        "-p", "teddy-kernel",
        "--target", "kernel/x86_64-teddy-kernel.json"
    )
    if ($Release) {
        $kernelArgs += "--release"
    }
    $kernelArgs += "--"
    $kernelArgs += "-C"
    $kernelArgs += "link-arg=-Tkernel/linker.ld"

    & cargo @kernelArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Kernel build failed."
    }

    $espBootDir = Join-Path $repoRoot "build\staging\esp\EFI\BOOT"
    New-Item -ItemType Directory -Force -Path $espBootDir | Out-Null

    $bootloaderArtifact = Join-Path $repoRoot "build\target\x86_64-unknown-uefi\$profile\teddy-bootloader.efi"
    $kernelArtifact = Join-Path $repoRoot "build\target\x86_64-teddy-kernel\$profile\teddy-kernel"

    if (-not (Test-Path $bootloaderArtifact)) {
        throw "Bootloader artifact not found at $bootloaderArtifact"
    }
    if (-not (Test-Path $kernelArtifact)) {
        throw "Kernel artifact not found at $kernelArtifact"
    }

    Copy-Item $bootloaderArtifact (Join-Path $espBootDir "BOOTX64.EFI") -Force
    Copy-Item $kernelArtifact (Join-Path $espBootDir "KERNEL.ELF") -Force

    Write-Host "EFI staging tree ready at build\staging\esp" -ForegroundColor Green

    if (-not $SkipIso) {
        & (Join-Path $PSScriptRoot "make-iso.ps1") -Profile $profile
        if ($LASTEXITCODE -ne 0) {
            throw "ISO generation failed."
        }
    }
}
finally {
    Pop-Location
}

