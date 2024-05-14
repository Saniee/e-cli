//! Special functions for the cli that would otherwise be reused multiple times.

use std::cmp::Ordering;
use std::path::Path;

use indicatif::{ProgressBar, ProgressStyle};
use tokio::{fs::create_dir_all, io::AsyncWriteExt};
use tokio::fs::File;

use crate::type_defs::api_defs::{LowerQuality, Post, Tags};

/// This function downloads the file with reqwest and returns the size of it in bytes.
pub async fn download(target_url: &String, file_ext: &String, post_id: u64, artist_name: &String) -> f64 {
    let mut res = reqwest::get(target_url).await.expect("Err");
    let content_length = res.content_length().unwrap();
    let mut out = File::create(format!("./dl/{}-{}.{}", artist_name, post_id, file_ext)).await.expect("Err");
    let mut bytes: usize = 0;

    let pb = ProgressBar::new(content_length);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{bar:.green/red}] {bytes}/{total_bytes} ({bytes_per_sec}), {eta} {msg}")
    .unwrap()
    .progress_chars("#>-"));

    while let Some(chunk) = res.chunk().await.unwrap_or(None) {
        bytes += out.write(&chunk).await.unwrap();
        pb.set_position(bytes as u64)
    }

    out.flush().await.expect("Err");
    pb.finish_with_message("Done");

    bytes as f64
}

/// This function uses the [fn@download] function to download parsed lower quality versions of the posts.
pub async fn lower_quality_dl(post: &Post, artist_name: &String) -> f64 {
    println!("Trying to download lower quality media...");
    if post.sample.has {
        match &post.sample.alternates.lower_quality {
            Some(LowerQuality { media_type, urls }) => {
                if media_type == "video" {
                    download(&urls[0], &post.file.ext, post.id, artist_name).await
                } else {
                    match &post.sample.url {
                        Some(url) => {
                            download(url, &post.file.ext, post.id, artist_name).await
                        }
                        None => {
                            match &post.file.url {
                                Some(url) => {
                                    download(url, &post.file.ext, post.id, artist_name).await
                                }
                                None => {
                                    println!("Cannot download post {}-{} due it not having any file url.", artist_name, &post.id);
                                    0.0
                                }
                            }
                        }
                    }
                }
            }
            None => {
                match &post.sample.url {
                    Some(url) => {
                        download(url, &post.file.ext, post.id, artist_name).await
                    }
                    None => {
                        match &post.file.url {
                            Some(url) => {
                                download(url, &post.file.ext, post.id, artist_name).await
                            }
                            None => {
                                println!("Cannot download post {}-{} due it not having any file url.", artist_name, &post.id);
                                0.0
                            }
                        }
                    }
                }
            }
        }
    } else {
        match &post.file.url {
            Some(url) => {
                download(url, &post.file.ext, post.id, artist_name).await
            }
            None => {
                println!("Cannot download post {}-{} due it not having any file url.", artist_name, &post.id);
                0.0
            }
        }
    }
}

/// This function parses the artists and returns a neatly formatted string.
/// 
/// Example Output
/// 
/// `name, name`
pub fn parse_artists(tags: &Tags) -> String {
    match tags.artist.len().cmp(&1) {
        Ordering::Greater => {
            let mut artists: String = String::new();
            for artist in tags.artist.iter() {
                artists = artists + artist + ", "
            }
            artists[..artists.len()-2].to_string()
        }
        Ordering::Equal => {
            tags.artist[0].to_string()
        }
        Ordering::Less => {
            "unknown-artist".to_string()
        }
    }

    /* if tags.artist.len() > 1 {
        let mut artists: String = String::new();
        for artist in tags.artist.iter() {
            artists = artists + artist + ", "
        }
        artists[..artists.len()-2].to_string()
    } else if tags.artist.len() == 1 {
        tags.artist[0].to_string()
    } else {
        "unknown-artist".to_string()
    } */
}

/// Single function to create the ./dl/ dir for all media downloaded by this tool.
pub async fn create_dl_dir() -> bool {
    let dir_path = Path::new("./dl/");
    if !dir_path.exists() {
        create_dir_all("./dl/").await.expect("Err");
        true
    } else {
        false
    }
}

/// Used for debugging in the get-pages subcommand. It saves the response body for disecting later.
pub async fn debug_response_file(response_body: &String) {
    let file_path = Path::new("./debug.json");
    let file_result = File::create(file_path).await;
    let mut file = match file_result {
        Ok(f) => {f}
        Err(_err) => {
            return
        }
    };

    file.write_all(response_body.as_bytes()).await.expect("Err");
    file.flush().await.expect("Err");
}