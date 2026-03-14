# Phase 2

## Implemented in This Phase

- early memory management for the current project stage:
  - boot-time memory statistics derived from the bootloader memory map
  - a simple bump-style frame allocator over the largest usable region after
    the kernel image
- interrupt setup:
  - IDT with breakpoint, double-fault, timer, and keyboard handlers
  - PIC remap and IRQ acknowledgement
  - CPU interrupt enable/disable helpers
- timer:
  - PIT configured to `100 Hz`
  - global tick counter and timer snapshot API
- keyboard input:
  - PS/2 keyboard IRQ handling on IRQ1
  - scancode decoding with a small event queue
  - kernel-facing input snapshot and polling API
- simple device/input abstraction:
  - `input::InputEvent`
  - `input::InputSnapshot`
  - event queue decoupling IRQ delivery from runtime consumption
- framebuffer drawing primitives:
  - pixel writes
  - filled rectangles
  - rectangle outlines
  - Bresenham line drawing
  - text rendering over arbitrary background colors
- basic execution model:
  - cooperative fixed-size scheduler
  - runtime tasks for status redraw and input processing

## Runtime Behavior

Phase 2 no longer halts immediately after boot.

The kernel now:

1. initializes memory accounting and a simple frame allocator
2. initializes the timer and keyboard input layer
3. loads the IDT and remaps the PIC
4. programs the PIT to generate periodic interrupts
5. enables interrupts
6. enters a cooperative runtime loop that:
   - redraws a live kernel status panel
   - consumes keyboard events
   - idles with `hlt` between interrupts

## What Appears on Screen

The framebuffer now shows a simple diagnostic scene built from primitives:

- background fill
- bordered paneling
- diagonal line rendering
- status box with live tick and memory information
- latest keyboard event display

This is still a kernel diagnostic surface, not the Phase 3 desktop shell.

## Build And Run

Phase 2 uses the same build pipeline as Phase 1:

```powershell
./scripts/build.ps1
```

Release:

```powershell
./scripts/build.ps1 -Release
```

## VMware Test Instructions

1. Build the ISO.
2. Boot the VM in `UEFI` mode.
3. Verify the diagnostic scene appears.
4. Confirm the tick counters advance over time.
5. Press keys and confirm the latest keyboard event panel changes.
6. Capture COM1 output and confirm boot logs mention:
   - memory statistics
   - frame allocator test allocation
   - PIT frequency
   - keyboard IRQ initialization

## Known Limitations

- no heap allocator yet
- no APIC, SMP, or advanced timer source yet
- keyboard input is limited to the legacy PS/2 path
- no mouse input yet
- the cooperative scheduler is intentionally minimal and not a true process
  model
- no syscall boundary or user-mode isolation yet
- this phase was not compiled or VMware-tested in the current shell because the
  Rust toolchain is still unavailable on `PATH`

## Next Recommended Step

Phase 3 should build on these primitives to create the graphical desktop shell:

- desktop background
- bottom taskbar/panel
- launcher
- window frames
- cursor rendering
- draggable windows
