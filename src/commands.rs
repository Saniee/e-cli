//! The module with all the command functions.

use std::path::Path;
use std::time::Duration;

use reqwest::Client;
use reqwest::{header::HeaderMap, header::HeaderValue, header::USER_AGENT};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::runtime;

use crate::funcs::{self, create_dl_dir, debug_response_file, parse_artists, slice_arr};
use crate::type_defs::api_defs::{Post, Posts};

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

/// This function accepts a file path from the args. Parses that json file and then downloads the posts. This function uses the get-pages subcommand.
pub async fn download_posts_from_file(file_path: &String, lower_quality: &bool) -> Option<f64> {
    println!("Downloading posts from a file with data.");

    let created_dir = create_dl_dir().await;
    if created_dir {
        println!("Created a ./dl/ directory for all the downloaded files.\n")
    }

    let mut dl_size: f64 = 0.0;

    let data_file_result = File::open(file_path).await;
    let mut data_file = match data_file_result {
        Ok(f) => f,
        Err(err) => {
            println!("An error occured when the program tried to open the file containing the data. Err {err}");
            return None;
        }
    };

    let mut str_data: String = String::new();
    data_file.read_to_string(&mut str_data).await.expect("Err");

    let parse_result = serde_json::from_str::<Vec<Post>>(&str_data);
    let posts = match parse_result {
        Ok(posts) => posts,
        Err(err) => {
            println!(
                "An error occured when the program tried to parse the data from the file. Err {err}"
            );
            return None;
        }
    };

    println!("Downloading {} posts.\n", posts.len());

    for post in posts {
        let artist_name = parse_artists(&post.tags);

        let path_string = format!("./dl/{}-{}.{}", artist_name, post.id, post.file.ext);
        let path = Path::new(&path_string);

        println!(
            "Starting download of {}-{}.{}",
            artist_name, post.id, post.file.ext
        );

        if !path.exists() {
            let file_size: f64;
            if *lower_quality {
                file_size = funcs::lower_quality_dl_file(&post, &artist_name).await
            } else {
                match &post.file.url {
                    Some(url) => {
                        file_size =
                            funcs::download_file(url, &post.file.ext, post.id, &artist_name).await
                    }
                    None => {
                        println!(
                            "Cannot download post {}-{} due to it missing a file url",
                            artist_name, post.id
                        );
                        file_size = 0.0;
                    }
                }
            }

            println!(
                "Downloaded {}-{}.{}! File size: {:.2} MB\n",
                artist_name,
                post.id,
                post.file.ext,
                file_size / 1024.0 / 1024.0
            );

            dl_size += file_size
        } else {
            println!(
                "File {}-{}.{} already Exists!\n",
                artist_name, post.id, post.file.ext
            )
        }
    }

    if dl_size > 0.0 {
        Some(dl_size)
    } else {
        None
    }
}

/// This function takes tags and the page count and fetches posts. Then it saves them into a file named posts.json in the root dir.
pub async fn fetch_posts(tags: &String, count: &u8, api_source: &String) {
    println!(
        "Getting {count} pages and putting the id's into a json file."
    );

    let client = Client::builder();
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("rust-powered-post-download/0.1"),
    );
    let client = client.default_headers(headers).build().unwrap();

    let json_file_result = File::create("./posts.json").await;
    let mut json_file = match json_file_result {
        Ok(f) => f,
        Err(err) => {
            println!("Cannot continue due to an err. {err}");
            return;
        }
    };

    let mut page: u8 = 0;
    let mut posts = Vec::new();
    while &page < count {
        let target: String = format!(
            "https://{}/posts.json?tags={}&limit=250&page={}",
            api_source,
            tags,
            page + 1
        );

        let data_result = client
            .get(&target)
            .send()
            .await
            .expect("Err")
            .json::<Posts>()
            .await;

        let data = match data_result {
            Ok(data) => data,
            Err(err) => {
                println!(
                    "Parsing data failed. Creating a debug json file with all of it. Err: {err}"
                );
                let response = client
                    .get(&target)
                    .send()
                    .await
                    .expect("Err")
                    .text()
                    .await
                    .expect("Err");
                debug_response_file(&response).await;
                return;
            }
        };

        for post in data.posts {
            posts.push(post)
        }

        page += 1;
    }

    let json = serde_json::to_string(&posts).unwrap();
    json_file.write_all(json.as_bytes()).await.expect("Err");
    json_file.flush().await.expect("Err");
}

/// Reads a txt file, parses it into a list and then iters over it to download each post.
pub async fn download_posts_from_txt(
    file_path: &String,
    api_source: &String,
    lower_quality: &bool,
) -> Option<f64> {
    let txt_file_path = Path::new(file_path);
    let txt_file_result = File::open(txt_file_path).await;

    let mut dl_size: f64 = 0.0;

    let txt_file = match txt_file_result {
        Ok(file) => file,
        Err(err) => {
            println!("Err {err}");
            panic!("{}", err)
        }
    };

    let created_dir = create_dl_dir().await;
    if created_dir {
        println!("Created a ./dl/ directory for all the downloaded files.\n")
    }

    let mut reader = BufReader::new(txt_file);
    let mut txt_file_contents = String::new();
    let _ = reader.read_to_string(&mut txt_file_contents).await;
    let id_list: Vec<&str> = txt_file_contents.trim().split('\n').collect();

    if id_list.len() > 15 {
        println!("Cannot have more then 15 Id's in the file. Exiting...");
        return None;
    }

    let client = Client::builder();
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("rust-powered-post-download/0.1"),
    );
    let client = client.default_headers(headers).build().unwrap();

    for id in id_list.iter() {
        let target = format!("https://{api_source}/posts.json?tags=id:{id}");

        let posts = client
            .get(&target)
            .send()
            .await
            .expect("Err")
            .json::<Posts>()
            .await
            .expect("Err");
        let post = &posts.posts[0];

        let artist_name = parse_artists(&post.tags);

        let path_string = format!("./dl/{}-{}.{}", artist_name, post.id, post.file.ext);
        let path = Path::new(&path_string);

        println!(
            "Starting download of {}-{}.{}",
            artist_name, post.id, post.file.ext
        );

        if !path.exists() {
            let file_size: f64;
            if *lower_quality {
                file_size = funcs::lower_quality_dl_file(post, &artist_name).await
            } else {
                match &post.file.url {
                    Some(url) => {
                        file_size =
                            funcs::download_file(url, &post.file.ext, post.id, &artist_name).await
                    }
                    None => {
                        println!(
                            "Cannot download post {}-{} due to it missing a file url",
                            artist_name, post.id
                        );
                        file_size = 0.0;
                    }
                }
            }

            println!(
                "Downloaded {}-{}.{}! File size: {:.2} MB\n",
                artist_name,
                post.id,
                post.file.ext,
                file_size / 1024.0 / 1024.0
            );

            dl_size += file_size
        } else {
            println!(
                "File {}-{}.{} already Exists!\n",
                artist_name, post.id, post.file.ext
            )
        }
    }

    if dl_size > 0.0 {
        Some(dl_size)
    } else {
        None
    }
}
