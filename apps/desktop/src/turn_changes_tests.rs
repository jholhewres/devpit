use super::*;

fn git(dir: &Path, args: &[&str]) {
    let ok = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git")
        .status
        .success();
    assert!(ok, "git {args:?} failed");
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "t@example.invalid"]);
    git(dir.path(), &["config", "user.name", "T"]);
    git(dir.path(), &["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "start"]);
    dir
}

#[test]
fn a_turn_that_edited_a_file_reports_it() {
    let dir = repo();
    let before = before(dir.path());
    std::fs::write(dir.path().join("a.txt"), "one\ntwo\n").expect("edit");
    std::fs::write(dir.path().join("b.txt"), "new\n").expect("create");

    let Some(Part::Changes { files }) = since(dir.path(), before.as_deref()) else {
        panic!("expected a rollup");
    };
    assert_eq!(
        files,
        [
            ChangedFile {
                path: "a.txt".to_owned(),
                added: 1,
                removed: 0
            },
            ChangedFile {
                path: "b.txt".to_owned(),
                added: 1,
                removed: 0
            },
        ]
    );
}

#[test]
fn a_turn_that_changed_nothing_says_nothing() {
    let dir = repo();
    let before = before(dir.path());
    assert!(since(dir.path(), before.as_deref()).is_none());
}

#[test]
fn a_folder_that_is_not_a_repository_has_no_rollup() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(before(dir.path()).is_none());
    assert!(since(dir.path(), None).is_none());
}
