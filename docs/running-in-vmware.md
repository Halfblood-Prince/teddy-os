# Running Teddy-OS In VMware

## VM Settings

- Guest type: Other
- Firmware: BIOS / Legacy
- Attach ISO: `build/dist/teddy-os-debug.iso`
- Add a small virtual IDE disk for filesystem persistence
- Video memory: default is fine

## Expected Result

You should see Teddy-OS boot directly into the graphics desktop without stopping
at the old BIOS command shell.

Additional graphics boot commands:

- the Settings app can save `640x480`, `800x600`, or `1024x768` for the next boot
- if no saved preference exists yet, Teddy-OS defaults to `1024x768`

The kernel screen should include:

- a Teddy-OS desktop header
- a bottom taskbar with a live clock
- a `Welcome` window
- a `System Monitor` window with boot metadata and live counters
- a launcher panel when you press `F1`

The `kernelgfx` graphics screen should include:

- a graphical Teddy-OS header
- a bottom taskbar
- desktop icons for `Terminal`, `Explorer`, `Writer`, and `Settings`
- window-like app panels with bitmap-rendered labels
- a status strip showing uptime, last key, scancode, mouse coordinates, and button state
- a software cursor that follows VMware mouse movement
- draggable `Terminal`, `File Explorer`, `Teddy Write`, `Image Viewer`, and `Settings` windows when you hold the left mouse button on their title bars
- a richer Explorer window with a path bar, toolbar, sidebar, entry list, and details strip
- title-bar controls for minimize, maximize/restore, and close
- taskbar buttons that restore minimized windows and minimize the focused one
- keyboard shortcuts for focus cycling, minimize, maximize/restore, and layout reset
- lower-right resize grips for mouse-driven window resizing

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

## Graphics Desktop Test

- boot the VM from the Teddy-OS ISO
- verify that the screen goes directly to the pixel desktop
- wait a few seconds and confirm the uptime changes
- press a few keys and confirm the status area updates
- move the mouse and confirm the cursor and `X` / `Y` values update
- double-click the `TERMINAL`, `EXPLORER`, and `SETTINGS` desktop icons to open their windows
- use the taskbar buttons to focus or hide those windows
- hold the left mouse button on a window title bar and drag it
- click the yellow title-bar button to minimize a window and confirm it stays on the taskbar
- click the blue title-bar button to maximize and restore a window
- click the same taskbar button again while a window is focused to minimize it
- drag the lower-right corner of a window to resize it
- press `F2` to cycle focus between visible windows
- press `F3` to minimize the focused window
- press `F4` to maximize and restore the focused window
- press `F5` to reset the default layout
- press `Esc` to clear focus back to the desktop
- in Explorer, click `HOME`, `UP`, `DIR`, `FILE`, `REN`, and `DEL` to exercise file operations
- double-click a `.txt` file in Explorer and confirm it opens in `Teddy Write`
- double-click `sample.timg` in Explorer and confirm it opens in `Image Viewer`
- import a `.png`, `.jpg`, or `.svg` on the host with `python scripts/import-image.py`, place the resulting `.timg` where Teddy-OS can access it, then open it from Explorer and confirm it appears in `Image Viewer`
- press `r` inside the focused Explorer window to rename the selected entry
- open `Settings`, click `640`, `800`, or `1024`, reboot the VM, and confirm Teddy-OS comes back in that resolution
- confirm the Settings window reflects the active resolution and color depth
- click the other mouse buttons and confirm the `B` value changes

Current limitation:

- `kernelgfx` now has real mouse input and a basic Settings app, but it is still only the first interactive GUI scaffold
- the real Terminal, Explorer, and Teddy Write apps now live on the `kernelgfx*` desktop path
- the older `kernel` desktop is now a text fallback shell
