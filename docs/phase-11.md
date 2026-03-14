# Phase 11

## Implemented in This Phase

- improved desktop window manager state inside `kernel/src/shell.rs`
- explicit desktop event dispatch path for keyboard and mouse handling
- focused-window input routing
- taskbar window toggling and launcher-based window restore/open
- window minimize, maximize, and close controls
- bottom-right resize grip support
- focus cycling via keyboard `Tab`
- launcher dismissal via keyboard `Escape`

## Windowing Behavior

The shell now tracks per-window visibility, minimized state, maximized state,
restore geometry, and active focus. The z-order model is still simple but now
supports real focus transitions and taskbar restoration behavior.

Mouse interactions:

- click title bar to focus and drag
- click the lower-right grip to resize
- click title-bar controls to minimize, maximize/restore, or close
- click taskbar buttons to restore or minimize windows

Keyboard interactions:

- `Tab` cycles focus across visible windows
- `Escape` closes the launcher popup

## VMware Test Instructions

1. Build and boot Teddy-OS in VMware.
2. Open the desktop shell.
3. Click different windows to verify focus changes.
4. Drag the terminal and explorer windows.
5. Resize a window using the lower-right grip.
6. Minimize a window from the title bar, then restore it from the taskbar.
7. Maximize and restore a window.
8. Close a window, then reopen it from the launcher.
9. Press `Tab` to cycle focus and `Escape` to dismiss the launcher.

## Known Limitations

- keyboard shortcuts are intentionally minimal for now
- explorer still receives mouse-only app input
- closed windows are restored through shell state rather than a separate app lifecycle
- compile and VMware verification were not possible in this shell because the Rust toolchain is not available on `PATH`
