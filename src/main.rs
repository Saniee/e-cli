use std::{fs::{self}, io::{self}, path::Path, time::Instant};
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

pub struct CliContext {
    pub verbose: bool,
    pub api_source: String,
    pub lower_quality: bool,
    pub pages: i64,
    pub num_threads: usize,
}

pub struct Login {
    pub username: String,
    pub api_key: String,
}

fn main() {
    let args = cli::Args::parse();

    let context = CliContext{ 
        verbose: args.verbose, 
        api_source: args.api_source, 
        lower_quality: args.lower_quality, 
        pages: args.pages, 
        num_threads: args.num_threads
    };
    let log_format = fmt::format().without_time().with_target(false).compact();
    if args.verbose {
        fmt().event_format(log_format).with_max_level(Level::DEBUG).with_target(true).init();
    } else {
        fmt().event_format(log_format).init();
    }
    
    if args.num_threads > 10 {
        return error!("Cannot go above 10 threads for downloads.");
    }

    if let Some(Commands::DFavs {
        username: _,
        count,
        random: _,
        tags: _,
    }) = &args.command
        && *count > 250
    {
        return error!("Cannot go above 250 posts per page.");
    }
    if let Some(Commands::DTags { tags: _, count, random: _ }) = &args.command && *count > 250{
        return error!("Cannot go above 250 posts per page.");
    }

    let mut username = String::new();
    let mut api_key = String::new();
    if args.login {
        let client = reqwest::blocking::Client::builder().user_agent(AGENT).build().expect("Error creating auth client.");

        info!("Sign-In via inputing your username and api_key.");
        info!("This info isn't sent anywhere. Only when the cli runs.");
        info!("Username: ");
        io::stdin().read_line(&mut username).expect("Error getting user input.");
        username = username.trim().to_owned();
        info!("Api Key: ");
        io::stdin().read_line(&mut api_key).expect("Error getting user input.");
        api_key = api_key.trim().to_owned();
        info!("Testing if valid...");
        let resp = client.get(format!("https://{}/posts.json?tags=&limit=5", context.api_source)).basic_auth(&username, Some(api_key.clone())).send().expect("Error getting Auth response.");
        match resp.error_for_status() {
            Ok(_) => {
                info!("Sign-in Passed! Continuing...")
            },
            Err(err) => {
                return error!("The credentials provided aren't valid, or something else happened. Err: {err}");
            }
        }
    }
    let login = Login{ username, api_key };

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
        Some(Commands::DFavs {
            username,
            count,
            random,
            tags,
        }) => {
            bytes_downloaded = download_favourites(
                &context,
                &login,
                username,
                count,
                random,
                tags,
            );
        }
        Some(Commands::DTags {
            tags,
            count,
            random,
        }) => {
            if args.pages == -1 {
                return error!("You NEED to specify the page amount for downloading with tags. Exiting...");
            }
            bytes_downloaded = download_search(
                &context,
                &login,
                tags,
                count,
                random,
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
