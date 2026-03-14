# Teddy-OS Updater Manifests

## Status

Phase 7 is deferred, but the release pipeline now defines the manifest format
that the future updater should consume.

## Current Release Manifest Shape

Example:

```json
{
  "version": "0.1.0",
  "channel": "stable",
  "profile": "release",
  "artifacts": [
    {
      "name": "teddy-os-release.iso",
      "path": "build/dist/teddy-os-release.iso",
      "sha256": "replace-with-real-sha256"
    }
  ]
}
```

An example file is stored at
[build/release-manifest.example.json](/c:/Users/HP/Downloads/teddy-os/build/release-manifest.example.json).

## Planned Extension Fields

When Phase 7 is implemented, the manifest should grow to include:

- `min_bootloader_version`
- `published_at`
- `notes_url`
- `size`
- `install_strategy`
- `rollback_supported`

## Validation Expectations

The updater should eventually:

- download the manifest from GitHub
- compare installed and available versions
- verify the SHA-256 of downloaded artifacts
- stage updates safely before switching boot targets
