use super::*;

fn modified_of(path: &std::path::Path) -> f64 {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|since| since.as_millis() as f64)
        .unwrap_or_default()
}

#[test]
fn a_save_that_matches_what_was_read_goes_through() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "one\n").expect("write");

    let read_at = modified_of(&file);
    assert!(!is_stale(read_at, modified_of(&file)));
}

/// The case the guard exists for: the file moved on somewhere else, and
/// winning that race silently is how work done in another window is lost.
#[test]
fn a_save_built_on_a_stale_read_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "one\n").expect("write");
    let read_at = modified_of(&file);

    // Something else writes it. A sleep, because a filesystem's mtime is
    // not guaranteed finer than a millisecond and the guard compares in
    // milliseconds.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&file, "two\n").expect("second write");

    assert!(
        is_stale(read_at, modified_of(&file)),
        "a stale save was allowed: read at {read_at}, on disk {}",
        modified_of(&file)
    );
}

/// A file that did not exist when it was read has no mtime to compare, and
/// refusing on that would make a new file unsaveable.
#[test]
fn a_file_with_no_recorded_mtime_is_saveable() {
    assert!(!is_stale(0.0, 1_700_000_000_000.0));
}
