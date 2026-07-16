use super::*;

#[test]
fn zip_downloads_fails_when_dir_missing() {
    let base = tempfile::tempdir().expect("tempdir");
    let missing = base.path().join("does-not-exist");

    assert!(!zip_downloads(&missing, "archive", ArchiveFormat::Zip));
}
