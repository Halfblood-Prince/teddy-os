# Teddy-OS

Teddy-OS is a hobby x86_64 operating system built in Rust for educational use.
The target environment is UEFI boot on VMware from a reproducible ISO image.

This repository is being developed in phases. Phase 0 establishes the system
architecture, repository layout, and implementation plan for later phases.

## Current Status

- Phase 0 complete: architecture and repo layout
- Phase 1 next: bootable UEFI foundation with framebuffer and debug output

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

## Design Principles

- Keep the early system small and bootable before adding complexity
- Prefer modular crates and stable interfaces over clever shortcuts
- Use original Teddy-OS branding and assets only
- Preserve a familiar desktop workflow without copying proprietary UI assets
- Document tradeoffs as the system grows

