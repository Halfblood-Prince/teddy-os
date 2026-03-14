# Terminal App

This directory tracks the Teddy-OS terminal application.

Current Phase 4 implementation notes:

- the current MVP terminal is implemented in-kernel for simplicity
- command parsing and scrollback are live
- the command set uses a temporary in-memory filesystem-facing API
- Phase 5 should replace that temporary layer with the real filesystem backend
