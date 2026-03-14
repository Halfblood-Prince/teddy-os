# Bootloader

This directory will contain the Teddy-OS UEFI bootloader.

Planned responsibilities:

- start as `BOOTX64.EFI`
- initialize early logging
- load the kernel image and boot metadata
- configure a framebuffer mode
- exit UEFI boot services
- transfer control to the kernel with `BootInfo`

