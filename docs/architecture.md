# Teddy-OS Reset Architecture

This reset starts from a small but extensible BIOS baseline:

- one 16-bit BIOS boot sector
- one fixed-location second-stage program loaded from disk
- no separate kernel yet
- no advanced drivers, input, or networking beyond BIOS services
- text-mode Teddy-OS status screen
- reproducible ISO output for VMware legacy BIOS boot

## Why This Reset Exists

The previous tree had accumulated too many unstable assumptions at once:

- a fragile UEFI-only boot path
- higher-level graphics initialization before a stable baseline existed
- too much pre-desktop complexity for the current state of the repo

The new baseline restores a known-good target:

1. BIOS loads the Teddy-OS boot sector
2. Stage 1 reads a fixed second stage from disk sectors
3. Stage 2 switches to text mode and paints a Teddy-OS status screen
4. Stage 2 stays alive in a simple halt loop

## Next Phases

- Phase 1 reset: bootable BIOS text screen
- Phase 2 reset: load a second stage from disk
- Phase 3 reset: move stage 2 toward graphics and kernel handoff
