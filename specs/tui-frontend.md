# Spec: TUI Frontend

## Goal
Add a full-screen terminal UI launched with `e-cli tui` for configuring, starting, monitoring, cancelling, and retrying downloads without replacing existing CLI commands.

## Constraints
- Support Windows and Unix terminals through `crossterm`.
- Use `ratatui` for rendering.
- Reuse the existing download, retry, tracker, duplicate-index, manifest, failure-manifest, and preset systems.
- Run network/filesystem work in background workers; never block the TUI event loop.
- All actions must work by keyboard; mouse input is optional.
- Preserve normal terminal state on exit, including Ctrl+C and download errors.

## Approach
Add a TUI application with event-driven state and background worker channels. Provide screens for source selection, tag/favorites/pool input, download settings, progress/queue status, failures and retry, presets, history, and configuration editing. Extend download orchestration with progress events so the TUI can display per-post status, bytes, retries, skips, duplicates, and failures in real time.

Configuration and presets are loaded from the existing TOML file and saved through the existing config module.

## Files touched
- `Cargo.toml` — add `ratatui` and `crossterm`.
- `src/cli.rs` — add the `tui` subcommand.
- `src/main.rs` — launch the TUI.
- `src/tui.rs` or `src/tui/` — terminal lifecycle, screens, input, state, rendering, and worker events.
- `src/commands.rs` — expose progress events and cancellation-aware orchestration.
- `src/config.rs` — support editing and saving TUI configuration/presets.
- `src/tui_tests.rs` — state transitions, input handling, and rendering-state tests.
- `README.md` — TUI usage and key bindings.

## Acceptance criteria
- [ ] `e-cli tui` enters and exits the TUI cleanly on Windows and Unix terminals.
- [ ] Users can configure and start tag, favorites, and pool downloads.
- [ ] Users can select and edit saved presets.
- [ ] Edited presets/configuration can be saved to `config.toml`.
- [ ] Downloads run off the UI event loop.
- [ ] The UI shows progress, current files, bytes, retries, skips, duplicates, and failures.
- [ ] Users can cancel an active operation without corrupting completed files or state.
- [ ] Users can retry failed downloads from the UI.
- [ ] Existing CLI commands continue to work unchanged.
- [ ] Terminal cleanup occurs on normal exit, errors, and Ctrl+C.
- [ ] Unit tests cover key state transitions and worker-event handling.

## Verification
```text
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Manual verification:

```text
cargo run -- tui
```

Test a tag download, cancellation, preset edit/save, failure retry, and clean exit.
