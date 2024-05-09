//! ## E-Tools
//! The command line tool for downloading E-Posts from a certain site.
//! 
//! It aims to be fast, and have verbose downloading console print outs.
//! 
//! What it can do:
//! - [x] Downloading Favourites of a user.
//! - [x] Downloading a single post from the site.
//! 
//! What it cannot do (so far):
//! - [ ] Downloading multiple pages
//! - [ ] Downloading of multiple specific posts from a txt file
//! 
//! ## Usage
//! 
//! Downloading Favourites of a user with specific amount of posts.
//! ```
//! e-tools.exe download-favourites {Username} -c {Number of Posts}
//! ```
//! 
//! Downloading Random Favourites of a user
//! ```
//! e-tools.exe download-favourites {Username} -r
//! ```
//! 
//! Downloading a single post with lower quality
//! ```
//! e-tools.exe --lower-quality download-post {Post Id}
//! ```

use std::path::Path;

use clap::Parser;
use cli::Commands;
use commands::{download_favourites, download_post};
use tokio::{fs::remove_dir_all, time::Instant};

pub mod funcs;
pub mod commands;
pub mod cli;
pub mod type_defs;

#[tokio::main]
async fn main() {
    let args = cli::Args::parse();

    let fn_start = Instant::now();

    let bytes_downloaded;

    match &args.command {
        Some(Commands::ClearDl) => {
            if Path::new("./dl/").exists() {
                remove_dir_all("./dl/").await.expect("Err");
                return println!("Cleaned the ./dl/ folder and also deleted the folder fully!")
            } else {
                return println!("Nothing to clean... Exiting!")
            }
        }
        Some(Commands::DownloadFavourites {username, count, random, tags}) => {
            let finished_return = download_favourites(username, count, random, tags, &args.lower_quality).await;
            match finished_return {
                Some(x) => { bytes_downloaded = x }
                None => { bytes_downloaded = 0.0}
            }
        }
        Some(Commands::DownloadPost {post_id }) => {
            let finished_return = download_post(post_id, &args.lower_quality).await;
            match finished_return {
                Some(x) => { bytes_downloaded = x }
                None => {bytes_downloaded = 0.0}
            }
        }
        None => {
            return
        }
    }

    let notice: String = {
        if args.lower_quality {
            "\nMay be incorrect with the --lower-quality tag.".to_string()
        } else {
            "".to_string()
        }
    };

    println!("Whole Program took: {} seconds! Downloaded a total of: {:.2} MB.{}", fn_start.elapsed().as_secs(), (bytes_downloaded/1024.0/1024.0).to_string(), notice);
}
