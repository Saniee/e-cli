use std::{fs, path::Path, time::Instant};

use clap::Parser;
use cli::Commands;
use commands::download_favourites;

use crate::commands::download_search;

pub mod cli;
pub mod commands;
pub mod funcs;
pub mod type_defs;

pub static AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

fn main() {
    let args = cli::Args::parse();

    if args.num_threads > 10 {
        return println!("Cannot go above 10 threads for downloads.");
    }

    if let Some(Commands::DFavourites {
        username: _,
        count,
        random: _,
        tags: _,
    }) = &args.command
        && *count > 320
    {
        return println!("Cannot go above 320 posts per page query.");
    }

    #[allow(unused_mut)]
    let mut bytes_downloaded;
    let fn_start = Instant::now();

    match &args.command {
        Some(Commands::ClearDl) => {
            if !Path::new("./dl/").exists() {
                return println!("Nothing to clean... Exiting!");
            }

            fs::remove_dir_all("./dl/").expect("Err");
            return println!("Cleaned the ./dl/ folder and also deleted the folder fully!");
        }
        Some(Commands::DFavourites {
            username,
            count,
            random,
            tags,
        }) => {
            bytes_downloaded = download_favourites(
                username,
                count,
                &args.pages,
                random,
                tags,
                &args.lower_quality,
                &args.api_source,
                args.num_threads,
            );
        }
        Some(Commands::DTags {
            tags,
            count,
            random,
        }) => {
            bytes_downloaded = download_search(
                tags,
                count,
                &args.pages,
                random,
                &args.lower_quality,
                &args.api_source,
                args.num_threads,
            );
        }
        None => return,
    }

    println!(
        "Downloaded {:.2} MB in {} seconds!",
        bytes_downloaded / 1024.0 / 1024.0,
        fn_start.elapsed().as_secs(),
    );
}
