# Teddy-OS

Teddy-OS has been reset and rebuilt from scratch around one goal: boot cleanly
in VMware in legacy BIOS mode.

The current repository is a minimal BIOS-first baseline. It builds a tiny
legacy BIOS boot image with a real second-stage loader, a small interactive
text-mode shell, a simple graphics-mode demo, and a tiny x86_64 long-mode
kernel entry demo that can jump into a real Rust x86_64 kernel, then packages it into a
bootable ISO for VMware.

## What Exists Now

- a BIOS boot sector and second stage in `bios/`
- keyboard input and a tiny BIOS shell in stage 2
- a VGA mode `13h` graphics demo launched from the shell
- a `kernel` command that loads and jumps to a real Rust x86_64 kernel binary
- a modular Rust kernel with stable VGA text output
- a legacy BIOS ISO build path
- reproducible PowerShell build and ISO scripts
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

Local ISO outputs:

- `build/dist/teddy-os-debug.iso`
- `build/dist/teddy-os-debug.iso.sha256`
- `iso/teddy-os-debug.iso`
- `iso/teddy-os-debug.iso.sha256`

Build the release ISO:

```powershell
./scripts/build.ps1 -Profile release
```

Local ISO outputs:

- `build/dist/teddy-os-release.iso`
- `build/dist/teddy-os-release.iso.sha256`
- `iso/teddy-os-release.iso`
- `iso/teddy-os-release.iso.sha256`

Clean outputs:

```powershell
./scripts/clean.ps1
```

## VMware Test

1. Create a VM with legacy BIOS firmware.
2. Attach `iso/teddy-os-debug.iso` or `build/dist/teddy-os-debug.iso`.
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
- `reboot`

When you run `kernel`, the current kernel MVP should show:

- `TEDDY-OS KERNEL`
- `Rust x86_64 kernel loaded successfully`
- `Checkpoint: VGA console online`
- `Boot contract: deferred for next phase`
- `Kernel core is stable again`
- `Kernel idle loop active`

## Next Step

Once this BIOS baseline is proven stable in VMware, the next phase is to
grow the Rust kernel with interrupt handling, a timer, and keyboard input while
keeping the kernel handoff path stable.
