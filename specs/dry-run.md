# Spec: Dry Run

## Goal
Allow users to inspect the expected work without downloading or modifying files.

## Constraints
- Use the same API queries, config, tracker, and existing-file filtering as real downloads.
- Do not create download directories, tracking entries, manifests, or media files.
- Keep output useful for scripts and humans.

## Approach
Add a global `--dry-run` flag. Resolve the normal post set, calculate posts to download and skip, estimate total bytes from API metadata, and print a summary with miscellaneous settings such as destination, thread count, and API source.

## Files touched
- `src/cli.rs` — `--dry-run`.
- `src/main.rs` — dry-run dispatch.
- `src/commands.rs` — planning/statistics logic.
- `src/funcs.rs` — shared filtering and size calculation if needed.
- `src/cli_tests.rs` — argument parsing tests.
- `README.md` — usage documentation.

## Acceptance criteria
- [ ] Dry runs perform API lookup but download no files.
- [ ] Output includes total posts, planned downloads, skipped posts, and estimated bytes.
- [ ] Output includes destination and relevant download settings.
- [ ] Existing files and tracking entries are excluded from planned downloads.
- [ ] No local state is modified.

## Verification
`cargo test`

Manual run with `e-cli d-tags ... --dry-run`.
