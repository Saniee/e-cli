use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{DownloadRecord, DownloadStatistics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureManifest {
    pub api_source: String,
    pub destination: PathBuf,
    pub lower_quality: bool,
    pub retries: u32,
    pub records: Vec<DownloadRecord>,
}

impl FailureManifest {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read failure manifest {}: {e}", path.display()))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse failure manifest: {e}"))
    }

    pub fn from_statistics(
        api_source: &str,
        destination: &Path,
        lower_quality: bool,
        retries: u32,
        stats: &DownloadStatistics,
    ) -> Option<Self> {
        let records = stats
            .records
            .iter()
            .filter(|r| r.status == "failed")
            .cloned()
            .collect::<Vec<_>>();
        (!records.is_empty()).then(|| Self {
            api_source: api_source.to_owned(),
            destination: destination.to_path_buf(),
            lower_quality,
            retries,
            records,
        })
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize failure manifest: {e}"))?;
        fs::write(path, content)
            .map_err(|e| format!("Failed to write failure manifest {}: {e}", path.display()))
    }
}
