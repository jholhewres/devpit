use super::{exclude, ignored};

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .expect("git init");
    assert!(status.success());
    dir
}

#[test]
fn a_file_is_kept_out_of_git_without_a_gitignore_change() {
    let dir = repo();
    assert!(!ignored(dir.path(), ".env").expect("check"));
    exclude(dir.path(), ".env").expect("exclude");
    assert!(ignored(dir.path(), ".env").expect("check"));
    assert!(!dir.path().join(".gitignore").exists());
}

#[test]
fn outside_a_repository_nothing_needs_ignoring() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(ignored(dir.path(), ".env").expect("check"));
}
