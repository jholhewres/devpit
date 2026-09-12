//! Classifying a checkout, and reading back what was hidden.

use super::*;

fn dirs() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mine = dir.path().join("home").join("worktrees");
    std::fs::create_dir_all(&mine).expect("create");
    (dir, mine)
}

#[test]
fn a_checkout_under_our_own_folder_is_ours() {
    let (dir, mine) = dirs();
    let here = mine.join("prj_1").join("card_1");
    std::fs::create_dir_all(&here).expect("create");
    assert_eq!(origin_of(&here, &[mine]), WorktreeOrigin::Devpit);
    drop(dir);
}

#[test]
fn a_second_configured_base_counts_as_ours_too() {
    // Changing the setting does not reclassify every worktree made under the
    // old one: both folders are ours, and the old checkouts are still ours.
    let (dir, mine) = dirs();
    let other = dir.path().join("elsewhere");
    let here = other.join("prj_1").join("card_1");
    std::fs::create_dir_all(&here).expect("create");
    assert_eq!(origin_of(&here, &[mine, other]), WorktreeOrigin::Devpit);
}

#[test]
fn a_symlinked_base_is_still_the_same_base() {
    // Comparing the strings is the check that looks right and is not: a
    // devpit worktree reached through a link would be classified as somebody
    // else's and vanish from the list that made it.
    #[cfg(unix)]
    {
        let (dir, mine) = dirs();
        let here = mine.join("prj_1").join("card_1");
        std::fs::create_dir_all(&here).expect("create");
        let link = dir.path().join("linked");
        std::os::unix::fs::symlink(&mine, &link).expect("symlink");
        assert_eq!(
            origin_of(&link.join("prj_1").join("card_1"), &[mine]),
            WorktreeOrigin::Devpit
        );
    }
}

#[test]
fn a_checkout_under_dot_claude_is_the_agents() {
    let (dir, mine) = dirs();
    let here = dir
        .path()
        .join("project")
        .join(".claude")
        .join("worktrees")
        .join("feature");
    std::fs::create_dir_all(&here).expect("create");
    assert_eq!(origin_of(&here, &[mine]), WorktreeOrigin::Claude);
}

#[test]
fn a_folder_merely_called_worktrees_is_not_the_agents() {
    // The pair matters, not either word: plenty of people keep a `worktrees`
    // folder of their own and none of them meant `.claude/worktrees`.
    let (dir, mine) = dirs();
    let here = dir.path().join("project").join("worktrees").join("feature");
    std::fs::create_dir_all(&here).expect("create");
    assert_eq!(origin_of(&here, &[mine]), WorktreeOrigin::Other);
}

#[test]
fn anything_else_is_somebody_elses() {
    let (dir, mine) = dirs();
    let here = dir.path().join("scratch");
    std::fs::create_dir_all(&here).expect("create");
    assert_eq!(origin_of(&here, &[mine]), WorktreeOrigin::Other);
}

#[test]
fn nothing_stored_hides_nothing() {
    for stored in ["", "  ", ","] {
        assert!(hidden_in(stored).is_empty(), "{stored:?} hid something");
        assert!(shown(stored, WorktreeOrigin::Claude));
    }
}

#[test]
fn a_word_this_build_does_not_know_hides_nothing() {
    // A typo that hides nothing is recoverable. One that hides everything is
    // a settings pane whose lists are empty for a reason nobody can see.
    assert!(hidden_in("gsd,claude").len() == 1);
    assert!(!shown("gsd,claude", WorktreeOrigin::Claude));
    assert!(shown("gsd,claude", WorktreeOrigin::Devpit));
}

#[test]
fn what_was_hidden_survives_a_round_trip() {
    let hidden = hidden_in("other,claude");
    let written = hidden_as(&hidden);
    assert_eq!(hidden_in(&written), hidden);
    // Stable order, so two equal sets are one string and a write that changed
    // nothing does not bump the revision.
    assert_eq!(written, "claude,other");
}
