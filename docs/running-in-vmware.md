# Running Teddy-OS In VMware

## VM Settings

- Guest type: Other
- Firmware: BIOS / Legacy
- Attach ISO: `build/dist/teddy-os-debug.iso`
- Video memory: default is fine

## Expected Result

You should see:

- a black text-mode screen
- a `TEDDY-OS` title
- a `Legacy BIOS boot path online` status line
- a `Boot OK - BIOS mode` footer

If that screen appears and remains stable, the reset baseline is working.
