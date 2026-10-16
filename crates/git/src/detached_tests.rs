use super::*;
use crate::fixture;

fn git(dir: &Path, args: &[&str]) -> String {
    run(dir, args).expect("git")
}

#[test]
fn a_detached_worktree_leaves_the_persons_checkout_as_it_was() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    std::fs::create_dir(&root).expect("root");
    fixture::repo(&root);
    std::fs::write(root.join("a.txt"), "one\n").expect("write");
    fixture::commit(&root, "first");
    git(&root, &["checkout", "-q", "-b", "feature"]);
    std::fs::write(root.join("a.txt"), "two\n").expect("write");
    fixture::commit(&root, "second");
    git(&root, &["checkout", "-q", "main"]);
    std::fs::write(root.join("b.txt"), "unsaved\n").expect("write");
    let (head, status) = (
        git(&root, &["rev-parse", "HEAD"]),
        git(&root, &["status", "--porcelain"]),
    );

    let feature = commit_of(&root, "feature").expect("feature");
    let at = dir.path().join("trees").join("one");
    add_detached(&root, &at, &feature).expect("added");
    assert_eq!(
        std::fs::read_to_string(at.join("a.txt")).expect("a"),
        "two\n"
    );
    assert_eq!(
        merge_base(&root, "main", &feature).expect("base"),
        head.trim()
    );
    remove_detached(&root, &at);

    assert!(!at.exists());
    assert_eq!(git(&root, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&root, &["status", "--porcelain"]), status);
    assert_eq!(
        git(&root, &["worktree", "list", "--porcelain"])
            .matches("worktree ")
            .count(),
        1
    );
}

#[test]
fn a_revision_is_never_an_option_and_the_main_checkout_never_a_tree() {
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
    fixture::commit(dir.path(), "first");
    for said in ["--output=x", "main x", "", "a\nb"] {
        assert!(!plain_revision(said), "{said:?}");
        assert!(commit_of(dir.path(), said).is_err());
    }
    let head = commit_of(dir.path(), "main").expect("main");
    assert!(add_detached(dir.path(), dir.path(), &head).is_err());
    assert!(add_detached(dir.path(), &dir.path().join("t"), "main").is_err());
}
