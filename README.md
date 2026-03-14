# Teddy-OS

Teddy-OS is a hobby x86_64 operating system built in Rust for educational use.
The target environment is UEFI boot on VMware from a reproducible ISO image.

This repository is being developed in phases. Phase 6 is now implemented as a
desktop shell with both a terminal and a GUI file explorer backed by the
persistent TeddyFS volume. Phase 8 and Phase 9 are now implemented as the
build/release workflow and documentation pass. Phase 7 remains intentionally
deferred.

## Current Status

- Phase 0 complete: architecture and repo layout
- Phase 1 complete: bootloader, kernel handoff, framebuffer logging, ISO scripts
- Phase 2 complete: interrupts, timer, keyboard input, memory stats, runtime loop
- Phase 3 complete: desktop shell, taskbar, launcher, clock, cursor, and dragging
- Phase 4 complete: terminal app, text rendering, parser, commands, and scrollback
- Phase 5 complete: persistent filesystem, path handling, metadata, and mounted storage
- Phase 6 complete: file explorer GUI and shell integration
- Phase 7 deferred: updater, release manifest handling, and staged install flow
- Phase 8 complete: reproducible build scripts, release manifest generation, and VMware helper
- Phase 9 complete: architecture/build/VMware/release/theming/app docs
- Phase 10 complete: PCI NIC detection and networking diagnostics foundation
- Phase 11 complete: desktop input routing, focus, window controls, and resizing
- Phase 14 complete: storage diagnostics and TeddyFS integrity reporting

## Repository Layout

- `bootloader/` - UEFI entry point and kernel handoff
- `kernel/` - core kernel, memory, interrupts, drivers, and graphics primitives
- `userland/` - shared runtime model for future user-space style components
- `apps/terminal/` - terminal application
- `apps/file_explorer/` - graphical file browser
- `apps/disk_utility/` - disk and filesystem diagnostics
- `apps/updater/` - GUI updater and update orchestration logic
- `shell/` - desktop shell, taskbar, launcher, compositor, and theme logic
- `libs/` - shared crates for ABI, graphics, storage, UI, and utilities
- `assets/` - original Teddy-OS fonts, icons, themes, wallpapers, and manifests
- `build/` - build artifacts layout, image staging, and release metadata
- `scripts/` - build, image creation, ISO generation, and VMware helper scripts
- `docs/` - architecture, roadmap, and workflow documentation

## Phase 0 Documents

- `docs/architecture.md` - system design and component plan
- `docs/phase-0.md` - Phase 0 deliverables and next steps
- `docs/phase-1.md` - Phase 1 implementation, build, and VMware test steps
- `docs/phase-2.md` - Phase 2 kernel MVP implementation and test steps
- `docs/phase-3.md` - Phase 3 desktop shell implementation and VMware test steps
- `docs/phase-4.md` - Phase 4 terminal implementation and VMware test steps
- `docs/phase-5.md` - Phase 5 persistent filesystem and VMware disk workflow
- `docs/phase-6.md` - Phase 6 file explorer implementation and VMware test steps
- `docs/phase-8.md` - Phase 8 build and release pipeline
- `docs/phase-9.md` - Phase 9 documentation overview
- `docs/phase-10.md` - Phase 10 networking foundation and diagnostics
- `docs/building-from-scratch.md` - full local setup and build workflow
- `docs/running-in-vmware.md` - VMware setup and test loop
- `docs/creating-releases.md` - release artifact and manifest workflow
- `docs/updater-manifests.md` - planned update manifest schema for Phase 7
- `docs/adding-apps.md` - how to integrate future apps/components
- `docs/theming.md` - shell styling and Teddy-OS branding notes
- `docs/limitations-and-roadmap.md` - current limits and next milestones
- `docs/phase-11.md` - Phase 11 window manager and input dispatch improvements
- `docs/phase-14.md` - Phase 14 storage diagnostics and integrity checks

## Build

Prerequisites:

- `cargo`
- `rustup`
- `mtools` (`mformat`, `mmd`, `mcopy`)
- `xorriso`
- `vmrun` for the optional VMware helper

Build and package a debug ISO:

```powershell
./scripts/build.ps1
```

Build and package a release ISO:

```powershell
./scripts/build.ps1 -Profile release
```

Clean staged and distribution outputs:

```powershell
./scripts/clean.ps1
```

Create a release manifest for an already-built ISO:

```powershell
./scripts/release.ps1 -Version 0.1.0
```

Start a VMware VM from an existing `.vmx`:

```powershell
./scripts/run-vmware.ps1 -VmxPath C:\VMs\Teddy-OS\Teddy-OS.vmx
```

## Design Principles

- Keep the early system small and bootable before adding complexity
- Prefer modular crates and stable interfaces over clever shortcuts
- Use original Teddy-OS branding and assets only
- Preserve a familiar desktop workflow without copying proprietary UI assets
- Document tradeoffs as the system grows
