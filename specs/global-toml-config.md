# Spec: Global TOML Configuration

## Goal
Add a cross-platform TOML configuration file that stores global CLI settings and per-subcommand defaults. `e-cli config` creates the file if needed and opens it using `$EDITOR`.

## Constraints
- Windows and Linux are required.
- Config location:
  - Windows: `%APPDATA%\\e-cli\\config.toml`
  - Linux: `$XDG_CONFIG_HOME/e-cli/config.toml`, falling back to `~/.config/e-cli/config.toml`
- Explicit CLI values override config values.
- Credentials/API keys are not stored.
- The `config` command uses `$EDITOR`; if it is unset or fails, report the config path and tell the user to open it manually, with a nonzero exit status.
- Use TOML and a Rust TOML serialization dependency.

## Approach
- Add a config module with serializable global and command-specific structs.
- Load config before effective argument validation.
- Represent CLI fields as optional where config can provide defaults.
- Merge values with precedence: CLI > TOML > built-in defaults.
- Validate the merged configuration using the existing argument validation rules.
- Add `config` as a subcommand that creates parent directories and a commented/example TOML file, then launches `$EDITOR`.

## Config Sections
- `[global]`: `verbose`, `nsfw`, `login`, `lower_quality`, `pages`, `num_threads`, `dir`, `track_file`
- `[d-favs]`: `username`, `count`, `random`, `tags`
- `[d-tags]`: `tags`, `count`, `random`
- `[d-pool]`: `pool_id`
- `[zip]`: `name`, `format`

## Likely Files
- `src/config.rs` — TOML schema, platform path resolution, loading, merging, editor launch
- `src/cli.rs` — `config` subcommand and optional/configurable command arguments
- `src/main.rs` — load config, merge CLI values, dispatch config editor
- `src/lib.rs` — expose shared config/context types if needed
- `Cargo.toml` / `Cargo.lock` — TOML and platform path dependencies
- `src/*_tests.rs` — merge, path, parsing, and precedence coverage
- `README.md` — configuration usage and file locations

## Acceptance Criteria
- [ ] `e-cli config` creates the platform config directory and TOML file when absent.
- [ ] `e-cli config` launches the command specified by `EDITOR`.
- [ ] TOML values work for global and each subcommand’s settings.
- [ ] Explicit CLI values override TOML values.
- [ ] Missing required effective values are rejected clearly.
- [ ] Invalid TOML/config values produce a useful error without a panic.
- [ ] Windows and Linux path resolution are covered without hardcoded host-specific paths.
- [ ] Existing behavior and tests remain passing.

## Verification
- `cargo test`
- `cargo fmt -- --check`
- Manual editor launch with `EDITOR` set to a harmless test executable.
- Cross-target compile checks for Windows and Linux via the existing GitHub Actions builds.
