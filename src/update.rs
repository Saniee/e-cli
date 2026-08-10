//! Update checking against the GitHub Releases API.

use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::Value;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Checks the given GitHub repo's latest release against the current version.
///
/// Returns `Ok(Some(latest))` when `latest` is strictly newer than `current`,
/// `Ok(None)` when you're up to date (or the repo has no releases yet), and
/// `Err(..)` on network/API failures. Versions are compared numerically on
/// their dotted components, so `0.10.0 > 0.9.0`, and a leading `v` is ignored.
pub fn check_update(repo: &str, current: &str) -> Result<Option<String>, String> {
    let Some(latest) = latest_release(repo)? else {
        return Ok(None);
    };
    Ok(is_newer(&latest, current).then_some(latest))
}

/// Fetches the tag name of the latest release for `repo` (e.g. `Saniee/e-cli`).
pub fn latest_release(repo: &str) -> Result<Option<String>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Could not build HTTP client: {e}"))?;
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let json: Value = response
        .error_for_status()
        .map_err(|e| format!("GitHub returned an error: {e}"))?
        .json()
        .map_err(|e| format!("Could not parse GitHub response: {e}"))?;
    Ok(json["tag_name"].as_str().map(str::to_owned))
}

/// Returns true if `latest` is a strictly newer version than `current`.
///
/// Both are compared as dot-separated numeric components, ignoring a leading
/// `v` and any pre-release/build suffix.
pub fn is_newer(latest: &str, current: &str) -> bool {
    version_numbers(latest) > version_numbers(current)
}

fn version_numbers(version: &str) -> Vec<u64> {
    version
        .trim_start_matches('v')
        .split(['.', '-', '+'])
        .filter_map(|part| part.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn ignores_leading_v_and_equal_versions() {
        assert!(!is_newer("v0.5.0", "0.5.0"));
        assert!(!is_newer("0.5.0", "0.5.0"));
        assert!(!is_newer("0.5.0", "v0.5.0"));
    }

    #[test]
    fn compares_numerically_not_lexically() {
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.9.0", "0.10.0"));
    }

    #[test]
    fn detects_newer_and_older() {
        assert!(is_newer("0.6.0", "0.5.0"));
        assert!(!is_newer("0.5.0", "0.6.0"));
    }

    #[test]
    fn handles_shorter_and_prerelease_versions() {
        assert!(is_newer("0.5.0", "0.5"));
        assert!(!is_newer("0.5.0-beta", "0.5.0"));
        assert!(!is_newer("0.5", "0.5.0"));
    }
}
