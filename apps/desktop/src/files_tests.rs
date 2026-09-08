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
