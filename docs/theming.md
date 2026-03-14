# Teddy-OS Theming And Customization

## Design Direction

Teddy-OS should feel familiar to desktop users without copying vendor branding
or exact UI artwork.

Current shell styling uses:

- a bottom taskbar
- framed overlapping windows
- a launcher menu
- a warm blue/green Teddy-OS palette

## Rules

- keep names, logos, icons, and wallpaper original
- avoid trademarked or copied visual assets
- prefer clear contrast and readable text for the software-rendered UI
- keep decorative elements simple because the compositor is still minimal

## Where Theme Values Live

Current theme constants are defined in
[kernel/src/shell.rs](/c:/Users/HP/Downloads/teddy-os/kernel/src/shell.rs).

As the project grows, move theme tokens into shared assets or libraries instead
of hard-coding them into the shell.

## Practical Customization Targets

- wallpaper colors
- title bar colors
- taskbar colors
- window accent colors
- future icon and font assets in `assets/`
