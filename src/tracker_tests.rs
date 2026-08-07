use super::*;

#[test]
fn load_creates_missing_file() {
    let base = tempfile::tempdir().expect("tempdir");
    let path = base.path().join("tracked.txt");

    let tracker = Tracker::load(&path).expect("load");

    assert!(path.exists());
    assert_eq!(tracker.len(), 0);
    assert!(tracker.is_empty());
    assert_eq!(tracker.path(), path);
}

#[test]
fn load_reads_existing_ids_and_ignores_bad_lines() {
    let base = tempfile::tempdir().expect("tempdir");
    let path = base.path().join("tracked.txt");
    fs::write(&path, "1\n2\n\nnot-a-number\n3\n").expect("write");

    let tracker = Tracker::load(&path).expect("load");

    assert!(tracker.contains(1));
    assert!(tracker.contains(2));
    assert!(tracker.contains(3));
    assert!(!tracker.contains(4));
    assert_eq!(tracker.len(), 3);
}

#[test]
fn insert_persists_across_reloads() {
    let base = tempfile::tempdir().expect("tempdir");
    let path = base.path().join("tracked.txt");

    let tracker = Tracker::load(&path).expect("load");
    tracker.insert(42);
    tracker.insert(42);
    tracker.insert(7);
    drop(tracker);

    let tracker = Tracker::load(&path).expect("reload");
    assert!(tracker.contains(42));
    assert!(tracker.contains(7));
    assert_eq!(tracker.len(), 2);
}

#[test]
fn insert_duplicate_stays_single_in_file() {
    let base = tempfile::tempdir().expect("tempdir");
    let path = base.path().join("tracked.txt");

    let tracker = Tracker::load(&path).expect("load");
    tracker.insert(42);
    tracker.insert(42);
    drop(tracker);

    let content = fs::read_to_string(&path).expect("read");
    assert_eq!(content.lines().filter(|l| l.trim() == "42").count(), 1);
}
