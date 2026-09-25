use std::path::PathBuf;

use super::{carry, inside, inside_new, listed, plain, remove};

fn roots(dir: &std::path::Path) -> Vec<PathBuf> {
    vec![dir.join("repo").canonicalize().expect("repo")]
}

#[test]
fn a_name_is_a_plain_relative_path() {
    assert!(plain("notes.md").is_ok());
    assert!(plain("specs/api.md").is_ok());
    assert!(plain("../escape.md").is_err());
    assert!(plain("/etc/passwd").is_err());
    assert!(plain("a/../../b").is_err());
    assert!(plain("  ").is_err());
}

#[test]
fn a_file_comes_only_from_inside_the_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("repo/docs")).expect("mkdir");
    std::fs::write(dir.path().join("repo/docs/plan.md"), "plan").expect("write");
    std::fs::write(dir.path().join("outside.md"), "no").expect("write");
    let roots = roots(dir.path());
    let cwd = dir.path().join("repo");

    assert!(inside(&roots, &cwd, "docs/plan.md").is_ok());
    assert!(inside(&roots, &cwd, "../outside.md").is_err());
    assert!(inside(
        &roots,
        &cwd,
        &dir.path().join("outside.md").display().to_string()
    )
    .is_err());
    assert!(
        inside(&roots, &cwd, "docs").is_err(),
        "a folder was taken for a file"
    );
    assert!(inside_new(&roots, &cwd, "docs/new.md").is_ok());
    assert!(inside_new(&roots, &cwd, "../new.md").is_err());
    assert!(inside_new(&roots, &cwd, "missing/new.md").is_err());
}

#[test]
fn saving_copies_or_moves_and_never_writes_over_unasked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let from = dir.path().join("a.md");
    let kept = dir.path().join("kept/specs/a.md");
    std::fs::write(&from, "one").expect("write");

    carry(&from, &kept, false, false).expect("copied");
    assert!(from.exists() && kept.exists());
    assert!(
        carry(&from, &kept, false, false).is_err(),
        "wrote over one unasked"
    );
    std::fs::write(&from, "two").expect("write");
    carry(&from, &kept, true, true).expect("moved over it");
    assert!(!from.exists());
    assert_eq!(std::fs::read_to_string(&kept).expect("read"), "two");
}

#[test]
fn the_list_is_every_file_by_name_and_removing_tidies_up() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join("artifacts");
    std::fs::create_dir_all(folder.join("specs")).expect("mkdir");
    std::fs::write(folder.join("specs/api.md"), "api").expect("write");
    std::fs::write(folder.join("notes.md"), "n").expect("write");

    let names: Vec<String> = listed(&folder).into_iter().map(|one| one.name).collect();
    assert_eq!(names, ["notes.md", "specs/api.md"]);
    remove(&folder, "specs/api.md").expect("removed");
    assert!(
        !folder.join("specs").exists(),
        "an empty folder was left behind"
    );
    assert!(folder.exists());
    assert!(remove(&folder, "../notes.md").is_err());
}
