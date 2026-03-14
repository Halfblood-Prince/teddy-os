# Updater App

This directory will hold the Teddy-OS software updater.

Phase 7 is still deferred. Phase 8 and Phase 9 document the planned release and
manifest workflow so the updater has a concrete target format when it is
implemented.

Planned scope:

- fetch release manifests from GitHub
- compare installed and available versions
- download and verify update packages
- stage updates safely
- support rollback-aware slot switching

See
[docs/updater-manifests.md](/c:/Users/HP/Downloads/teddy-os/docs/updater-manifests.md)
and
[docs/creating-releases.md](/c:/Users/HP/Downloads/teddy-os/docs/creating-releases.md)
for the Phase 8/9 release format that the updater is expected to consume later.
