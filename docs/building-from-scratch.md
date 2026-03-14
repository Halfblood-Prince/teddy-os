# Building Teddy-OS From Scratch

## Host Requirements

Teddy-OS currently expects a Windows host with PowerShell and the following
tools on `PATH`:

- `cargo`
- `rustup`
- `mformat`
- `mmd`
- `mcopy`
- `xorriso`

Optional:

- `vmrun` for starting a prepared VMware VM from the command line

## Toolchain Setup

Install Rust and make sure the nightly toolchain is available, because the
kernel uses `build-std` and nightly-only flags.

The build script installs these Rust components if needed:

- `x86_64-unknown-uefi`
- `rust-src`
- `llvm-tools-preview`

## Clean Build

```powershell
./scripts/clean.ps1
./scripts/build.ps1
```

This produces:

- `build/dist/teddy-os-debug.iso`
- `build/dist/teddy-os-debug.iso.sha256`
- `build/dist/build-debug.json`

## Release Build

```powershell
./scripts/build.ps1 -Profile release
./scripts/release.ps1 -Version 0.1.0
```

This produces:

- `build/dist/teddy-os-release.iso`
- `build/dist/teddy-os-release.iso.sha256`
- `build/dist/build-release.json`
- `build/dist/release-0.1.0.json`

## Build Notes

- the Cargo target directory is pinned to `build/target/`
- EFI staging is rebuilt from a clean tree each run
- ISO output names are stable per profile
- the scripts fail fast when required host tools are missing

## Current Limitation

This repo still depends on host build tools and has not yet been verified from
this shell because the Rust toolchain was unavailable on `PATH` during this
phase implementation.
