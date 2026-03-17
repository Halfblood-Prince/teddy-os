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
64-bit Rust kernel desktop shell, arm hardware interrupts, and update live
status fields.

The kernel screen should include:

- a Teddy-OS desktop header
- a bottom taskbar with a live clock
- a `Welcome` window
- a `System Monitor` window with boot metadata and live counters
- a launcher panel when you press `l`

Press a few keys in VMware after the kernel screen appears:

- `l` opens and closes the launcher
- `tab` switches focus between visible windows
- `m` toggles move mode
- `w`, `a`, `s`, `d` move the focused window while move mode is enabled
- `x` closes the focused window
- `r` restores the default layout
- `1`, `2`, `3` open `Welcome`, `System Monitor`, and `Roadmap`

The `System Monitor` window should update `Ticks`, `Uptime`, `Last key`, and
`Scancode` while the taskbar clock advances once per second.
