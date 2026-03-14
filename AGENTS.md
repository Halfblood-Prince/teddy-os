You are a senior systems programmer and OS architect. Help me build a hobby operating system called Teddy-OS for educational use. The target is to boot in VMware from an ISO image and present a desktop environment with a Windows-inspired visual style, but without copying Microsoft trademarks, logos, proprietary assets, or exact UI artwork.

High-level goal:
Create a minimal but usable x86_64 hobby OS with:
1. UEFI boot support
2. A kernel
3. A graphical desktop shell with a Windows-inspired layout
4. A terminal app
5. A filesystem layer
6. A file explorer GUI
7. A software updater that pulls releases/updates from my GitHub repository, installs them on the OS, and can update itself safely
8. A reproducible build system that outputs a bootable ISO for VMware

Important constraints:
- Target architecture: x86_64
- Boot mode: UEFI
- Virtualization target: VMware
- Output: bootable ISO
- Language preference: Rust
- Must be modular and readable
- Must include comments and docs
- Must not depend on Linux after boot
- Use only original branding/assets named Teddy-OS
- UI should feel familiar to Windows users, but use original themes/icons/layout details to avoid cloning copyrighted/trademarked material

Preferred approach:
- Start with a small, realistic MVP and expand incrementally
- Favor reliability and simplicity over advanced features
- Use a monorepo structure with clear folders
- Produce code, build scripts, docs, and test steps
- At every stage, explain what was added, what remains, and how to test it in VMware

Please design and implement Teddy-OS in phases.

Phase 0 — Architecture and repo layout
First, propose a complete architecture and repo structure. Include:
- bootloader strategy for UEFI
- kernel architecture
- graphics stack
- window manager / desktop shell plan
- terminal design
- virtual filesystem design
- file explorer design
- updater design
- package/update format
- persistence/storage model
- logging/debugging strategy
- ISO generation strategy
- VMware test workflow

Then create a repo layout like:
- /bootloader
- /kernel
- /userland
- /apps/terminal
- /apps/file_explorer
- /apps/updater
- /shell
- /libs
- /assets
- /build
- /scripts
- /docs

Phase 1 — Bootable foundation
Implement a minimal bootable UEFI system that:
- boots in VMware
- loads the kernel
- initializes framebuffer graphics
- prints debug output to screen and serial/log if possible
- halts cleanly on fatal error

Deliverables:
- source code
- build scripts
- ISO generation script
- README with exact commands
- VMware test instructions

Phase 2 — Kernel MVP
Implement a minimal kernel with:
- memory management sufficient for the project stage
- interrupt setup
- timer
- keyboard input
- simple device/input abstraction
- framebuffer drawing primitives
- basic task/process model if needed
- syscall or kernel API boundary for userland/apps if using userland separation

Keep scope realistic. If full process isolation is too large for first pass, start with a simpler model and clearly mark future upgrades.

Phase 3 — Desktop shell
Implement a graphical desktop shell with a Windows-inspired but original layout:
- wallpaper/background
- bottom taskbar/panel
- start-menu-like app launcher
- clock
- window frames
- mouse cursor
- simple theming
- draggable windows

Do not use Microsoft names, logos, or copied assets.
Create original placeholder assets for Teddy-OS.

Phase 4 — Terminal
Implement a terminal application with:
- text rendering
- input handling
- scrollback
- built-in commands such as:
  - help
  - echo
  - clear
  - ls
  - cd
  - pwd
  - cat
  - mkdir
  - rm
  - touch
  - uname
  - reboot
  - shutdown
- command parser
- integration with filesystem APIs

Phase 5 — Filesystem
Implement a simple filesystem strategy suitable for a hobby OS. You may:
- either implement a simple custom filesystem
- or support a simpler existing format if practical for VMware disk images

Requirements:
- create/read/write/delete files
- directories
- path handling
- file metadata
- mounting startup volume
- persistence across reboots in VMware

Document tradeoffs clearly.

Phase 6 — File Explorer
Implement a GUI file explorer with:
- navigation pane or breadcrumb path bar
- directory listing
- open folders
- select files
- basic file operations:
  - create folder
  - rename
  - delete
  - copy/move if feasible
- double-click or equivalent open behavior
- integration with desktop shell

Phase 7 — Software updater
Implement a Teddy-OS updater with self-update support.

Requirements:
- connect to my GitHub repository
- check a releases endpoint or manifest
- compare installed version vs latest
- download update package
- verify integrity with checksum/signature
- stage update safely
- install update on reboot or via atomic replacement strategy
- updater must be able to update itself safely
- provide rollback or fail-safe behavior if update fails
- show progress in GUI
- log update steps for debugging

Design requirements:
- use a manifest format such as JSON
- support versioning
- include example GitHub release layout and manifest schema
- include code for updater client and installation flow
- explain how I should publish releases from GitHub for Teddy-OS

Phase 8 — Build and release pipeline
Create:
- reproducible build steps
- ISO output script
- debug and release builds
- a script to run in VMware if possible
- docs for adding future apps/components

Phase 9 — Documentation
Write documentation for:
- architecture overview
- building from scratch
- running in VMware
- creating releases
- updater manifests
- adding apps
- theming/customization
- limitations and roadmap

Engineering standards:
- prefer small, complete increments
- never leave placeholder pseudocode when real code is reasonable
- when something is too large, implement the smallest working version and note next steps
- include comments for low-level code
- explain assumptions
- keep code compileable
- when editing files, always show the full content of new files and clear diffs for changed files
- do not skip build scripts or config files
- after each phase, provide:
  1. what was implemented
  2. file tree changes
  3. full code for new/changed files
  4. build/run instructions
  5. known limitations
  6. next recommended step

Success criteria:
- ISO boots in VMware under UEFI
- graphical desktop appears
- terminal works
- filesystem persists files
- file explorer can browse files
- updater can fetch a new release from GitHub and install it
- updater can update itself safely
- project remains understandable for a hobby OS developer