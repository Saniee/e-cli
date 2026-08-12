# Spec: Reliable Downloads

## Goal
Make interrupted downloads safe to retry without leaving corrupted files that appear complete.

## Constraints
- Reuse the existing `reqwest` blocking client and download pipeline.
- Preserve tracking-file behavior.
- Retry transient network errors, HTTP `429`, and `5xx` responses.
- Do not retry permanent HTTP errors indefinitely.

## Approach
Download to a `.part` file, resume with HTTP range requests when supported, and fall back to restarting when not. Atomically rename the file after success. Add bounded exponential backoff and configurable retry count.

## Files touched
- `src/funcs.rs` — retry, resume, temporary-file, and atomic-rename logic.
- `src/cli.rs` — retry-related options.
- `src/main.rs` — failure exit status.
- `src/commands.rs` — aggregate final failures.
- `src/funcs_tests.rs` — download-state tests.
- `README.md` — usage documentation.

## Acceptance criteria
- [ ] Interrupted downloads do not create a completed-looking final file.
- [ ] Supported servers resume `.part` files.
- [ ] Unsupported range requests restart safely.
- [ ] Transient failures retry up to the configured limit.
- [ ] Commands exit nonzero when downloads remain failed.
- [ ] Successful files are tracked only after atomic completion.

## Verification
`cargo test`

Manual test with a simulated interrupted download and a transient HTTP failure.
