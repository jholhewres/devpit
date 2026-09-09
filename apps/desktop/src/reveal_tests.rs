use super::*;

#[test]
fn a_file_in_a_registered_project_may_be_opened() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path().join("repo");
    std::fs::create_dir_all(project.join("src")).expect("create");
    std::fs::write(project.join("src/main.rs"), "").expect("write");

    let found = openable(
        std::slice::from_ref(&project),
        dir.path(),
        &project.join("src/main.rs"),
    );
    assert!(found.is_some());
}

#[test]
fn a_file_in_the_devpit_workspace_may_be_opened() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).expect("create");
    std::fs::write(home.join("board.db"), "").expect("write");

    assert!(openable(&[], &home, &home.join("board.db")).is_some());
}

#[test]
fn a_file_somewhere_else_entirely_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path().join("repo");
    let home = dir.path().join("home");
    let elsewhere = dir.path().join("private");
    for path in [&project, &home, &elsewhere] {
        std::fs::create_dir_all(path).expect("create");
    }
    std::fs::write(elsewhere.join("keys"), "").expect("write");

    assert!(openable(&[project], &home, &elsewhere.join("keys")).is_none());
}

/// The check resolves before it compares, so a link inside a project that
/// points out of it is refused rather than followed.
#[test]
fn a_symlink_pointing_out_of_the_project_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path().join("repo");
    let home = dir.path().join("home");
    let elsewhere = dir.path().join("private");
    for path in [&project, &home, &elsewhere] {
        std::fs::create_dir_all(path).expect("create");
    }
    std::fs::write(elsewhere.join("keys"), "").expect("write");
    std::os::unix::fs::symlink(elsewhere.join("keys"), project.join("looks-inside"))
        .expect("symlink");

    assert!(openable(
        std::slice::from_ref(&project),
        &home,
        &project.join("looks-inside")
    )
    .is_none());
}

#[test]
fn a_path_that_is_not_there_is_refused_rather_than_handed_over() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(openable(
        &[dir.path().to_path_buf()],
        dir.path(),
        &dir.path().join("gone")
    )
    .is_none());
}
