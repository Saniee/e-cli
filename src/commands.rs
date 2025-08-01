//! The module with all the command functions.

use std::sync::mpsc::channel;
use std::time::Duration;

use ureq::Agent;

use rayon::prelude::*;

use crate::funcs::{self, create_dl_dir, slice_arr};
use crate::type_defs::api_defs::Posts;

/// This function takes the arguments of [crate::cli::Commands::DownloadFavourites] and uses them to download the specified amount of media.
pub fn download_favourites(
    username: &str,
    count: &u8,
    random: &bool,
    tags: &str,
    lower_quality: &bool,
    api_source: &str,
    num_threads: usize
) -> f64 {
    // println!("{}", args.random);
    println!(
        "Downloading {count} Favorites of {username} into the ./dl/ folder!\n"
    );

    let config = Agent::config_builder().user_agent("e-cli/0.2.0").https_only(true).timeout_global(Some(Duration::from_secs(5))).build();

    let client: Agent = config.into();

    let random_check: &str = if *random { "order:random" } else { "" };

    let tags: &str = if !tags.is_empty() { tags } else { "" };

    let target: String = format!(
        "https://{api_source}/posts.json?tags=fav:{username}+{tags}+{random_check}&limit={count}"
    );

    let data: Posts = client
        .get(target)
        .call()
        .expect("Error getting response!")
        .body_mut()
        .read_json::<Posts>()
        .expect("Error reading response json!");

    if data.posts.is_empty() {
        println!("No posts found...");
        return 0.0;
    }

    let created_dir = create_dl_dir();
    if created_dir {
        println!("Created a ./dl/ directory for all the downloaded files.\n")
    }

    let sliced_data = slice_arr(data, 5);

    let pool = rayon::ThreadPoolBuilder::new().num_threads(num_threads).build().unwrap();

    let (tx, rx) = channel::<Vec<f64>>();

    // Multi-threaded implementation.
    pool.install(|| {
        let dl_size: Vec<f64> = sliced_data.into_par_iter().map(|posts| {
            #[allow(clippy::clone_on_copy)]
            let low_quality = lower_quality.clone();
            funcs::download(posts.to_vec(), low_quality)
        }).collect();

        tx.send(dl_size).unwrap();
    });

    rx.recv().unwrap().iter().sum()
}