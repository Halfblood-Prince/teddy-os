# Phase 1

## Implemented in This Phase

- a Rust workspace with:
  - `bootloader` as a UEFI application
  - `kernel` as a freestanding x86_64 ELF binary
  - `libs/boot_proto` for the bootloader-to-kernel handoff contract
- a custom UEFI loader that:
  - reads `KERNEL.ELF` from the EFI system partition
  - initializes GOP framebuffer access
  - captures the UEFI memory map
  - finds the ACPI RSDP if present
  - exits boot services
  - jumps into the kernel with a typed `BootInfo`
- a minimal kernel that:
  - initializes COM1 serial output
  - writes boot logs to serial and framebuffer
  - validates boot metadata
  - reports framebuffer and memory map information
  - halts cleanly on completion or panic
- PowerShell scripts for:
  - building the bootloader and kernel
  - staging an EFI system partition tree
  - creating a UEFI-bootable ISO

## Boot Flow

1. VMware UEFI firmware loads `BOOTX64.EFI`.
2. The bootloader loads `KERNEL.ELF` from the same EFI partition.
3. The bootloader sets up framebuffer metadata and boot information.
4. The bootloader exits UEFI boot services and transfers control to
   `kernel_main`.
5. The kernel writes diagnostic output to the framebuffer and COM1, then
   intentionally halts pending Phase 2.

## Build Prerequisites

Install the following on the Windows host:

- Rust with `cargo` and `rustup` on `PATH`
- nightly toolchain
- UEFI target: `x86_64-unknown-uefi`
- `rust-src` and `llvm-tools-preview` components
- `mtools` commands: `mformat`, `mmd`, `mcopy`
- `xorriso`
- VMware Workstation or VMware Player configured for UEFI boot

The build script will request the Rust target and components automatically, but
the tools must exist on `PATH`.

## Exact Build Commands

Debug build plus ISO:

```powershell
./scripts/build.ps1
```

Release build plus ISO:

```powershell
./scripts/build.ps1 -Release
```

Build only and skip ISO packaging:

```powershell
./scripts/build.ps1 -SkipIso
```

Create an ISO from an already staged EFI tree:

```powershell
./scripts/make-iso.ps1 -Profile debug
```

## VMware Test Instructions

1. Create a new `x86_64` VMware VM.
2. Set firmware to `UEFI`, not legacy BIOS.
3. Attach the generated ISO from `build/dist/teddy-os-debug.iso`.
4. Add a serial port and log it to a file if possible.
5. Boot the VM.

Expected Phase 1 result:

- the UEFI bootloader prints early status messages
- the kernel takes over and logs to the framebuffer
- the screen shows Teddy-OS kernel boot text
- COM1 contains matching boot messages
- the kernel halts intentionally after initialization

## Known Limitations

- the kernel is still single-threaded and runs in a flat early-boot setup
- no interrupts, timers, keyboard input, heap allocator, or filesystem yet
- no runtime page-table ownership transfer beyond the firmware mappings
- ISO generation depends on external host tools (`mtools` and `xorriso`)
- this phase was not compiled locally in this environment because `cargo` and
  `rustup` were not available on `PATH`

## Next Recommended Step

Phase 2 should add:

- interrupt tables and exception handlers
- timer initialization
- keyboard input
- a simple device/input abstraction
- better framebuffer primitives beyond boot logging

