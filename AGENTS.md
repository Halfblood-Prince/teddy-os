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

Phase 10 — Networking
Implement basic networking support so Teddy-OS can communicate with the internet and fetch updates.
Goals:
- Detect a supported virtual network interface in VMware
- Initialize the network device
- Implement a minimal network stack
- Support DHCP for automatic IP configuration
- Implement DNS resolution
- Provide basic TCP and UDP socket functionality
- Enable HTTP/HTTPS communication needed for the updater

Requirements
- Network interface must work inside VMware
- Networking layer must be modular
- Network code must be well documented
- Debug logging should exist for networking events

Deliverables
 - Network device driver
 - Network stack implementation
 - DHCP client
 - DNS resolver
 - TCP/UDP socket interface
 - Example diagnostic commands

Success criteria
- Teddy-OS obtains an IP address in VMware
- DNS resolution works
- OS can download a file from the internet
- Updater can fetch update manifests from GitHub


Phase 11 — Input and Windowing Improvements
Improve the desktop shell so the system feels like a usable graphical desktop.

Goals
- Mouse input support
- Keyboard shortcut handling
- Window focus system
- Window minimize / maximize / close
- Window dragging
- Window resizing
- Window stacking order (z-order)
- Desktop event system

Requirements
- Window manager must be stable
- Multiple windows must function correctly
- Input handling must be abstracted from hardware

Deliverables
- Improved window manager
- Input event dispatch system
- Mouse cursor support
- Window control buttons
- Focus management

Success criteria
- Multiple windows can be opened and interacted with
- Windows can be moved, resized, minimized, and closed
- Input events are correctly routed to the focused window



Phase 12 — Application Framework (SDK)

Create a framework that simplifies building applications for Teddy-OS.

Goals

Define a consistent application lifecycle

Create shared UI widgets

Provide reusable application libraries

Reduce boilerplate code for apps

Requirements

Applications should not interact directly with low-level kernel APIs

GUI apps should use shared libraries

APIs should remain stable

Deliverables

Libraries such as:

/libs/ui
/libs/appkit
/libs/input
/libs/fs

Widgets:

buttons

menus

dialog boxes

lists

text boxes

scroll views

Developer tools:

app template

developer documentation

Success criteria

Developers can create a GUI app with minimal code

Existing apps (terminal, file explorer) use shared UI libraries



Phase 13 — Package Manager
Add a package management system so applications can be installed independently from OS updates.

Goals
- Define a Teddy-OS package format
- Install applications from package files
- Remove installed applications
- List installed packages
- Support application versioning

Requirements
- Package metadata format must be documented
- Package verification should exist
- Package installation must not break system integrity

Deliverables
Components:
- Package manager CLI
- Package installer backend
- Package metadata schema
- Repository format

Example commands:
- pkg install
- pkg remove
- pkg update
- pkg list

Success criteria
- Applications can be installed without reinstalling the OS
- Packages can be updated independently



Phase 14 — Storage and Disk Management
Improve disk and storage handling.

Goals
- Detect storage devices
- Manage partitions
- Mount filesystems
- Provide disk information utilities
- Improve data persistence reliability

Requirements
- Disk operations must be safe
- Filesystem corruption risk must be minimized
- Storage architecture must be documented

Deliverables

- Modules:

-- /kernel/storage
-- /apps/disk_utility

- Tools:

-- disk usage viewer

-- filesystem check tool

-- partition viewer

Success criteria

- Files persist across reboots

- Disk information can be inspected from inside the OS

- Filesystem integrity checks work


Phase 15 — Fonts and Text Rendering

Improve text quality across the system.

Goals

Implement a font rendering system

Support TrueType or bitmap fonts

Provide consistent text layout

Improve terminal and GUI readability

Requirements

Text rendering must work in all apps

Font loading should be efficient

Unicode support is desirable if feasible

Deliverables

Libraries:

/libs/font
/libs/text

Improvements to:

terminal rendering

UI text elements

file explorer text display

Success criteria

All applications use the same text system

Text appears clear and consistent across the desktop

Phase 16 — Update Security and Integrity

Harden the updater system to prevent corruption or malicious updates.

Goals

Verify update integrity

Validate update manifests

Implement update rollback capability

Improve update logging

Requirements

Update packages must be validated before installation

Update failures must not break the system

Recovery must be possible

Deliverables

Security mechanisms:

checksum verification

signed manifests (optional but recommended)

update rollback support

Documentation:

update format

manifest schema

publishing workflow

Success criteria

Corrupted updates are rejected

Failed updates can be rolled back safely

Updater logs provide clear diagnostics

Phase 17 — Recovery Environment

Create a recovery mode for repairing the system.

Goals

Provide a recovery boot mode

Allow filesystem repairs

Allow rollback of failed updates

Provide terminal access for debugging

Requirements

Recovery environment must be minimal and reliable

Must not depend on the main OS installation

Deliverables

Components:

/recovery
/apps/recovery_tools

Features:

system repair utilities

update rollback tools

system logs viewer

Success criteria

Users can recover from failed updates

Filesystem repairs can be performed

Recovery environment boots independently

Phase 18 — System Settings Application

Create a graphical settings application.

Goals

Allow users to configure system settings through a GUI.

Settings categories:

appearance

wallpaper

system information

updates

input devices

time/date

Requirements

Settings must persist across reboots

Configuration storage must be structured

Deliverables

Application:

/apps/settings

Configuration storage:

/system/config
Success criteria

Settings are applied correctly

Configuration changes persist after reboot

Phase 19 — Software Center

Create a graphical application store.

Goals

Allow users to browse and install software.

Features:

browse available applications

search packages

install or uninstall applications

display version information

show package descriptions

Requirements

Integrate with the package manager

Support remote repositories

Provide clear UI feedback

Deliverables

Application:

/apps/software_center

Repository metadata format.

Success criteria

Users can install apps through the GUI

Software center reflects repository updates

Phase 20 — Audio Support

Add sound output support.

Goals

Detect supported audio devices

Implement audio output

Enable simple sound playback

Requirements

Audio system must work in VMware

API must be simple for applications

Deliverables

Modules:

/kernel/audio
/libs/audio

Test applications:

/apps/audio_test
Success criteria

Teddy-OS can play basic sounds

Applications can access audio APIs





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