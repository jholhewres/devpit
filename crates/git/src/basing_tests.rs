//! What a worktree base may and may not be.
//!
//! Every refusal here has a failure behind it that is not a crash: a checkout
//! inside `.git`, a relative path that quietly left the project, an agent
//! committing to the branch the person is looking at. None of them announce
//! themselves, so each one is a test.

use super::*;

fn project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("project");
    std::fs::create_dir_all(root.join(".git")).expect("create");
    (dir, root)
}

#[test]
fn nothing_typed_is_the_default_and_not_an_error() {
    let (_dir, root) = project();
    assert!(allowed(&root, "").is_ok());
    assert!(allowed(&root, "   ").is_ok());
    assert_eq!(chosen("  "), None);
}

#[test]
fn nothing_typed_keeps_putting_worktrees_where_they_already_are() {
    let (dir, root) = project();
    let home = dir.path().join("home");
    assert_eq!(
        worktree_at("", &home, &root, "prj_1", "card_1"),
        home.join("worktrees").join("prj_1").join("card_1"),
    );
}

#[test]
fn a_relative_base_lands_inside_the_project_without_repeating_it() {
    // The project id is what makes one shared folder hold several projects.
    // Inside the project there is only ever one, and the extra segment would
    // bury every checkout a level deeper for nothing.
    let (dir, root) = project();
    let home = dir.path().join("home");
    assert_eq!(
        worktree_at(".devpit/worktrees", &home, &root, "prj_1", "card_1"),
        root.join(".devpit").join("worktrees").join("card_1"),
    );
}

#[test]
fn an_absolute_base_keeps_the_projects_apart() {
    // Two projects, one card id each. Without the project segment they are
    // the same folder, and the second `git worktree add` fails on the first.
    let (dir, root) = project();
    let home = dir.path().join("home");
    let shared = dir.path().join("shared");
    let one = worktree_at(
        shared.to_str().expect("utf8"),
        &home,
        &root,
        "prj_1",
        "card",
    );
    let two = worktree_at(
        shared.to_str().expect("utf8"),
        &home,
        &root,
        "prj_2",
        "card",
    );
    assert_ne!(one, two);
    assert_eq!(one, shared.join("prj_1").join("card"));
}

#[test]
fn a_base_that_climbs_out_of_the_project_is_refused() {
    let (_dir, root) = project();
    assert_eq!(allowed(&root, "../elsewhere"), Err(Refused::Climbs));
    assert_eq!(allowed(&root, "a/../../b"), Err(Refused::Climbs));
    // Absolute too: `..` in the middle of one is the same escape by a
    // different door.
    assert_eq!(allowed(&root, "/data/../etc"), Err(Refused::Climbs));
}

#[test]
fn a_base_inside_the_object_store_is_refused() {
    let (_dir, root) = project();
    assert_eq!(allowed(&root, ".git/worktrees"), Err(Refused::InsideGit));
    assert_eq!(allowed(&root, ".git"), Err(Refused::InsideGit));
}

#[test]
fn the_project_root_itself_is_refused() {
    // `assignable` catches this at `create` time. Catching it here means the
    // person hears about it while typing, not once per card afterwards.
    let (_dir, root) = project();
    assert_eq!(allowed(&root, "."), Err(Refused::TheProjectItself));
    assert_eq!(
        allowed(&root, root.to_str().expect("utf8")),
        Err(Refused::TheProjectItself)
    );
}

#[test]
fn a_symlink_to_the_project_is_still_the_project() {
    // Comparing the strings is the check that looks right and is not.
    #[cfg(unix)]
    {
        let (dir, root) = project();
        let link = dir.path().join("same");
        std::os::unix::fs::symlink(&root, &link).expect("symlink");
        assert_eq!(
            allowed(&root, link.to_str().expect("utf8")),
            Err(Refused::TheProjectItself)
        );
    }
}

#[test]
fn an_ordinary_base_is_allowed() {
    let (dir, root) = project();
    assert!(allowed(&root, ".devpit/worktrees").is_ok());
    assert!(allowed(&root, dir.path().join("wt").to_str().expect("utf8")).is_ok());
}
