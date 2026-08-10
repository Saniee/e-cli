use std::{
    fs,
    io::{self, Write},
    path::Path,
    process,
    time::Instant,
};

use clap::Parser;
use e_cli::{
    CliContext, DownloadStatistics, Login, Tracker,
    cli::{self, Commands},
    commands::{self, download_favourites, download_pool, download_search},
    config,
    funcs,
    update,
};
use indicatif::MultiProgress;
use tracing::{Level, error, info, span};
use tracing_subscriber::{
    EnvFilter, Layer, fmt, fmt::MakeWriter, layer::SubscriberExt, util::SubscriberInitExt,
};

/// A `tracing` writer that suspends any active `indicatif` progress bars
/// while a log line is written, so bar rendering doesn't get clobbered.
#[derive(Clone)]
struct ProgressWriter(MultiProgress);

impl Write for ProgressWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.suspend(|| io::stderr().write(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stderr().flush()
    }
}

impl<'a> MakeWriter<'a> for ProgressWriter {
    type Writer = ProgressWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn main() {
    let mut args = cli::Args::parse();

    if matches!(&args.command, Some(Commands::Config)) {
        if let Err(e) = config::open() {
            eprintln!("{e}");
            process::exit(1);
        }
        return;
    }

    if matches!(&args.command, Some(Commands::CheckUpdate)) {
        check_update_cmd();
        return;
    }

    let config_path = match config::path() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let file_config = match config::load(&config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    if let Err(e) = cli::apply_config(&mut args, &file_config)
        .and_then(|_| cli::fill_defaults(&mut args))
        .and_then(|_| cli::validate_args(&args))
    {
        eprintln!("{e}");
        return;
    }

    let context = CliContext {
        verbose: args.verbose,
        nsfw: args.nsfw,
        lower_quality: args.lower_quality,
        pages: args.pages.unwrap_or(-1),
        num_threads: args.num_threads.unwrap_or(5),
    };
    let mp = MultiProgress::new();
    let progress_writer = ProgressWriter(mp.clone());
    if args.verbose {
        let logging = fmt::layer()
            .compact()
            .with_target(false)
            .with_writer(progress_writer)
            .with_filter(EnvFilter::new("info,e_cli=debug"));
        let log_file = std::fs::File::create("debug.log").expect("Error creating log file.");
        let file_logging = fmt::layer()
            .json()
            .with_writer(log_file)
            .with_filter(EnvFilter::new("info,e_cli=debug"));
        tracing_subscriber::registry()
            .with(logging)
            .with(file_logging)
            .init();
    } else {
        let logging = fmt::layer()
            .without_time()
            .with_target(false)
            .compact()
            .with_writer(progress_writer)
            .with_filter(EnvFilter::new("info"));
        tracing_subscriber::registry().with(logging).init();
    }

    let mut username = String::new();
    let mut api_key = String::new();
    if args.login {
        let client = commands::get_client();

        info!("Sign-In via inputing your username and api_key.");
        info!("This info isn't sent anywhere. Only when the cli runs.");
        info!("Username: ");
        io::stdin()
            .read_line(&mut username)
            .expect("Error getting user input.");
        username = username.trim().to_owned();
        info!("Api Key: ");
        io::stdin()
            .read_line(&mut api_key)
            .expect("Error getting user input.");
        api_key = api_key.trim().to_owned();
        info!("Testing if valid...");
        let resp = client
            .get(format!(
                "https://{}/posts.json?tags=&limit=5",
                context.api_source()
            ))
            .basic_auth(&username, Some(api_key.clone()))
            .send()
            .expect("Error getting Auth response.");
        match resp.error_for_status() {
            Ok(_) => {
                info!("Sign-in Passed! Continuing...")
            }
            Err(err) => {
                return error!(
                    "The credentials provided aren't valid, or something else happened. Err: {err}"
                );
            }
        }
    }
    let login = Login { username, api_key };

    let dl_dir = Path::new(args.dir.as_deref().unwrap_or(cli::DL_DIR));

    // Create the download directory up front (before the tracking file is
    // opened), since users commonly keep the tracking file inside the
    // download directory or next to it.
    if matches!(
        &args.command,
        Some(Commands::DFavs { .. } | Commands::DTags { .. } | Commands::DPool { .. })
    ) {
        funcs::ensure_dl_dir(dl_dir);
    }

    let tracker = match &args.track_file {
        Some(path) => match Tracker::load(path) {
            Ok(t) => {
                info!("Tracking downloaded posts in {}.", path.display());
                Some(t)
            }
            Err(e) => {
                return error!("Failed to open tracking file {}: {e}", path.display());
            }
        },
        None => None,
    };

    #[allow(unused_mut)]
    let mut download_stats;
    let fn_start = Instant::now();
    let span = span!(Level::DEBUG, "main");
    let _guard = span.enter();

    match &args.command {
        Some(Commands::Config) => return,
        Some(Commands::CheckUpdate) => return,
        Some(Commands::ClearDl) => {
            if !dl_dir.exists() {
                return info!("Nothing to clean... Exiting!");
            }

            fs::remove_dir_all(dl_dir).expect("Err");
            return info!(
                "Cleaned the {} folder and also deleted the folder fully!",
                dl_dir.display()
            );
        }
        Some(Commands::DFavs {
            username,
            count,
            random,
            tags,
        }) => {
            download_stats = download_favourites(
                &context,
                &login,
                username.as_deref().expect("validated username"),
                &count.unwrap_or(5),
                random,
                tags.as_deref().unwrap_or_default(),
                &mp,
                dl_dir,
                tracker.as_ref(),
            );
        }
        Some(Commands::DTags {
            tags,
            count,
            random,
        }) => {
            download_stats = download_search(
                &context,
                &login,
                tags.as_deref().expect("validated tags"),
                &count.unwrap_or(5),
                random,
                &mp,
                dl_dir,
                tracker.as_ref(),
            );
        }
        Some(Commands::DPool { pool_id }) => {
            download_stats =
                download_pool(&context, &login, &pool_id.expect("validated pool ID"), &mp, dl_dir, tracker.as_ref());
        }
        Some(Commands::Zip { name, format }) => {
            if !commands::zip_downloads(
                dl_dir,
                name.as_deref().expect("validated archive name"),
                format.unwrap_or(cli::ArchiveFormat::Zip),
            ) {
                error!("Failed to package {} into an archive.", dl_dir.display());
            }
            return;
        }
        None => return,
    }

    finish(download_stats, fn_start);
}

fn check_update_cmd() {
    let current = env!("CARGO_PKG_VERSION");
    match update::check_update("Saniee/e-cli", current) {
        Ok(Some(latest)) => {
            println!(
                "A new version of e-cli is available: v{latest} (you're on v{current}).\n\
                 https://github.com/Saniee/e-cli/releases/tag/v{latest}"
            );
        }
        Ok(None) => println!("e-cli is up to date (v{current})."),
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    }
}

fn finish(statistics: DownloadStatistics, timer: Instant) {
    info!(
        "Finished! Downloaded: {} Posts. Skipped: {} already-downloaded Posts. Couldn't Download: {} Posts. Total data downloaded: {:.2} MB, in {} seconds.",
        statistics.completed,
        statistics.skipped,
        statistics.failed,
        statistics.downloaded_amount / 1024.0 / 1024.0,
        timer.elapsed().as_secs(),
    );
}
