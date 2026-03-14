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

If `vmrun` is installed and your VM already points at the correct ISO:

```powershell
./scripts/run-vmware.ps1 -VmxPath C:\VMs\Teddy-OS\Teddy-OS.vmx
```

For headless startup:

```powershell
./scripts/run-vmware.ps1 -VmxPath C:\VMs\Teddy-OS\Teddy-OS.vmx -NoGui
```

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
