use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};

use crate::config::Config;

/// Default directory that downloads, `zip`, and `clear-dl` operate on.
pub const DL_DIR: &str = "./dl/";

#[derive(Parser)]
#[command(about = "A fast, multi-threaded downloader for e926/e621-style booru APIs.")]
#[command(
    version,
    long_about = "e-cli downloads posts from e926.net/e621.net by user favorites, tag search, or pool, using multiple threads to download \
    files in parallel. Supports optional authenticated login and a lower-quality fallback mode."
)]
#[command(arg_required_else_help = true)]
#[command(after_help = "EXAMPLES:\n  \
    e-cli d-tags \"scalie\" -c 250 -r -p 1        Download 250 random posts tagged 'scalie', 1 page\n  \
    e-cli d-favs someuser -c 100                 Download 100 favorites from 'someuser'\n  \
    e-cli d-pool 22364                           Download a pool into ./dl/\n  \
    e-cli d-pool 22364 -d ./pool/                Download a pool into ./pool/\n  \
     e-cli d-favs someuser -c 100 -T seen.txt     Download favorites, skipping posts tracked in seen.txt\n  \
     e-cli zip -n Cloudjumping -f cbz             Package ./dl/ into Cloudjumping.cbz\n  \
     e-cli clear-dl                               Delete the ./dl/ output directory\n  \
     e-cli config                                 Create or edit the TOML configuration")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short = 'v', long, help = "Verbose Output.", action = ArgAction::SetTrue)]
    pub verbose: bool,

    #[arg(short = 'L', long, help = "Ability to sign-in into the API for better fetching of posts.", action = ArgAction::SetTrue)]
    pub login: bool,

    #[arg(
        short = 'n',
        long,
        help = "Use the NSFW API (e621.net) instead of the SFW API (e926.net).",
        action = ArgAction::SetTrue
    )]
    pub nsfw: bool,

    #[arg(short = 'l', long, help = "Tries to download the lower quality media files.", action = ArgAction::SetTrue)]
    pub lower_quality: bool,

    #[arg[short = 'p', long, help = "Number of pages to download, p = -1, gets all pages. p > 0, gets that amount of pages."]]
    pub pages: Option<i64>,

    #[arg[short = 't', long, help = "The number of threads to use for downloads. Cannot set above 10."]]
    pub num_threads: Option<usize>,

    #[arg[short = 'd', long, global = true, help = "The directory to download files into (also used by zip and clear-dl)."]]
    pub dir: Option<String>,

    #[arg[short = 'T', long, global = true, help = "Path to a tracking file that records downloaded post IDs, so re-runs only download new posts. Created if it doesn't exist."]]
    pub track_file: Option<PathBuf>,

    #[arg(long, global = true, help = "Plan downloads and print a summary without writing files.", action = ArgAction::SetTrue)]
    pub dry_run: bool,
    #[arg(
        long,
        global = true,
        help = "Write a JSON metadata manifest to this path."
    )]
    pub manifest: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "Persistent JSON MD5 duplicate index path."
    )]
    pub duplicate_index: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        default_value_t = 3,
        help = "Maximum retries for transient download failures."
    )]
    pub retries: u32,
    #[arg(
        long,
        global = true,
        help = "Persistent failed-download manifest path."
    )]
    pub failure_manifest: Option<PathBuf>,
}

#[derive(Subcommand, PartialEq, Eq)]
pub enum Commands {
    #[command(about = "Opens the interactive terminal UI.")]
    Tui,
    #[command(about = "Opens the global TOML configuration file in the default editor.")]
    Config,
    #[command[about = "Checks whether a newer e-cli release is available on GitHub."]]
    CheckUpdate,
    #[command[about = "Deletes the whole download directory (./dl/ by default, see -d) with it's contents."]]
    ClearDl,
    #[command[about = "Downloads the set amount of favourites from the username provided."]]
    DFavs {
        username: Option<String>,
        #[arg[short = 'c', help = "The amount of posts to get. Max=250."]]
        count: Option<u32>,
        #[arg(short = 'r', help = "Adds the order:random in the search.", action = ArgAction::SetTrue)]
        random: bool,
        #[arg[short = 't', help = "Specify the search further with tags."]]
        tags: Option<String>,
    },
    #[command[about = "Downloads the set amount of posts with the tags provided."]]
    #[command[long_about = "Downloads the set amount of posts with the tags provided.\n\n\
        Requires the global -p/--pages flag to be set explicitly (e.g. -p 1), since there is \
        no default page count for tag search."]]
    DTags {
        tags: Option<String>,
        #[arg[short = 'c', help = "The amount of posts to get. Max=250."]]
        count: Option<u32>,
        #[arg(short = 'r', help = "Adds the order:random in the search.", action = ArgAction::SetTrue)]
        random: bool,
    },
    #[command[about = "Downloads a pool with the indexes in the names of the files."]]
    DPool {
        #[arg(help = "The Pool ID")]
        pool_id: Option<u64>,
    },
    #[command[about = "Packages a downloaded pool (./dl/) into an archive."]]
    #[command[long_about = "Packages a downloaded pool (./dl/) into an archive.\n\n\
        Intended for pools downloaded with d-pool, since the index-prefixed filenames \
        (1-, 2-, 3-, ...) are what makes the resulting archive readable in order — \
        that's also why the cbz format exists. Requires the '7z' executable to be \
        available on your PATH."]]
    Zip {
        #[arg[short = 'n', long, help = "Name for the output archive, without extension."]]
        name: Option<String>,
        #[arg[short = 'f', long, value_enum, help = "Archive format to use."]]
        format: Option<ArchiveFormat>,
    },
    #[command(about = "Runs a named tag-search preset from config.toml.")]
    Preset {
        name: String,
        #[arg(short = 'c')]
        count: Option<u32>,
        #[arg(short = 'r', action = ArgAction::SetTrue)]
        random: bool,
    },
    #[command(about = "Retries posts recorded in the failed-download manifest.")]
    RetryFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ArchiveFormat {
    #[value(help = "Standard .zip archive.")]
    Zip,
    #[value(name = "7z", help = "Native .7z archive.")]
    SevenZip,
    #[value(help = "A .zip renamed to .cbz, for comic/e-book readers to read pools in order.")]
    Cbz,
}

impl ArchiveFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::SevenZip => "7z",
            ArchiveFormat::Cbz => "cbz",
        }
    }
}

pub fn apply_config(args: &mut Args, config: &Config) -> Result<(), String> {
    let global = &config.global;
    if !args.verbose {
        args.verbose = global.verbose.unwrap_or(false);
    }
    if !args.login {
        args.login = global.login.unwrap_or(false);
    }
    if !args.nsfw {
        args.nsfw = global.nsfw.unwrap_or(false);
    }
    if !args.lower_quality {
        args.lower_quality = global.lower_quality.unwrap_or(false);
    }
    if args.pages.is_none() {
        args.pages = global.pages;
    }
    if args.num_threads.is_none() {
        args.num_threads = global.num_threads;
    }
    if args.dir.is_none() {
        args.dir = global.dir.clone();
    }
    if args.track_file.is_none() {
        args.track_file = global.track_file.clone();
    }

    match &mut args.command {
        Some(Commands::DFavs {
            username,
            count,
            random,
            tags,
        }) => {
            if username.is_none() {
                *username = config.d_favs.username.clone();
            }
            if count.is_none() {
                *count = config.d_favs.count;
            }
            if !*random {
                *random = config.d_favs.random.unwrap_or(false);
            }
            if tags.is_none() {
                *tags = config.d_favs.tags.clone();
            }
        }
        Some(Commands::DTags {
            tags,
            count,
            random,
        }) => {
            if tags.is_none() {
                *tags = config.d_tags.tags.clone();
            }
            if count.is_none() {
                *count = config.d_tags.count;
            }
            if !*random {
                *random = config.d_tags.random.unwrap_or(false);
            }
        }
        Some(Commands::DPool { pool_id }) => {
            if pool_id.is_none() {
                *pool_id = config.d_pool.pool_id;
            }
        }
        Some(Commands::Zip { name, format }) => {
            if name.is_none() {
                *name = config.zip.name.clone();
            }
            if format.is_none()
                && let Some(value) = config.zip.format.as_deref()
            {
                *format = Some(match value {
                    "zip" => ArchiveFormat::Zip,
                    "7z" => ArchiveFormat::SevenZip,
                    "cbz" => ArchiveFormat::Cbz,
                    _ => {
                        return Err(format!(
                            "Invalid zip format '{value}' in the config; expected zip, 7z, or cbz."
                        ));
                    }
                });
            }
        }
        Some(Commands::Preset {
            name,
            count,
            random,
        }) => {
            let preset = config
                .presets
                .get(name)
                .ok_or_else(|| format!("Unknown preset '{name}'."))?;
            if args.pages.is_none() {
                args.pages = preset.pages;
            }
            if args.dir.is_none() {
                args.dir = preset.dir.clone();
            }
            if args.track_file.is_none() {
                args.track_file = preset.track_file.clone();
            }
            if !args.lower_quality {
                args.lower_quality = preset.lower_quality.unwrap_or(false);
            }
            if !args.nsfw {
                args.nsfw = preset.nsfw.unwrap_or(false);
            }
            if count.is_none() {
                *count = preset.count;
            }
            if !*random {
                *random = preset.random.unwrap_or(false);
            }
        }
        Some(Commands::Config)
        | Some(Commands::Tui)
        | Some(Commands::ClearDl)
        | Some(Commands::CheckUpdate)
        | Some(Commands::RetryFailed)
        | None => {}
    }
    Ok(())
}

pub fn fill_defaults(args: &mut Args) -> Result<(), String> {
    args.pages.get_or_insert(-1);
    args.num_threads.get_or_insert(5);
    args.dir.get_or_insert_with(|| DL_DIR.to_owned());

    match &mut args.command {
        Some(Commands::DFavs { count, tags, .. }) => {
            count.get_or_insert(5);
            tags.get_or_insert_with(String::new);
        }
        Some(Commands::DTags { count, .. }) => {
            count.get_or_insert(5);
        }
        Some(Commands::Zip { format, .. }) => {
            format.get_or_insert(ArchiveFormat::Zip);
        }
        Some(Commands::Preset { .. }) => {}
        _ => {}
    }
    Ok(())
}

pub fn validate_args(args: &Args) -> Result<(), String> {
    if args.num_threads.unwrap_or(5) == 0 {
        return Err("Must use at least 1 thread.".into());
    }
    if args.num_threads.unwrap_or(5) > 10 {
        return Err("Cannot go above 10 threads for downloads.".into());
    }
    if let Some(Commands::DFavs { count, .. }) = &args.command
        && count.unwrap_or(5) > 250
    {
        return Err("Cannot go above 250 posts per page.".into());
    }
    if let Some(Commands::DTags { count, .. }) = &args.command
        && count.unwrap_or(5) > 250
    {
        return Err("Cannot go above 250 posts per page.".into());
    }
    if let Some(Commands::DTags { .. }) = &args.command
        && args.pages.unwrap_or(-1) == -1
    {
        return Err(
            "You NEED to specify the page amount for downloading with tags. Exiting...".into(),
        );
    }
    match &args.command {
        Some(Commands::DFavs { username, .. }) if username.is_none() => {
            return Err("d-favs requires a username argument or a configured username.".into());
        }
        Some(Commands::DTags { tags, .. }) if tags.is_none() => {
            return Err("d-tags requires tags or a configured tags value.".into());
        }
        Some(Commands::DPool { pool_id }) if pool_id.is_none() => {
            return Err("d-pool requires a pool ID argument or a configured pool_id.".into());
        }
        Some(Commands::Preset { name, .. }) if name.is_empty() => {
            return Err("preset requires a name.".into());
        }
        Some(Commands::Zip { name, .. }) if name.is_none() => {
            return Err("zip requires a name argument or a configured name.".into());
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
