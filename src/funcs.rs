use std::path::Path;

use std::fs::File;
use std::{fs::create_dir_all, io::Write};

use ureq::http::header;

use crate::type_defs::api_defs::{Post, Posts};

pub fn download(data: Vec<Post>, lower_quality: bool) -> f64 {
    let mut dl_size = 0.0;

    for post in data {
        let artist_name = post.tags.parse_artists();

        let path_string = format!("./dl/{}-{}.{}", artist_name, post.id, post.file.ext);
        let path = Path::new(&path_string);

        // println!("Starting download of {}-{}.{}",artist_name, post.id, post.file.ext);

        if !path.exists() {
            let file_size: f64;
            if lower_quality {
                file_size = lower_quality_dl_file(&post, &artist_name);
                dl_size += file_size;
            } else {
                match &post.file.url {
                    Some(url) => {
                        file_size =
                            download_file(url, &post.file.ext, post.id, &artist_name);
                        dl_size += file_size;
                    }
                    None => {
                        println!(
                            "Cannot download post {}-{} due to it missing a file url",
                            artist_name, post.id
                        );
                        file_size = 0.0;
                        dl_size += file_size;
                    }
                }
            }

            println!(
                "Downloaded {}-{}.{}! File size: {:.2} MB",
                artist_name,
                post.id,
                post.file.ext,
                file_size / 1024.0 / 1024.0
            );
        } else {
            println!(
                "File {}-{}.{} already Exists!",
                artist_name, post.id, post.file.ext
            )
        }
    };
    
    dl_size
}

pub fn download_file(
    target_url: &str,
    file_ext: &str,
    post_id: u64,
    artist_name: &str,
) -> f64 {
    let mut res = ureq::get(target_url).header(header::USER_AGENT, "e-cli/0.2.0").call().expect("Error getting remote file response!");
    let mut out = File::create(format!("./dl/{artist_name}-{post_id}.{file_ext}")).expect("Error creating file!");
    let bytes: f64 = res.body().content_length().expect("Error getting content length!") as f64;

    std::io::copy(&mut res.body_mut().as_reader(), &mut out).expect("Error writing file!");

    out.flush().expect("Err");

    bytes
}

pub fn lower_quality_dl_file(post: &Post, artist_name: &str) -> f64 {
    if post.sample.has {
        if let Some(lower_quality) = &post.sample.alternates.lower_quality {
            if lower_quality.media_type == "video" {
                download_file(&lower_quality.urls[0], &post.file.ext, post.id, artist_name)
            } else if let Some(sample_url) = &post.sample.url {
                download_file(sample_url, &post.file.ext, post.id, artist_name)
            } else {
                println!(
                    "Cannot download post {}-{} due it not having any file url.",
                    artist_name, &post.id
                );
                0.0
            }
        } else if let Some(sample_url) = &post.sample.url {
            download_file(sample_url, &post.file.ext, post.id, artist_name)
        } else {
            println!(
                "Cannot download post {}-{} due it not having any file url.",
                artist_name, &post.id
            );
            0.0
        }
    } else if let Some(url) = &post.file.url {
        download_file(url, &post.file.ext, post.id, artist_name)
    } else {
        println!(
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
