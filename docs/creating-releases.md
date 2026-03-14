# Creating Teddy-OS Releases

## Current Release Artifacts

Phase 8 defines a simple release set:

- bootable ISO
- SHA-256 checksum file
- JSON release manifest

The updater client is still deferred to Phase 7, so this document describes the
artifact contract rather than a full end-user update workflow.

## Example Workflow

```powershell
./scripts/clean.ps1
./scripts/build.ps1 -Profile release
./scripts/release.ps1 -Version 0.1.0
```

Expected files:

- `build/dist/teddy-os-release.iso`
- `build/dist/teddy-os-release.iso.sha256`
- `build/dist/build-release.json`
- `build/dist/release-0.1.0.json`

## Suggested GitHub Release Layout

Upload:

- the release ISO
- the `.sha256` checksum file
- the generated release manifest JSON

Keep version numbers aligned between the GitHub release tag and the manifest.

## Publishing Notes

- build the release on a stable, repeatable host setup
- keep the generated checksum file with the ISO
- do not hand-edit the checksum after generation
- if you later add updater payload bundles, keep them as additional artifacts rather than replacing the ISO
