//! Core library behind the `e-cli` binary: downloads posts from e926.net/e621.net
//! by favorites, tag search, or pool, and can package a downloaded pool into an
//! archive.
//!
//! The binary (`src/main.rs`) is a thin CLI wrapper around this crate — everything
//! here is usable directly by another Rust program (e.g. a backend service or GUI)
//! without going through a subprocess. Start with [`commands`] for the high-level
//! operations (`download_favourites`, `download_search`, `download_pool`,
//! `zip_downloads`); [`funcs`] holds the lower-level HTTP/filesystem building
//! blocks those are made of, and [`tracker`] the optional record of
//! already-downloaded posts.

pub mod cli;
pub mod commands;
pub mod config;
pub mod duplicate;
pub mod failure_manifest;
pub mod funcs;
pub mod manifest;
pub mod tracker;
pub mod type_defs;
pub mod update;

pub use tracker::Tracker;

/// The `User-Agent` header sent with every HTTP request, e.g. `e-cli/0.4.3`.
pub static AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Aggregate result of a download operation, returned by
/// [`commands::download_favourites`], [`commands::download_search`], and
/// [`commands::download_pool`].
#[derive(Default)]
pub struct DownloadStatistics {
    /// Number of posts successfully downloaded.
    pub completed: i64,
    /// Number of posts that failed to download (or were missing a file URL).
    pub failed: i64,
    /// Number of posts skipped because they were already downloaded — either
    /// the target file already existed on disk, or the post was recorded in
    /// the tracking file (see [`tracker::Tracker`]).
    pub skipped: i64,
    /// Total number of posts considered (`completed + failed + skipped`).
    pub total: usize,
    /// Total bytes written across all successfully downloaded files.
    pub downloaded_amount: f64,
    pub records: Vec<DownloadRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DownloadRecord {
    pub post_id: u64,
    pub source_url: Option<String>,
    pub md5: Option<String>,
    pub artist: String,
    pub extension: String,
    pub local_filename: Option<String>,
    pub status: String,
    pub bytes: u64,
    pub error: Option<String>,
}

/// Request-scoped settings shared by every download operation: which API variant
/// to hit, how many pages/threads to use, and whether to prefer lower-quality media.
pub struct CliContext {
    /// Whether verbose logging is enabled.
    pub verbose: bool,
    /// Whether to use the NSFW API (`e621.net`) instead of the SFW API
    /// (`e926.net`).
    pub nsfw: bool,
    /// If true, prefer a lower-quality/sample file over the full-resolution original.
    pub lower_quality: bool,
    /// Number of pages to fetch: `-1` means "all pages", `> 0` means that many pages.
    pub pages: i64,
    /// Number of threads to use for parallel downloads (expected to be `1..=10`;
    /// see [`cli::validate_args`] for the CLI-level bound).
    pub num_threads: usize,
    pub retries: u32,
    pub duplicate_index: Option<std::sync::Arc<duplicate::DuplicateIndex>>,
}

impl CliContext {
    /// Returns the API host selected by the SFW/NSFW toggle.
    pub fn api_source(&self) -> &'static str {
        if self.nsfw { "e621.net" } else { "e926.net" }
    }
}

/// Optional API credentials. An empty `username`/`api_key` means unauthenticated
/// requests (see [`funcs::send_request`]).
pub struct Login {
    pub username: String,
    pub api_key: String,
}
