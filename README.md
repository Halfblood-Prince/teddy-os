# Teddy-OS

Teddy-OS is a hobby x86_64 operating system built in Rust for educational use.
The target environment is UEFI boot on VMware from a reproducible ISO image.

This repository is being developed in phases. Phase 6 is now implemented as a
desktop shell with both a terminal and a GUI file explorer backed by the
persistent TeddyFS volume.

## Current Status

- Phase 0 complete: architecture and repo layout
- Phase 1 complete: bootloader, kernel handoff, framebuffer logging, ISO scripts
- Phase 2 complete: interrupts, timer, keyboard input, memory stats, runtime loop
- Phase 3 complete: desktop shell, taskbar, launcher, clock, cursor, and dragging
- Phase 4 complete: terminal app, text rendering, parser, commands, and scrollback
- Phase 5 complete: persistent filesystem, path handling, metadata, and mounted storage
- Phase 6 complete: file explorer GUI and shell integration
- Phase 7 next: updater, release manifest handling, and staged install flow

## Repository Layout

- `bootloader/` - UEFI entry point and kernel handoff
- `kernel/` - core kernel, memory, interrupts, drivers, and graphics primitives
- `userland/` - shared runtime model for future user-space style components
- `apps/terminal/` - terminal application
- `apps/file_explorer/` - graphical file browser
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

## Build

Prerequisites:

- `cargo`
- `rustup`
- `mtools` (`mformat`, `mmd`, `mcopy`)
- `xorriso`

Build and package a debug ISO:

```powershell
./scripts/build.ps1
```

Build and package a release ISO:

```powershell
./scripts/build.ps1 -Release
```

## Design Principles

- Keep the early system small and bootable before adding complexity
- Prefer modular crates and stable interfaces over clever shortcuts
- Use original Teddy-OS branding and assets only
- Preserve a familiar desktop workflow without copying proprietary UI assets
- Document tradeoffs as the system grows
