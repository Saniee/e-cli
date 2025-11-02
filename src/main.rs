use std::{fs::{self, File}, path::Path, time::Instant};
use tracing::Level;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use clap::Parser;
use cli::Commands;

use commands::{download_favourites, download_search};
use tracing_subscriber::fmt;

pub mod cli;
pub mod commands;
pub mod funcs;
pub mod type_defs;

pub static AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

fn main() {
    let args = cli::Args::parse();

    let log_format = fmt::format().without_time().compact();
    if args.verbose == 1 {
        fmt().event_format(log_format).with_max_level(Level::DEBUG).init();
    } else if args.verbose == 2 {
        println!("Writing to 'trace.log'.");
        let file = File::create("trace.log").expect("Error creating tracing log.");
        fmt().with_writer(file).event_format(log_format).with_max_level(Level::TRACE).json().init();
    } else {
        fmt().event_format(log_format).init();
    }
    
    if args.num_threads > 10 {
        return error!("Cannot go above 10 threads for downloads.");
    }

    if let Some(Commands::DFavourites {
        username: _,
        count,
        random: _,
        tags: _,
    }) = &args.command
        && *count > 320
    {
        return error!("Cannot go above 320 posts per page query.");
    }

    #[allow(unused_mut)]
    let mut bytes_downloaded;
    let fn_start = Instant::now();

    match &args.command {
        Some(Commands::ClearDl) => {
            if !Path::new("./dl/").exists() {
                return info!("Nothing to clean... Exiting!");
            }

            fs::remove_dir_all("./dl/").expect("Err");
            return info!("Cleaned the ./dl/ folder and also deleted the folder fully!");
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

    info!(
        "Downloaded {:.2} MB in {} seconds!",
        bytes_downloaded / 1024.0 / 1024.0,
        fn_start.elapsed().as_secs(),
    );
}
