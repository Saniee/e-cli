# E-Cli

A fast, multi-threaded command line tool for downloading posts from e926.net/e621.net.

It aims to be:
* Fast
* Verbose

## What it can do

- [x] Downloading favourites of a user.
- [x] Downloading posts with specific tags.
- [x] Multi-threaded downloads (favourites, tags, and pools).
- [x] Bulk downloads.
- [x] Pool downloads, with zero-padded page indexes (`0001-`, `0002-`, ...) so files sort correctly.
- [x] Packaging a downloaded pool into a `.zip`, `.7z`, or `.cbz` archive.
- [x] Optional authenticated login for better-quality fetching.
- [x] Live progress bars for downloads.
- [x] Optional tracking file (`-T`) that records downloaded post IDs, so re-runs only fetch new posts.
- [x] Resumable `.part` downloads with bounded retries and nonzero failure exit status.
- [x] Dry-run summaries with post counts and estimated download sizes.
- [x] Optional JSON metadata manifests (`--manifest`).
- [x] Persistent MD5 duplicate detection.
- [x] Named TOML tag presets (`preset <name>`).
- [x] Persistent failed-download retry (`retry-failed`).
- [x] Interactive terminal UI (`tui`) with background downloads and editable settings.

## Other notable things:
[GUI App](https://github.com/Saniee/e-cli-gui)

Or just run:
`e-cli tui` (In fairness use the actual GUI implementation as the TUI is in it's early stages.)

## Usage

```
e-cli d-tags "scalie" -c 250 -r -p 1        Download 250 random posts tagged 'scalie', 1 page
e-cli d-favs someuser -c 100                Download 100 favorites from 'someuser'
e-cli d-pool 22364                          Download a pool into ./dl/
e-cli d-pool 22364 -d ./pool/               Download a pool into ./pool/
e-cli d-favs someuser -c 100 -T seen.txt    Download favorites, skipping posts tracked in seen.txt
e-cli zip -n Cloudjumping -f cbz            Package ./dl/ into Cloudjumping.cbz
e-cli clear-dl                              Delete the ./dl/ output directory
e-cli config                                 Create or edit the TOML configuration
e-cli d-tags "scalie" -p 1 --dry-run        Show the planned work without writing files
e-cli d-tags "scalie" -p 1 --manifest run.json  Export download metadata
e-cli retry-failed                          Retry the previous failed downloads
e-cli tui                                   Open the interactive terminal UI
```

Run `e-cli --help` or `e-cli <command> --help` for the full list of flags.

The SFW API (`e926.net`) is used by default. Pass `--nsfw` to use the NSFW API (`e621.net`).

Run `e-cli config` to create or edit the configuration file. It stores global flags and
subcommand defaults. The file is located at `%APPDATA%\e-cli\config.toml` on Windows and
`$XDG_CONFIG_HOME/e-cli/config.toml` on Linux, falling back to `~/.config/e-cli/config.toml`.
The command uses the editor named by the `EDITOR` environment variable. Command-line values
override values from the configuration file.

Run `e-cli tui` for an interactive keyboard-driven interface. Use Left/Right to change the
download source, Up/Down to select a setting, Enter to edit, Space to start, `s` to save
configuration, Esc to request cancellation, and `q` to quit.

Downloads first use a temporary `.part` file. Interrupted files are resumed when the
server supports HTTP ranges, and otherwise restarted safely. Failed posts are stored in
`.e-cli-failed.json` inside the destination directory by default. The MD5 index is stored
in `.e-cli-md5.json` in the destination directory by default.

Tag presets use `[presets.<name>]` sections in `config.toml`, for example:

```toml
[presets.art]
tags = "dragon"
count = 25
pages = 1
random = false
```

Packaging a pool into an archive (`zip`) shells out to the `7z` executable, which must be available on your `PATH`.

## Building

```
cargo build --release
```

## Testing

```
cargo test
```
