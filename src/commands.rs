use std::collections::HashMap;
use std::fs;
use std::process::Command;
use std::sync::mpsc::channel;

use reqwest::blocking::Client;

use rayon::prelude::*;
use tracing::{Level, debug, error, info, span};

use crate::funcs::{
    self, DownloadFinished, create_dl_dir, get_pages, get_pool, get_post_data, slice_posts,
    sum_posts,
};
use crate::type_defs::api_defs::{self, Post};
use crate::{AGENT, CliContext, DownloadStatistics, Login};

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
    let data: Vec<Vec<Post>> = get_pages(context, login, client, &fav, tags, random_check, count);
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
        let sliced_data = slice_posts(api_defs::Posts { posts }, 5);
        let (tx, rx) = channel::<Vec<DownloadFinished>>();
        // Multi-threaded implementation.
        pool.install(|| {
            debug!("Starting download of {} posts.", sliced_data.len());
            let dl_size: Vec<DownloadFinished> = sliced_data
                .into_par_iter()
                .map(|posts| {
                    let low_quality = &context.lower_quality;
                    funcs::download(login, posts.to_vec(), None, low_quality)
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
    DownloadStatistics {
        completed: finished,
        failed,
        total,
        downloaded_amount: full_sum,
    }
}

pub fn download_search(
    context: &CliContext,
    login: &Login,
    tags: &str,
    page_count: &u32,
    random: &bool,
) -> DownloadStatistics {
    let span = span!(Level::DEBUG, "DTags");
    let _guard = span.enter();

    info!("Downloading posts, with '{tags}' tag/s, into the ./dl/ folder!");
    let client = get_client();
    let random_check: &str = if *random { "order:random" } else { "" };
    let tags: &str = if !tags.is_empty() { tags } else { "" };
    let fav = "";
    info!("Getting posts from pages!");
    let data: Vec<Vec<Post>> =
        get_pages(context, login, client, fav, tags, random_check, page_count);
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
        let sliced_data = slice_posts(api_defs::Posts { posts }, 5);

        let (tx, rx) = channel::<Vec<DownloadFinished>>();

        // Multi-threaded implementation.
        pool.install(|| {
            let dl_size: Vec<DownloadFinished> = sliced_data
                .into_par_iter()
                .map(|posts| {
                    let low_quality = &context.lower_quality;
                    funcs::download(login, posts.to_vec(), None, low_quality)
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
    DownloadStatistics {
        completed: finished,
        failed,
        total,
        downloaded_amount: full_sum,
    }
}

pub fn download_pool(
    context: &CliContext,
    login: &Login,
    pool_id: &u64,
    zip: bool,
    cbz: bool,
) -> DownloadStatistics {
    let span = span!(Level::DEBUG, "DPool");
    let _guard = span.enter();

    let client = get_client();
    if let Some(data) = get_pool(context, &client, login, pool_id) {
        let created_dir = create_dl_dir();
        if created_dir {
            info!("Created a ./dl/ directory for all the downloaded files.")
        }
        info!("Downloading pool with id '{pool_id}' into the ./dl/ folder!");
        let mut posts_indexed: HashMap<u64, Post> = HashMap::new();
        let posts = get_post_data(context, &client, login, &data.post_ids);
        if posts.is_empty() {
            error!("Error getting post data.");
            return DownloadStatistics::default();
        }
        for (i, _) in data.post_ids.iter().enumerate() {
            let index: u64 = (i as u64) + 1;
            posts_indexed.insert(
                index,
                posts
                    .iter()
                    .find(|&p| p == &posts[i])
                    .expect("Post not found.")
                    .clone(),
            );
        }
        info!("Downloading {} posts...", data.post_ids.len());
        let mut posts_sorted = posts_indexed.into_iter().collect::<Vec<_>>();
        posts_sorted.sort_by(|a, b| a.0.cmp(&b.0));
        let mut full_sum = 0.0;
        let mut finished: i64 = 0;
        let mut failed: i64 = 0;
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(context.num_threads)
            .build()
            .unwrap();
        for post in posts_sorted {
            let (tx, rx) = channel::<DownloadFinished>();

            pool.install(|| {
                let dl_size: DownloadFinished =
                    funcs::download(login, vec![post.1], Some(&post.0), &context.lower_quality);

                tx.send(dl_size).unwrap();
            });
            let status = rx.recv().unwrap();

            finished += status.amount_finished;
            failed += status.amount_failed;
            full_sum += status.amount;
        }

        if zip && cbz {
            let p_name = data.name.replace("/", "");
            let mut zip = Command::new("7z");
            zip.arg("a").arg(format!("{}.zip", p_name)).arg("./dl/*");
            zip.output().unwrap();
            
            fs::rename(format!("./{}.zip", p_name), format!("./{}.cbz", p_name.replace("_", " "))).unwrap();
        } else if zip {
            let p_name = data.name.replace("/", "");
            let mut zip = Command::new("7z");
            zip.arg("a").arg(format!("{}.zip", p_name)).arg("./dl/*");
            zip.output().unwrap();
        }

        DownloadStatistics {
            completed: finished,
            failed,
            total: data.post_ids.len(),
            downloaded_amount: full_sum,
        }
    } else {
        DownloadStatistics::default()
    }
}
