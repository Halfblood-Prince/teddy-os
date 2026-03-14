# Scripts

This directory contains the developer entry points for the Teddy-OS workflow.

## Available Scripts

- `build.ps1` - builds the bootloader and kernel, stages EFI files, optionally creates an ISO, and writes a build manifest
- `make-iso.ps1` - assembles the staged EFI tree into a UEFI ISO and writes a SHA-256 checksum
- `clean.ps1` - removes staged and distribution outputs
- `release.ps1` - writes a release metadata JSON file for a built ISO
- `run-vmware.ps1` - starts an existing VMware VM through `vmrun`

## Typical Flow

```powershell
./scripts/build.ps1
./scripts/release.ps1 -Version 0.1.0
./scripts/run-vmware.ps1 -VmxPath C:\VMs\Teddy-OS\Teddy-OS.vmx
```

See
[docs/building-from-scratch.md](/c:/Users/HP/Downloads/teddy-os/docs/building-from-scratch.md)
and
[docs/running-in-vmware.md](/c:/Users/HP/Downloads/teddy-os/docs/running-in-vmware.md)
for the full workflow.
