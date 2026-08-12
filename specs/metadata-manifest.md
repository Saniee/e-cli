# Spec: Metadata Manifest

## Goal
Preserve machine-readable metadata linking downloaded files to their source posts.

## Constraints
- Use the existing serde models.
- Write one manifest per download operation.
- Do not expose API credentials.
- Preserve pool ordering and generated filenames.

## Approach
Add an optional manifest output path or enable manifest generation through configuration. Record post ID, source URL, tags, artist, MD5, dimensions, file size, local filename, download status, and timestamp in JSON.

## Files touched
- `src/cli.rs` — manifest option.
- `src/commands.rs` — collect operation metadata and statuses.
- `src/type_defs/api_defs.rs` — expose any required serialized fields.
- `src/main.rs` — write the final manifest.
- `src/cli_tests.rs` — option parsing tests.
- `README.md` — manifest format and usage.

## Acceptance criteria
- [ ] A successful run can produce a valid JSON manifest.
- [ ] Each attempted post has a status and local filename when applicable.
- [ ] Pool entries retain their original order.
- [ ] Failed and skipped posts are represented.
- [ ] Credentials are never written.
- [ ] Manifest-writing failures are reported clearly.

## Verification
`cargo test`

Manual run followed by JSON parsing with `cargo run -- ...` and a JSON tool.
