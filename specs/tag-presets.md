# Spec: Tag Presets

## Goal
Let users save and reuse named tag-search configurations without repeating long command lines.

## Constraints
- Store presets in the existing TOML configuration file.
- Preserve command-line precedence over configuration values.
- Keep presets focused on tag searches rather than arbitrary shell commands.

## Approach
Add a `[presets.<name>]` configuration section containing tags, count, pages, random mode, quality mode, API choice, output directory, and tracking settings. Add a `preset` command that accepts a preset name and optional command-line overrides.

## Files touched
- `src/config.rs` — preset model and TOML serialization.
- `src/cli.rs` — preset command and override handling.
- `src/main.rs` — preset resolution.
- `src/config_tests.rs` — loading and precedence tests.
- `src/cli_tests.rs` — command parsing tests.
- `README.md` — preset examples and syntax.

## Acceptance criteria
- [ ] A named preset can be loaded from `config.toml`.
- [ ] Presets execute tag searches using their saved settings.
- [ ] CLI flags override preset values.
- [ ] Missing presets produce a clear nonzero failure.
- [ ] Invalid preset values are validated before API requests.
- [ ] Existing non-preset commands behave unchanged.

## Verification
`cargo test`

Manual creation of a preset followed by `e-cli preset <name>`.
