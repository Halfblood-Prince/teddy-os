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
$targetDir = Join-Path $buildRoot "target"
$isoRoot = Join-Path $stagingRoot "iso"
$bootAsm = Join-Path $repoRoot "bios\\boot.asm"
$stage2Asm = Join-Path $repoRoot "bios\\stage2.asm"
$bootBin = Join-Path $binDir "boot.bin"
$stage2Bin = Join-Path $binDir "stage2.bin"
$kernelElf = Join-Path (Join-Path $targetDir "x86_64-unknown-none\\$Profile") "teddy-kernel"
$kernelRaw = Join-Path $binDir "kernel.bin"
$bootImg = Join-Path $isoRoot "boot.img"
$stage2Size = 96 * 512
$kernelSize = 128 * 512

Require-Command nasm
Require-Command cargo
Require-Command rustup

Push-Location $repoRoot
try {
    Reset-Directory $stagingRoot
    New-Item -ItemType Directory -Force -Path $buildRoot, $distDir, $binDir, $isoRoot, $targetDir | Out-Null
    $env:CARGO_TARGET_DIR = $targetDir

    rustup target add x86_64-unknown-none | Out-Host

    & nasm -f bin $bootAsm -o $bootBin
    if ($LASTEXITCODE -ne 0) {
        throw "BIOS boot sector build failed."
    }

    & nasm -f bin $stage2Asm -o $stage2Bin
    if ($LASTEXITCODE -ne 0) {
        throw "BIOS stage 2 build failed."
    }

    $kernelArgs = @("build", "-p", "teddy-kernel", "--target", "x86_64-unknown-none")
    if ($Profile -eq "release") {
        $kernelArgs += "--release"
    }

    $previousRustFlags = $env:RUSTFLAGS
    $kernelRustFlags = @(
        "-Ctarget-cpu=x86-64",
        "-Ctarget-feature=-mmx,-sse,-sse2,-avx,-avx2",
        "-Cforce-frame-pointers=yes",
        "-Cno-redzone=yes",
        "-Crelocation-model=static",
        "-Clink-arg=-Tkernel/linker.ld",
        "-Clink-arg=--build-id=none"
    ) -join " "
    if ([string]::IsNullOrWhiteSpace($previousRustFlags)) {
        $env:RUSTFLAGS = $kernelRustFlags
    } else {
        $env:RUSTFLAGS = "$previousRustFlags $kernelRustFlags"
    }

    & cargo @kernelArgs
    $env:RUSTFLAGS = $previousRustFlags
    if ($LASTEXITCODE -ne 0) {
        throw "Rust kernel build failed."
    }

    $objcopy = Get-Command llvm-objcopy -ErrorAction SilentlyContinue
    if (-not $objcopy) {
        $objcopy = Get-Command objcopy -ErrorAction SilentlyContinue
    }
    $objcopyPath = $null
    if ($objcopy) {
        $objcopyPath = $objcopy.Source
        if ([string]::IsNullOrWhiteSpace($objcopyPath)) {
            $objcopyPath = $objcopy.Path
        }
    }
    if (-not $objcopy) {
        $rustcPath = & rustup which rustc
        $toolBin = Join-Path (Split-Path $rustcPath) "llvm-objcopy"
        if (Test-Path $toolBin) {
            $objcopyPath = $toolBin
        }
    }
    if ([string]::IsNullOrWhiteSpace($objcopyPath)) {
        throw "Could not locate objcopy or llvm-objcopy."
    }

    & $objcopyPath -O binary $kernelElf $kernelRaw
    if ($LASTEXITCODE -ne 0) {
        throw "Rust kernel objcopy failed."
    }

    if ((Get-Item $bootBin).Length -ne 512) {
        throw "Boot sector must be exactly 512 bytes."
    }
    if ((Get-Item $stage2Bin).Length -ne $stage2Size) {
        throw "Stage 2 must be exactly $stage2Size bytes."
    }
    if ((Get-Item $kernelRaw).Length -gt $kernelSize) {
        throw "Rust kernel exceeds $kernelSize bytes."
    }

    $kernelStream = [System.IO.File]::Open($kernelRaw, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Write)
    try {
        $kernelStream.SetLength($kernelSize)
    }
    finally {
        $kernelStream.Dispose()
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
        $kernelBytes = [System.IO.File]::ReadAllBytes($kernelRaw)
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
