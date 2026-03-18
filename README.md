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

- Teddy-OS text screen in BIOS mode
- the message `Legacy BIOS stage 2 online`
- the message `Boot OK - Stage 2 running`
- a `>` prompt that accepts keyboard input

Example commands:

- `help`
- `info`
- `clear`
- `echo hello`
- `graphics`
- `kernel`
- `kernelgfx`
- `kernelgfx800`
- `kernelgfx1024`
- `reboot`

When you run `kernel`, the current kernel MVP should show:

- a Teddy-OS desktop header and themed background
- a bottom taskbar with a live uptime clock
- a `File Explorer` window plus support windows
- boot metadata from stage 2 inside the system window
- live timer ticks plus the last keyboard scancode and ASCII value
- a launcher panel you can open from the taskbar area

When you run `kernelgfx`, Teddy-OS should boot a graphics-mode GUI scaffold:

- a VBE linear framebuffer graphics desktop
- a graphical top bar and taskbar
- desktop icons for `Terminal`, `Explorer`, and `Settings`
- window-like GUI apps rendered by the kernel
- bitmap text drawn by the new graphics layer
- a status panel with uptime, keyboard state, mouse coordinates, and button state
- a software mouse cursor driven by PS/2 IRQ12 input
- draggable `Terminal`, `File Explorer`, and `Settings` windows

Graphics boot modes:

- `kernelgfx` boots a `640x480x8` desktop
- `kernelgfx800` boots an `800x600x8` desktop
- `kernelgfx1024` boots a `1024x768x8` desktop

Kernel desktop controls:

- `F1` opens or closes the launcher
- `F2` focuses the next visible window
- `F3` toggles move mode for the focused window
- `w`, `a`, `s`, `d` move the focused window while move mode is active
- `F4` closes the focused window
- `F5` restores the default window layout
- launcher keys `1`, `2`, `3`, `4` open `Explorer`, `Welcome`, `System Monitor`, and `Roadmap`

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
- the real Terminal app is now wired into `kernelgfx*`
- the existing `kernel` command now boots the text Explorer/system desktop

## Next Step

Once this persistence milestone is proven stable in VMware, the next phase is
to improve the file explorer UI and then move toward broader app/windowing work
on top of the now-persistent filesystem layer.
