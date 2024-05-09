//! ## E-Tools
//! The command line tool for downloading E-Posts from a certain site.
//! 
//! It aims to be fast, and have verbose downloading console print outs.
//! 
//! What it can do:
//! - [x] Downloading Favourites of a user.
//! - [x] Downloading a single post from the site.
//! - [x] Downloading of multiple specific posts from a txt file
//! 
//! What it cannot do (so far):
//! - [ ] Downloading multiple pages
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
use tokio::{fs, time::Instant};

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
                fs::remove_dir_all("./dl/").await.expect("Err");
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
        Some(Commands::DownloadPosts { text_file }) => {
            let txt_file_path = Path::new(text_file);
            if !txt_file_path.exists() {
                println!("The file specified wasn't found!");
                return
            }

            let txt_file_contents = fs::read_to_string(txt_file_path).await.expect("Err");
            let id_list: Vec<&str> = txt_file_contents.lines().collect();
            
            let finished_return = commands::download_posts_from_txt(id_list, &args.lower_quality).await;
            match finished_return {
                Some(x) => {bytes_downloaded = x}
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

    // Dynamically output the converted file_size appropriate to its designation
    let size_conversion: String = if bytes_downloaded/1024.0/1024.0 < 1.0 {
        format!("{:.2} KB", bytes_downloaded/1024.0)
    } else {
        format!("{:.2} MB", bytes_downloaded/1024.0/1024.0)
    };

    println!("Whole Program took: {} seconds! Downloaded a total of: {}{}", fn_start.elapsed().as_secs(), size_conversion, notice);
}
