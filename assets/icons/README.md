Teddy-OS Desktop Icons

Drop custom desktop app icons here before building.

Current supported files:

- `terminal.bmp`
- `explorer.bmp`
- `settings.bmp`

Format requirements:

- uncompressed BMP
- 24-bit or 32-bit
- recommended size: `24x24` or `32x32`

Transparency rules:

- 32-bit BMP alpha `0` becomes transparent
- pure magenta `#FF00FF` also becomes transparent

Notes:

- icons are converted at kernel build time by `kernel/build.rs`
- colors are mapped to Teddy-OS's current VGA-style palette automatically
- if an icon file is missing, Teddy-OS falls back to its built-in procedural icon

First custom icon:

- save your terminal icon as `assets/icons/terminal.bmp`
- rebuild Teddy-OS
- boot `kernelgfx`
