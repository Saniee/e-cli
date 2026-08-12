# Context: Six Features

Specs: `specs/reliable-downloads.md`, `specs/dry-run.md`, `specs/metadata-manifest.md`, `specs/md5-duplicate-detection.md`, `specs/tag-presets.md`, `specs/retry-failed-downloads.md`

## Current state
Global configuration is complete. The six approved feature specs are implemented and the repository passes formatting, tests, and clippy.

## Key files
- `src/cli.rs` — CLI commands, flags, config application, validation.
- `src/main.rs` — command dispatch, logging, exit behavior.
- `src/commands.rs` — API fetching and download orchestration.
- `src/funcs.rs` — HTTP/file download primitives.
- `src/config.rs` — TOML configuration.
- `src/tracker.rs` — persistent post-ID tracking.
- `src/type_defs/api_defs.rs` — API response models.

## Decisions made
- Implement all six features, with no backward-compatibility layer unless required by existing behavior.
- Reliable downloads use `.part` files, HTTP resume when supported, safe restart fallback, bounded retries, and nonzero failure status.
- Retry-failed uses a persistent failure manifest and preserves relevant run settings, without credentials.
- Default state files live in the destination: `.e-cli-md5.json` and `.e-cli-failed.json`.

## Next steps
Review the final diff and manually exercise API-backed download, dry-run, preset, manifest, and retry workflows.
- Dry-run must not modify local state.
