use crate::paths::*;
use crate::tree::TreeError;

#[test]
fn a_missing_files_existing_parent_still_resolves() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("create");

    let resolved = resolve_new(dir.path(), "src/gone.rs").expect("resolve");
    let root = dir.path().canonicalize().expect("canonicalize");
    assert_eq!(resolved, root.join("src/gone.rs"));
}

/// The guard this module exists for, and the reason it must be asserted by
/// variant: a non-existent escaping path fails for a different reason
/// (`Unreadable`, nothing there to canonicalise), and a test that only checks
/// `is_err()` would keep passing after the containment check itself broke.
#[test]
fn a_parent_that_climbs_out_is_refused_as_outside() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Reaches a real file so the failure is containment, not "not there". One
    // `..` per component climbs to `/` wherever the tempdir happens to live.
    let climb = "../".repeat(dir.path().components().count());
    let refused = resolve_new(dir.path(), &format!("{climb}etc/passwd"));
    assert!(
        matches!(refused, Err(TreeError::Outside { .. })),
        "wrong reason: {refused:?}"
    );
}

#[test]
fn a_bare_name_at_the_root_resolves_against_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let resolved = resolve_new(dir.path(), "new.txt").expect("resolve");
    assert_eq!(
        resolved,
        dir.path()
            .canonicalize()
            .expect("canonicalize")
            .join("new.txt")
    );
}

#[test]
fn a_trailing_dot_dot_is_refused_without_reaching_the_filesystem() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(matches!(
        resolve_new(dir.path(), ".."),
        Err(TreeError::Outside { .. })
    ));
}

/// The hole a "may not exist" resolver invites: a name that already exists
/// is not the absent case at all, and assembling it by hand instead of
/// checking it hands back a symlink's target unexamined.
#[cfg(unix)]
#[test]
fn a_name_that_already_exists_as_a_symlink_out_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("tempdir");
    std::fs::write(outside.path().join("secret"), "x").expect("write");
    std::os::unix::fs::symlink(outside.path().join("secret"), dir.path().join("link"))
        .expect("symlink");

    assert!(matches!(
        resolve_new(dir.path(), "link"),
        Err(TreeError::Outside { .. })
    ));
}

/// The easier attack of the two: `exists()` follows a symlink and answers
/// about its *target*, so a link pointing at nothing reads as "not there" and
/// would fall through to the parent-only answer unexamined. `symlink_metadata`
/// is what tells the two apart.
#[cfg(unix)]
#[test]
fn a_broken_symlink_is_refused_rather_than_treated_as_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::os::unix::fs::symlink(dir.path().join("never-written"), dir.path().join("link"))
        .expect("symlink");

    // Refused because `resolve` cannot canonicalise it, not because it climbs
    // out — the name is taken by something this cannot vouch for either way.
    assert!(matches!(
        resolve_new(dir.path(), "link"),
        Err(TreeError::Unreadable { .. })
    ));
}

/// `.git` is the repository's own database, not a file the tree hands out a
/// writable path to — wherever it sits in the path, not only at the end.
#[test]
fn a_dot_git_segment_is_refused_wherever_it_sits() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("a")).expect("create");

    for path in [".git", ".git/config", "a/.git/b"] {
        assert!(
            matches!(
                resolve_new(dir.path(), path),
                Err(TreeError::Outside { .. })
            ),
            "{path} was allowed through"
        );
    }
}

/// The rule itself, called directly rather than only through `resolve_new`.
#[test]
fn touches_git_is_true_for_a_dot_git_segment_anywhere() {
    assert!(touches_git(".git"));
    assert!(touches_git(".git/config"));
    assert!(touches_git("a/.git/b"));
    assert!(!touches_git("a/gitignore"));
    assert!(!touches_git("a/b"));
}

#[test]
fn create_refuses_a_name_that_is_already_taken() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("taken.txt"), "x").expect("write");

    assert!(create(dir.path(), "taken.txt", false).is_err());
    // The existing file is still the existing file, not a truncated one.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("taken.txt")).expect("read"),
        "x"
    );
}

#[test]
fn create_makes_a_folder_when_asked_for_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    create(dir.path(), "a", true).expect("create");
    assert!(dir.path().join("a").is_dir());
}

/// The rule a drag onto the wrong folder depends on: `std::fs::rename`
/// replaces the destination silently, so `move_to` has to refuse first.
#[test]
fn move_to_refuses_to_overwrite_the_destination() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("from.txt"), "moving").expect("write");
    std::fs::write(dir.path().join("onto.txt"), "keep me").expect("write");

    assert!(move_to(dir.path(), "from.txt", "onto.txt").is_err());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("onto.txt")).expect("read"),
        "keep me"
    );
}

#[test]
fn move_to_carries_a_file_into_a_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("into")).expect("create");
    std::fs::write(dir.path().join("a.txt"), "x").expect("write");

    move_to(dir.path(), "a.txt", "into/a.txt").expect("move");
    assert!(dir.path().join("into/a.txt").exists());
    assert!(!dir.path().join("a.txt").exists());
}

#[test]
fn remove_takes_a_folder_and_everything_under_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("a/b")).expect("create");
    std::fs::write(dir.path().join("a/b/c.txt"), "x").expect("write");

    remove(dir.path(), "a").expect("remove");
    assert!(!dir.path().join("a").exists());
}

/// Every operation goes through `resolve_new`, so each inherits its refusals
/// rather than re-checking them — asserted here so a future shortcut that
/// skips the resolver fails instead of quietly reaching outside.
#[test]
fn every_operation_refuses_a_path_that_climbs_out() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("here.txt"), "x").expect("write");

    assert!(create(dir.path(), "../escaped.txt", false).is_err());
    assert!(move_to(dir.path(), "here.txt", "../escaped.txt").is_err());
    assert!(remove(dir.path(), "../../etc/passwd").is_err());
    assert!(create(dir.path(), ".git/hooks/evil", false).is_err());
    assert!(remove(dir.path(), ".git").is_err());
}
