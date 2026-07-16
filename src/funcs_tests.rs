use super::*;
use crate::type_defs::api_defs::{Alternates, File as ApiFile, Sample, Tags};

fn dummy_post(id: u64) -> Post {
    Post {
        id,
        file: ApiFile {
            ext: "jpg".into(),
            url: Some(format!("http://example.invalid/{id}.jpg")),
        },
        tags: Tags {
            artist: vec!["someartist".into()],
        },
        sample: Sample {
            has: false,
            url: None,
            alternates: Alternates {
                lower_quality: None,
            },
        },
    }
}

#[test]
fn sum_posts_counts_across_pages() {
    let data = vec![
        vec![dummy_post(1), dummy_post(2)],
        vec![dummy_post(3)],
        vec![],
    ];
    assert_eq!(sum_posts(&data), 3);
}

#[test]
fn sum_posts_empty() {
    let data: Vec<Vec<Post>> = vec![];
    assert_eq!(sum_posts(&data), 0);
}

#[test]
fn slice_posts_even_division() {
    let posts = (1..=6).map(dummy_post).collect();
    let sliced = slice_posts(Posts { posts }, 2);
    assert_eq!(sliced.len(), 3);
    assert!(sliced.iter().all(|chunk| chunk.len() == 2));
}

#[test]
fn slice_posts_remainder_chunk() {
    let posts = (1..=5).map(dummy_post).collect();
    let sliced = slice_posts(Posts { posts }, 2);
    assert_eq!(sliced.len(), 3);
    assert_eq!(sliced.last().unwrap().len(), 1);
}

#[test]
fn slice_posts_chunk_larger_than_input() {
    let posts = vec![dummy_post(1), dummy_post(2)];
    let sliced = slice_posts(Posts { posts }, 10);
    assert_eq!(sliced.len(), 1);
    assert_eq!(sliced[0].len(), 2);
}

#[test]
fn slice_posts_single_item() {
    let posts = vec![dummy_post(1)];
    let sliced = slice_posts(Posts { posts }, 5);
    assert_eq!(sliced.len(), 1);
    assert_eq!(sliced[0].len(), 1);
}

#[test]
fn slice_pool_posts_even_division() {
    let arr: Vec<(u64, Post)> = (1..=6).map(|i| (i, dummy_post(i))).collect();
    let sliced = slice_pool_posts(arr, 3);
    assert_eq!(sliced.len(), 2);
    assert!(sliced.iter().all(|chunk| chunk.len() == 3));
}

#[test]
fn slice_pool_posts_remainder_chunk() {
    let arr: Vec<(u64, Post)> = (1..=5).map(|i| (i, dummy_post(i))).collect();
    let sliced = slice_pool_posts(arr, 3);
    assert_eq!(sliced.len(), 2);
    assert_eq!(sliced.last().unwrap().len(), 2);
}

#[test]
fn create_dl_dir_reports_first_creation_only() {
    let base = tempfile::tempdir().expect("tempdir");
    let dir = base.path().join("dl");

    assert!(create_dl_dir(&dir), "should create the dir on first call");
    assert!(dir.exists());
    assert!(
        !create_dl_dir(&dir),
        "should report false when dir already exists"
    );
}

#[test]
fn download_skips_already_downloaded_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let post = dummy_post(123);

    // Pre-create the file download() would otherwise try to fetch, so
    // this exercises the skip-if-exists path with zero network calls.
    std::fs::write(dir.path().join("someartist-123.jpg"), b"existing").expect("write");

    let client = crate::commands::get_client();
    let login = Login {
        username: String::new(),
        api_key: String::new(),
    };

    let result = download(&client, &login, vec![post], None, &false, dir.path());

    assert_eq!(result.amount_finished, 0);
    assert_eq!(result.amount_failed, 0);
    assert_eq!(result.amount, 0.0);
}
