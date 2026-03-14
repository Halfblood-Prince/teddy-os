# Phase 3

## Implemented in This Phase

- a graphical desktop shell rendered directly on the kernel framebuffer
- an original Teddy-OS visual theme with:
  - wallpaper gradient
  - accent graphics
  - bottom taskbar
  - launcher button
  - clock
- window chrome and desktop composition:
  - multiple overlapping demo windows
  - active and inactive title bars
  - original title bar and frame styling
- mouse support for the shell:
  - PS/2 mouse initialization
  - IRQ12 handler
  - mouse packet decoding
  - cursor position tracking
  - left-button dragging for windows
- launcher behavior:
  - start-menu-like popup panel
  - placeholder entries for terminal, files, updater, and settings
- runtime integration:
  - cooperative tasks now pump keyboard input, mouse input, and shell redraws

## Runtime Behavior

Phase 3 replaces the pure diagnostic scene from Phase 2 with a desktop shell.

The kernel now:

1. initializes framebuffer, memory, timer, keyboard, and mouse support
2. enters the shell runtime
3. renders a desktop background and taskbar
4. shows several overlapping windows
5. tracks the mouse cursor
6. allows dragging windows by their title bars
7. toggles the launcher popup from the taskbar button

## What Appears on Screen

The desktop now includes:

- original Teddy-OS wallpaper
- bottom taskbar with launcher button and clock
- top-right session/status panel
- multiple draggable windows
- mouse cursor
- launcher popup panel

This is still an in-kernel shell. Applications are placeholders until later
phases.

## Build And Run

Phase 3 uses the existing build pipeline:

```powershell
./scripts/build.ps1
```

Release build:

```powershell
./scripts/build.ps1 -Release
```

## VMware Test Instructions

1. Build the ISO.
2. Boot VMware in `UEFI` mode.
3. Verify the desktop shell appears instead of the Phase 2 status-only scene.
4. Move the mouse and confirm the cursor moves.
5. Drag a window by its title bar.
6. Click the launcher button and confirm the popup opens and closes.
7. Verify the taskbar clock advances as ticks accumulate.
8. Capture COM1 output and confirm boot logs mention keyboard and mouse IRQs.

## Known Limitations

- the shell is still rendered entirely in software in kernel space
- no true window compositor damage tracking yet
- launcher items are placeholders and do not start real apps yet
- no mouse wheel or middle button support
- no desktop icons yet
- no terminal/file explorer/updater implementations in this phase
- this phase was not compiled or VMware-tested in the current shell because the
  Rust toolchain is still unavailable on `PATH`

## Next Recommended Step

Phase 4 should add the first real application:

- terminal window content
- text buffer rendering
- command parsing
- filesystem-oriented built-in commands
