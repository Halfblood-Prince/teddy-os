# Phase 4

## Implemented in This Phase

- a real terminal window integrated into the Phase 3 desktop shell
- text rendering inside the terminal content area
- keyboard-driven input handling for:
  - printable characters
  - backspace
  - enter
  - history navigation with up/down
- scrollback support with a fixed in-memory line buffer
- command parsing with whitespace tokenization
- built-in commands:
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
- a small in-memory filesystem-facing API used by the terminal commands

## Terminal Behavior

The terminal is currently the main interactive application in Teddy-OS.

It supports:

- prompt rendering with the current working directory
- command history cycling with up/down arrows
- simple in-memory directory traversal
- file creation and deletion in a temporary session-only filesystem
- basic output and multi-line command responses

## Filesystem Integration Strategy

Phase 4 does not implement the persistent filesystem yet. Instead, the terminal
uses a small in-memory VFS-style layer so the command set works now without
pretending Phase 5 is complete.

That means:

- `ls`, `cd`, `pwd`, `cat`, `mkdir`, `rm`, and `touch` all work
- changes are session-only
- persistence across reboot is still a Phase 5 task

## Build And Run

Build with the existing scripts:

```powershell
./scripts/build.ps1
```

Release build:

```powershell
./scripts/build.ps1 -Release
```

## VMware Test Instructions

1. Build and boot the ISO in `UEFI` mode.
2. Wait for the desktop shell to appear.
3. Focus the terminal window.
4. Type commands such as:
   - `help`
   - `pwd`
   - `ls`
   - `mkdir demo`
   - `cd demo`
   - `touch note.txt`
   - `ls`
   - `cd ..`
   - `cat readme.txt`
   - `uname`
5. Verify scrollback is visible and command history works with up/down arrows.

## Known Limitations

- the terminal currently runs in kernel space
- the filesystem used by the terminal is in-memory only
- no persistent storage yet
- no terminal process spawning yet
- `reboot` and `shutdown` are still low-level kernel actions rather than full
  OS-managed power flows
- this phase was not compiled or VMware-tested in the current shell because the
  Rust toolchain is still unavailable on `PATH`

## Next Recommended Step

Phase 5 should replace the temporary in-memory filesystem behind the terminal
with a real mounted filesystem that persists across VMware reboots.

