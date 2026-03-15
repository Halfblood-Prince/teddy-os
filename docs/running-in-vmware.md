# Running Teddy-OS In VMware

## VM Settings

- Guest type: Other
- Firmware: BIOS / Legacy
- Attach ISO: `build/dist/teddy-os-debug.iso`
- Video memory: default is fine

## Expected Result

You should see:

- a black text-mode screen
- a `TEDDY-OS` title
- a `Legacy BIOS stage 2 online` status line
- a `Boot OK - Stage 2 running` footer
- a `>` shell prompt that responds to keyboard input

Try `graphics` at the prompt. You should see a simple Teddy-OS graphics screen
and return to the shell after pressing a key.

Try `kernel` at the prompt. You should see Teddy-OS switch into a protected-mode
64-bit Rust kernel screen, verify the boot contract, and remain in an idle loop.

The kernel screen should include:

- `TEDDY-OS KERNEL`
- `Rust x86_64 kernel loaded successfully`
- `Serial logging active on COM1 (0x3F8)`
- `Boot contract: verified`
- `Kernel idle loop active`

If VMware is configured to expose a serial port, the same boot progress is also
written to COM1 for debugging.
