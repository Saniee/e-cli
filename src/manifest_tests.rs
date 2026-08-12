#[test]
fn writes_json_records() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("manifest.json");
    let stats = crate::DownloadStatistics {
        records: vec![crate::DownloadRecord {
            post_id: 1,
            source_url: Some("https://example.invalid/1.jpg".into()),
            md5: Some("abc".into()),
            artist: "artist".into(),
            extension: "jpg".into(),
            local_filename: Some("artist-1.jpg".into()),
            status: "completed".into(),
            bytes: 12,
            error: None,
        }],
        ..Default::default()
    };
    crate::manifest::write(&path, &stats).expect("write");
    let content = std::fs::read_to_string(path).expect("read");
    assert!(content.contains("artist-1.jpg"));
}
