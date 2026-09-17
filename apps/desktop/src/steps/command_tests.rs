//! The command step, against a real store and a real command.

use super::*;

use devpit_rpc::StepKind;

/// A project with one card, and the step that will run on it.
fn a_card_to_run_on(dir: &std::path::Path, command: &str) -> (Store, String, Step) {
    let store = Store::open(&dir.join("state.db")).expect("open");
    let project = store.add_project(dir, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");

    // No worktree: what is under test is the output, and a checkout would put
    // minutes and a git clone between the test and the thing it asks about.
    let config = serde_json::json!({ "command": command, "needsWorktree": false });
    let step = Step {
        id: "step_1".to_owned(),
        kind: StepKind::Command,
        name: "say".to_owned(),
        config: config.to_string(),
        irreversible: false,
    };
    (store, card, step)
}

/// `docs/the-model.md` has promised this since the first release and the
/// window never got a line until the command ended: the callback the agent
/// step was given was never passed to this one.
///
/// Sabotage: stop calling `on_progress` in `command::run` and the first line
/// arrives after the command's own second, not before it.
#[test]
fn the_window_hears_a_command_while_it_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, card, step) = a_card_to_run_on(dir.path(), "echo now; sleep 1; echo later");

    let started = std::time::Instant::now();
    let mut first: Option<std::time::Duration> = None;
    let mut heard: Vec<String> = Vec::new();
    let finished = run(
        &store,
        &card,
        &step,
        |line| {
            first.get_or_insert_with(|| started.elapsed());
            heard.push(line.to_owned());
        },
        |_| {},
    )
    .expect("the command ran");

    let first = first.expect("the window heard nothing at all");
    assert!(
        first < std::time::Duration::from_millis(800),
        "the first line waited {first:?} for a command that ran for a second"
    );
    assert_eq!(heard, ["now", "later"]);
    assert!(finished.ok);
    assert!(finished.output.contains("later"));
}

/// What the window heard and what the run recorded are the same lines. A card
/// whose stored output differs from what streamed past is a card nobody can
/// use to answer what happened.
#[test]
fn what_streamed_is_what_the_run_keeps() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, card, step) = a_card_to_run_on(dir.path(), "echo out; echo err 1>&2");

    let mut heard: Vec<String> = Vec::new();
    let finished = run(
        &store,
        &card,
        &step,
        |line| heard.push(line.to_owned()),
        |_| {},
    )
    .expect("the command ran");

    for line in &heard {
        assert!(
            finished.output.contains(line.as_str()),
            "{line:?} streamed but is not in what the run kept"
        );
    }
    assert!(heard.contains(&"out".to_owned()));
    assert!(heard.contains(&"err".to_owned()), "stderr never streamed");
}
