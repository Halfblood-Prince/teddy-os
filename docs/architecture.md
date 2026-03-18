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
- a minimal kernel-side VGA text console
- a keyboard-driven text-mode desktop shell layered on top of the kernel
- a real terminal app with command parsing and a kernel filesystem layer
- a keyboard-driven file explorer window using the same filesystem APIs
- ATA-backed persistence for the filesystem when a VMware IDE disk is present
- a separate graphics-mode kernel path with a framebuffer drawing scaffold
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
9. The Rust kernel currently includes a stable long-mode VGA console baseline
10. The kernel now owns a minimal IDT, PIC/PIT timer path, and PS/2 keyboard IRQ path
11. The kernel desktop shell owns screen rendering while IRQ handlers only update state
12. The terminal app is an isolated module that can later move behind app/window abstractions
13. The filesystem logic now lives in a dedicated kernel module instead of inside the terminal
14. The graphics shell now hosts both Terminal and Explorer against the same filesystem state
15. The filesystem can now serialize itself to a reserved disk region on a VMware IDE disk
16. A `kernelgfx` boot path now hands the kernel a mode `13h` framebuffer for GUI prerequisites

## Next Phases

- Phase 1 reset: bootable BIOS text screen
- Phase 2 reset: load a second stage from disk
- Phase 3 reset: move stage 2 toward graphics and kernel handoff
- Phase 4 reset: add interrupts, timer ticks, and keyboard input inside the Rust kernel
- Phase 5 reset: land a desktop-shell MVP in text mode so layout and window state exist before a framebuffer jump
- Phase 6 reset: add a real terminal app and an in-memory filesystem model without destabilizing the BIOS baseline
- Phase 7 reset: move filesystem logic into a dedicated kernel module and keep it memory-backed first
- Phase 8 reset: add a file explorer window on top of the shared filesystem APIs
- Phase 9 reset: add ATA-backed persistence behind that filesystem module so app-visible changes survive reboot
- Phase 10 reset: add a graphics framebuffer scaffold as the first prerequisite for clickable GUI work
- Phase 11 reset: add PS/2 mouse input, a software cursor, a small input event layer, desktop icons, window hit-testing, and VBE framebuffer boot modes on that graphics path
