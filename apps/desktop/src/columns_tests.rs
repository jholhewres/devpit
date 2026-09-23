use super::*;

fn lanes() -> Vec<String> {
    ["todo", "doing", "done"].map(String::from).to_vec()
}

#[test]
fn a_lane_with_cards_and_no_destination_is_refused_with_the_count() {
    assert_eq!(
        where_cards_go(&lanes(), "doing", None, 3).expect("answer"),
        Deleting::Refused(3)
    );
}

#[test]
fn a_destination_takes_the_cards() {
    assert_eq!(
        where_cards_go(&lanes(), "doing", Some("done"), 3).expect("answer"),
        Deleting::MovingTo("done".to_owned())
    );
}

#[test]
fn the_lane_itself_or_another_boards_lane_is_not_a_destination() {
    let itself = where_cards_go(&lanes(), "doing", Some("doing"), 3).expect_err("refused");
    assert_eq!(itself.code, ErrorCode::Invalid);
    let elsewhere =
        where_cards_go(&lanes(), "doing", Some("col_of_another_project"), 3).expect_err("refused");
    assert_eq!(elsewhere.code, ErrorCode::Invalid);
}

#[test]
fn an_empty_lane_still_takes_its_archived_cards_somewhere() {
    assert_eq!(
        where_cards_go(&lanes(), "doing", None, 0).expect("answer"),
        Deleting::MovingTo("todo".to_owned())
    );
    assert_eq!(
        where_cards_go(&["only".to_owned()], "only", None, 0).expect("answer"),
        Deleting::Plain
    );
}

#[test]
fn a_lane_of_another_board_is_not_found() {
    let refused = where_cards_go(&lanes(), "elsewhere", None, 0).expect_err("refused");
    assert_eq!(refused.code, ErrorCode::NotFound);
}

/// Each branch says a different thing, because each is a different problem:
/// work in flight, history pointing here, or nothing in the way.
#[test]
fn a_step_that_is_running_is_not_deleted() {
    assert_eq!(
        step_delete_refusal(3, 1).as_deref(),
        Some("a card is running this step right now")
    );
    assert!(step_delete_refusal(5, 2)
        .expect("refused")
        .contains("2 cards"));
}

#[test]
fn a_step_with_runs_behind_it_is_not_deleted_either() {
    let why = step_delete_refusal(1, 0).expect("refused");
    assert!(why.contains("still shows it"), "{why}");
    assert!(step_delete_refusal(4, 0)
        .expect("refused")
        .contains("4 runs"));
}

#[test]
fn a_step_nothing_has_run_is_deleted() {
    assert_eq!(step_delete_refusal(0, 0), None);
}

/// The line continuation in the refusals used to carry the next line's indent
/// into the sentence.
#[test]
fn a_step_refusal_reads_as_one_sentence() {
    for why in [step_delete_refusal(1, 0), step_delete_refusal(4, 0)] {
        let why = why.expect("refused");
        assert!(!why.contains("  "), "{why}");
    }
}

/// Positions are unique per board and a delete leaves a gap: a new lane at the
/// count of lanes landed on the last lane's position and was refused.
#[test]
fn a_lane_is_created_after_a_middle_one_is_deleted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    let middle = store.columns(&project).expect("columns")[1].id.clone();
    store.delete_column(&middle).expect("delete");

    let made = create_lane(&store, &project, "later").expect("created");
    let lanes = store.columns(&project).expect("columns");
    assert_eq!(
        lanes.last().map(|lane| lane.id.as_str()),
        Some(made.as_str())
    );
}

/// With every lane gone, the next one is the first.
#[test]
fn a_lane_made_on_an_empty_board_is_at_the_start() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    for lane in store.columns(&project).expect("columns") {
        store.delete_column(&lane.id).expect("delete");
    }
    create_lane(&store, &project, "first").expect("created");
    assert_eq!(store.columns(&project).expect("columns")[0].position, 0);
}

/// An edit goes through the rule that refused the step when it was made.
///
/// Read off the source: the alternative is a store, a project and a board to
/// prove a single call, and what this guards against is that call quietly
/// going away — which is how `step_create` came to be the only door with a
/// lock on it.
#[test]
fn an_edit_is_refused_by_the_same_rule_as_a_new_step() {
    let source = include_str!("columns.rs");
    let update = source
        .split("fn step_update_now(")
        .nth(1)
        .expect("step_update is in this file");
    let body = update.split("\n}\n").next().unwrap_or(update);
    assert!(
        body.contains("refused(&step.kind, &config)"),
        "step_update saves a config nothing checked"
    );
}
