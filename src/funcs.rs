use std::path::Path;

use std::fs::{File, OpenOptions};
use std::{fs, fs::create_dir_all, io::Write, thread, time::Duration};

use reqwest::blocking::{Client, Response};
use tracing::{Level, debug, error, info, span, warn};

use crate::tracker::Tracker;
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
    pub amount_skipped: i64,
    pub amount: f64,
    pub records: Vec<crate::DownloadRecord>,
}

impl DownloadFinished {
    pub fn into_statistics(self, total: usize) -> crate::DownloadStatistics {
        crate::DownloadStatistics {
            completed: self.amount_finished,
            failed: self.amount_failed,
            skipped: self.amount_skipped,
            total,
            downloaded_amount: self.amount,
            records: self.records,
        }
    }
}

pub struct DownloadOptions<'a> {
    pub retries: u32,
    pub duplicate_index: Option<&'a crate::duplicate::DuplicateIndex>,
}

/// Downloads a batch of posts into `output_dir`, skipping (and counting in
/// [`DownloadFinished::amount_skipped`]) any post that is already downloaded:
/// either recorded in `tracker` (if `Some`), or whose target file already
/// exists on disk (which is then also recorded in `tracker`, so future runs
/// skip it without touching the filesystem).
///
/// `index` is applied to *every* post in `data` as a shared filename prefix (or
/// `None` for no prefix) — for per-post distinct indexes (as pool downloads need),
/// call this once per post with that post's own index rather than passing a
/// multi-post batch. If `lower_quality` is true, [`lower_quality_dl_file`] is used
/// instead of the full-resolution [`download_file`] where a sample/lower-quality
/// variant is available. Successfully downloaded posts are recorded in `tracker`.
#[allow(clippy::too_many_arguments)]
pub fn download(
    client: &Client,
    login: &Login,
    data: Vec<Post>,
    index: Option<&u64>,
    lower_quality: &bool,
    output_dir: &Path,
    tracker: Option<&Tracker>,
) -> DownloadFinished {
    download_with_options(
        client,
        login,
        data,
        index,
        lower_quality,
        output_dir,
        tracker,
        DownloadOptions {
            retries: 3,
            duplicate_index: None,
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub fn download_with_options(
    client: &Client,
    login: &Login,
    data: Vec<Post>,
    index: Option<&u64>,
    lower_quality: &bool,
    output_dir: &Path,
    tracker: Option<&Tracker>,
    options: DownloadOptions<'_>,
) -> DownloadFinished {
    let span = span!(Level::DEBUG, "download_handler");
    let _guard = span.enter();

    let mut downloaded_bytes = 0.0;
    let mut amount_finished = 0;
    let mut amount_failed = 0;
    let mut amount_skipped = 0;
    let mut records = Vec::new();

    for post in data {
        let artist_name = post.tags.parse_artists();

        if let Some(tracker) = tracker
            && tracker.contains(post.id)
        {
            debug!(
                "Post {}-{} already tracked, skipping.",
                artist_name, post.id
            );
            amount_skipped += 1;
            records.push(crate::DownloadRecord {
                post_id: post.id,
                source_url: post.file.url.clone(),
                md5: post.file.md5.clone(),
                artist: artist_name.clone(),
                extension: post.file.ext.clone(),
                local_filename: None,
                status: "skipped".into(),
                bytes: 0,
                error: None,
            });
            continue;
        }

        let path = output_dir.join(file_name(index, &artist_name, post.id, &post.file.ext));

        if path.exists() {
            warn!(
                "File {}-{}.{} already Exists!",
                artist_name, post.id, post.file.ext
            );
            if let Some(tracker) = tracker {
                tracker.insert(post.id);
            }
            amount_skipped += 1;
            records.push(crate::DownloadRecord {
                post_id: post.id,
                source_url: post.file.url.clone(),
                md5: post.file.md5.clone(),
                artist: artist_name.clone(),
                extension: post.file.ext.clone(),
                local_filename: Some(path.file_name().unwrap().to_string_lossy().into()),
                status: "skipped".into(),
                bytes: 0,
                error: None,
            });
            continue;
        }

        if let Some(md5) = post.file.md5.as_deref()
            && let Some(index) = options.duplicate_index
            && let Some(existing) = index.contains(md5)
        {
            amount_skipped += 1;
            records.push(crate::DownloadRecord {
                post_id: post.id,
                source_url: post.file.url.clone(),
                md5: post.file.md5.clone(),
                artist: artist_name.clone(),
                extension: post.file.ext.clone(),
                local_filename: Some(existing),
                status: "duplicate".into(),
                bytes: 0,
                error: None,
            });
            continue;
        }

        if *lower_quality {
            let stat = lower_quality_dl_file_with_retries(
                client,
                login,
                &post,
                &artist_name,
                index,
                output_dir,
                options.retries,
            );
            if stat.finished {
                downloaded_bytes += stat.downloaded_bytes;
                amount_finished += 1;
                if let Some(tracker) = tracker {
                    tracker.insert(post.id);
                }
                info!(
                    "Downloaded {}-{}.{}! File size: {:.2} MB",
                    artist_name,
                    post.id,
                    post.file.ext,
                    stat.downloaded_bytes / 1024.0 / 1024.0
                );
                let filename = file_name(index, &artist_name, post.id, &post.file.ext);
                if let (Some(md5), Some(index)) =
                    (post.file.md5.as_deref(), options.duplicate_index)
                {
                    index.insert(md5, &filename);
                }
                records.push(crate::DownloadRecord {
                    post_id: post.id,
                    source_url: post.file.url.clone(),
                    md5: post.file.md5.clone(),
                    artist: artist_name.clone(),
                    extension: post.file.ext.clone(),
                    local_filename: Some(filename),
                    status: "completed".into(),
                    bytes: stat.downloaded_bytes as u64,
                    error: None,
                });
            } else {
                amount_failed += 1;
                warn!(
                    "Failed to download {}-{}.{}",
                    artist_name, post.id, post.file.ext
                );
                records.push(crate::DownloadRecord {
                    post_id: post.id,
                    source_url: post.file.url.clone(),
                    md5: post.file.md5.clone(),
                    artist: artist_name.clone(),
                    extension: post.file.ext.clone(),
                    local_filename: None,
                    status: "failed".into(),
                    bytes: 0,
                    error: Some("download failed".into()),
                });
            }
        } else {
            match &post.file.url {
                Some(url) => {
                    let stat = download_file_with_retries(
                        client,
                        login,
                        url,
                        &post.file.ext,
                        post.id,
                        &artist_name,
                        index,
                        output_dir,
                        options.retries,
                    );
                    if stat.finished {
                        downloaded_bytes += stat.downloaded_bytes;
                        amount_finished += 1;
                        if let Some(tracker) = tracker {
                            tracker.insert(post.id);
                        }
                        info!(
                            "Downloaded {}-{}.{}! File size: {:.2} MB",
                            artist_name,
                            post.id,
                            post.file.ext,
                            stat.downloaded_bytes / 1024.0 / 1024.0
                        );
                        let filename = file_name(index, &artist_name, post.id, &post.file.ext);
                        if let (Some(md5), Some(index)) =
                            (post.file.md5.as_deref(), options.duplicate_index)
                        {
                            index.insert(md5, &filename);
                        }
                        records.push(crate::DownloadRecord {
                            post_id: post.id,
                            source_url: post.file.url.clone(),
                            md5: post.file.md5.clone(),
                            artist: artist_name.clone(),
                            extension: post.file.ext.clone(),
                            local_filename: Some(filename),
                            status: "completed".into(),
                            bytes: stat.downloaded_bytes as u64,
                            error: None,
                        });
                    } else {
                        amount_failed += 1;
                        warn!(
                            "Failed to download {}-{}.{}",
                            artist_name, post.id, post.file.ext
                        );
                        records.push(crate::DownloadRecord {
                            post_id: post.id,
                            source_url: post.file.url.clone(),
                            md5: post.file.md5.clone(),
                            artist: artist_name.clone(),
                            extension: post.file.ext.clone(),
                            local_filename: None,
                            status: "failed".into(),
                            bytes: 0,
                            error: Some("download failed".into()),
                        });
                    }
                }
                None => {
                    warn!(
                        "Cannot download post {}-{} due to it missing a file url",
                        artist_name, post.id
                    );
                    amount_failed += 1;
                    records.push(crate::DownloadRecord {
                        post_id: post.id,
                        source_url: None,
                        md5: post.file.md5.clone(),
                        artist: artist_name.clone(),
                        extension: post.file.ext.clone(),
                        local_filename: None,
                        status: "failed".into(),
                        bytes: 0,
                        error: Some("missing file URL".into()),
                    });
                }
            }
        }
    }

    DownloadFinished {
        amount_finished,
        amount_failed,
        amount_skipped,
        amount: downloaded_bytes,
        records,
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

    download_file_with_retries(
        client,
        login,
        target_url,
        file_ext,
        post_id,
        artist_name,
        index,
        output_dir,
        3,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn download_file_with_retries(
    client: &Client,
    login: &Login,
    target_url: &str,
    file_ext: &str,
    post_id: u64,
    artist_name: &str,
    index: Option<&u64>,
    output_dir: &Path,
    retries: u32,
) -> DownloadStatus {
    let span = span!(Level::DEBUG, "file_download");
    let _guard = span.enter();
    let name = file_name(index, artist_name, post_id, file_ext);
    let target = output_dir.join(&name);
    let part = output_dir.join(format!("{name}.part"));

    for attempt in 0..=retries {
        let existing = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        let mut request = if !login.username.is_empty() && !login.api_key.is_empty() {
            client
                .get(target_url)
                .basic_auth(&login.username, Some(&login.api_key))
        } else {
            client.get(target_url)
        };
        if existing > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={existing}-"));
        }
        let mut response = match request.send() {
            Ok(response) if response.status().is_success() => response,
            Ok(response)
                if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
                    || response.status().is_server_error() =>
            {
                if attempt < retries {
                    thread::sleep(Duration::from_millis(200 * 2u64.pow(attempt.min(4))));
                    continue;
                }
                warn!("Failed to request {name}: HTTP {}", response.status());
                return DownloadStatus::default();
            }
            Ok(response) => {
                warn!("Failed to request {name}: HTTP {}", response.status());
                return DownloadStatus::default();
            }
            Err(error) if attempt < retries => {
                thread::sleep(Duration::from_millis(200 * 2u64.pow(attempt.min(4))));
                debug!("Retrying {name} after request failure: {error}");
                continue;
            }
            Err(error) => {
                warn!("Failed to request {name}: {error}");
                return DownloadStatus::default();
            }
        };
        let append = existing > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
        let mut out = if append {
            match OpenOptions::new().create(true).append(true).open(&part) {
                Ok(file) => file,
                Err(_) => return DownloadStatus::default(),
            }
        } else {
            match File::create(&part) {
                Ok(file) => file,
                Err(_) => return DownloadStatus::default(),
            }
        };
        match response.copy_to(&mut out) {
            Ok(written) if out.flush().is_ok() && fs::rename(&part, &target).is_ok() => {
                let total = if append { existing + written } else { written };
                return DownloadStatus {
                    finished: true,
                    downloaded_bytes: total as f64,
                };
            }
            _ if attempt < retries => {
                thread::sleep(Duration::from_millis(200 * 2u64.pow(attempt.min(4))));
            }
            _ => {
                warn!("Failed to write {name}");
                return DownloadStatus::default();
            }
        }
    }
    DownloadStatus::default()
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
    lower_quality_dl_file_with_retries(client, login, post, artist_name, index, output_dir, 3)
}

#[allow(clippy::too_many_arguments)]
pub fn lower_quality_dl_file_with_retries(
    client: &Client,
    login: &Login,
    post: &Post,
    artist_name: &str,
    index: Option<&u64>,
    output_dir: &Path,
    retries: u32,
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
        Some(url) => download_file_with_retries(
            client,
            login,
            url,
            &post.file.ext,
            post.id,
            artist_name,
            index,
            output_dir,
            retries,
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

/// Ensures `dir` exists, creating it (and any missing parent directories) if
/// it doesn't. Returns `true` if the directory was created, `false` if it
/// already existed, and logs an informational message when it creates it.
///
/// This is the directory-creation helper meant for higher-level callers — the
/// CLI ensures the download directory before opening a tracking file (so a
/// tracker kept inside or next to the directory works), and external consumers
/// like a GUI can call it the same way before starting a download.
pub fn ensure_dl_dir(dir: &Path) -> bool {
    let created = create_dl_dir(dir);
    if created {
        info!(
            "Created a {} directory for all the downloaded files.",
            dir.display()
        );
    }
    created
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
                context.api_source(),
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
                context.api_source(),
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
        context.api_source(),
        pool_id
    );
    let res = send_request(client, login, &target);
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

    Some(data[0].clone())
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
            context.api_source(),
            id
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
