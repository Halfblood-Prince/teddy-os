# Teddy-OS

Teddy-OS has been reset and rebuilt from scratch around one goal: boot cleanly
in VMware under UEFI.

The current repository is a minimal Phase 1 baseline implemented in stable
Rust. It builds a single `x86_64-unknown-uefi` Teddy-OS application that opens
the UEFI framebuffer and renders an original desktop-style screen.

## What Exists Now

- a clean Rust workspace
- a single UEFI boot application in `bootloader/`
- framebuffer desktop-style rendering
- reproducible PowerShell build and ISO scripts
- GitHub Actions ISO build-and-release workflow
- fresh architecture and VMware docs

## Repo Layout

- `bootloader/` - stable Rust UEFI Teddy-OS app
- `docs/` - reset architecture and VMware notes
- `scripts/` - build, ISO, and clean scripts

## Build

Host requirements:

- `cargo`
- `rustup`
- `mtools` (`mformat`, `mmd`, `mcopy`)
- `xorriso`

Build the debug ISO:

```powershell
./scripts/build.ps1
```

Build the release ISO:

```powershell
./scripts/build.ps1 -Profile release
```

Clean outputs:

```powershell
./scripts/clean.ps1
```

## GitHub Actions

The workflow in [.github/workflows/build-iso.yml](c:/Users/HP/Downloads/teddy-os/.github/workflows/build-iso.yml)
can:

- build a debug or release ISO with `workflow_dispatch`
- upload the ISO and checksum as workflow artifacts
- publish them as a GitHub release
- publish automatically for pushed tags like `v0.1.0`

## VMware Test

1. Create a VM with `UEFI` firmware.
2. Attach `build/dist/teddy-os-debug.iso`.
3. Boot the VM.

Expected result:

- blue desktop background
- light bottom taskbar
- green start button
- status panel in the top-right

## Next Step

Once this reset baseline is proven stable in VMware, the next phase is to add a
small input/event loop before reintroducing a separate kernel.
