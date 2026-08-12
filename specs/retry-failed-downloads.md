# Spec: Retry Failed Downloads

## Goal
Allow users to retry only failed posts from a previous run while preserving the original run settings.

## Constraints
- Use a persistent machine-readable failure manifest.
- Store no credentials or API keys.
- Preserve destination, API source, quality mode, and relevant download settings.
- Remove or mark entries after successful retry.

## Approach
Write failed post records and run settings to a failure manifest after each operation. Add a `retry-failed` command that loads the manifest, retries its entries through the normal download pipeline, and rewrites the manifest with only remaining failures.

## Files touched
- `src/failure_manifest.rs` — manifest model and persistence.
- `src/commands.rs` — retry orchestration.
- `src/main.rs` — command dispatch and exit status.
- `src/cli.rs` — `retry-failed` command and manifest path.
- `src/failure_manifest_tests.rs` — persistence and update tests.
- `src/cli_tests.rs` — parsing tests.
- `README.md` — retry workflow.

## Acceptance criteria
- [ ] Failed posts are persisted after a run.
- [ ] Retry preserves the original destination and relevant settings.
- [ ] Successful retries are removed from the manifest.
- [ ] Still-failing posts remain available for another retry.
- [ ] Empty or missing manifests produce a clear result.
- [ ] Credentials are never persisted.
- [ ] Retry exits nonzero if failures remain.

## Verification
`cargo test`

Manual run with a forced failure, followed by `e-cli retry-failed`.
