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
$distDir = Join-Path $buildRoot "dist"
$binDir = Join-Path $buildRoot "bin"
$isoRoot = Join-Path $stagingRoot "iso"
$bootAsm = Join-Path $repoRoot "bios\\boot.asm"
$stage2Asm = Join-Path $repoRoot "bios\\stage2.asm"
$kernelAsm = Join-Path $repoRoot "bios\\kernel.asm"
$bootBin = Join-Path $binDir "boot.bin"
$stage2Bin = Join-Path $binDir "stage2.bin"
$kernelBin = Join-Path $binDir "kernel.bin"
$bootImg = Join-Path $isoRoot "boot.img"
$stage2Size = 24 * 512
$kernelSize = 16 * 512

Require-Command nasm

Push-Location $repoRoot
try {
    Reset-Directory $stagingRoot
    New-Item -ItemType Directory -Force -Path $buildRoot, $distDir, $binDir, $isoRoot | Out-Null

    & nasm -f bin $bootAsm -o $bootBin
    if ($LASTEXITCODE -ne 0) {
        throw "BIOS boot sector build failed."
    }

    & nasm -f bin $stage2Asm -o $stage2Bin
    if ($LASTEXITCODE -ne 0) {
        throw "BIOS stage 2 build failed."
    }

    & nasm -f bin $kernelAsm -o $kernelBin
    if ($LASTEXITCODE -ne 0) {
        throw "BIOS kernel build failed."
    }

    if ((Get-Item $bootBin).Length -ne 512) {
        throw "Boot sector must be exactly 512 bytes."
    }
    if ((Get-Item $stage2Bin).Length -ne $stage2Size) {
        throw "Stage 2 must be exactly $stage2Size bytes."
    }
    if ((Get-Item $kernelBin).Length -ne $kernelSize) {
        throw "Kernel image must be exactly $kernelSize bytes."
    }

    $stream = [System.IO.File]::Create($bootImg)
    try {
        $stream.SetLength(1474560)
    }
    finally {
        $stream.Dispose()
    }

    $bootBytes = [System.IO.File]::ReadAllBytes($bootBin)
    $image = [System.IO.File]::Open($bootImg, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Write)
    try {
        $image.Write($bootBytes, 0, $bootBytes.Length)
        $stage2Bytes = [System.IO.File]::ReadAllBytes($stage2Bin)
        $image.Seek(512, [System.IO.SeekOrigin]::Begin) | Out-Null
        $image.Write($stage2Bytes, 0, $stage2Bytes.Length)
        $kernelBytes = [System.IO.File]::ReadAllBytes($kernelBin)
        $image.Seek(512 + $stage2Size, [System.IO.SeekOrigin]::Begin) | Out-Null
        $image.Write($kernelBytes, 0, $kernelBytes.Length)
    }
    finally {
        $image.Dispose()
    }

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
