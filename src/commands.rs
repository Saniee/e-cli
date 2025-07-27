//! The module with all the command functions.

use std::path::Path;

use reqwest::Client;
use reqwest::{header::HeaderMap, header::HeaderValue, header::USER_AGENT};

use crate::funcs::{self, create_dl_dir, parse_artists, slice_arr};
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

/// This function takes the arguments of [crate::cli::Commands::DownloadPost] and uses them to download a single post with the specified ID.
pub async fn download_post(
    post_id: &u64,
    lower_quality: &bool,
    api_source: &String,
) -> Option<f64> {
    let client = Client::builder();
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("rust-powered-post-download/0.1"),
    );
    let client = client.default_headers(headers).build().unwrap();

    let target: String = format!("https://{api_source}/posts.json?tags=id:{post_id}");

    let data = client
        .get(target)
        .send()
        .await
        .expect("Err")
        .json::<Posts>()
        .await
        .expect("Couldn't get json.");

    let created_dir = create_dl_dir().await;
    if created_dir {
        println!("Created a ./dl/ directory for all the downloaded files.\n")
    }

    if data.posts.is_empty() {
        println!("No post found...");
        return None;
    }

    let artist_name = parse_artists(&data.posts[0].tags);

    let path_string = format!(
        "./dl/{}-{}.{}",
        artist_name, data.posts[0].id, data.posts[0].file.ext
    );
    let path = Path::new(&path_string);

    println!(
        "Starting download of {}-{}.{}",
        artist_name, data.posts[0].id, data.posts[0].file.ext
    );

    if !path.exists() {
        let file_size: f64;
        if *lower_quality {
            file_size = funcs::lower_quality_dl_file(&data.posts[0], &artist_name).await;
        } else {
            match &data.posts[0].file.url {
                Some(url) => {
                    file_size = funcs::download_file(
                        url,
                        &data.posts[0].file.ext,
                        data.posts[0].id,
                        &artist_name,
                    )
                    .await;
                }
                None => {
                    println!(
                        "Cannot download post {}-{} due to it missing a file url",
                        artist_name, data.posts[0].id
                    );
                    file_size = 0.0;
                }
            }
        }
        println!(
            "Downloaded {}-{}.{}! File size: {:.2} KB\n",
            artist_name,
            data.posts[0].id,
            data.posts[0].file.ext,
            file_size / 1024.0
        );
        Some(file_size)
    } else {
        println!(
            "File {}-{}.{} already Exists!\n",
            artist_name, data.posts[0].id, data.posts[0].file.ext
        );
        None
    }
}
