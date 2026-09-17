//! What a step would run, and the card that does not move because of it.

use super::*;

use devpit_rpc::ErrorCode;

/// A project with one card and one command step, not on the card's lane.
fn a_card_and_a_step(
    dir: &std::path::Path,
    config: &str,
) -> (Store, String, String, String, String) {
    let store = Store::open(&dir.join("state.db")).expect("open");
    let project = store.add_project(dir, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    let step = store
        .create_step(&project, "command", "tests", config, false)
        .expect("step");
    (store, card, step, column, project)
}

/// A check is somebody else's command against your machine. Sabotage: drop
/// `command` from the answer and the screen is back to a button that says
/// only "Run".
#[test]
fn a_preview_says_the_command_before_anybody_runs_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, card, step, _, _) = a_card_and_a_step(
        dir.path(),
        r#"{"command": "pnpm test", "timeoutSeconds": 600, "needsWorktree": false}"#,
    );

    let would = previewed(&store, &card, &step).expect("previewed");
    assert_eq!(would.command.as_deref(), Some("pnpm test"));
    assert_eq!(would.timeout_seconds, Some(600.0));
    assert_eq!(would.step_name, "tests");
    assert!(!would.irreversible);
    assert!(!would.in_a_worktree);
    assert_eq!(would.in_directory.as_deref(), dir.path().to_str());
}

/// A step that would get a checkout of its own says so, and does not make one
/// to answer the question.
#[test]
fn a_step_that_wants_a_checkout_says_so_without_making_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, card, step, _, _) = a_card_and_a_step(dir.path(), r#"{"command": "pnpm test"}"#);

    let would = previewed(&store, &card, &step).expect("previewed");
    assert!(would.in_a_worktree, "a command step defaults to a checkout");
    assert_eq!(
        would.in_directory, None,
        "the card has no checkout yet and one was invented"
    );
    assert_eq!(
        std::fs::read_dir(dir.path())
            .expect("read")
            .filter_map(Result::ok)
            // SQLite's own journal files are not something this made.
            .filter(|entry| !entry.file_name().to_string_lossy().starts_with("state.db"))
            .count(),
        0,
        "asking what a step would do made something on disk"
    );
}

/// The environment is named, never valued — the same rule the run's own
/// snapshot follows, said in the place a person reads before trusting a check.
#[test]
fn a_preview_names_the_environment_and_carries_no_values() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, card, step, _, _) = a_card_and_a_step(dir.path(), r#"{"command": "pnpm test"}"#);

    let would = previewed(&store, &card, &step).expect("previewed");
    assert!(would.declared_env.contains(&"DEVPIT_BRANCH".to_owned()));
    assert!(would
        .declared_env
        .contains(&"DEVPIT_PROJECT_PATH".to_owned()));
    assert!(would
        .declared_env
        .iter()
        .all(|name| name.starts_with("DEVPIT_")));
}

/// An agent step has no command, and an empty line would read like one that
/// does nothing.
#[test]
fn a_step_with_no_command_says_nothing_rather_than_an_empty_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    let step = store
        .create_step(&project, "agent", "review", "{}", false)
        .expect("step");

    let would = previewed(&store, &card, &step).expect("previewed");
    assert_eq!(would.command, None);
    assert_eq!(would.timeout_seconds, None);
}

#[test]
fn a_card_or_a_step_nobody_knows_is_not_found() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, card, step, _, _) = a_card_and_a_step(dir.path(), r#"{"command": "pnpm test"}"#);

    assert_eq!(
        previewed(&store, "card_nothing", &step)
            .expect_err("a refusal")
            .code,
        ErrorCode::NotFound
    );
    assert_eq!(
        previewed(&store, &card, "step_nothing")
            .expect_err("a refusal")
            .code,
        ErrorCode::NotFound
    );
}

/// The one the plan asks for out loud: running a check from here must not
/// drag the card anywhere. `card.play` runs the lane where the card stands,
/// and this is the test that says the card stays there.
///
/// Sabotage: make `runs::start` move the card to the next column and this
/// fails on the column it started in.
#[test]
fn running_a_check_leaves_the_card_where_it_was() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, card, _step, column, project) =
        a_card_and_a_step(dir.path(), r#"{"command": "true"}"#);

    let before = store.card(&card).expect("read").expect("a card");
    assert_eq!(before.column_id, column);

    // The run is opened the way the board opens one, without the thread that
    // carries it out: what is under test is the row, not the command.
    let step = store
        .create_step(&project, "command", "again", "{}", false)
        .expect("step");
    let run = store.start_run(&card, &step, None).expect("run");
    store
        .finish_run(&run, "ok", Some("done"), None, None, Some(0))
        .expect("finish");

    let after = store.card(&card).expect("read").expect("a card");
    assert_eq!(
        after.column_id, column,
        "the card moved because something ran on it"
    );
}
