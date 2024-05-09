//! The module with all the command functions.

use std::path::Path;

use reqwest::Client;
use reqwest::{header::HeaderMap, header::HeaderValue, header::USER_AGENT};
use tokio::fs::create_dir_all;

use crate::type_defs::api_defs::Posts;
use crate::funcs::{self, parse_artists};

/// This function takes the arguments of [crate::cli::Commands::DownloadFavourites] and uses them to download the specified amount of media.
pub async fn download_favourites(username: &String, count: &u8, random: &bool, tags: &String, lower_quality: &bool) -> Option<f64> {
    // println!("{}", args.random);
    println!("Downloading {} Favorites of {} into the ./dl/ folder!\n", count, username);

    let client = Client::builder();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("rust-powered-post-download/0.1"));
    let client = client.default_headers(headers).build().unwrap();

    let random_check: &str = if *random {
        "order:random"
    } else {
        ""
    };

    let tags: &str = if tags != "" {
        &tags
    } else {
        ""
    };

    let target: String = format!("https://e621.net/posts.json?tags=fav:{} {} {}&limit={}", username, tags, random_check , count.to_string());

    let data: Posts  = client.get(target).send().await.expect("Err").json::<Posts>().await.expect("Err");

    println!("Creating a ./dl/ directory if it doesn't exist...\n");
    create_dir_all("./dl/").await.expect("Err");


    let mut dl_size: f64 = 0.0;
    for post in data.posts {
        let artist_name = parse_artists(&post.tags);

        let path_string = format!("./dl/{}-{}.{}", artist_name, post.id, post.file.ext);
        let path = Path::new(&path_string);

        println!("Starting download of {}-{}.{}", artist_name, post.id, post.file.ext);

        if !path.exists() {
            let file_size: f64;
            if *lower_quality {
                file_size = funcs::lower_quality_dl(&post, &artist_name).await
            } else {
                file_size = funcs::download(&post.file.url, &post.file.ext, post.id, &artist_name).await
            }

            println!("Downloaded {}-{}.{}! File size: {:.2} MB\n", artist_name, post.id, post.file.ext, file_size/1024.0/1024.0);

            dl_size += file_size
        } else {
            println!("File {}-{}.{} already Exists!\n", artist_name, post.id, post.file.ext)
        }
    }

    if dl_size > 0.0 {
        Some(dl_size)
    } else {
        None
    }
}

/// Thus function takes the arguments of [crate::cli::Commands::DownloadPost] and uses them to download a single post with the specified ID.
pub async fn download_post(post_id: &u64, lower_quality: &bool) -> Option<f64> {
    println!("Downloading Post: {}\n", post_id.to_string());

    let client = Client::builder();
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("rust-powered-post-download/0.1"));
    let client = client.default_headers(headers).build().unwrap();

    let target: String = format!("https://e621.net/posts.json?tags=id:{}", post_id.to_string());

    let data = client.get(target).send().await.expect("Err").json::<Posts>().await.expect("Couldn't get json.");

    println!("Creating a ./dl/ directory if it doesn't exist...\n");
    create_dir_all("./dl/").await.expect("Err");

    let artist_name = parse_artists(&data.posts[0].tags);

    let path_string = format!("./dl/{}-{}.{}", artist_name, data.posts[0].id, data.posts[0].file.ext);
    let path = Path::new(&path_string);

    println!("{}", path.to_string_lossy());

    println!("Starting download of {}-{}.{}", artist_name, data.posts[0].id, data.posts[0].file.ext);

    if !path.exists() {
        let file_size: f64;
        if *lower_quality {
            println!("Trying to download lower quality file.");
            file_size = funcs::lower_quality_dl(&data.posts[0], &artist_name).await
        } else {
            file_size = funcs::download(&data.posts[0].file.url, &data.posts[0].file.ext, data.posts[0].id, &artist_name).await;
        }
        println!("Downloaded {}-{}.{}! File size: {:.2} KB\n", artist_name, data.posts[0].id, data.posts[0].file.ext, file_size/1024.0);
        Some(file_size)
    } else {
        println!("File {}-{}.{} already Exists!\n", artist_name, data.posts[0].id, data.posts[0].file.ext);
        None
    }
}