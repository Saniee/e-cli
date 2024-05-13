#![doc = include_str!("../docs_markdown/main.md")]

use std::path::Path;

use clap::Parser;
use cli::Commands;
use commands::{download_favourites, download_post, download_posts_from_file, download_posts_from_txt, fetch_posts};
use tokio::{fs, time::Instant};

pub mod funcs;
pub mod commands;
pub mod cli;
pub mod type_defs;

/// Main function, handles the cli arguments.
#[tokio::main]
async fn main() {
    let args = cli::Args::parse();

    let fn_start = Instant::now();

    let bytes_downloaded;

    match &args.command {
        Some(Commands::ClearDl) => {
            if Path::new("./dl/").exists() {
                fs::remove_dir_all("./dl/").await.expect("Err");
                return println!("Cleaned the ./dl/ folder and also deleted the folder fully!")
            } else {
                return println!("Nothing to clean... Exiting!")
            }
        }
        Some(Commands::DownloadFavourites {username, count, random, tags}) => {
            let finished_return = download_favourites(username, count, random, tags, &args.lower_quality, &args.api_source).await;
            match finished_return {
                Some(x) => { bytes_downloaded = x }
                None => { bytes_downloaded = 0.0}
            }
        }
        Some(Commands::DownloadPost {post_id }) => {
            let finished_return = download_post(post_id, &args.lower_quality, &args.api_source).await;
            match finished_return {
                Some(x) => { bytes_downloaded = x }
                None => {bytes_downloaded = 0.0}
            }
        }
        Some(Commands::DownloadPosts { file_path }) => {
            let finished_return = download_posts_from_file(file_path, &args.lower_quality).await;
            match finished_return {
                Some(x) => {bytes_downloaded = x}
                None => {bytes_downloaded = 0.0}
            }
        }
        Some(Commands::DownloadPostsFromTxt { file_path }) => {
            download_posts_from_txt(file_path, &args.api_source, &args.lower_quality).await;
            return
        }
        Some(Commands::GetPages { tags, count }) => {
            fetch_posts(tags, count, &args.api_source).await;
            return
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

    // Dynamically output the converted file_size appropriate to its designation
    let size_conversion: String = if bytes_downloaded/1024.0/1024.0 < 1.0 {
        format!("{:.2} KB", bytes_downloaded/1024.0)
    } else {
        format!("{:.2} MB", bytes_downloaded/1024.0/1024.0)
    };

    println!("Whole Program took: {} seconds! Downloaded a total of: {}{}", fn_start.elapsed().as_secs(), size_conversion, notice);
}
