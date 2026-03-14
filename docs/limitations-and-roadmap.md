# Limitations And Roadmap

## Current Limitations

- the kernel, shell, terminal, and explorer still share one address space
- the UI is fully software-rendered
- the terminal and explorer are MVP implementations
- TeddyFS is intentionally simple and fixed-layout
- the updater is not implemented yet because Phase 7 was skipped for now
- this phase set was not compiled or VMware-tested in the current shell because `cargo` and `rustup` were unavailable on `PATH`

## Near-Term Roadmap

- Phase 7: implement the updater using the documented release manifest format
- strengthen release packaging beyond ISO plus checksum
- expand shell polish, app management, and text/file editing flows

## Longer-Term Roadmap

- process isolation and a stable syscall boundary
- richer storage and crash-recovery semantics
- a real widget toolkit and app runtime
- improved hardware support beyond the current VMware-first target
