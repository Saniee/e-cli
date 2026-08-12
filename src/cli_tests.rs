use super::*;

fn parse(args: &[&str]) -> Args {
    let mut full = vec!["e-cli"];
    full.extend_from_slice(args);
    Args::try_parse_from(full).expect("args should parse")
}

#[test]
fn defaults() {
    let args = parse(&["d-pool", "1"]);
    assert!(!args.nsfw);
    assert_eq!(args.pages, None);
    assert_eq!(args.num_threads, None);
    assert_eq!(args.dir, None);
}

#[test]
fn nsfw_toggle_parses() {
    let args = parse(&["--nsfw", "d-pool", "1"]);
    assert!(args.nsfw);
}

#[test]
fn dir_defaults_to_dl_dir_const() {
    let mut args = parse(&["d-pool", "1"]);
    fill_defaults(&mut args).expect("defaults should fill");
    assert_eq!(args.dir, Some(DL_DIR.into()));
}

#[test]
fn dir_accepts_custom_value() {
    let args = parse(&["-d", "./custom/", "d-pool", "1"]);
    assert_eq!(args.dir, Some("./custom/".into()));

    let args = parse(&["d-pool", "1", "--dir", "D:\\downloads"]);
    assert_eq!(args.dir, Some("D:\\downloads".into()));
}

#[test]
fn track_file_defaults_to_none() {
    let args = parse(&["d-pool", "1"]);
    assert_eq!(args.track_file, None);
}

#[test]
fn track_file_accepts_custom_value() {
    let args = parse(&["-T", "seen.txt", "d-pool", "1"]);
    assert_eq!(args.track_file, Some(std::path::PathBuf::from("seen.txt")));

    let args = parse(&["d-pool", "1", "--track-file", "history.txt"]);
    assert_eq!(
        args.track_file,
        Some(std::path::PathBuf::from("history.txt"))
    );
}

#[test]
fn all_subcommands_parse() {
    assert!(matches!(parse(&["config"]).command, Some(Commands::Config)));
    assert!(matches!(
        parse(&["clear-dl"]).command,
        Some(Commands::ClearDl)
    ));
    assert!(matches!(
        parse(&["d-favs", "someuser"]).command,
        Some(Commands::DFavs { .. })
    ));
    assert!(matches!(
        parse(&["-p", "1", "d-tags", "dragon"]).command,
        Some(Commands::DTags { .. })
    ));
    assert!(matches!(
        parse(&["d-pool", "123"]).command,
        Some(Commands::DPool { .. })
    ));
    assert!(matches!(
        parse(&["zip", "-n", "test"]).command,
        Some(Commands::Zip { .. })
    ));
    assert!(matches!(
        parse(&["preset", "art"]).command,
        Some(Commands::Preset { .. })
    ));
    assert!(matches!(
        parse(&["retry-failed"]).command,
        Some(Commands::RetryFailed)
    ));
}

#[test]
fn zip_format_defaults_to_zip() {
    match parse(&["zip", "-n", "test"]).command {
        Some(Commands::Zip { format, .. }) => assert!(format.is_none()),
        _ => panic!("expected Zip command"),
    }
}

#[test]
fn config_values_fill_missing_cli_values() {
    let mut args = parse(&["d-pool"]);
    let mut config = Config::default();
    config.global.nsfw = Some(true);
    config.global.num_threads = Some(2);
    config.d_pool.pool_id = Some(123);

    apply_config(&mut args, &config).expect("config should apply");
    fill_defaults(&mut args).expect("defaults should fill");

    assert!(args.nsfw);
    assert_eq!(args.num_threads, Some(2));
    assert!(validate_args(&args).is_ok());
}

#[test]
fn cli_values_override_config_values() {
    let mut args = parse(&["--nsfw", "-t", "2", "d-pool", "123"]);
    let mut config = Config::default();
    config.global.nsfw = Some(false);
    config.global.num_threads = Some(10);

    apply_config(&mut args, &config).expect("config should apply");

    assert!(args.nsfw);
    assert_eq!(args.num_threads, Some(2));
}

#[test]
fn zip_format_parses_each_variant() {
    for (flag, expect_seven, expect_cbz) in [
        ("zip", false, false),
        ("7z", true, false),
        ("cbz", false, true),
    ] {
        match parse(&["zip", "-n", "test", "-f", flag]).command {
            Some(Commands::Zip { format, .. }) => {
                assert_eq!(
                    matches!(format, Some(ArchiveFormat::SevenZip)),
                    expect_seven
                );
                assert_eq!(matches!(format, Some(ArchiveFormat::Cbz)), expect_cbz);
            }
            _ => panic!("expected Zip command"),
        }
    }
}

#[test]
fn extension_mapping() {
    assert_eq!(ArchiveFormat::Zip.extension(), "zip");
    assert_eq!(ArchiveFormat::SevenZip.extension(), "7z");
    assert_eq!(ArchiveFormat::Cbz.extension(), "cbz");
}

#[test]
fn rejects_zero_threads() {
    let args = parse(&["-t", "0", "d-pool", "1"]);
    assert!(validate_args(&args).is_err());
}

#[test]
fn rejects_too_many_threads() {
    let args = parse(&["-t", "11", "d-pool", "1"]);
    assert!(validate_args(&args).is_err());
}

#[test]
fn accepts_valid_thread_range() {
    for t in [1, 5, 10] {
        let args = parse(&["-t", &t.to_string(), "d-pool", "1"]);
        assert!(validate_args(&args).is_ok());
    }
}

#[test]
fn rejects_dfavs_count_over_250() {
    let args = parse(&["d-favs", "someuser", "-c", "251"]);
    assert!(validate_args(&args).is_err());
}

#[test]
fn accepts_dfavs_count_at_250() {
    let args = parse(&["d-favs", "someuser", "-c", "250"]);
    assert!(validate_args(&args).is_ok());
}

#[test]
fn rejects_dtags_count_over_250() {
    let args = parse(&["-p", "1", "d-tags", "dragon", "-c", "251"]);
    assert!(validate_args(&args).is_err());
}

#[test]
fn rejects_dtags_without_pages() {
    // default -p is -1, meaning "not set" for d-tags
    let args = parse(&["d-tags", "dragon"]);
    assert!(validate_args(&args).is_err());
}

#[test]
fn accepts_dtags_with_pages_set() {
    let args = parse(&["-p", "1", "d-tags", "dragon"]);
    assert!(validate_args(&args).is_ok());
}

#[test]
fn dpool_unaffected_by_count_and_page_rules() {
    let args = parse(&["d-pool", "1"]);
    assert!(validate_args(&args).is_ok());
}
