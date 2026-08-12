use std::fs;
use std::path::Path;

use crate::DownloadStatistics;

pub fn write(path: &Path, statistics: &DownloadStatistics) -> Result<(), String> {
    let content = serde_json::to_string_pretty(&statistics.records)
        .map_err(|e| format!("Failed to serialize manifest: {e}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create manifest directory: {e}"))?;
    }
    fs::write(path, content)
        .map_err(|e| format!("Failed to write manifest {}: {e}", path.display()))
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
