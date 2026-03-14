# Phase 6

## Implemented in This Phase

- a GUI file explorer integrated into the Teddy-OS desktop shell
- a dedicated explorer window rendered alongside the terminal
- filesystem browsing on top of TeddyFS with:
  - current path bar
  - directory listing
  - selection
  - file preview pane
- folder open behavior via double-click-equivalent mouse interaction
- basic file operations from the explorer toolbar:
  - up one level
  - create folder
  - rename selected entry
  - delete selected entry

## Explorer Behavior

The explorer window now provides a simple two-pane layout:

- left pane: directory contents
- right pane: file preview or folder details

Interaction model:

- single-click selects an entry
- double-click opens a folder or previews a file
- toolbar buttons perform navigation and simple file operations

The explorer uses the same TeddyFS volume that the terminal uses.

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

1. Boot Teddy-OS with the writable TeddyFS disk attached.
2. Wait for the desktop shell to load.
3. Click inside the `Teddy Explorer` window.
4. Select files and folders from the list.
5. Double-click a folder to open it.
6. Use the toolbar buttons:
   - `Up`
   - `New`
   - `Rename`
   - `Delete`
7. Confirm the preview pane updates for selected files.
8. Use the terminal to verify the same changes with `ls`, `cd`, and `cat`.

## Known Limitations

- the explorer currently uses mouse interaction only
- rename uses a generated suffix-based rename flow rather than free-form text entry
- copy and move are still deferred
- file preview is text-oriented and basic
- this phase was not compiled or VMware-tested in the current shell because the
  Rust toolchain is still unavailable on `PATH`

## Next Recommended Step

Phase 7 should implement the updater on top of TeddyFS and the shell window
model, including staged updates and rollback-safe installation flow.

