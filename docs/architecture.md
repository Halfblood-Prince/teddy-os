# Teddy-OS Reset Architecture

This reset starts from a small but extensible BIOS baseline:

- one 16-bit BIOS boot sector
- one fixed-location second-stage program loaded from disk
- no separate kernel yet
- no advanced drivers or networking beyond BIOS services
- text-mode Teddy-OS status screen and tiny shell
- optional VGA mode `13h` graphics demo launched from the shell
- optional x86_64 long-mode entry demo launched from the shell
- a real Rust x86_64 kernel binary loaded by stage 2
- a minimal kernel-side console layer for VGA text output
- reproducible ISO output for VMware legacy BIOS boot

## Why This Reset Exists

The previous tree had accumulated too many unstable assumptions at once:

- a fragile UEFI-only boot path
- higher-level graphics initialization before a stable baseline existed
- too much pre-desktop complexity for the current state of the repo

The new baseline restores a known-good target:

1. BIOS loads the Teddy-OS boot sector
2. Stage 1 reads a fixed second stage from disk sectors
3. Stage 2 switches to text mode, paints a Teddy-OS status screen, and starts a shell
4. Keyboard input is handled via BIOS INT 16h
5. A graphics demo can switch to VGA mode `13h` and return to the shell
6. A kernel demo can switch stage 2 through protected mode into x86_64 long mode
7. Long mode uses identity-mapped paging so the kernel path is truly 64-bit
8. Stage 2 can load a flat Rust kernel binary from later disk sectors and jump to it
9. The Rust kernel validates the stage-2 boot handoff and displays it through VGA text mode

## Next Phases

- Phase 1 reset: bootable BIOS text screen
- Phase 2 reset: load a second stage from disk
- Phase 3 reset: move stage 2 toward graphics and kernel handoff
- Phase 4 reset: add interrupts, timer ticks, and keyboard input inside the Rust kernel
