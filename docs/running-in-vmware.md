# Running Teddy-OS In VMware

## VM Configuration

Recommended baseline:

- firmware: `UEFI`
- guest architecture: `x86_64`
- boot media: `build/dist/teddy-os-debug.iso` or `build/dist/teddy-os-release.iso`
- serial port: enabled and connected to a file for COM1 logs
- secondary virtual disk: attached for TeddyFS persistence testing

## Manual Workflow

1. Build the ISO with `./scripts/build.ps1`.
2. Open your VMware VM configuration.
3. Attach the generated ISO as the virtual CD/DVD.
4. Attach a writable virtual disk for TeddyFS tests.
5. Boot the VM with UEFI firmware enabled.

## Command-Line Workflow

If `vmrun` is installed, the helper can update your VMX to force UEFI boot and
attach the generated ISO before launch:

```powershell
./scripts/run-vmware.ps1 -VmxPath C:\VMs\Teddy-OS\Teddy-OS.vmx
```

This avoids common VMware misconfiguration where the VM falls through to PXE
network boot (`Operating System not found`) because CD/DVD boot media was not
connected.

For headless startup:

```powershell
./scripts/run-vmware.ps1 -VmxPath C:\VMs\Teddy-OS\Teddy-OS.vmx -NoGui
```


## Quick Diagnosis For PXE / "Operating System not found"

If VMware shows repeated PXE text like in your screenshot, the VM is not booting
from the Teddy-OS UEFI ISO. Check these in order:

1. **VM firmware is UEFI (not BIOS/Legacy).**
2. **CD/DVD is connected at power-on** and points to `teddy-os-<profile>.iso`.
3. **Secure Boot is disabled** for this unsigned hobby boot path.
4. **You extracted the GitHub artifact ZIP** and attached the actual `.iso` file, not the ZIP itself.

You can validate the ISO contains UEFI boot metadata:

```powershell
./scripts/inspect-iso.ps1 -IsoPath build/dist/teddy-os-debug.iso
```

Expected: an EFI El Torito entry and `/EFI/BOOT/BOOTX64.EFI` listed.

## What To Verify

- UEFI launches `BOOTX64.EFI`
- the kernel reaches the Teddy-OS desktop shell
- the terminal accepts input
- Teddy Explorer can browse the TeddyFS volume
- filesystem changes persist across reboot when a writable disk is attached

## Debugging

- use the serial log file from VMware for early boot failures
- if the ISO boots but the desktop does not update, inspect framebuffer and input logs
- if persistence fails, verify the TeddyFS disk is attached and writable
