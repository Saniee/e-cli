#![doc = include_str!("../docs_markdown/main.md")]

use std::path::Path;

use clap::Parser;
use cli::Commands;
use commands::{
    download_favourites, download_post,
};
use tokio::{fs, time::Instant};

pub mod cli;
pub mod commands;
pub mod funcs;
pub mod type_defs;

/// Main function, handles the cli arguments.
#[tokio::main]
async fn main() {
    let args = cli::Args::parse();

    let fn_start = Instant::now();

    match &args.command {
        Some(Commands::ClearDl) => {
            if Path::new("./dl/").exists() {
                fs::remove_dir_all("./dl/").await.expect("Err");
                return println!("Cleaned the ./dl/ folder and also deleted the folder fully!");
            } else {
                return println!("Nothing to clean... Exiting!");
            }
        }
        Some(Commands::DownloadFavourites {
            username,
            count,
            random,
            tags,
        }) => {
            download_favourites(
                username,
                count,
                random,
                tags,
                &args.lower_quality,
                &args.api_source,
            )
            .await;
        }
        Some(Commands::DownloadPost { post_id }) => {
            download_post(post_id, &args.lower_quality, &args.api_source).await;
        }
        None => return,
    }

    println!(
        "Whole Program took: {} seconds!",
        fn_start.elapsed().as_secs(),
    );
}
