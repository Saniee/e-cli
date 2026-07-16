use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(about = "A fast, multi-threaded downloader for e926/e621-style booru APIs.")]
#[command(version, long_about = "e-cli downloads posts from e926.net/e621.net (or a compatible \
    booru-style API) by user favorites, tag search, or pool, using multiple threads to download \
    files in parallel. Supports optional authenticated login and a lower-quality fallback mode.")]
#[command(arg_required_else_help = true)]
#[command(after_help = "EXAMPLES:\n  \
    e-cli d-tags \"scalie\" -c 250 -r -p 1        Download 250 random posts tagged 'scalie', 1 page\n  \
    e-cli d-favs someuser -c 100                 Download 100 favorites from 'someuser'\n  \
    e-cli d-pool 22364                           Download a pool into ./dl/\n  \
    e-cli zip -n Cloudjumping -f cbz             Package ./dl/ into Cloudjumping.cbz\n  \
    e-cli clear-dl                               Delete the ./dl/ output directory")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg[short = 'v', long, help = "Verbose Output.", action]]
    pub verbose: bool,

    #[arg[short = 'L', long, help = "Ability to sign-in into the API for better fetching of posts.", action]]
    pub login: bool,

    #[arg[short = 'a', long, help = "Specify the api url to use.", default_value = "e926.net"]]
    pub api_source: String,

    #[arg[short = 'l', long, help = "Tries to download the lower quality media files."]]
    pub lower_quality: bool,

    #[arg[short = 'p', long, help = "Number of pages to download, p = -1, gets all pages. p > 0, gets that amount of pages.", default_value_t = -1]]
    pub pages: i64,

    #[arg[short = 't', long, help = "The number of threads to use for downloads. Cannot set above 10.", default_value_t = 5]]
    pub num_threads: usize,
}

#[derive(Subcommand, PartialEq, Eq)]
pub enum Commands {
    #[command[about = "Deletes the whole ./dl/ directory with it's contents."]]
    ClearDl,
    #[command[about = "Downloads the set amount of favourites from the username provided."]]
    DFavs {
        username: String,
        #[arg[short = 'c', help = "The amount of posts to get. Max=250.", default_value_t = 5]]
        count: u32,
        #[arg[short = 'r', help = "Adds the order:random in the search.", action]]
        random: bool,
        #[arg[short = 't', help = "Specify the search further with tags.", default_value = ""]]
        tags: String,
    },
    #[command[about = "Downloads the set amount of posts with the tags provided."]]
    #[command[long_about = "Downloads the set amount of posts with the tags provided.\n\n\
        Requires the global -p/--pages flag to be set explicitly (e.g. -p 1), since there is \
        no default page count for tag search."]]
    DTags {
        tags: String,
        #[arg[short = 'c', help = "The amount of posts to get. Max=250.", default_value_t = 5]]
        count: u32,
        #[arg[short = 'r', help = "Adds the order:random in the search.", action]]
        random: bool,
    },
    #[command[about = "Downloads a pool with the indexes in the names of the files."]]
    DPool {
        #[arg(help = "The Pool ID")]
        pool_id: u64,
    },
    #[command[about = "Packages a downloaded pool (./dl/) into an archive."]]
    #[command[long_about = "Packages a downloaded pool (./dl/) into an archive.\n\n\
        Intended for pools downloaded with d-pool, since the index-prefixed filenames \
        (1-, 2-, 3-, ...) are what makes the resulting archive readable in order — \
        that's also why the cbz format exists. Requires the '7z' executable to be \
        available on your PATH."]]
    Zip {
        #[arg[short = 'n', long, help = "Name for the output archive, without extension."]]
        name: String,
        #[arg[short = 'f', long, value_enum, default_value_t = ArchiveFormat::Zip, help = "Archive format to use."]]
        format: ArchiveFormat,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
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

pub fn validate_args(args: &Args) -> Result<(), String> {
    if args.num_threads == 0 {
        return Err("Must use at least 1 thread.".into());
    }
    if args.num_threads > 10 {
        return Err("Cannot go above 10 threads for downloads.".into());
    }
    if let Some(Commands::DFavs { count, .. }) = &args.command
        && *count > 250
    {
        return Err("Cannot go above 250 posts per page.".into());
    }
    if let Some(Commands::DTags { count, .. }) = &args.command
        && *count > 250
    {
        return Err("Cannot go above 250 posts per page.".into());
    }
    if let Some(Commands::DTags { .. }) = &args.command
        && args.pages == -1
    {
        return Err(
            "You NEED to specify the page amount for downloading with tags. Exiting...".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
