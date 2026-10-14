use super::*;
use std::process::Command;

fn git(at: &Path, args: &[&str]) -> String {
    let done = Command::new("git")
        .args(args)
        .current_dir(at)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .expect("git");
    assert!(
        done.status.success(),
        "{}",
        String::from_utf8_lossy(&done.stderr)
    );
    String::from_utf8_lossy(&done.stdout).trim().to_owned()
}

fn committed(at: &Path, file: &str, text: &str) -> String {
    std::fs::write(at.join(file), text).expect("write");
    git(at, &["add", "-A"]);
    git(at, &["commit", "-q", "-m", file]);
    git(at, &["rev-parse", "HEAD"])
}

#[test]
fn the_commits_and_the_work_not_committed_are_read_apart() {
    let dir = tempfile::tempdir().expect("tempdir");
    let at = dir.path();
    git(at, &["init", "-q", "-b", "main"]);
    let base = committed(at, "a.txt", "one\n");
    let head = committed(at, "b.txt", "two\n");
    std::fs::write(at.join("a.txt"), "one, edited\n").expect("edit");

    let read = review_diff(at, &base, &head).expect("diff");
    assert_eq!(read.files, ["b.txt", "a.txt"]);
    let (commits, loose) = read.diff.split_once(UNCOMMITTED).expect("apart");
    assert!(
        commits.contains("+two") && !commits.contains("edited"),
        "{commits}"
    );
    assert!(loose.contains("+one, edited"), "{loose}");
}

#[test]
fn nothing_uncommitted_adds_no_section() {
    let dir = tempfile::tempdir().expect("tempdir");
    let at = dir.path();
    git(at, &["init", "-q", "-b", "main"]);
    let base = committed(at, "a.txt", "one\n");
    let head = committed(at, "b.txt", "two\n");
    let read = review_diff(at, &base, &head).expect("diff");
    assert!(!read.diff.contains(UNCOMMITTED));
    assert_eq!(read.files, ["b.txt"]);
}

#[test]
fn only_a_change_outside_devpit_is_a_change_to_the_code() {
    let dir = tempfile::tempdir().expect("tempdir");
    let at = dir.path();
    git(at, &["init", "-q", "-b", "main"]);
    let reviewed = committed(at, "a.txt", "one\n");
    std::fs::create_dir_all(at.join(".devpit/devflow")).expect("mkdir");
    let noted = committed(at, ".devpit/devflow/notes.md", "a note\n");
    assert!(!code_changed_between(at, &reviewed, &noted).expect("diff"));
    let fixed = committed(at, "a.txt", "one, fixed\n");
    assert!(code_changed_between(at, &reviewed, &fixed).expect("diff"));
}
