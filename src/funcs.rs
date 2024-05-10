//! Special functions for the cli that would otherwise be reused multiple times.

use std::path::Path;

use tokio::{fs::create_dir_all, io::AsyncWriteExt};
use tokio::fs::File;

use crate::type_defs::api_defs::{LowerQuality, Post, Tags};

/// This function downloads the file with reqwest and returns the size of it in bytes.
pub async fn download(target_url: &String, file_ext: &String, post_id: u64, artist_name: &String) -> f64 {
    let bytes = reqwest::get(target_url).await.expect("Err").bytes().await.expect("Err");

    let mut out = File::create(format!("./dl/{}-{}.{}", artist_name, post_id, file_ext)).await.expect("Err");
    out.write_all(&bytes).await.expect("Err");
    out.flush().await.expect("Err");

    bytes.len() as f64
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
    if tags.artist.len() > 1 {
        let mut artists: String = String::new();
        for (_i, artist) in tags.artist.iter().enumerate() {
            artists = artists + artist + ", "
        }
        artists[..artists.len()-2].to_string()
    } else if tags.artist.len() == 1 {
        tags.artist[0].to_string()
    } else {
        "unknown-artist".to_string()
    }
}

/// Single function to parse the array of artists and make a neat single string for file naming.
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