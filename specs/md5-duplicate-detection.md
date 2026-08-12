# Spec: MD5 Duplicate Detection

## Goal
Avoid downloading identical media when different posts reference the same content.

## Constraints
- Use the API-provided MD5 value.
- Work across separate runs.
- Do not delete or overwrite existing files automatically.
- Remain compatible with the existing post-ID tracker.

## Approach
Maintain a persistent local MD5 index mapping hashes to downloaded files. Before downloading, check the index and report duplicate skips. Update the index only after successful downloads.

## Files touched
- `src/duplicate.rs` — persistent hash index.
- `src/funcs.rs` — duplicate checks and index updates.
- `src/commands.rs` — duplicate statistics.
- `src/main.rs` — index initialization and reporting.
- `src/cli.rs` — optional index path/configuration.
- `src/duplicate_tests.rs` — index and duplicate tests.
- `README.md` — behavior and storage format.

## Acceptance criteria
- [ ] Matching MD5 values are skipped before media download.
- [ ] Duplicate skips are reported separately from tracker skips.
- [ ] The index survives subsequent runs.
- [ ] Failed downloads do not enter the index.
- [ ] Missing MD5 values do not prevent downloads.
- [ ] Existing tracker behavior remains unchanged.

## Verification
`cargo test`

Manual test using two posts with the same MD5 in a temporary output directory.
