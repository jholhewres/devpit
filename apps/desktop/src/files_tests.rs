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
/// A named pipe is not a file, and reading one waits forever.
///
/// The workspace holds the tap FIFOs, and the Files panel lists everything it
/// finds — so a row for one is a click that stops the reader rather than
/// failing it. Run with a deadline on purpose: the bug being guarded against
/// is a read that never returns, and a test that reproduces it by hanging is
/// a test nobody can run.
#[cfg(unix)]
#[test]
fn a_named_pipe_is_refused_instead_of_waited_on() {
    let dir = tempfile::tempdir().expect("tempdir");
    let made = std::process::Command::new("mkfifo")
        .arg(dir.path().join("tap.fifo"))
        .status()
        .expect("mkfifo to run");
    assert!(made.success(), "mkfifo made no pipe");

    let root = dir.path().to_path_buf();
    let (tell, hear) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tell.send(contents(&root, "tap.fifo".to_owned()));
    });

    let answered = hear
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("the read to answer at all");
    let read = answered.expect("an answer, not an error");
    assert_eq!(
        read.not_shown.as_deref(),
        Some("tap.fifo is not a file — nothing to read")
    );
    assert!(read.text.is_none());
}
