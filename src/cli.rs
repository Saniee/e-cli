//! The cli definitions using [clap] derive

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "E6 Tools")]
#[command(version, long_about = None)]
#[command(arg_required_else_help = true)]
/// The arguments, includes the subcommands from the [enum@Commands] and also a bool for lower quality dl.
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg[long, help = "Specify the api url to use.", default_value = "e926.net"]]
    pub api_source: String,

    #[arg[long, help = "Tries to download the lower quality media files."]]
    pub lower_quality: bool,
}

#[derive(Subcommand, PartialEq, Eq)]
/// All the commands the CLI has.
pub enum Commands {
    #[command[about = "Deletes the whole ./dl/ directory with it's contents."]]
    ClearDl,
    #[command[about = "Downloads the set amount of favourites from the username provided."]]
    DownloadFavourites {
        username: String,
        #[arg(short, default_value_t = 5)]
        count: u8,
        #[arg(short, default_value_t = false)]
        random: bool,
        #[arg(long, default_value = "")]
        tags: String,
    }
}
