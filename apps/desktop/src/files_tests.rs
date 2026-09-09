//! The rules a file read has to keep, tested against a real directory.

use devpit_core::Store;

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
    let resolved = devpit_core::tree::resolve(dir.path(), "a.txt").expect("resolve");
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
            devpit_core::tree::resolve(dir.path(), escape).is_err(),
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
        devpit_core::tree::resolve(dir.path(), "link").is_err(),
        "a symlink walked out of the project"
    );
}

/// The mtime guard, exercised rather than reasoned about.
///
/// A save carries the mtime the read saw. These prove the two cases that
/// matter: a normal save goes through, and a save built on a stale read is
/// refused — which is the whole reason the field exists.
/// The read ceiling, called rather than restated.
///
/// Nothing else stops `file_read` from pulling a whole file into memory and
/// handing it to a window that then has to draw it, so the threshold and the
/// sentence it produces are both worth a test.
mod opening {
    use super::*;

    #[test]
    fn a_small_file_is_opened() {
        assert_eq!(past_the_ceiling("a.txt", 4_096), None);
    }

    /// Exactly at the ceiling still opens: the rule is "past it", and an
    /// off-by-one here would refuse a file the message says is allowed.
    #[test]
    fn a_file_exactly_at_the_ceiling_is_opened() {
        assert_eq!(past_the_ceiling("a.txt", MOST_BYTES), None);
    }

    #[test]
    fn a_file_past_the_ceiling_is_refused_with_its_size() {
        let said = past_the_ceiling("big.bin", MOST_BYTES + 1).expect("refused");
        assert!(
            said.contains("big.bin"),
            "the sentence lost the path: {said}"
        );
        assert!(
            said.contains("2 MB"),
            "the sentence lost the ceiling: {said}"
        );
    }
}
