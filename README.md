# Teddy-OS

Teddy-OS has been reset and rebuilt from scratch around one goal: boot cleanly
in VMware in legacy BIOS mode.

The current repository is a minimal BIOS-first baseline. It builds a tiny
legacy BIOS boot image with a real second-stage loader, a small interactive
text-mode shell, a simple graphics-mode demo, and a tiny x86_64 long-mode
kernel entry demo that can jump into a real Rust x86_64 kernel. The kernel now
hosts a text-mode desktop shell MVP, then packages everything into a bootable
ISO for VMware.

## What Exists Now

- a BIOS boot sector and second stage in `bios/`
- keyboard input and a tiny BIOS shell in stage 2
- a VGA mode `13h` graphics demo launched from the shell
- a `kernel` command that loads and jumps to a real Rust x86_64 kernel binary
- a modular Rust kernel with stable VGA text output, timer IRQs, keyboard IRQs, boot-info parsing, a text-mode desktop shell, a real terminal window MVP, a dedicated kernel filesystem module, a keyboard-driven file explorer window, and ATA-backed filesystem persistence
- a legacy BIOS ISO build path
- reproducible PowerShell build and ISO scripts
- GitHub Actions ISO build-and-release workflow
- fresh architecture and VMware docs

## Repo Layout

- `bios/` - legacy BIOS boot sector and second-stage program
- `docs/` - reset architecture and VMware notes
- `assets/` - desktop icon source images and future visual assets
- `scripts/` - build, ISO, and clean scripts

## Build

Host requirements:

- `nasm`
- `xorriso`
- `cargo`
- `rustup`
- `objcopy` or `llvm-objcopy`

Build the debug ISO:

```powershell
./scripts/build.ps1
```

Build the release ISO:

```powershell
./scripts/build.ps1 -Profile release
```

Clean outputs:

```powershell
./scripts/clean.ps1
```

## GitHub Actions

The workflow in [.github/workflows/build-iso.yml](c:/Users/HP/Downloads/teddy-os/.github/workflows/build-iso.yml)
can:

- build a debug or release ISO with `workflow_dispatch`
- upload the ISO and checksum as workflow artifacts
- publish them as a GitHub release
- publish automatically for pushed tags like `v0.1.0`

## VMware Test

1. Create a VM with legacy BIOS firmware.
2. Attach `build/dist/teddy-os-debug.iso`.
3. Boot the VM.

Expected result:

- Teddy-OS boots directly from BIOS stage 2 into the graphics desktop
- the Settings app controls which graphics boot mode is used on the next reboot
- if no saved setting exists yet, Teddy-OS defaults to `1024x768`

The current kernel desktop should show:

- a framebuffer graphics desktop
- a graphical top bar and taskbar
- desktop icons for `Terminal`, `Explorer`, and `Settings`
- window-like GUI apps rendered by the kernel
- bitmap text drawn by the new graphics layer
- a status panel with uptime, keyboard state, mouse coordinates, and button state
- a software mouse cursor driven by PS/2 IRQ12 input
- draggable `Terminal`, `File Explorer`, and `Settings` windows

Graphics boot modes:

- the Settings app can save `640x480`, `800x600`, or `1024x768` for the next boot
- if VMware BIOS rejects a VBE mode, Teddy-OS falls back to the stable `320x200x8` graphics path

Kernel desktop controls:

- `F1` opens or closes the launcher
- `F2` focuses the next visible window
- `F3` toggles move mode for the focused window
- `w`, `a`, `s`, `d` move the focused window while move mode is active
- `F4` closes the focused window
- `F5` restores the default window layout
- launcher keys `1`, `2`, `3` open `Welcome`, `System Monitor`, and `Roadmap`

Terminal commands:

- `help`
- `echo`
- `clear`
- `ls`
- `cd`
- `pwd`
- `cat`
- `mkdir`
- `rm`
- `touch`
- `uname`
- `reboot`
- `shutdown`

Filesystem note:

- the terminal and explorer now talk to the same kernel filesystem layer
- attach a small VMware IDE disk to enable persistence across reboot
- without that disk, Teddy-OS falls back to an in-memory filesystem

Explorer controls:

- `j` and `k` move the selection
- `Enter` opens folders or previews a file in the status area
- `b` goes to the parent directory
- `n` creates a new folder
- `t` creates a new file
- `x` deletes the selected entry

Graphics scaffold note:

- `kernelgfx` now includes PS/2 mouse input, a software cursor, desktop icons, taskbar buttons, draggable GUI windows, and a basic Settings app
- it is still a scaffold, not yet the full desktop replacement
- the real Terminal and Explorer apps are now wired into `kernelgfx*`
- the existing `kernel` command now boots the text fallback desktop

Custom desktop icons:

- put custom icon bitmaps in `assets/icons/`
- supported names are `terminal.bmp`, `explorer.bmp`, and `settings.bmp`
- supported format is uncompressed `24-bit` or `32-bit` BMP
- recommended icon size is `24x24` or `32x32`
- rebuild after adding an icon; the kernel build converts it automatically
- if an icon file is missing, Teddy-OS uses its built-in fallback icon

Explorer improvements in this phase:

- a clearer path bar and richer explorer layout on `kernelgfx*`
- toolbar actions for `HOME`, `UP`, `DIR`, `FILE`, `REN`, and `DEL`
- rename support in the shared filesystem layer and Explorer app
- a details strip that shows the selected entry type and size
- explorer rows now scale with the window height instead of being capped to four

Updated Explorer controls:

- `j` and `k` move the selection
- `Enter` opens folders or opens `.txt` files in Teddy Write
- `b` goes to the parent directory
- `h` returns to `/`
- `n` creates a new folder
- `t` creates a new file
- `r` renames the selected entry
- `x` deletes the selected entry

Windowing improvements in this phase:

- title bars now have working minimize, maximize/restore, and close buttons
- taskbar buttons now minimize the focused app and restore minimized apps
- minimized windows stay open on the taskbar without intercepting hit-testing
- maximized windows fill the desktop work area without overlapping the taskbar
- lower-right resize grips now let you resize windows with the mouse
- graphics desktop shortcuts now support focus cycling and window state changes

Graphics desktop shortcuts:

- `F2` cycles focus across visible windows
- `F3` minimizes the focused window
- `F4` maximizes or restores the focused window
- `F5` resets the default window layout
- `Esc` clears focus back to the desktop

## Next Step

With keyboard shortcuts and resize handles in place, the next recommended phase
is a cleaner desktop event system: route focus, keyboard, mouse, and command
events through shared app/window abstractions instead of ad hoc shell logic.
