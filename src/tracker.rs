use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracing::{debug, warn};

/// Records which post IDs have already been downloaded, so repeated runs can
/// skip them without relying on per-file existence checks (which can miss
/// posts whose filename layout differs between commands, e.g. pool downloads'
/// index prefixes).
///
/// Backed by a plain-text file with one post ID per line (unparseable lines
/// are ignored on load). Every [`Tracker::insert`] is appended to the file
/// immediately, so history survives an interrupted run. All methods are safe
/// to call from multiple threads (as [`crate::commands`] does via rayon).
pub struct Tracker {
    path: PathBuf,
    inner: Mutex<TrackerInner>,
}

struct TrackerInner {
    seen: HashSet<u64>,
    file: File,
}

impl Tracker {
    /// Opens (or creates) the tracking file at `path`, loading any post IDs
    /// it already contains. Fails if the file can't be read or created.
    pub fn load(path: &Path) -> io::Result<Self> {
        let mut seen = HashSet::new();
        if path.exists() {
            let content = fs::read_to_string(path)?;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match line.parse::<u64>() {
                    Ok(id) => {
                        seen.insert(id);
                    }
                    Err(_) => debug!("Ignoring unparseable line '{line}' in tracking file."),
                }
            }
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        debug!(
            "Loaded {} tracked post IDs from {}.",
            seen.len(),
            path.display()
        );
        Ok(Self {
            path: path.to_path_buf(),
            inner: Mutex::new(TrackerInner { seen, file }),
        })
    }

    /// The path of the backing tracking file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Number of post IDs currently tracked.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().seen.len()
    }

    /// Whether no post IDs are tracked yet.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().seen.is_empty()
    }

    /// Whether `post_id` has already been recorded as downloaded.
    pub fn contains(&self, post_id: u64) -> bool {
        self.inner.lock().unwrap().seen.contains(&post_id)
    }

    /// Records `post_id` as downloaded, appending it to the tracking file.
    /// A no-op if it's already tracked. Write failures are logged, not
    /// returned — a tracking hiccup shouldn't fail the download itself.
    pub fn insert(&self, post_id: u64) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.seen.insert(post_id) {
            return;
        }
        if let Err(e) = writeln!(inner.file, "{post_id}").and_then(|_| inner.file.flush()) {
            warn!(
                "Failed to write post {post_id} to tracking file {}: {e}",
                self.path.display()
            );
        }
    }
}

#[cfg(test)]
#[path = "tracker_tests.rs"]
mod tests;
