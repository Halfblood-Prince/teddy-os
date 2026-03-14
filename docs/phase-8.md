# Phase 8

## Implemented in This Phase

- reproducible PowerShell build flow with a fixed `build/target/` output root
- explicit staging cleanup for EFI and ISO assembly
- ISO checksum generation beside the final artifact
- build manifest generation for debug and release outputs
- release manifest generation for publishing artifacts
- VMware launch helper built around `vmrun`
- clean script for resetting staged and distribution outputs

## Scripts Added or Updated

- `scripts/build.ps1`
- `scripts/make-iso.ps1`
- `scripts/clean.ps1`
- `scripts/release.ps1`
- `scripts/run-vmware.ps1`

## Build Flow

1. `build.ps1` builds the bootloader and kernel into `build/target/`.
2. EFI files are staged into `build/staging/esp/EFI/BOOT/`.
3. `make-iso.ps1` creates `build/dist/teddy-os-<profile>.iso`.
4. A SHA-256 checksum is written beside the ISO.
5. A small JSON build manifest is written into `build/dist/`.

## Release Flow

1. Build a release ISO.
2. Run `scripts/release.ps1 -Version <version>`.
3. Publish the ISO, its `.sha256` file, and the generated release manifest.

The updater itself is still Phase 7 work. This phase only defines the release
artifacts and scripts that Phase 7 can consume later.

## VMware Flow

`scripts/run-vmware.ps1` starts an existing VMware `.vmx` file via `vmrun`.
The helper assumes the VM is already configured to use the generated ISO and
any TeddyFS data disk you want attached.

## Known Limitations

- `run-vmware.ps1` does not rewrite `.vmx` files or attach media automatically
- reproducibility is bounded by the installed Rust toolchain and host tools
- release packaging is JSON metadata plus ISO/checksum today, not a full updater bundle
