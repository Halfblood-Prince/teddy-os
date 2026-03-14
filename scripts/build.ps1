param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "debug",
    [switch]$Release,
    [switch]$SkipIso,
    [switch]$Clean
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

function Write-BuildManifest {
    param(
        [string]$Path,
        [string]$ProfileName,
        [string]$BootloaderPath,
        [string]$KernelPath,
        [string]$IsoPath
    )

    $manifest = [ordered]@{
        profile = $ProfileName
        bootloader = (Resolve-Path $BootloaderPath).Path
        kernel = (Resolve-Path $KernelPath).Path
        iso = if (Test-Path $IsoPath) { (Resolve-Path $IsoPath).Path } else { $null }
    }

    $json = $manifest | ConvertTo-Json -Depth 4
    [System.IO.File]::WriteAllText($Path, $json + [Environment]::NewLine, [System.Text.UTF8Encoding]::new($false))
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$buildRoot = Join-Path $repoRoot "build"
$targetDir = Join-Path $buildRoot "target"
$stagingRoot = Join-Path $buildRoot "staging"
$distDir = Join-Path $buildRoot "dist"

Require-Command cargo
Require-Command rustup

Push-Location $repoRoot
try {
    if ($Release) {
        $Profile = "release"
    }

    if ($Clean) {
        Reset-Directory $stagingRoot
        Reset-Directory $distDir
    } else {
        New-Item -ItemType Directory -Force -Path $stagingRoot | Out-Null
        New-Item -ItemType Directory -Force -Path $distDir | Out-Null
    }

    $env:CARGO_TARGET_DIR = $targetDir

    rustup target add x86_64-unknown-uefi | Out-Host
    rustup component add rust-src llvm-tools-preview | Out-Host

    $cargoArgs = @("build", "-p", "teddy-bootloader", "--target", "x86_64-unknown-uefi")
    if ($Profile -eq "release") {
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
        "-Z", "json-target-spec",
        "-p", "teddy-kernel",
        "--target", "kernel/x86_64-teddy-kernel.json"
    )
    if ($Profile -eq "release") {
        $kernelArgs += "--release"
    }
    $previousRustFlags = $env:RUSTFLAGS
    if ([string]::IsNullOrWhiteSpace($previousRustFlags)) {
        $env:RUSTFLAGS = "-Clink-arg=-Tkernel/linker.ld"
    } else {
        $env:RUSTFLAGS = "$previousRustFlags -Clink-arg=-Tkernel/linker.ld"
    }

    & cargo @kernelArgs
    $env:RUSTFLAGS = $previousRustFlags
    if ($LASTEXITCODE -ne 0) {
        throw "Kernel build failed."
    }

    $espRoot = Join-Path $stagingRoot "esp"
    $espEfiDir = Join-Path $espRoot "EFI"
    $espBootDir = Join-Path $espEfiDir "BOOT"
    Reset-Directory $espRoot
    New-Item -ItemType Directory -Force -Path $espBootDir | Out-Null

    $bootloaderTargetDir = Join-Path (Join-Path $targetDir "x86_64-unknown-uefi") $Profile
    $kernelTargetDir = Join-Path (Join-Path $targetDir "x86_64-teddy-kernel") $Profile
    $bootloaderArtifact = Join-Path $bootloaderTargetDir "teddy-bootloader.efi"
    $kernelArtifact = Join-Path $kernelTargetDir "teddy-kernel"

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
        & (Join-Path $PSScriptRoot "make-iso.ps1") -Profile $Profile
        if ($LASTEXITCODE -ne 0) {
            throw "ISO generation failed."
        }
    }

    $isoPath = Join-Path $distDir "teddy-os-$Profile.iso"
    $buildManifestPath = Join-Path $distDir "build-$Profile.json"
    Write-BuildManifest `
        -Path $buildManifestPath `
        -ProfileName $Profile `
        -BootloaderPath $bootloaderArtifact `
        -KernelPath $kernelArtifact `
        -IsoPath $isoPath

    Write-Host "Build manifest written to $buildManifestPath" -ForegroundColor Green
}
finally {
    Pop-Location
}
