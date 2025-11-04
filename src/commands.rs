use std::sync::mpsc::channel;

use reqwest::blocking::Client;

use rayon::prelude::*;
use tracing::{Level, debug, error, info, span};

use crate::{AGENT, CliContext, DownloadStatistics, Login};
use crate::funcs::{self, DownloadFinished, create_dl_dir, get_pages, slice_arr, sum_posts};
use crate::type_defs::api_defs::{self, Post};

pub fn get_client() -> Client {
    Client::builder()
        .user_agent(AGENT)
        // !Experimental
        .timeout(None)
        .build()
        .expect("Error creating Client")
}

#[allow(clippy::too_many_arguments)]
pub fn download_favourites(
    context: &CliContext,
    login: &Login,
    username: &str,
    count: &u32,
    random: &bool,
    tags: &str,
) -> DownloadStatistics {
    let span = span!(Level::DEBUG, "DFavs");
    let _guard = span.enter();

    info!("Downloading Favorites of {username} into the ./dl/ folder!");
    let client = get_client();
    let random_check: &str = if *random { "order:random" } else { "" };
    let tags: &str = if !tags.is_empty() { tags } else { "" };
    let fav: String = format!("fav:{}", username);
    info!("Getting posts from pages!");
    let data: Vec<Vec<Post>> = get_pages(
        context,
        login,
        client,
        &fav,
        tags,
        random_check,
        count,
    );
    if data.is_empty() {
        error!("No posts found...");
        return DownloadStatistics::default();
    }
    let created_dir = create_dl_dir();
    if created_dir {
        info!("Created a ./dl/ directory for all the downloaded files.")
    }
    let total = sum_posts(&data);
    info!("Downloading {} posts...", total);
    let mut full_sum = 0.0;
    let mut finished: i64 = 0;
    let mut failed: i64 = 0;
    let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(context.num_threads)
            .build()
            .unwrap();
    for posts in data {
        let sliced_data = slice_arr(api_defs::Posts { posts }, 5);
        let (tx, rx) = channel::<Vec<DownloadFinished>>();
        // Multi-threaded implementation.
        pool.install(|| {
            debug!("Starting download of {} posts.", sliced_data.len());
            let dl_size: Vec<DownloadFinished> = sliced_data
                .into_par_iter()
                .map(|posts| {
                    let low_quality = &context.lower_quality;
                    funcs::download(login, posts.to_vec(), low_quality)
                })
                .collect();

            tx.send(dl_size).unwrap();
        });
        for status in rx.recv().unwrap() {
            finished += status.amount_finished;
            failed += status.amount_failed;
            full_sum += status.amount;
        }
    }
    DownloadStatistics { completed: finished, failed, total, downloaded_amount: full_sum }
}

pub fn download_search(
    context: &CliContext,
    login: &Login,
    tags: &str,
    count: &u32,
    random: &bool,
) -> DownloadStatistics {
    info!("Downloading posts, with '{tags}' tag/s, into the ./dl/ folder!");
    let client = get_client();
    let random_check: &str = if *random { "order:random" } else { "" };
    let tags: &str = if !tags.is_empty() { tags } else { "" };
    let fav = "";
    info!("Getting posts from pages!");
    let data: Vec<Vec<Post>> = get_pages(
        context,
        login,
        client,
        fav,
        tags,
        random_check,
        count,
    );
    if data.is_empty() {
        error!("No posts found...");
        return DownloadStatistics::default();
    }
    let created_dir = create_dl_dir();
    if created_dir {
        info!("Created a ./dl/ directory for all the downloaded files.")
    }
    let total = sum_posts(&data);
    info!("Downloading {} posts...", total);
    let mut full_sum = 0.0;
    let mut finished: i64 = 0;
    let mut failed: i64 = 0;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(context.num_threads)
        .build()
        .unwrap();
    for posts in data {
        let sliced_data = slice_arr(api_defs::Posts { posts }, 5);

        let (tx, rx) = channel::<Vec<DownloadFinished>>();

        // Multi-threaded implementation.
        pool.install(|| {
            let dl_size: Vec<DownloadFinished> = sliced_data
                .into_par_iter()
                .map(|posts| {
                    let low_quality = &context.lower_quality;
                    funcs::download(login, posts.to_vec(), low_quality)
                })
                .collect();

            tx.send(dl_size).unwrap();
        });
        for status in rx.recv().unwrap() {
            finished += status.amount_finished;
            failed += status.amount_failed;
            full_sum += status.amount;
        }
    }
    DownloadStatistics { completed: finished, failed, total, downloaded_amount: full_sum }
}
