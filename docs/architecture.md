# Teddy-OS Architecture

## Goals

Teddy-OS targets a practical educational MVP:

- `x86_64`
- `UEFI` boot
- `VMware` as the first supported virtual platform
- `Rust` as the primary implementation language
- `ISO` output for easy testing
- graphical desktop shell with original Teddy-OS branding

The system is staged so Phase 1 can boot cleanly with a framebuffer and debug
console, while later phases grow toward a desktop environment, persistence, and
safe updates.

## System Overview

The initial architecture uses a single-address-space kernel with in-kernel
applications. This keeps the MVP realistic and debuggable while leaving a clean
path toward stronger isolation later.

High-level boot and runtime flow:

1. UEFI firmware launches the Teddy-OS bootloader from the ISO.
2. The bootloader initializes graphics, obtains the memory map, and loads the
   kernel ELF plus boot metadata.
3. The kernel takes ownership of the framebuffer, interrupt tables, physical
   memory map, and early logging.
4. Core kernel services initialize in order: memory, interrupts, timer, input,
   storage, VFS, compositor, shell.
5. Built-in applications are started through a lightweight app runtime and use
   stable kernel-facing interfaces from shared libraries.

## Bootloader Strategy for UEFI

### Choice

Use a dedicated Rust UEFI bootloader crate built with the `uefi` crate and a
small custom handoff protocol.

### Why

- direct control over boot data passed to the kernel
- predictable framebuffer setup for early graphics
- serial and screen logging from the first instruction path
- no dependency on a larger third-party boot framework for long-term control

### Responsibilities

- start from `BOOTX64.EFI`
- locate and load the kernel ELF from the EFI system partition on the ISO
- allocate memory for kernel segments and boot structures
- capture the UEFI memory map
- select a framebuffer mode and pass framebuffer metadata
- optionally initialize serial output when available
- exit boot services cleanly
- jump to the kernel entry point with a typed `BootInfo` structure

### Handoff Data

The bootloader-to-kernel contract should include:

- framebuffer address, dimensions, stride, pixel format
- UEFI memory map
- RSDP pointer if present for future ACPI work
- kernel command line
- boot time and boot source identifiers
- optional init archive pointer for early assets and configs

## Kernel Architecture

### Initial Model

The kernel starts as a monolithic Rust kernel with clear internal modules:

- `arch/x86_64` - GDT, IDT, paging, APIC/PIC, CPU helpers
- `memory` - frame allocator, heap bootstrap, virtual mapping helpers
- `interrupts` - exception and IRQ handling
- `time` - PIT/HPET/APIC timer abstraction as phases progress
- `devices` - keyboard, mouse, framebuffer, block devices
- `graphics` - drawing primitives and compositor support
- `fs` - VFS, mount table, path handling, filesystem drivers
- `exec` - built-in app runtime and later process model
- `sys` - ABI surface for future userland separation
- `debug` - logging, panic, diagnostics, serial console

### MVP Scope

For the first several phases:

- single kernel address space
- cooperative or simple event-driven task scheduling
- built-in applications linked into the system image
- stable internal APIs that later can become syscalls

### Upgrade Path

After the desktop MVP exists, the architecture can evolve toward:

- ELF program loading
- per-process address spaces
- userspace drivers or services where worthwhile
- a stricter syscall ABI using crates already shared from `libs/`

## Graphics Stack

### Early Graphics

The first graphics target is raw framebuffer rendering:

- solid fills
- rectangle drawing
- line drawing if needed
- bitmap font text
- software cursor

### Layering Plan

1. `libs/graphics` defines pixel formats, rectangles, colors, surfaces
2. `kernel/graphics` provides framebuffer backends and primitive rendering
3. `shell` adds a software compositor with window surfaces and damage tracking
4. apps draw into owned buffers and ask the shell to present them

### Rationale

A software compositor is enough for VMware and dramatically simpler than GPU
acceleration. It also keeps the system deterministic and easy to debug.

## Window Manager and Desktop Shell Plan

The shell should feel familiar to users of mainstream desktop systems without
copying protected branding or exact layouts.

### MVP Features

- wallpaper/background
- bottom panel with launcher, task list, status area, and clock
- draggable overlapping windows
- active/inactive window frames
- simple start-menu-like application launcher
- mouse cursor
- original Teddy-OS icons and theme tokens

### Structure

- `shell/compositor` - z-order, surfaces, clipping, redraw scheduling
- `shell/chrome` - taskbar, launcher, status widgets, window decorations
- `shell/theme` - colors, spacing, icons, fonts
- `shell/session` - app lifecycle and desktop state

### Style Direction

The UI should be "Windows-inspired" only in familiarity:

- bottom anchored panel
- windowed desktop metaphor
- clear buttons and title bars

It must avoid copied icons, logos, names, artwork, or exact visual imitation.

## Terminal Design

The terminal begins as an in-kernel desktop app using shared UI and filesystem
interfaces.

### Core Pieces

- text grid backed by a framebuffer text renderer
- keyboard input and key repeat handling
- scrollback ring buffer
- built-in command parser
- current working directory state
- filesystem API integration through the VFS layer

### MVP Commands

- `help`
- `echo`
- `clear`
- `ls`
- `cd`
- `pwd`
- `cat`
- `mkdir`
- `rm`
- `touch`
- `uname`
- `reboot`
- `shutdown`

### Future Path

Later, the terminal can host spawned programs through the same ABI used by GUI
apps once process isolation exists.

## Virtual Filesystem Design

### VFS Model

Use a path-based VFS with inode-like handles and a mount table.

Key abstractions:

- `VNode` for files and directories
- `Mount` for mounted filesystem instances
- `FileHandle` for open state and cursor position
- `Path` normalization utilities

### Initial Mount Plan

- `/` root volume
- `/system` immutable-ish OS content
- `/data` writable user/app data
- `/logs` persisted log files when enabled
- `/tmp` memory-backed temporary storage

### Why a VFS First

It lets the shell, terminal, file explorer, and updater target stable APIs even
if the backing filesystem changes later.

## File Explorer Design

The file explorer is a GUI app layered on the desktop shell.

### MVP Features

- path bar or breadcrumb navigation
- directory listing with icons and names
- selection model
- open folder action
- create folder
- rename
- delete
- optional copy/move after the basics are stable

### Dependencies

- VFS for enumeration and file operations
- shell windowing APIs
- shared widget toolkit from `libs/ui`
- file associations from the shell session layer

## Updater Design

The updater must support safe system updates and updater self-updates.

### Strategy

Use an A/B-style system partition layout in later phases:

- active system slot: `system_a` or `system_b`
- inactive slot used for staging a new release
- small persistent config area tracks current slot and rollback flags

### Update Flow

1. query a GitHub-hosted manifest
2. compare installed version with latest compatible release
3. download update package into staging storage
4. verify checksum and, later, a signature
5. unpack into the inactive system slot
6. mark pending boot target
7. reboot
8. boot validation finalizes or rolls back

### Self-Update

Because the updater runs from the currently active system slot, it never
replaces its own executable in place. It stages the entire next system image in
the inactive slot instead.

### Failure Handling

- failed download leaves current slot untouched
- failed verification aborts install
- failed first boot rolls back to prior slot
- persistent logs record each step

## Package and Update Format

### Manifest

Use JSON for human-readable release metadata.

Example fields:

- `version`
- `channel`
- `min_bootloader_version`
- `artifacts`
- `sha256`
- `size`
- `published_at`
- `notes_url`

### Artifact Format

Use a `tar`-like archive or custom bundle with:

- kernel image
- bootloader EFI binaries
- shell/apps/libs payloads
- assets
- manifest

The early practical path is a versioned update archive generated from the same
staging tree used for ISO creation.

## Persistence and Storage Model

### Early Phases

Start with a simple block-device-backed filesystem suitable for VMware virtual
disks. The pragmatic path is:

- GPT partition table
- EFI System Partition for boot files
- Teddy system/data partition for OS content and writable files

### Filesystem Choice

Phase 5 should prefer either:

- FAT32 initially for maximum simplicity, then a Teddy native filesystem later
- or a small custom filesystem if the implementation remains realistic

Recommended plan:

1. use FAT32 support first so the system can read/write a simple persistent
   VMware disk without solving a complex filesystem immediately
2. abstract this behind the VFS
3. add a Teddy native filesystem later if needed for stronger semantics

## Logging and Debugging Strategy

Use multiple logging sinks from day one.

### Sinks

- on-screen emergency console
- serial output on COM1 for VMware capture
- in-memory ring buffer for recent logs
- persisted log files once storage is online

### Debugging Features

- panic handler with file and line output
- fatal error screen with recent log tail
- structured log levels: `error`, `warn`, `info`, `debug`, `trace`
- boot stage markers to narrow startup failures quickly

## ISO Generation Strategy

The build pipeline should produce deterministic staging directories before ISO
assembly.

### Planned Build Flow

1. build bootloader EFI binary
2. build kernel and built-in apps
3. collect assets and manifests
4. stage an EFI system partition tree
5. package it into a bootable UEFI ISO

### Repository Roles

- `scripts/` contains developer entry points such as `build.ps1`
- `build/` describes staging layouts and output conventions
- later CI uses the same scripts to avoid drift

## VMware Test Workflow

### Target VM Settings

- firmware: `UEFI`
- architecture: `x86_64`
- graphics: default VMware SVGA framebuffer path
- serial port: enabled and connected to a file for logs if possible
- disk: one virtual disk for persistent storage tests

### Recommended Iteration Loop

1. build debug ISO
2. boot in VMware with serial logging enabled
3. verify bootloader handoff and framebuffer initialization
4. exercise keyboard and shell input
5. inspect serial logs on failure

### Milestone-Oriented Test Sequence

- Phase 1: boot banner, framebuffer fill, serial logs
- Phase 2: timer ticks, keyboard input, primitive drawing
- Phase 3: desktop shell, cursor, taskbar, window movement
- Phase 4: terminal commands and redraw correctness
- Phase 5: persistence across reboot
- Phase 6: file explorer navigation and operations
- Phase 7: staged update and rollback test

## Monorepo Structure

```text
/
|-- bootloader/
|-- kernel/
|-- userland/
|-- apps/
|   |-- terminal/
|   |-- file_explorer/
|   `-- updater/
|-- shell/
|-- libs/
|-- assets/
|-- build/
|-- scripts/
`-- docs/
```

### Planned Shared Libraries

- `libs/boot_proto` - bootloader/kernel handoff types
- `libs/abi` - stable interfaces for apps and future syscalls
- `libs/graphics` - geometry, color, surfaces, text primitives
- `libs/ui` - widget and event abstractions
- `libs/storage` - block and filesystem-neutral traits
- `libs/teddy_config` - manifest and release metadata parsing

## Phase Sequencing Summary

- Phase 1: bootloader, kernel entry, framebuffer, logs, fatal halt
- Phase 2: interrupts, timer, keyboard, memory, graphics primitives
- Phase 3: shell, compositor, windows, taskbar, launcher
- Phase 4: terminal app and command set
- Phase 5: persistent filesystem and VFS-backed storage
- Phase 6: file explorer app
- Phase 7: updater and release format
- Phase 8: reproducible build and release workflow
- Phase 9: broad documentation pass

## Assumptions

- early builds prioritize VMware support over real hardware breadth
- software rendering is sufficient for the MVP
- a monolithic kernel is the fastest path to a usable educational desktop
- update safety matters more than minimizing storage usage

