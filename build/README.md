# Build

This directory defines the Teddy-OS build output and staging layout used by the
PowerShell automation in `scripts/`.

## Layout

- `build/target/` - Cargo target directory for both the bootloader and kernel
- `build/staging/esp/` - EFI system partition tree before image creation
- `build/staging/iso/` - ISO assembly directory
- `build/staging/efiboot.img` - FAT EFI boot image used by `xorriso`
- `build/dist/` - final ISO files, checksums, and release/build manifests

## Reproducibility Notes

- scripts always stage from a clean `build/staging/esp/` directory
- the Cargo target directory is pinned to `build/target/`
- ISO output names are profile-based and stable
- checksums and release manifests are written beside the generated ISO

The full workflow is documented in
[docs/building-from-scratch.md](/c:/Users/HP/Downloads/teddy-os/docs/building-from-scratch.md).
