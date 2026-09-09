use super::*;

fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for folder in ["src", "node_modules/pkg", ".git/objects", "docs"] {
        std::fs::create_dir_all(dir.path().join(folder)).expect("create");
    }
    std::fs::write(dir.path().join("src/main.rs"), "").expect("write");
    std::fs::write(dir.path().join("docs/guide.md"), "").expect("write");
    std::fs::write(dir.path().join("node_modules/pkg/index.js"), "").expect("write");
    std::fs::write(dir.path().join(".git/objects/blob"), "").expect("write");
    std::fs::write(dir.path().join("README.md"), "").expect("write");
    dir
}

#[test]
fn every_file_that_is_the_persons_own_is_listed() {
    let dir = tree();
    let mut found = Vec::new();
    walk(dir.path(), dir.path(), &mut found);
    found.sort();
    assert_eq!(
        found,
        vec![
            "README.md".to_owned(),
            "docs/guide.md".to_owned(),
            "src/main.rs".to_owned(),
        ]
    );
}

/// A symlinked directory is not walked into, so a loop cannot hang the walk.
#[test]
fn a_symlink_loop_does_not_hang_the_walk() {
    let dir = tree();
    std::os::unix::fs::symlink(dir.path(), dir.path().join("src/loop")).expect("symlink");

    let mut found = Vec::new();
    assert!(walk(dir.path(), dir.path(), &mut found));
    assert_eq!(found.len(), 3, "the link was walked into: {found:?}");
}

#[test]
fn a_project_with_nothing_in_it_lists_nothing_rather_than_failing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut found = Vec::new();
    assert!(walk(dir.path(), dir.path(), &mut found));
    assert!(found.is_empty());
}
