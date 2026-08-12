use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::channel;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::blocking::Client;

use rayon::prelude::*;
use tracing::{Level, debug, error, info, span};

use crate::cli::ArchiveFormat;
use crate::funcs::{
    self, DownloadFinished, ensure_dl_dir, get_pages, get_pool, get_post_data, slice_pool_posts,
    slice_posts, sum_posts,
};
use crate::tracker::Tracker;
use crate::type_defs::api_defs::{self, Post};
use crate::{AGENT, CliContext, DownloadStatistics, Login};

fn report_progress(
    context: &CliContext,
    completed: i64,
    failed: i64,
    skipped: i64,
    total: usize,
    downloaded_amount: f64,
) {
    if let Some(observer) = &context.progress {
        observer(crate::DownloadProgress {
            completed,
            failed,
            skipped,
            total,
            downloaded_amount,
            phase: None,
        });
    }
}

/// Builds a `reqwest::blocking::Client` configured with e-cli's `User-Agent` and
/// no request timeout (downloads of large files can legitimately take a while).
/// Callers should build one client per top-level operation and reuse it across
/// requests/downloads rather than constructing a new one per file, so that
/// connection pooling/keep-alive actually kicks in.
pub fn get_client() -> Client {
    Client::builder()
        .user_agent(AGENT)
        // !Experimental
        .timeout(None)
        .build()
        .expect("Error creating Client")
}

fn new_progress_bar(mp: &MultiProgress, total: u64) -> ProgressBar {
    let bar = mp.add(ProgressBar::new(total));
    bar.set_style(
        ProgressStyle::with_template(
            "{spinner} [{bar:40}] {pos}/{len} files ({per_sec}/s, ETA {eta})",
        )
        .expect("Invalid progress bar template")
        .progress_chars("#>-"),
    );
    bar
}

/// Downloads a user's favourited posts into `output_dir`, optionally narrowed by
/// `tags`. Pages are fetched according to `context.pages`, and each page's posts
/// are downloaded in parallel chunks sized by `context.num_threads`.
///
/// `mp` receives one progress bar tracking overall files completed/total; `count`
/// is the API page size (posts per request), not a total cap. Posts already
/// downloaded are skipped and counted in [`DownloadStatistics::skipped`]: either
/// recorded in `tracker` (if `Some`), or their file already exists in
/// `output_dir`. Returns [`DownloadStatistics::default`] (all zero) if no posts
/// were found for the given favourites/tags.
#[allow(clippy::too_many_arguments)]
pub fn download_favourites(
    context: &CliContext,
    login: &Login,
    username: &str,
    count: &u32,
    random: &bool,
    tags: &str,
    mp: &MultiProgress,
    output_dir: &Path,
    tracker: Option<&Tracker>,
) -> DownloadStatistics {
    let span = span!(Level::DEBUG, "DFavs");
    let _guard = span.enter();

    info!(
        "Downloading Favorites of {username} into the {} folder!",
        output_dir.display()
    );
    let client = get_client();
    let random_check: &str = if *random { "order:random" } else { "" };
    let tags: &str = if !tags.is_empty() { tags } else { "" };
    let fav: String = format!("fav:{}", username);
    info!("Getting posts from pages!");
    let data: Vec<Vec<Post>> = get_pages(context, login, &client, &fav, tags, random_check, count);
    if data.is_empty() {
        error!("No posts found...");
        return DownloadStatistics::default();
    }
    ensure_dl_dir(output_dir);
    let total = sum_posts(&data);
    info!("Downloading {} posts...", total);
    let bar = new_progress_bar(mp, total as u64);
    let mut full_sum = 0.0;
    let mut finished: i64 = 0;
    let mut failed: i64 = 0;
    let mut skipped: i64 = 0;
    let mut records = Vec::new();
    let chunk_size = context.num_threads as i32;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(context.num_threads)
        .build()
        .unwrap();
    for posts in data {
        let sliced_data = slice_posts(api_defs::Posts { posts }, chunk_size);
        let (tx, rx) = channel::<Vec<DownloadFinished>>();
        let bar = bar.clone();
        // Multi-threaded implementation.
        pool.install(|| {
            debug!("Starting download of {} posts.", sliced_data.len());
            let dl_size: Vec<DownloadFinished> = sliced_data
                .into_par_iter()
                .map(|posts| {
                    let low_quality = &context.lower_quality;
                    let count = posts.len() as u64;
                    let result = funcs::download_with_options(
                        &client,
                        login,
                        posts.to_vec(),
                        None,
                        low_quality,
                        output_dir,
                        tracker,
                        funcs::DownloadOptions {
                            retries: context.retries,
                            duplicate_index: context.duplicate_index.as_deref(),
                            cancel: context.cancel.clone(),
                        },
                    );
                    bar.inc(count);
                    result
                })
                .collect();

            tx.send(dl_size).unwrap();
        });
        for status in rx.recv().unwrap() {
            finished += status.amount_finished;
            failed += status.amount_failed;
            skipped += status.amount_skipped;
            full_sum += status.amount;
            records.extend(status.records);
            report_progress(context, finished, failed, skipped, total, full_sum);
        }
    }
    bar.finish_with_message("Done!");
    DownloadStatistics {
        completed: finished,
        failed,
        skipped,
        total,
        downloaded_amount: full_sum,
        records,
    }
}

/// Downloads posts matching a tag search into `output_dir`. Behaves like
/// [`download_favourites`] but searches by `tags` directly instead of a user's
/// favourites. `context.pages == -1` fetches every page until the API returns an
/// empty one, which for a broad tag search can mean the entire matching corpus —
/// the CLI disallows that default for this specific command via
/// [`crate::cli::validate_args`], but this function itself has no such guard.
#[allow(clippy::too_many_arguments)]
pub fn download_search(
    context: &CliContext,
    login: &Login,
    tags: &str,
    page_count: &u32,
    random: &bool,
    mp: &MultiProgress,
    output_dir: &Path,
    tracker: Option<&Tracker>,
) -> DownloadStatistics {
    let span = span!(Level::DEBUG, "DTags");
    let _guard = span.enter();

    info!(
        "Downloading posts, with '{tags}' tag/s, into the {} folder!",
        output_dir.display()
    );
    let client = get_client();
    let random_check: &str = if *random { "order:random" } else { "" };
    let tags: &str = if !tags.is_empty() { tags } else { "" };
    let fav = "";
    info!("Getting posts from pages!");
    let data: Vec<Vec<Post>> =
        get_pages(context, login, &client, fav, tags, random_check, page_count);
    if data.is_empty() {
        error!("No posts found...");
        return DownloadStatistics::default();
    }
    ensure_dl_dir(output_dir);
    let total = sum_posts(&data);
    info!("Downloading {} posts...", total);
    let bar = new_progress_bar(mp, total as u64);
    let mut full_sum = 0.0;
    let mut finished: i64 = 0;
    let mut failed: i64 = 0;
    let mut skipped: i64 = 0;
    let mut records = Vec::new();
    let chunk_size = context.num_threads as i32;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(context.num_threads)
        .build()
        .unwrap();
    for posts in data {
        let sliced_data = slice_posts(api_defs::Posts { posts }, chunk_size);

        let (tx, rx) = channel::<Vec<DownloadFinished>>();
        let bar = bar.clone();

        // Multi-threaded implementation.
        pool.install(|| {
            let dl_size: Vec<DownloadFinished> = sliced_data
                .into_par_iter()
                .map(|posts| {
                    let low_quality = &context.lower_quality;
                    let count = posts.len() as u64;
                    let result = funcs::download_with_options(
                        &client,
                        login,
                        posts.to_vec(),
                        None,
                        low_quality,
                        output_dir,
                        tracker,
                        funcs::DownloadOptions {
                            retries: context.retries,
                            duplicate_index: context.duplicate_index.as_deref(),
                            cancel: context.cancel.clone(),
                        },
                    );
                    bar.inc(count);
                    result
                })
                .collect();

            tx.send(dl_size).unwrap();
        });
        for status in rx.recv().unwrap() {
            finished += status.amount_finished;
            failed += status.amount_failed;
            skipped += status.amount_skipped;
            full_sum += status.amount;
            records.extend(status.records);
            report_progress(context, finished, failed, skipped, total, full_sum);
        }
    }
    bar.finish_with_message("Done!");
    DownloadStatistics {
        completed: finished,
        failed,
        skipped,
        total,
        downloaded_amount: full_sum,
        records,
    }
}

/// Downloads every post in the pool identified by `pool_id` into `output_dir`,
/// with each file named `{0001, 0002, ...}-{artist}-{post_id}.{ext}` so the pool's
/// original order is preserved regardless of parallel download order (index
/// zero-padded to 4 digits, matching pool page ordering — important for archive
/// readers, see [`zip_downloads`]). Returns
/// [`DownloadStatistics::default`] if the pool doesn't exist or has no posts.
pub fn download_pool(
    context: &CliContext,
    login: &Login,
    pool_id: &u64,
    mp: &MultiProgress,
    output_dir: &Path,
    tracker: Option<&Tracker>,
) -> DownloadStatistics {
    let span = span!(Level::DEBUG, "DPool");
    let _guard = span.enter();

    let client = get_client();
    if let Some(data) = get_pool(context, &client, login, pool_id) {
        ensure_dl_dir(output_dir);
        info!(
            "Downloading pool with id '{pool_id}' into the {} folder!",
            output_dir.display()
        );
        let mut posts_indexed: HashMap<u64, Post> = HashMap::new();
        let posts = get_post_data(context, &client, login, &data.post_ids);
        if posts.is_empty() {
            error!("Error getting post data.");
            return DownloadStatistics::default();
        }
        for (i, _) in data.post_ids.iter().enumerate() {
            let index: u64 = (i as u64) + 1;
            posts_indexed.insert(
                index,
                posts
                    .iter()
                    .find(|&p| p == &posts[i])
                    .expect("Post not found.")
                    .clone(),
            );
        }
        info!("Downloading {} posts...", data.post_ids.len());
        let mut posts_sorted = posts_indexed.into_iter().collect::<Vec<_>>();
        posts_sorted.sort_by_key(|a| a.0);
        let bar = new_progress_bar(mp, posts_sorted.len() as u64);
        let mut full_sum = 0.0;
        let mut finished: i64 = 0;
        let mut failed: i64 = 0;
        let mut skipped: i64 = 0;
        let mut records = Vec::new();
        let chunk_size = context.num_threads as i32;
        let sliced_posts = slice_pool_posts(posts_sorted, chunk_size);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(context.num_threads)
            .build()
            .unwrap();
        let (tx, rx) = channel::<Vec<DownloadFinished>>();
        let bar_clone = bar.clone();
        pool.install(|| {
            let dl_sizes: Vec<DownloadFinished> = sliced_posts
                .into_par_iter()
                .map(|chunk| {
                    let mut sum = DownloadFinished {
                        amount_finished: 0,
                        amount_failed: 0,
                        amount_skipped: 0,
                        amount: 0.0,
                        records: Vec::new(),
                    };
                    for (index, post) in chunk {
                        let result = funcs::download_with_options(
                            &client,
                            login,
                            vec![post],
                            Some(&index),
                            &context.lower_quality,
                            output_dir,
                            tracker,
                            funcs::DownloadOptions {
                                retries: context.retries,
                                duplicate_index: context.duplicate_index.as_deref(),
                                cancel: context.cancel.clone(),
                            },
                        );
                        sum.amount_finished += result.amount_finished;
                        sum.amount_failed += result.amount_failed;
                        sum.amount_skipped += result.amount_skipped;
                        sum.amount += result.amount;
                        bar_clone.inc(1);
                    }
                    sum
                })
                .collect();

            tx.send(dl_sizes).unwrap();
        });
        for status in rx.recv().unwrap() {
            finished += status.amount_finished;
            failed += status.amount_failed;
            skipped += status.amount_skipped;
            full_sum += status.amount;
            records.extend(status.records);
            report_progress(
                context,
                finished,
                failed,
                skipped,
                data.post_ids.len(),
                full_sum,
            );
        }
        bar.finish_with_message("Done!");

        DownloadStatistics {
            completed: finished,
            failed,
            skipped,
            total: data.post_ids.len(),
            downloaded_amount: full_sum,
            records,
        }
    } else {
        DownloadStatistics::default()
    }
}

/// Packages the contents of `dir` (as produced by [`download_pool`]) into an
/// archive named `{name}.{ext}` in the current working directory, where `ext`
/// comes from [`ArchiveFormat::extension`]. Only meaningful for pool downloads,
/// since the index-prefixed filenames are what makes a resulting cbz/zip
/// readable in order. Shells out to the `7z` executable, which must be on
/// `PATH`. Returns `false` (and logs the reason) if `dir` doesn't exist or `7z`
/// fails/isn't found; `name` is sanitized by stripping `/` before use.
pub fn zip_downloads(dir: &Path, name: &str, format: ArchiveFormat) -> bool {
    if !dir.exists() {
        error!(
            "Nothing to zip! The {} folder doesn't exist. Run d-pool first.",
            dir.display()
        );
        return false;
    }

    let safe_name = name.replace("/", "");
    let ok = match format {
        ArchiveFormat::Zip => run_7z(dir, &safe_name, "zip", "zip"),
        ArchiveFormat::SevenZip => run_7z(dir, &safe_name, "7z", "7z"),
        ArchiveFormat::Cbz => {
            run_7z(dir, &safe_name, "zip", "zip")
                && fs::rename(format!("./{safe_name}.zip"), format!("./{safe_name}.cbz"))
                    .map_err(|e| error!("Failed to rename archive to .cbz: {e}"))
                    .is_ok()
        }
    };

    if ok {
        info!(
            "Packaged {} into '{safe_name}.{}'.",
            dir.display(),
            format.extension()
        );
    }
    ok
}

fn run_7z(dir: &Path, name: &str, archive_type: &str, ext: &str) -> bool {
    let mut cmd = Command::new("7z");
    cmd.arg("a")
        .arg(format!("-t{archive_type}"))
        .arg(format!("{name}.{ext}"))
        .arg(dir.join("*"));

    match cmd.output() {
        Ok(out) if out.status.success() => true,
        Ok(out) => {
            error!(
                "7z exited with an error: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            false
        }
        Err(e) => {
            error!("Failed to run 7z (is it installed and on PATH?): {e}");
            false
        }
    }
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
