//! What a run recorded about itself, checked against what was true.

use super::*;

fn context() -> Context {
    Context {
        project: "devpit".to_owned(),
        project_path: "/w/devpit".to_owned(),
        worktree_path: "/w/devpit/.worktrees/card_1".to_owned(),
        branch: "card/one".to_owned(),
        base_ref: "2f0bee9".to_owned(),
        card: "card_1".to_owned(),
        card_title: "a card".to_owned(),
    }
}

#[test]
fn a_snapshot_says_what_ran_and_where() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ran = gathered("make test", dir.path(), &context(), true);

    assert_eq!(ran.command.as_deref(), Some("make test"));
    assert_eq!(
        ran.in_directory.as_deref(),
        Some(dir.path().to_str().unwrap())
    );
    assert_eq!(ran.base_revision.as_deref(), Some("2f0bee9"));
    assert_eq!(ran.in_a_worktree, Some(true));
    assert!(ran.is_known());
}

/// A card with no checkout has no commit it began at, and the snapshot says
/// nothing rather than an empty string that reads like a revision.
#[test]
fn a_card_that_never_began_anywhere_has_no_base_revision() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ran = gathered(
        "make test",
        dir.path(),
        &Context {
            base_ref: String::new(),
            ..context()
        },
        false,
    );

    assert_eq!(ran.base_revision, None);
    assert_eq!(ran.in_a_worktree, Some(false));
}

/// The environment is recorded by name. Sabotage: record the pairs instead and
/// this finds the branch's value on the row — which on a real project is a
/// path, a title, and whatever a profile added.
#[test]
fn the_environment_is_recorded_by_name_and_never_by_value() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ran = gathered("make test", dir.path(), &context(), true);

    assert!(
        ran.declared_env.contains(&"DEVPIT_BRANCH".to_owned()),
        "{:?}",
        ran.declared_env
    );
    for name in &ran.declared_env {
        assert!(
            name.starts_with("DEVPIT_"),
            "{name} is not the name of anything devpit declared"
        );
        assert!(
            !name.contains("card/one") && !name.contains("/w/devpit"),
            "{name} carries a value, not a name"
        );
    }
}

/// A directory that is no repository answers no revision, and that is an
/// answer: the run still ran, and the row says the commit is unknown.
#[test]
fn a_directory_that_is_no_repository_has_no_head() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(
        gathered("make test", dir.path(), &context(), false).head_revision,
        None
    );
}

/// A snapshot that cannot be saved does not take the run down with it.
#[test]
fn a_snapshot_that_cannot_be_saved_says_so_and_lets_the_run_stand() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = devpit_core::Store::open(&dir.path().join("state.db")).expect("open");
    // No such run: the update touches nothing and this must still return.
    recorded(&store, "run_nothing", &Ran::unknown());
}
