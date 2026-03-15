# Teddy-OS

Teddy-OS has been reset and rebuilt from scratch around one goal: boot cleanly
in VMware in legacy BIOS mode.

The current repository is a minimal BIOS-first baseline. It builds a tiny
legacy BIOS boot image with a real second-stage loader and packages it into a
bootable ISO for VMware.

## What Exists Now

- a BIOS boot sector and second stage in `bios/`
- a legacy BIOS ISO build path
- reproducible PowerShell build and ISO scripts
- GitHub Actions ISO build-and-release workflow
- fresh architecture and VMware docs

## Repo Layout

- `bios/` - legacy BIOS boot sector and second-stage program
- `docs/` - reset architecture and VMware notes
- `scripts/` - build, ISO, and clean scripts

## Build

Host requirements:

- `nasm`
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

1. Create a VM with legacy BIOS firmware.
2. Attach `build/dist/teddy-os-debug.iso`.
3. Boot the VM.

Expected result:

- Teddy-OS text screen in BIOS mode
- the message `Legacy BIOS stage 2 online`
- the message `Boot OK - Stage 2 running`

## Next Step

Once this BIOS baseline is proven stable in VMware, the next phase is to add
keyboard input or switch the second stage into graphics mode before moving back
toward a Rust kernel.
