use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "e-cli")]
#[command(version, long_about = None)]
#[command(arg_required_else_help = true)]
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
    DTags {
        tags: String,
        #[arg[short = 'c', help = "The amount of posts to get. Max=250.", default_value_t = 5]]
        count: u32,
        #[arg[short = 'r', help = "Adds the order:random in the search.", action]]
        random: bool,
    },
}
