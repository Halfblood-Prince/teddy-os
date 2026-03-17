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
64-bit Rust kernel screen, arm hardware interrupts, and update live status fields.

The kernel screen should include:

- `TEDDY-OS KERNEL`
- `Rust x86_64 kernel loaded successfully`
- `Checkpoint: VGA console online`
- `Boot contract: BIOS handoff stable`
- `Kernel core is stable again`
- boot metadata from the stage 2 handoff
- `Interrupts: IDT+PIC+PIT online`

Press a few keys in VMware after the kernel screen appears. The `Timer ticks`,
`Uptime seconds`, `Last keyboard scancode`, and `Last keyboard ascii` fields
should update without returning to the BIOS shell. You should also see an
`Input:` line, retained `Previous:` and `Output:` lines, and a `Result:` line
respond to `help`, `clear`, `ticks`, and `about`.
