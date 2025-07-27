//! The module with all the command functions.

use reqwest::Client;
use reqwest::{header::HeaderMap, header::HeaderValue, header::USER_AGENT};

use crate::funcs::{self, create_dl_dir, slice_arr};
use crate::type_defs::api_defs::Posts;

/// This function takes the arguments of [crate::cli::Commands::DownloadFavourites] and uses them to download the specified amount of media.
pub async fn download_favourites(
    username: &str,
    count: &u8,
    random: &bool,
    tags: &str,
    lower_quality: &bool,
    api_source: &str,
) -> Option<f64> {
    // println!("{}", args.random);
    println!(
        "Downloading {count} Favorites of {username} into the ./dl/ folder!\n"
    );

    let client = Client::builder();
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("rust-powered-post-download/0.1"),
    );
    let client = client.default_headers(headers).build().unwrap();

    let random_check: &str = if *random { "order:random" } else { "" };

    let tags: &str = if !tags.is_empty() { tags } else { "" };

    let target: String = format!(
        "https://{api_source}/posts.json?tags=fav:{username} {tags} {random_check}&limit={count}"
    );

    let data: Posts = client
        .get(target)
        .send()
        .await
        .expect("Err")
        .json::<Posts>()
        .await
        .expect("Err");

    if data.posts.is_empty() {
        println!("No post found...");
        return None;
    }

    let created_dir = create_dl_dir().await;
    if created_dir {
        println!("Created a ./dl/ directory for all the downloaded files.\n")
    }

    #[allow(unused_mut)]
    let mut dl_size: f64 = 0.0;
    let sliced_data = slice_arr(data, 5);
    

    for posts in sliced_data.iter() {
        let _ = tokio::spawn(funcs::download(posts.clone(), *lower_quality)).await;
    };

    if dl_size > 0.0 {
        Some(dl_size)
    } else {
        None
    }
}