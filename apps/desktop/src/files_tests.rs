//! The rules a file read has to keep, tested against a real directory.

use super::*;

/// A project registered in a temp store, and its root.
fn project() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join(".state/state.db")).expect("open");
    let id = store.add_project(dir.path(), None).expect("project");
    (dir, id)
}

#[test]
fn a_text_file_comes_back_with_its_text() {
    let (dir, _) = project();
    std::fs::write(dir.path().join("a.txt"), "hello\n").expect("write");
    let resolved = quockpit_core::tree::resolve(dir.path(), "a.txt").expect("resolve");
    assert_eq!(std::fs::read_to_string(&resolved).expect("read"), "hello\n");
}

/// The rule that matters most here: a path out of the project is refused
/// before anything is read. This process runs terminals; reaching it is
/// reaching the machine.
#[test]
fn a_path_climbing_out_of_the_project_is_refused() {
    let (dir, _) = project();
    std::fs::write(dir.path().join("inside.txt"), "x").expect("write");

    for escape in ["../../../etc/passwd", "..", "a/../../../etc/hosts"] {
        assert!(
            quockpit_core::tree::resolve(dir.path(), escape).is_err(),
            "{escape} was allowed out of the project"
        );
    }
}

/// A symlink is followed first and then checked, so pointing one out of the
/// project does not smuggle a path past the check.
#[test]
fn a_symlink_pointing_out_is_refused_too() {
    let (dir, _) = project();
    let outside = tempfile::tempdir().expect("tempdir");
    std::fs::write(outside.path().join("secret"), "x").expect("write");

    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path().join("secret"), dir.path().join("link"))
        .expect("link");

    #[cfg(unix)]
    assert!(
        quockpit_core::tree::resolve(dir.path(), "link").is_err(),
        "a symlink walked out of the project"
    );
}

/// The mtime guard, exercised rather than reasoned about.
///
/// A save carries the mtime the read saw. These prove the two cases that
/// matter: a normal save goes through, and a save built on a stale read is
/// refused — which is the whole reason the field exists.
mod saving {
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
}
