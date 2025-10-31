use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "e-cli")]
#[command(version, long_about = None)]
#[command(arg_required_else_help = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg[long, short = 'a', help = "Specify the api url to use.", default_value = "e926.net"]]
    pub api_source: String,

    #[arg[long, short = 'l', help = "Tries to download the lower quality media files."]]
    pub lower_quality: bool,

    #[arg[long, short = 'b', help = "If set, the app will try to download pages of posts.", action]]
    pub bulk: bool,

    #[arg[long, short = 'p', help = "Number of pages to download, p = -1, gets all pages. p > 0, gets that amount of pages."]]
    pub pages: i64,

    #[arg[long, short = 't', help = "The number of threads to use for downloads. Cannot set above 10.", default_value_t = 5]]
    pub num_threads: usize,
}

#[derive(Subcommand, PartialEq, Eq)]
pub enum Commands {
    #[command[about = "Deletes the whole ./dl/ directory with it's contents."]]
    ClearDl,
    #[command[about = "Downloads the set amount of favourites from the username provided."]]
    DFavourites {
        username: String,
        #[arg[short, help = "The amount of posts to get. Cannot set above 320 (Api Max.)", default_value_t = 5]]
        count: u32,
        #[arg[short, help = "Adds the order:random in the search.", action]]
        random: bool,
        #[arg[long, help = "Specify the search further with tags.", default_value = ""]]
        tags: String,
    },
    #[command[about = "Downloads the set amount of posts with the tags provided."]]
    DTags {
        tags: String,
        #[arg[short, help = "The amount of posts to get. Cannot set above 320 (Api Max.)", default_value_t = 5]]
        count: u32,
        #[arg[short, help = "Adds the order:random in the search.", action]]
        random: bool,
    }
}
