# Running Teddy-OS In VMware

## VM Settings

- Guest type: Other
- Firmware: BIOS / Legacy
- Attach ISO: `build/dist/teddy-os-debug.iso`
- Add a small virtual IDE disk for filesystem persistence
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
- a `Terminal` window
- a `Welcome` window
- a `System Monitor` window with boot metadata and live counters
- a launcher panel when you press `F1`

Press a few keys in VMware after the kernel screen appears:

- `F1` opens and closes the launcher
- `F2` switches focus between visible windows
- `F3` toggles move mode
- `w`, `a`, `s`, `d` move the focused window while move mode is enabled
- `F4` closes the focused window
- `F5` restores the default layout
- launcher keys `1`, `2`, `3`, `4` open `Terminal`, `Welcome`, `System Monitor`, and `Roadmap`

Try these terminal commands in the focused `Terminal` window:

- `help`
- `ls`
- `pwd`
- `cat readme.txt`
- `cd docs`
- `ls`
- `touch demo.txt`
- `mkdir tmp`
- `uname`

The `System Monitor` window should update `Ticks`, `Uptime`, `Last key`, and
`Scancode` while the taskbar clock advances once per second.

## Persistence Setup

- Add a new virtual hard disk in VMware before booting Teddy-OS.
- Use an IDE disk if possible so it appears on the legacy primary ATA ports.
- A tiny disk is enough; `16 MB` is plenty for the current filesystem image.
- Boot from the ISO as usual, then run `kernel`.

Expected persistence result:

- the `System Monitor` window should show `Storage  disk loaded` or `Storage  disk seeded`
- create a file or folder in Terminal or Explorer
- reboot the VM
- the created entries should still be present after returning to the kernel desktop
