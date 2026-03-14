# Adding Apps And Components

## Current Model

Teddy-OS still runs built-in apps in kernel space. New apps therefore need to be
integrated deliberately and kept small.

## Integration Steps

1. Add the app source under `apps/<name>/` for project-level documentation.
2. Add the actual implementation module under `kernel/src/` while the system is still in-kernel.
3. Initialize the app from `kernel/src/main.rs` if it needs startup state.
4. Integrate rendering and input routing through `kernel/src/shell.rs`.
5. Use shared filesystem and input APIs rather than adding direct hardware access in the app.

## Design Guidance

- prefer narrow interfaces between the shell and the app
- keep rendering rectangle-based and framebuffer-friendly
- route persistence through TeddyFS helpers in `kernel/src/fs.rs`
- document any new commands, windows, or interactions in the matching `apps/` README

## Future Direction

Once userland separation exists, the `apps/` directories can evolve from
documentation placeholders into separate crates or binaries without changing the
high-level app inventory.
