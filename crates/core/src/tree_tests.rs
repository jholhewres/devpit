use crate::tree::*;

fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("create");
    std::fs::create_dir_all(dir.path().join(".git")).expect("create");
    for name in ["a.rs", "item100.rs", "item9.rs", "item99.rs"] {
        std::fs::write(dir.path().join("src").join(name), "").expect("write");
    }
    std::fs::write(dir.path().join("README.md"), "").expect("write");
    dir
}

#[test]
fn directories_come_first_and_numbers_sort_as_numbers() {
    let dir = tree();
    let top = children(dir.path(), "").expect("children");
    assert_eq!(top[0].name, "src", "a directory is not first");

    let src = children(dir.path(), "src").expect("children");
    let names: Vec<_> = src.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["a.rs", "item9.rs", "item99.rs", "item100.rs"]);
}

#[test]
fn the_git_directory_is_never_walked() {
    let dir = tree();
    let top = children(dir.path(), "").expect("children");
    assert!(!top.iter().any(|entry| entry.name == ".git"));
}

/// The guard this module exists for.
#[test]
fn a_path_that_climbs_out_is_refused() {
    let dir = tree();
    let escape = children(dir.path(), "../..");
    assert!(matches!(escape, Err(TreeError::Outside { .. })), "escaped");
}

/// A string comparison passes this and a resolved one does not, which is
/// why the order in `resolve` is not an implementation detail.
#[cfg(unix)]
#[test]
fn a_symlink_pointing_outside_is_refused() {
    let dir = tree();
    let outside = tempfile::tempdir().expect("tempdir");
    std::os::unix::fs::symlink(outside.path(), dir.path().join("escape")).expect("symlink");

    let followed = children(dir.path(), "escape");
    assert!(
        matches!(followed, Err(TreeError::Outside { .. })),
        "a symlink walked out of the project"
    );
}

#[test]
fn a_file_in_the_project_comes_back_relative() {
    let dir = tree();
    let dropped = dir.path().join("src/a.rs");
    assert_eq!(
        relative_to(dir.path(), &dropped).expect("relative"),
        "src/a.rs"
    );
}

#[test]
fn a_file_outside_the_project_is_refused() {
    let dir = tree();
    let elsewhere = tempfile::tempdir().expect("tempdir");
    std::fs::write(elsewhere.path().join("secret"), "").expect("write");
    let refused = relative_to(dir.path(), &elsewhere.path().join("secret"));
    assert!(matches!(refused, Err(TreeError::Outside { .. })));
}

#[test]
fn a_symlink_pointing_out_of_the_project_is_refused() {
    let dir = tree();
    let elsewhere = tempfile::tempdir().expect("tempdir");
    std::fs::write(elsewhere.path().join("secret"), "").expect("write");
    let link = dir.path().join("looks-inside");
    std::os::unix::fs::symlink(elsewhere.path().join("secret"), &link).expect("symlink");
    assert!(matches!(
        relative_to(dir.path(), &link),
        Err(TreeError::Outside { .. })
    ));
}
