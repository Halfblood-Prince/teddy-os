# Teddy-OS Reset Architecture

This reset starts from the smallest bootable BIOS baseline:

- one 16-bit BIOS boot sector
- no separate kernel yet
- no storage, input, interrupts, or networking beyond BIOS services
- text-mode Teddy-OS status screen
- reproducible ISO output for VMware legacy BIOS boot

## Why This Reset Exists

The previous tree had accumulated too many unstable assumptions at once:

- a fragile UEFI-only boot path
- higher-level graphics initialization before a stable baseline existed
- too much pre-desktop complexity for the current state of the repo

The new baseline restores a known-good target:

1. BIOS loads the Teddy-OS boot sector
2. Teddy-OS switches to text mode
3. Teddy-OS paints a simple original boot screen
4. Teddy-OS stays alive in a simple halt loop

## Next Phases

- Phase 1 reset: bootable BIOS text screen
- Phase 2 reset: load a second stage from disk
- Phase 3 reset: bring back a Rust kernel behind the BIOS stage
