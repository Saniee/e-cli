# E-Cli

A fast, multi-threaded command line tool for downloading posts from e926.net/e621.net (or a compatible booru-style API).

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

## What it can't do

* Have a fully fledged UI.

## Usage

```
e-cli d-tags "scalie" -c 250 -r -p 1        Download 250 random posts tagged 'scalie', 1 page
e-cli d-favs someuser -c 100                Download 100 favorites from 'someuser'
e-cli d-pool 22364                          Download a pool into ./dl/
e-cli zip -n Cloudjumping -f cbz            Package ./dl/ into Cloudjumping.cbz
e-cli clear-dl                              Delete the ./dl/ output directory
```

Run `e-cli --help` or `e-cli <command> --help` for the full list of flags.

Packaging a pool into an archive (`zip`) shells out to the `7z` executable, which must be available on your `PATH`.

## Building

```
cargo build --release
```

## Testing

```
cargo test
```

Runs the unit test suite (no network access required). The `.ps1` scripts in the repo root are manual smoke tests that exercise the CLI against the live e621/e926 API.
