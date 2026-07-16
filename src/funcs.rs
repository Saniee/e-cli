use std::path::Path;

use std::fs::File;
use std::{fs::create_dir_all, io::Write};

use reqwest::blocking::{Client, Response};
use tracing::{Level, debug, error, info, span, warn};

use crate::type_defs::api_defs::{PoolData, Post, Posts};
use crate::{CliContext, Login};

/// Total number of posts across all pages in `data` (i.e. the flattened count,
/// as returned by [`get_pages`]).
pub fn sum_posts(data: &Vec<Vec<Post>>) -> usize {
    let mut sum = 0;
    for posts in data {
        sum += posts.len();
    }
    sum
}

fn file_name(index: Option<&u64>, artist_name: &str, post_id: u64, file_ext: &str) -> String {
    match index {
        Some(i) => format!("{i:04}-{artist_name}-{post_id}.{file_ext}"),
        None => format!("{artist_name}-{post_id}.{file_ext}"),
    }
}

#[derive(Default)]
pub struct DownloadStatus {
    pub finished: bool,
    pub downloaded_bytes: f64,
}

pub struct DownloadFinished {
    pub amount_finished: i64,
    pub amount_failed: i64,
    pub amount: f64,
}

/// Downloads a batch of posts into `output_dir`, skipping (and logging a warning
/// for) any post whose target file already exists on disk.
///
/// `index` is applied to *every* post in `data` as a shared filename prefix (or
/// `None` for no prefix) — for per-post distinct indexes (as pool downloads need),
/// call this once per post with that post's own index rather than passing a
/// multi-post batch. If `lower_quality` is true, [`lower_quality_dl_file`] is used
/// instead of the full-resolution [`download_file`] where a sample/lower-quality
/// variant is available.
#[allow(clippy::too_many_arguments)]
pub fn download(
    client: &Client,
    login: &Login,
    data: Vec<Post>,
    index: Option<&u64>,
    lower_quality: &bool,
    output_dir: &Path,
) -> DownloadFinished {
    let span = span!(Level::DEBUG, "download_handler");
    let _guard = span.enter();

    let mut downloaded_bytes = 0.0;
    let mut amount_finished = 0;
    let mut amount_failed = 0;

    for post in data {
        let artist_name = post.tags.parse_artists();

        let path = output_dir.join(file_name(index, &artist_name, post.id, &post.file.ext));

        if path.exists() {
            warn!(
                "File {}-{}.{} already Exists!",
                artist_name, post.id, post.file.ext
            );
            continue;
        }

        if *lower_quality {
            let stat = lower_quality_dl_file(client, login, &post, &artist_name, index, output_dir);
            if stat.finished {
                downloaded_bytes += stat.downloaded_bytes;
                amount_finished += 1;
                info!(
                    "Downloaded {}-{}.{}! File size: {:.2} MB",
                    artist_name,
                    post.id,
                    post.file.ext,
                    stat.downloaded_bytes / 1024.0 / 1024.0
                );
            } else {
                amount_failed += 1;
                warn!("Failed to download {}-{}.{}", artist_name, post.id, post.file.ext);
            }
        } else {
            match &post.file.url {
                Some(url) => {
                    let stat = download_file(
                        client,
                        login,
                        url,
                        &post.file.ext,
                        post.id,
                        &artist_name,
                        index,
                        output_dir,
                    );
                    if stat.finished {
                        downloaded_bytes += stat.downloaded_bytes;
                        amount_finished += 1;
                        info!(
                            "Downloaded {}-{}.{}! File size: {:.2} MB",
                            artist_name,
                            post.id,
                            post.file.ext,
                            stat.downloaded_bytes / 1024.0 / 1024.0
                        );
                    } else {
                        amount_failed += 1;
                        warn!("Failed to download {}-{}.{}", artist_name, post.id, post.file.ext);
                    }
                }
                None => {
                    warn!(
                        "Cannot download post {}-{} due to it missing a file url",
                        artist_name, post.id
                    );
                    amount_failed += 1;
                }
            }
        }
    }

    DownloadFinished {
        amount_finished,
        amount_failed,
        amount: downloaded_bytes,
    }
}

/// Streams `target_url`'s response body directly to a file in `output_dir`
/// (via `Response::copy_to`, so the whole file is never buffered in memory),
/// named `{index-}{artist_name}-{post_id}.{file_ext}` (zero-padded 4-digit
/// index prefix if `Some`). `downloaded_bytes` on success reflects the actual
/// bytes written, not a trusted `Content-Length` header. Returns a
/// `DownloadStatus` with `finished: false` if the request, file creation, or
/// the copy itself fails (the failure is logged; this function does not panic
/// on network/IO errors).
#[allow(clippy::too_many_arguments)]
pub fn download_file(
    client: &Client,
    login: &Login,
    target_url: &str,
    file_ext: &str,
    post_id: u64,
    artist_name: &str,
    index: Option<&u64>,
    output_dir: &Path,
) -> DownloadStatus {
    let span = span!(Level::DEBUG, "file_download");
    let _guard = span.enter();

    let mut res = send_request(client, login, target_url);
    debug!(status = %res.status(), content_length = ?res.content_length(), "response received");
    let name = file_name(index, artist_name, post_id, file_ext);
    let mut out = match File::create(output_dir.join(name)) {
        Ok(o) => o,
        Err(_) => {
            return DownloadStatus::default();
        }
    };

    match res.copy_to(&mut out) {
        Ok(written) => {
            out.flush().expect("Err");
            DownloadStatus {
                finished: true,
                downloaded_bytes: written as f64,
            }
        }
        Err(e) => {
            warn!("Failed to stream {artist_name}-{post_id}.{file_ext}: {e}");
            DownloadStatus::default()
        }
    }
}

/// Downloads a lower-quality variant of `post`, for use when `--lower-quality`
/// is set. Precedence, preferring an actually-lower-quality source first:
/// the sample's 480p video alternate, then the sample image/thumbnail URL,
/// and only if neither exists does this fall back to the full-resolution
/// `post.file.url`. Returns a default (`finished: false`) `DownloadStatus` if
/// none of those are available.
#[allow(clippy::too_many_arguments)]
pub fn lower_quality_dl_file(
    client: &Client,
    login: &Login,
    post: &Post,
    artist_name: &str,
    index: Option<&u64>,
    output_dir: &Path,
) -> DownloadStatus {
    let span = span!(Level::DEBUG, "lower_quality_handler");
    let _guard = span.enter();

    let url = post
        .sample
        .alternates
        .lower_quality
        .as_ref()
        .filter(|lq| lq.media_type == "video")
        .map(|lq| &lq.urls[0])
        .or(post.sample.url.as_ref())
        .or(post.file.url.as_ref());

    match url {
        Some(url) => download_file(
            client,
            login,
            url,
            &post.file.ext,
            post.id,
            artist_name,
            index,
            output_dir,
        ),
        None => {
            warn!(
                "Cannot download post {}-{} due it not having any file url.",
                artist_name, &post.id
            );
            DownloadStatus::default()
        }
    }
}

/// Creates `dir` (and any missing parent directories) if it doesn't already
/// exist. Returns `true` if the directory was created, `false` if it already
/// existed. Panics if directory creation fails (e.g. permissions).
pub fn create_dl_dir(dir: &Path) -> bool {
    if !dir.exists() {
        create_dir_all(dir).expect("Error creating output directory!");
        true
    } else {
        false
    }
}

/// Splits `arr.posts` into chunks of at most `chunk_size` — the unit of work
/// handed to each parallel download task in [`crate::commands`]. Panics if
/// `chunk_size <= 0` (see `[T]::chunks`); callers should validate thread/chunk
/// counts before calling this (the CLI does so via
/// [`crate::cli::validate_args`]).
pub fn slice_posts(arr: Posts, chunk_size: i32) -> Vec<Vec<Post>> {
    let mut res: Vec<Vec<Post>> = Vec::new();
    let posts = arr.posts;
    let slices = posts.chunks(chunk_size as usize);
    for slice in slices {
        res.push(slice.to_vec());
    }
    res
}

/// Same as [`slice_posts`], but for `(index, post)` pairs as used by pool
/// downloads, where each post carries its own distinct filename index.
pub fn slice_pool_posts(arr: Vec<(u64, Post)>, chunk_size: i32) -> Vec<Vec<(u64, Post)>> {
    let mut res: Vec<Vec<(u64, Post)>> = Vec::new();
    let slices = arr.chunks(chunk_size as usize);
    for slice in slices {
        res.push(slice.to_vec());
    }
    res
}

/// Fetches all matching posts for a favourites/tag search, one page at a time,
/// stopping when the API returns an empty page. `context.pages == -1` fetches
/// every page; `context.pages > 0` fetches at most that many; any other value
/// (e.g. `0`) fetches nothing. `fav`/`tags`/`random` are combined into the
/// request's `tags` query parameter as-is (pass `""` for any that don't apply).
/// A non-2xx response stops pagination early (logged, not propagated as an
/// error — check the returned `Vec`'s length rather than expecting a
/// `Result`), but a response body that fails to parse as JSON will panic.
pub fn get_pages(
    context: &CliContext,
    login: &Login,
    client: &Client,
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

            let res = send_request(client, login, &target);
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

            let res = send_request(client, login, &target);
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

/// Looks up pool metadata (name, description, ordered `post_ids`) by `pool_id`.
/// Returns `None` if the request fails (non-2xx) or no pool with that ID
/// exists; panics if a 2xx response body fails to parse as JSON.
pub fn get_pool(
    context: &CliContext,
    client: &Client,
    login: &Login,
    pool_id: &u64,
) -> Option<PoolData> {
    let target: String = format!(
        "https://{}/pools.json?limit=1&search[id]={}",
        context.api_source, pool_id
    );
    let res = send_request(&client, login, &target);
    if let Err(e) = res.error_for_status_ref() {
        error!("Response returned: {}", e);
        return None;
    }

    let data = res
        .json::<Vec<PoolData>>()
        .expect("Error reading response json.");
    if data.is_empty() {
        return None;
    }

    return Some(data[0].clone());
}

/// Fetches full post data for each ID in `post_ids`, one request per ID, in
/// the order given (this is what lets [`crate::commands::download_pool`]
/// preserve a pool's original ordering). On the first failed request or empty
/// result, returns an empty `Vec` immediately rather than partial results —
/// callers should treat an empty return as "failed", not "no posts requested".
pub fn get_post_data(
    context: &CliContext,
    client: &Client,
    login: &Login,
    post_ids: &Vec<u64>,
) -> Vec<Post> {
    let mut posts: Vec<Post> = Vec::new();

    for id in post_ids {
        let target = format!(
            "https://{}/posts.json?tags=id:{}&page=1&limit=1",
            context.api_source, id
        );
        let data = send_request(client, login, &target);
        if let Err(e) = data.error_for_status_ref() {
            error!("Response returned: {}", e);
            return Vec::new();
        }

        let post = data.json::<Posts>().expect("Error reading response json.");
        if post.posts.is_empty() {
            return Vec::new();
        }
        posts.push(post.posts[0].clone());
    }

    posts
}

/// Performs a GET request to `target`, using HTTP basic auth with
/// `login.username`/`login.api_key` if both are non-empty, otherwise
/// unauthenticated. Does not check the response status — callers are
/// responsible for calling `.error_for_status_ref()` or similar. Panics if the
/// request itself fails to send (network error), rather than returning a
/// `Result`.
pub fn send_request(client: &Client, login: &Login, target: &str) -> Response {
    if !login.username.is_empty() && !login.api_key.is_empty() {
        client
            .get(target)
            .basic_auth(login.username.clone(), Some(login.api_key.clone()))
            .send()
            .expect("Error getting response!")
    } else {
        client.get(target).send().expect("Error getting response!")
    }
}

#[cfg(test)]
#[path = "funcs_tests.rs"]
mod tests;
