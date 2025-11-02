use std::sync::mpsc::channel;

use reqwest::blocking::Client;

use rayon::prelude::*;
use tracing::{debug, error, info};

use crate::AGENT;
use crate::funcs::{self, create_dl_dir, get_pages, slice_arr, sum_posts};
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
    username: &str,
    count: &u32,
    num_pages: &i64,
    random: &bool,
    tags: &str,
    lower_quality: &bool,
    api_source: &str,
    num_threads: usize,
) -> f64 {
    info!("Downloading Favorites of {username} into the ./dl/ folder!");

    let client = get_client();

    let random_check: &str = if *random { "order:random" } else { "" };

    let tags: &str = if !tags.is_empty() { tags } else { "" };
    let fav: String = format!("fav:{}", username);

    info!("Getting posts from pages!");
    let data: Vec<Vec<Post>> = get_pages(
        api_source,
        client,
        num_pages,
        &fav,
        tags,
        random_check,
        count,
    );

    if data.is_empty() {
        error!("No posts found...");
        return 0.0;
    }

    let created_dir = create_dl_dir();
    if created_dir {
        info!("Created a ./dl/ directory for all the downloaded files.")
    }
    info!("Downloading {} posts...", sum_posts(&data));
    let mut full_sum = 0.0;
    let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .unwrap();
    for posts in data {
        let sliced_data = slice_arr(api_defs::Posts { posts }, 5);

        let (tx, rx) = channel::<Vec<f64>>();

        // Multi-threaded implementation.
        pool.install(|| {
            debug!("Starting download of {} posts.", sliced_data.len());
            let dl_size: Vec<f64> = sliced_data
                .into_par_iter()
                .map(|posts| {
                    let low_quality = lower_quality;
                    funcs::download(posts.to_vec(), low_quality)
                })
                .collect();

            tx.send(dl_size).unwrap();
        });

        full_sum += rx.recv().unwrap().iter().sum::<f64>();
    }

    full_sum
}

pub fn download_search(
    tags: &str,
    count: &u32,
    num_pages: &i64,
    random: &bool,
    lower_quality: &bool,
    api_source: &str,
    num_threads: usize,
) -> f64 {
    info!("Downloading posts, with '{tags}' tags, into the ./dl/ folder!");
    let client = get_client();
    let random_check: &str = if *random { "order:random" } else { "" };
    let tags: &str = if !tags.is_empty() { tags } else { "" };
    let fav = "";
    info!("Getting posts from pages!");
    let data: Vec<Vec<Post>> = get_pages(
        api_source,
        client,
        num_pages,
        fav,
        tags,
        random_check,
        count,
    );
    if data.is_empty() {
        error!("No posts found...");
        return 0.0;
    }
    let created_dir = create_dl_dir();
    if created_dir {
        info!("Created a ./dl/ directory for all the downloaded files.")
    }
    info!("Downloading {} posts...", sum_posts(&data));
    let mut full_sum: f64 = 0.0;

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .unwrap();
    for posts in data {
        let sliced_data = slice_arr(api_defs::Posts { posts }, 5);

        let (tx, rx) = channel::<Vec<f64>>();

        // Multi-threaded implementation.
        pool.install(|| {
            let dl_size: Vec<f64> = sliced_data
                .into_par_iter()
                .map(|posts| {
                    let low_quality = lower_quality;
                    funcs::download(posts.to_vec(), low_quality)
                })
                .collect();

            tx.send(dl_size).unwrap();
        });

        full_sum += rx.recv().unwrap().iter().sum::<f64>();
    }
    full_sum
}
