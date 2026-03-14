# Disk Utility App

This directory tracks the Teddy-OS disk and storage utility work.

Current Phase 14 implementation notes:

- the first usable disk utility surface is exposed through terminal commands
- `diskinfo` shows ATA device model, sector count, and capacity
- `df` shows TeddyFS space and entry usage
- `fsck` runs a lightweight TeddyFS integrity pass
- a dedicated GUI disk utility window remains future work
