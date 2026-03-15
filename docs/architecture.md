# Teddy-OS Reset Architecture

This reset starts from the smallest bootable baseline:

- one Rust `x86_64-unknown-uefi` application
- no separate kernel yet
- no storage, input, interrupts, or networking in the active boot path
- a framebuffer-rendered Teddy-OS desktop mock screen
- reproducible ISO output for VMware UEFI boot

## Why This Reset Exists

The previous tree had accumulated too many unstable assumptions at once:

- nightly-only kernel features
- early hardware initialization
- storage probing during boot
- multi-stage handoff complexity before the base image was reliable

The new baseline restores a known-good target:

1. UEFI firmware loads `BOOTX64.EFI`
2. Teddy-OS initializes GOP
3. Teddy-OS paints an original desktop-style screen
4. Teddy-OS stays alive in a simple loop

## Next Phases

- Phase 1 reset: bootable UEFI desktop screen
- Phase 2 reset: add a small in-app event/input loop
- Phase 3 reset: split into bootloader + kernel only after the single-image path is stable

