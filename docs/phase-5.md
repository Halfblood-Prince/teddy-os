# Phase 5

## Implemented in This Phase

- a simple persistent filesystem named `TeddyFS`
- a VMware-friendly ATA PIO block device layer for a writable virtual disk
- a mounted startup data volume used by the terminal
- filesystem features now available through the terminal:
  - create files
  - read files
  - overwrite files using `echo text > file`
  - delete files
  - create directories
  - remove empty directories
  - path traversal with `/`, `.`, and `..`
  - file metadata tracking for size and timestamps
- on-disk formatting and auto-mount behavior:
  - format the disk with TeddyFS if no valid filesystem is present
  - mount the filesystem on boot when the writable disk is attached

## Filesystem Strategy

Phase 5 uses a small custom filesystem rather than FAT32.

Tradeoffs:

- simpler implementation for this project stage
- easy to reason about and debug
- tightly scoped to current Teddy-OS terminal and future shell apps
- less flexible than a general-purpose mature filesystem

Current TeddyFS layout:

- sector 0: superblock
- sectors 1-8: fixed entry table
- remaining sectors: fixed per-entry file storage regions

This is intentionally small and constrained, but it is enough to provide real
create/read/write/delete behavior with persistence across VMware reboots.

## VMware Storage Model

For Phase 5, Teddy-OS expects a writable secondary virtual disk dedicated to
TeddyFS.

Recommended VMware setup:

- boot from the Teddy-OS ISO
- attach one additional IDE-compatible virtual disk
- leave the disk raw for TeddyFS to format on first boot

The ISO remains read-only. The secondary virtual disk provides the persistent
filesystem volume.

## Terminal Integration

The terminal now uses the mounted TeddyFS volume instead of the temporary
session-only in-memory tree from Phase 4.

Examples:

```text
pwd
ls
mkdir demo
cd demo
echo hello from teddy > note.txt
cat note.txt
cd ..
rm demo/note.txt
rm demo
```

## Build And Run

Build with the existing scripts:

```powershell
./scripts/build.ps1
```

Release:

```powershell
./scripts/build.ps1 -Release
```

## VMware Test Instructions

1. Create or update the VMware VM to include a writable secondary virtual disk.
2. Boot Teddy-OS in `UEFI` mode from the ISO.
3. Open the terminal window.
4. Run:
   - `ls`
   - `mkdir demo`
   - `cd demo`
   - `echo persistent test > note.txt`
   - `cat note.txt`
5. Reboot the VM.
6. Run:
   - `cd demo`
   - `cat note.txt`
7. Confirm the file contents survived reboot.

## Known Limitations

- TeddyFS currently uses a fixed-size entry table and fixed file data regions
- no partition table parsing yet; the attached writable disk is treated as a
  dedicated TeddyFS volume
- the block driver is ATA PIO only
- no caching, journaling, or crash recovery yet
- no file permissions or ownership model yet
- this phase was not compiled or VMware-tested in the current shell because the
  Rust toolchain is still unavailable on `PATH`

## Next Recommended Step

Phase 6 should build the GUI file explorer on top of TeddyFS and the desktop
shell window model.

