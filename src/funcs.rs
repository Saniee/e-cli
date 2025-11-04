use std::path::Path;

use std::fs::File;
use std::{fs::create_dir_all, io::Write};

use reqwest::blocking::{Client, Response};
use tracing::{Level, debug, error, info, span, warn};

use crate::{CliContext, Login};
use crate::commands::{get_client};
use crate::type_defs::api_defs::{Post, Posts};

pub fn sum_posts(data: &Vec<Vec<Post>>) -> usize {
    let mut sum = 0;
    for posts in data {
        sum += posts.len();
    }
    sum
}

pub fn download(login: &Login, data: Vec<Post>, lower_quality: &bool) -> f64 {
    let span = span!(Level::DEBUG, "download_handler");
    let _guard = span.enter();
    
    let mut dl_size = 0.0;

    for post in data {
        let artist_name = post.tags.parse_artists();

        let path_string = format!("./dl/{}-{}.{}", artist_name, post.id, post.file.ext);
        let path = Path::new(&path_string);
        debug!("{path:?}");

        if path.exists() {
            warn!(
                "File {}-{}.{} already Exists!",
                artist_name, post.id, post.file.ext
            )
        }

        let file_size: f64;
        if *lower_quality {
            file_size = lower_quality_dl_file(login, &post, &artist_name);
            dl_size += file_size;
        } else {
            match &post.file.url {
                Some(url) => {
                    file_size = download_file(login, url, &post.file.ext, post.id, &artist_name);
                    dl_size += file_size;
                }
                None => {
                    warn!(
                        "Cannot download post {}-{} due to it missing a file url",
                        artist_name, post.id
                    );
                    file_size = 0.0;
                    dl_size += file_size;
                }
            }
        }

        info!(
            "Downloaded {}-{}.{}! File size: {:.2} MB",
            artist_name,
            post.id,
            post.file.ext,
            file_size / 1024.0 / 1024.0
        );
    }

    dl_size
}

pub fn download_file(login: &Login, target_url: &str, file_ext: &str, post_id: u64, artist_name: &str) -> f64 {
    let span = span!(Level::DEBUG, "file_download");
    let _guard = span.enter();

    let client = get_client();
    let res = send_request(&client, login, target_url);
    debug!("{res:?}");
    let mut out = File::create(format!("./dl/{artist_name}-{post_id}.{file_ext}"))
        .expect("Error creating file!");
    let byte_size: f64 = res.content_length().expect("Error getting byte amount!") as f64;
    let bytes = res.bytes().expect("Error getting file bytes!").to_vec();

    let _ = std::io::copy(&mut &bytes[..], &mut out);

    out.flush().expect("Err");

    byte_size
}

pub fn lower_quality_dl_file(login: &Login, post: &Post, artist_name: &str) -> f64 {
    if !post.sample.has {
        info!(
            "Cannot download post {}-{} due it not having any file url.",
            artist_name, &post.id
        );
        return 0.0;
    } else if let Some(url) = &post.file.url {
        return download_file(login, url, &post.file.ext, post.id, artist_name);
    }

    if let Some(lower_quality) = &post.sample.alternates.lower_quality {
        if lower_quality.media_type == "video" {
            download_file(login, &lower_quality.urls[0], &post.file.ext, post.id, artist_name)
        } else if let Some(sample_url) = &post.sample.url {
            download_file(login, sample_url, &post.file.ext, post.id, artist_name)
        } else {
            warn!(
                "Cannot download post {}-{} due it not having any file url.",
                artist_name, &post.id
            );
            0.0
        }
    } else if let Some(sample_url) = &post.sample.url {
        download_file(login, sample_url, &post.file.ext, post.id, artist_name)
    } else {
        warn!(
            "Cannot download post {}-{} due it not having any file url.",
            artist_name, &post.id
        );
        0.0
    }
}

pub fn create_dl_dir() -> bool {
    let dir_path = Path::new("./dl/");
    if !dir_path.exists() {
        create_dir_all("./dl/").expect("Error creating ./dl/ directory!");
        true
    } else {
        false
    }
}

pub fn slice_arr(arr: Posts, chunk_size: i32) -> Vec<Vec<Post>> {
    let mut res: Vec<Vec<Post>> = Vec::new();
    let posts = arr.posts;
    let slices = posts.chunks(chunk_size as usize);
    for slice in slices {
        res.push(slice.to_vec());
    }
    res
}

pub fn get_pages(
    context: &CliContext,
    login: &Login,
    client: Client,
    fav: &str,
    tags: &str,
    random: &str,
    count: &u32,
) -> Vec<Vec<Post>> {
    let mut pages = 0;
    let mut posts: Vec<Vec<Post>> = vec![];

    let span = span!(Level::DEBUG, "get_pages");
    let _guard = span.enter();

    if context.pages == -1 {
        loop {
            let target: String = format!(
                "https://{}/posts.json?tags={} {} {}&limit={}&page={}",
                context.api_source,
                fav,
                tags,
                random,
                count,
                pages + 1
            );
            debug!(target);

            let res = send_request(&client, login, &target);
            if let Err(e) = res.error_for_status_ref() {
                error!("Response returned: {}", e);
                break;
            }
            let data = res.json::<Posts>().expect("Error reading response json.");

            if data.posts.is_empty() {
                break;
            }

            posts.push(data.posts);
            pages += 1;
        }
    } else if context.pages > 0 {
        loop {
            if pages == context.pages {
                break;
            }

            let target: String = format!(
                "https://{}/posts.json?tags={} {} {}&limit={}&page={}",
                context.api_source,
                fav,
                tags,
                random,
                count,
                pages + 1
            );

            let res = send_request(&client, login, &target);
            if let Err(e) = res.error_for_status_ref() {
                error!("Response returned: {}", e);
                break;
            }
            let data = res.json::<Posts>().expect("Error reading response json.");

            if data.posts.is_empty() {
                break;
            }

            posts.push(data.posts);
            pages += 1;
        }
    }

    posts
}

pub fn send_request(client: &Client, login: &Login, target: &str) -> Response {
    if !login.username.is_empty() && !login.api_key.is_empty() {
        client.get(target).basic_auth(login.username.clone(), Some(login.api_key.clone())).send().expect("Error getting response!")
    } else {
        client.get(target).send().expect("Error getting response!")
    }
}
