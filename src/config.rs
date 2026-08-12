use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub global: GlobalConfig,
    #[serde(rename = "d-favs")]
    pub d_favs: FavouritesConfig,
    #[serde(rename = "d-tags")]
    pub d_tags: TagsConfig,
    #[serde(rename = "d-pool")]
    pub d_pool: PoolConfig,
    pub zip: ZipConfig,
    pub presets: HashMap<String, PresetConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    pub verbose: Option<bool>,
    pub nsfw: Option<bool>,
    pub login: Option<bool>,
    pub lower_quality: Option<bool>,
    pub pages: Option<i64>,
    pub num_threads: Option<usize>,
    pub dir: Option<String>,
    pub track_file: Option<PathBuf>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FavouritesConfig {
    pub username: Option<String>,
    pub count: Option<u32>,
    pub random: Option<bool>,
    pub tags: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TagsConfig {
    pub tags: Option<String>,
    pub count: Option<u32>,
    pub random: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PoolConfig {
    pub pool_id: Option<u64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ZipConfig {
    pub name: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PresetConfig {
    pub tags: Option<String>,
    pub count: Option<u32>,
    pub pages: Option<i64>,
    pub random: Option<bool>,
    pub lower_quality: Option<bool>,
    pub nsfw: Option<bool>,
    pub dir: Option<String>,
    pub track_file: Option<PathBuf>,
}

pub fn path() -> Result<PathBuf, String> {
    let base = if cfg!(windows) {
        env::var_os("APPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| "APPDATA is not set; cannot locate the config directory.".to_owned())?
    } else if let Some(xdg) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        PathBuf::from(xdg)
    } else {
        let home = env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set; cannot locate the config directory.".to_owned())?;
        home.join(".config")
    };

    Ok(base.join("e-cli").join(CONFIG_FILE))
}

pub fn load(path: &std::path::Path) -> Result<Config, String> {
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config {}: {e}", path.display()))?;
    toml::from_str(&content).map_err(|e| format!("Failed to parse config {}: {e}", path.display()))
}

/// Writes `config` back to the configuration file in the documented comment
/// style of the template: each managed key is written un-commented only when it
/// carries a value, while everything else stays commented with its example
/// default. The file is regenerated from the template, so any extra keys or
/// comments added by hand are not preserved.
pub fn save(config: &Config) -> Result<(), String> {
    let path = path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create config directory: {e}"))?;
    }
    fs::write(&path, render(config))
        .map_err(|e| format!("Could not write config {}: {e}", path.display()))
}

fn render(config: &Config) -> String {
    let mut out =
        String::from("# e-cli configuration\n# Command-line values override these settings.\n\n");

    out.push_str("[global]\n");
    bool_key(&mut out, "verbose", config.global.verbose, "false");
    bool_key(&mut out, "nsfw", config.global.nsfw, "false");
    bool_key(&mut out, "login", config.global.login, "false");
    bool_key(
        &mut out,
        "lower_quality",
        config.global.lower_quality,
        "false",
    );
    int_key(&mut out, "pages", config.global.pages, "-1");
    int_key(
        &mut out,
        "num_threads",
        config.global.num_threads.map(|v| v as i64),
        "5",
    );
    str_key(&mut out, "dir", config.global.dir.clone(), "\"./dl/\"");
    str_key(
        &mut out,
        "track_file",
        config
            .global
            .track_file
            .as_deref()
            .map(|p| p.to_string_lossy().to_string()),
        "\"./seen.txt\"",
    );
    out.push('\n');

    out.push_str("[d-favs]\n");
    str_key(
        &mut out,
        "username",
        config.d_favs.username.clone(),
        "\"someuser\"",
    );
    int_key(
        &mut out,
        "count",
        config.d_favs.count.map(|v| v as i64),
        "5",
    );
    bool_key(&mut out, "random", config.d_favs.random, "false");
    str_key(&mut out, "tags", config.d_favs.tags.clone(), "\"\"");
    out.push('\n');

    out.push_str("[d-tags]\n");
    str_key(&mut out, "tags", config.d_tags.tags.clone(), "\"scalie\"");
    int_key(
        &mut out,
        "count",
        config.d_tags.count.map(|v| v as i64),
        "5",
    );
    bool_key(&mut out, "random", config.d_tags.random, "false");
    out.push('\n');

    out.push_str("[d-pool]\n");
    int_key(
        &mut out,
        "pool_id",
        config.d_pool.pool_id.map(|v| v as i64),
        "22364",
    );
    out.push('\n');

    out.push_str("[zip]\n");
    str_key(
        &mut out,
        "name",
        config.zip.name.clone(),
        "\"Cloudjumping\"",
    );
    str_key(
        &mut out,
        "format",
        config.zip.format.clone(),
        "\"zip\" # Options: \"zip\", \"7z\", \"cbz\"",
    );
    out.push('\n');

    for (name, preset) in &config.presets {
        out.push_str(&format!("[presets.{name}]\n"));
        str_key(&mut out, "tags", preset.tags.clone(), "\"scalie\"");
        int_key(&mut out, "count", preset.count.map(|v| v as i64), "5");
        int_key(&mut out, "pages", preset.pages, "1");
        bool_key(&mut out, "random", preset.random, "false");
        bool_key(&mut out, "lower_quality", preset.lower_quality, "false");
        bool_key(&mut out, "nsfw", preset.nsfw, "false");
        str_key(&mut out, "dir", preset.dir.clone(), "\"./dl/\"");
        str_key(
            &mut out,
            "track_file",
            preset
                .track_file
                .as_deref()
                .map(|p| p.to_string_lossy().to_string()),
            "\"./seen.txt\"",
        );
        out.push('\n');
    }

    out
}

fn bool_key(out: &mut String, key: &str, value: Option<bool>, default: &str) {
    match value {
        Some(v) => out.push_str(&format!("{key} = {}\n", toml::Value::Boolean(v))),
        None => out.push_str(&format!("# {key} = {default}\n")),
    }
}

fn int_key(out: &mut String, key: &str, value: Option<i64>, default: &str) {
    match value {
        Some(v) => out.push_str(&format!("{key} = {}\n", toml::Value::Integer(v))),
        None => out.push_str(&format!("# {key} = {default}\n")),
    }
}

fn str_key(out: &mut String, key: &str, value: Option<String>, default: &str) {
    match value {
        Some(v) => out.push_str(&format!("{key} = {}\n", toml::Value::String(v))),
        None => out.push_str(&format!("# {key} = {default}\n")),
    }
}

/// Opens the global configuration file in the OS's default application
/// (an editor for `.toml` files), creating it with the template contents first
/// if it doesn't exist. Uses `cmd /C start` on Windows, `open` on macOS, and
/// `xdg-open` on Linux.
pub fn open() -> Result<(), String> {
    let path = path()?;
    if let Err(e) = create_file(&path) {
        return Err(format!(
            "Could not create config at {}: {e}",
            path.display()
        ));
    }

    open_path(&path)
}

fn open_path(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", ""]).arg(path);
        cmd
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut cmd = Command::new("open");
        cmd.arg(path);
        cmd
    };
    #[cfg(target_os = "linux")]
    let mut command = {
        let mut cmd = Command::new("xdg-open");
        cmd.arg(path);
        cmd
    };

    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!(
            "Failed to open {} (exited with {status}).",
            path.display()
        )),
        Err(e) => Err(format!("Could not launch the default application: {e}.")),
    }
}

fn create_file(path: &std::path::Path) -> std::io::Result<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, TEMPLATE)
}

const TEMPLATE: &str = r#"# e-cli configuration
# Command-line values override these settings.

[global]
# verbose = false
# nsfw = false
# login = false
# lower_quality = false
# pages = -1
# num_threads = 5
# dir = "./dl/"
# track_file = "./seen.txt"

[d-favs]
# username = "someuser"
# count = 5
# random = false
# tags = ""

[d-tags]
# tags = "scalie"
# count = 5
# random = false

[d-pool]
# pool_id = 22364

[zip]
# name = "Cloudjumping"
# format = "zip" # Options: "zip", "7z", "cbz"

# Reusable tag searches can be added as [presets.name] sections.
# [presets.art]
# tags = "dragon"
# count = 25
# pages = 1
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_reads_all_config_sections() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
                [global]
                nsfw = true
                num_threads = 3

                [d-favs]
                username = "someuser"
                count = 25

                [d-tags]
                tags = "dragon"

                [d-pool]
                pool_id = 123

                [zip]
                name = "archive"
                format = "cbz"

                [presets.art]
                tags = "dragon"
                count = 25
                pages = 1
            "#,
        )
        .expect("write config");

        let config = load(&path).expect("load config");

        assert_eq!(config.global.nsfw, Some(true));
        assert_eq!(config.global.num_threads, Some(3));
        assert_eq!(config.d_favs.username.as_deref(), Some("someuser"));
        assert_eq!(config.d_favs.count, Some(25));
        assert_eq!(config.d_tags.tags.as_deref(), Some("dragon"));
        assert_eq!(config.d_pool.pool_id, Some(123));
        assert_eq!(config.zip.name.as_deref(), Some("archive"));
        assert_eq!(config.zip.format.as_deref(), Some("cbz"));
        assert_eq!(config.presets["art"].tags.as_deref(), Some("dragon"));
        assert_eq!(config.presets["art"].count, Some(25));
    }

    #[test]
    fn load_rejects_invalid_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(&path, "[global\ninvalid").expect("write config");

        assert!(load(&path).is_err());
    }

    #[test]
    fn template_lists_archive_format_values() {
        assert!(TEMPLATE.contains("Options: \"zip\", \"7z\", \"cbz\""));
    }

    #[test]
    fn render_preserves_comments_for_unset_keys() {
        let out = render(&Config::default());
        assert!(out.contains("# verbose = false"));
        assert!(out.contains("# nsfw = false"));
        assert!(out.contains("# dir = \"./dl/\""));
        assert!(out.contains("# tags = \"\""));
        assert!(out.contains("# format = \"zip\" # Options: \"zip\", \"7z\", \"cbz\""));
        assert!(!out.contains("\nnsfw = false"));
    }

    #[test]
    fn render_writes_set_keys_and_round_trips() {
        let mut config = Config::default();
        config.global.nsfw = Some(true);
        config.global.pages = Some(3);
        config.global.num_threads = Some(2);
        config.global.track_file = Some(std::path::PathBuf::from("seen.txt"));
        config.d_favs.username = Some("someuser".to_owned());
        config.d_favs.tags = Some("dragon".to_owned());
        config.d_pool.pool_id = Some(22364);
        config.zip.format = Some("cbz".to_owned());

        let out = render(&config);
        assert!(out.contains("nsfw = true"));
        assert!(out.contains("# login = false"));
        assert!(out.contains("pages = 3"));
        assert!(out.contains("num_threads = 2"));
        assert!(out.contains("track_file = \"seen.txt\""));
        assert!(out.contains("username = \"someuser\""));
        assert!(out.contains("tags = \"dragon\""));
        assert!(out.contains("pool_id = 22364"));
        assert!(out.contains("format = \"cbz\""));

        let parsed: Config = toml::from_str(&out).expect("render output must parse");
        assert_eq!(parsed.global.nsfw, Some(true));
        assert_eq!(parsed.global.pages, Some(3));
        assert_eq!(parsed.global.num_threads, Some(2));
        assert_eq!(
            parsed.global.track_file.as_deref(),
            Some(std::path::Path::new("seen.txt"))
        );
        assert_eq!(parsed.d_favs.username.as_deref(), Some("someuser"));
        assert_eq!(parsed.d_favs.tags.as_deref(), Some("dragon"));
        assert_eq!(parsed.d_pool.pool_id, Some(22364));
        assert_eq!(parsed.zip.format.as_deref(), Some("cbz"));
    }
}
