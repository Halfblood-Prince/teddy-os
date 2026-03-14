# Terminal App

This directory tracks the Teddy-OS terminal application.

Current Phase 5 implementation notes:

- the current MVP terminal is implemented in-kernel for simplicity
- command parsing and scrollback are live
- the command set now targets the persistent TeddyFS volume
- `echo text > file` can be used to overwrite file contents
- Phase 6 should add a GUI file explorer on top of the same filesystem APIs
