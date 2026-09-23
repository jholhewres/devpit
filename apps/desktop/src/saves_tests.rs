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

/// A save replaces the file whole and leaves nothing beside it.
#[test]
fn a_save_is_written_beside_and_renamed_over() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "old\n").expect("write");

    written_whole(&file, b"new\n").expect("save");
    assert_eq!(std::fs::read_to_string(&file).expect("read"), "new\n");
    let left: Vec<_> = std::fs::read_dir(dir.path()).expect("list").collect();
    assert_eq!(left.len(), 1, "a temporary file was left behind");
}

/// An executable script saved from the editor is still executable.
#[cfg(unix)]
#[test]
fn a_save_keeps_the_files_permissions() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("run.sh");
    std::fs::write(&file, "#!/bin/sh\n").expect("write");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    written_whole(&file, b"#!/bin/sh\necho hi\n").expect("save");
    let mode = std::fs::metadata(&file).expect("meta").permissions().mode() & 0o777;
    assert_eq!(mode, 0o755);
}

/// A read-only file stays refused: the rename would otherwise replace it.
#[cfg(unix)]
#[test]
fn a_read_only_file_is_not_saved_over() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("ro.txt");
    std::fs::write(&file, "keep\n").expect("write");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o444)).expect("chmod");

    assert!(written_whole(&file, b"new\n").is_err());
    assert_eq!(std::fs::read_to_string(&file).expect("read"), "keep\n");
}

/// Saved through a link, the file it points at changes and the link stays.
#[cfg(unix)]
#[test]
fn a_save_through_a_link_keeps_the_link() {
    let dir = tempfile::tempdir().expect("tempdir");
    let real = dir.path().join("real.txt");
    std::fs::write(&real, "old\n").expect("write");
    std::os::unix::fs::symlink(&real, dir.path().join("link.txt")).expect("link");
    let resolved = dir.path().join("link.txt").canonicalize().expect("resolve");

    written_whole(&resolved, b"new\n").expect("save");
    assert_eq!(std::fs::read_to_string(&real).expect("read"), "new\n");
    assert!(dir
        .path()
        .join("link.txt")
        .symlink_metadata()
        .expect("meta")
        .file_type()
        .is_symlink());
}

/// Two saves of one file at once each land whole, one after the other.
#[test]
fn two_saves_at_once_never_mix() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("a.txt");
    std::fs::write(&file, "old\n").expect("write");
    let one = "1".repeat(200_000);
    let two = "2".repeat(200_000);

    std::thread::scope(|scope| {
        scope.spawn(|| written_whole(&file, one.as_bytes()).expect("one"));
        scope.spawn(|| written_whole(&file, two.as_bytes()).expect("two"));
    });
    let left = std::fs::read_to_string(&file).expect("read");
    assert!(left == one || left == two, "the two saves were mixed");
    assert_eq!(std::fs::read_dir(dir.path()).expect("list").count(), 1);
}
