use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
struct IndexFile {
    entries: HashMap<String, String>,
}

pub struct DuplicateIndex {
    path: PathBuf,
    entries: Mutex<HashMap<String, String>>,
}

impl DuplicateIndex {
    pub fn load(path: &Path) -> io::Result<Self> {
        let entries = if path.exists() {
            serde_json::from_str::<IndexFile>(&fs::read_to_string(path)?)
                .map_err(io::Error::other)?
                .entries
        } else {
            HashMap::new()
        };
        Ok(Self {
            path: path.to_path_buf(),
            entries: Mutex::new(entries),
        })
    }

    pub fn contains(&self, md5: &str) -> Option<String> {
        self.entries.lock().unwrap().get(md5).cloned()
    }

    pub fn insert(&self, md5: &str, filename: &str) {
        let mut entries = self.entries.lock().unwrap();
        if entries
            .insert(md5.to_owned(), filename.to_owned())
            .is_none()
            && let Err(e) = self.save_locked(&entries)
        {
            tracing::warn!(
                "Failed to save duplicate index {}: {e}",
                self.path.display()
            );
        }
    }

    fn save_locked(&self, entries: &HashMap<String, String>) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&IndexFile {
            entries: entries.clone(),
        })
        .map_err(io::Error::other)?;
        fs::write(&self.path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_hash_to_filename() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("index.json");
        let index = DuplicateIndex::load(&path).expect("load");
        index.insert("abc", "artist-1.jpg");
        assert_eq!(index.contains("abc").as_deref(), Some("artist-1.jpg"));
        let reloaded = DuplicateIndex::load(&path).expect("reload");
        assert_eq!(reloaded.contains("abc").as_deref(), Some("artist-1.jpg"));
    }
}
