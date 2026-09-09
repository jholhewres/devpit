use super::*;

#[test]
fn a_skills_first_line_of_prose_is_its_description() {
    let text = "---\nname: tdd\n---\n\n# tdd\n\nWrite the test first.\n";
    assert_eq!(description_of(text), "Write the test first.");
}

#[test]
fn a_skill_with_no_frontmatter_still_gives_a_description() {
    assert_eq!(
        description_of("# tdd\n\nWrite the test first.\n"),
        "Write the test first."
    );
}

/// The heading repeats the name, and `tdd — # tdd` says nothing.
#[test]
fn the_heading_is_not_the_description() {
    assert_eq!(description_of("# tdd\n"), "");
}

#[test]
fn a_directory_is_measured_with_what_is_in_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("agents")).expect("create");
    std::fs::write(dir.path().join("agents/a.md"), "12345").expect("write");
    std::fs::write(dir.path().join("agents/b.md"), "123").expect("write");

    let held = measure(dir.path(), "agents/", "agents");
    assert!(held.is_dir);
    assert!(held.exists);
    assert_eq!(held.bytes, 8.0);
    assert_eq!(held.count, Some(2));
}

#[test]
fn a_file_is_measured_by_its_own_size_and_has_no_count() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("board.db"), "1234").expect("write");

    let held = measure(dir.path(), "board.db", "board.db");
    assert!(!held.is_dir);
    assert_eq!(held.bytes, 4.0);
    assert_eq!(held.count, None);
}

/// A row that says "not yet" is honest; one that says 0 B is not.
#[test]
fn something_the_workspace_has_not_made_yet_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let held = measure(dir.path(), "worktrees/", "worktrees");
    assert!(!held.exists);
}
