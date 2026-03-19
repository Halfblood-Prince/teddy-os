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

Try `kernelgfx` at the prompt for the new graphics prerequisite path. You
should see Teddy-OS switch into a higher-resolution VBE framebuffer mode and
render a graphical desktop scaffold driven by the kernel framebuffer code.

Additional graphics boot commands:

- `kernelgfx` for preferred `640x480x32` with `640x480x8` fallback
- `kernelgfx800` for preferred `800x600x32` with `800x600x8` fallback
- `kernelgfx1024` for preferred `1024x768x32` with `1024x768x8` fallback

The kernel screen should include:

- a Teddy-OS desktop header
- a bottom taskbar with a live clock
- a `Welcome` window
- a `System Monitor` window with boot metadata and live counters
- a launcher panel when you press `F1`

The `kernelgfx` graphics screen should include:

- a graphical Teddy-OS header
- a bottom taskbar
- desktop icons for `Terminal`, `Explorer`, and `Settings`
- window-like app panels with bitmap-rendered labels
- a status strip showing uptime, last key, scancode, mouse coordinates, and button state
- a software cursor that follows VMware mouse movement
- draggable `Terminal`, `File Explorer`, `Teddy Write`, and `Settings` windows when you hold the left mouse button on their title bars
- a richer Explorer window with a path bar, toolbar, sidebar, entry list, and details strip

Press a few keys in VMware after the kernel screen appears:

- `F1` opens and closes the launcher
- `F2` switches focus between visible windows
- `F3` toggles move mode
- `w`, `a`, `s`, `d` move the focused window while move mode is enabled
- `F4` closes the focused window
- `F5` restores the default layout
- launcher keys `1`, `2`, `3` open `Welcome`, `System Monitor`, and `Roadmap`

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
- create a file or folder in the `kernelgfx` Explorer app
- reboot the VM
- the created entries should still be present after returning to the kernel desktop

## Graphics Scaffold Test

- boot to the BIOS shell
- run `kernelgfx`
- verify that the screen changes from text mode to a pixel UI
- optionally retry with `kernelgfx800` or `kernelgfx1024`
- wait a few seconds and confirm the uptime changes
- press a few keys and confirm the status area updates
- move the mouse and confirm the cursor and `X` / `Y` values update
- double-click the `TERMINAL`, `EXPLORER`, and `SETTINGS` desktop icons to open their windows
- use the taskbar buttons to focus or hide those windows
- hold the left mouse button on a window title bar and drag it
- in Explorer, click `HOME`, `UP`, `DIR`, `FILE`, `REN`, and `DEL` to exercise file operations
- double-click a `.txt` file in Explorer and confirm it opens in `Teddy Write`
- press `r` inside the focused Explorer window to rename the selected entry
- confirm the Settings window reflects the active resolution and color depth
- click the other mouse buttons and confirm the `B` value changes

Current limitation:

- `kernelgfx` now has real mouse input and a basic Settings app, but it is still only the first interactive GUI scaffold
- the real Terminal, Explorer, and Teddy Write apps now live on the `kernelgfx*` desktop path
- the older `kernel` desktop is now a text fallback shell
