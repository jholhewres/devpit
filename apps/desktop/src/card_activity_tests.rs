use devpit_agentcli::Event;
use devpit_rpc::SessionKind;

use super::*;

/// Every row of the plan's hook table, as written there.
#[test]
fn each_hook_means_what_the_table_says() {
    let rows: Vec<(Event, Option<Doing>)> = vec![
        (Event::SessionStarted, Some(Doing::Open)),
        (
            Event::Using {
                tool: "Edit".to_owned(),
            },
            Some(Doing::Working),
        ),
        (
            Event::Used {
                tool: "Edit".to_owned(),
            },
            Some(Doing::Working),
        ),
        (
            Event::SubagentStarted {
                agent: "a1".to_owned(),
                kind: None,
            },
            Some(Doing::Working),
        ),
        (
            Event::Delegated {
                agent: "a1".to_owned(),
                description: None,
                model: None,
                ended: false,
            },
            Some(Doing::Working),
        ),
        (Event::SubagentDone { agent: None }, Some(Doing::Working)),
        (Event::Waiting, Some(Doing::Waiting)),
        (Event::Stopped { said: None }, Some(Doing::Done)),
        (
            Event::SessionEnded {
                reason: Some("prompt_input_exit".to_owned()),
            },
            Some(Doing::Gone),
        ),
        (Event::SessionEnded { reason: None }, Some(Doing::Gone)),
        (
            Event::SessionEnded {
                reason: Some("clear".to_owned()),
            },
            None,
        ),
    ];
    for (event, doing) in rows {
        assert_eq!(state_of_event(&event), doing, "{event:?}");
    }
}

#[test]
fn a_pane_still_says_only_its_three_words() {
    assert_eq!(pane_word(Some(Doing::Working)), Some("working"));
    assert_eq!(pane_word(Some(Doing::Waiting)), Some("waiting"));
    assert_eq!(pane_word(Some(Doing::Done)), Some("done"));
    assert_eq!(pane_word(Some(Doing::Open)), None);
    assert_eq!(pane_word(Some(Doing::Failed)), None);
    assert_eq!(pane_word(Some(Doing::Gone)), None);
    assert_eq!(pane_word(None), None);
}

fn pane(card: &str, leaf: &str) -> Key {
    Key {
        card_id: card.to_owned(),
        kind: SessionKind::Pane,
        reference: leaf.to_owned(),
    }
}

fn place(leaf: &str) -> Place {
    Place {
        tab_id: Some("tab".to_owned()),
        leaf_id: Some(leaf.to_owned()),
    }
}

#[test]
fn a_session_end_still_reaches_the_card() {
    let mut activities = Activities::default();
    hear(
        &mut activities,
        pane("card_1", "leaf_a"),
        1,
        Doing::Working,
        place("leaf_a"),
    )
    .expect("working");
    let ended = state_of_event(&Event::SessionEnded {
        reason: Some("prompt_input_exit".to_owned()),
    })
    .expect("an end");
    let told = hear(
        &mut activities,
        pane("card_1", "leaf_a"),
        2,
        ended,
        place("leaf_a"),
    )
    .expect("said");
    assert_eq!(told.card_id, "card_1");
    assert_eq!(told.sessions[0].state, Some(Doing::Gone));
    assert_eq!(told.activity, Some(Doing::Gone));
}

#[test]
fn a_clear_is_not_an_end() {
    let mut activities = Activities::default();
    hear(
        &mut activities,
        pane("card_1", "leaf_a"),
        1,
        Doing::Working,
        place("leaf_a"),
    )
    .expect("working");
    let cleared = state_of_event(&Event::SessionEnded {
        reason: Some("clear".to_owned()),
    });
    assert_eq!(cleared, None, "nothing to hear");
    assert_eq!(
        activities.happening("card_1").activity,
        Some(Doing::Working)
    );
}

#[test]
fn a_late_post_does_not_undo_a_newer_state() {
    let mut activities = Activities::default();
    hear(
        &mut activities,
        pane("card_1", "leaf_a"),
        5,
        Doing::Waiting,
        place("leaf_a"),
    )
    .expect("waiting");
    assert_eq!(
        hear(
            &mut activities,
            pane("card_1", "leaf_a"),
            3,
            Doing::Working,
            place("leaf_a")
        ),
        None
    );
    assert_eq!(
        activities.happening("card_1").activity,
        Some(Doing::Waiting)
    );
}

#[test]
fn the_same_state_twice_is_said_once() {
    let mut activities = Activities::default();
    assert!(hear(
        &mut activities,
        pane("card_1", "leaf_a"),
        1,
        Doing::Working,
        place("leaf_a")
    )
    .is_some());
    assert_eq!(
        hear(
            &mut activities,
            pane("card_1", "leaf_a"),
            2,
            Doing::Working,
            place("leaf_a")
        ),
        None
    );
    // The repeat still counts as the newest word: an older post arriving now is late.
    assert_eq!(
        hear(
            &mut activities,
            pane("card_1", "leaf_a"),
            1,
            Doing::Done,
            place("leaf_a")
        ),
        None
    );
}

#[test]
fn two_panes_of_one_card_rank_by_activity() {
    let mut activities = Activities::default();
    hear(
        &mut activities,
        pane("card_1", "leaf_a"),
        1,
        Doing::Working,
        place("leaf_a"),
    )
    .expect("a");
    let both = hear(
        &mut activities,
        pane("card_1", "leaf_b"),
        2,
        Doing::Waiting,
        place("leaf_b"),
    )
    .expect("b");
    assert_eq!(both.sessions.len(), 2);
    assert_eq!(both.activity, Some(Doing::Waiting));
    let answered = hear(
        &mut activities,
        pane("card_1", "leaf_b"),
        3,
        Doing::Done,
        place("leaf_b"),
    )
    .expect("b done");
    assert_eq!(answered.activity, Some(Doing::Working));
    // Another card's panes are not this card's.
    hear(
        &mut activities,
        pane("card_2", "leaf_c"),
        4,
        Doing::Waiting,
        place("leaf_c"),
    )
    .expect("c");
    assert_eq!(activities.happening("card_1").sessions.len(), 2);
}

#[test]
fn a_card_adds_up_to_the_state_most_worth_looking_at() {
    let session = |state| CardSession {
        kind: SessionKind::Pane,
        reference: "leaf".to_owned(),
        state: Some(state),
        tab_id: None,
        leaf_id: None,
    };
    let order = [
        Doing::Gone,
        Doing::Done,
        Doing::Open,
        Doing::Failed,
        Doing::Working,
        Doing::Waiting,
    ];
    for pair in order.windows(2) {
        assert_eq!(
            activity(&[session(pair[0]), session(pair[1])]),
            Some(pair[1])
        );
        assert_eq!(
            activity(&[session(pair[1]), session(pair[0])]),
            Some(pair[1])
        );
    }
    assert_eq!(activity(&[]), None);
}

#[test]
fn closing_a_card_pane_takes_it_off_the_card() {
    let mut activities = Activities::default();
    hear(
        &mut activities,
        pane("card_1", "leaf_a"),
        1,
        Doing::Working,
        place("leaf_a"),
    )
    .expect("a");
    hear(
        &mut activities,
        pane("card_1", "leaf_b"),
        2,
        Doing::Waiting,
        place("leaf_b"),
    )
    .expect("b");

    let told = forget_leaf(&mut activities, "leaf_b").expect("told");
    let left: Vec<&str> = told
        .sessions
        .iter()
        .map(|session| session.reference.as_str())
        .collect();
    assert_eq!(left, ["leaf_a"]);
    assert_eq!(told.activity, Some(Doing::Working));
    // Closing it twice tells nobody anything.
    assert_eq!(forget_leaf(&mut activities, "leaf_b"), None);
}

#[test]
fn a_pane_continued_in_a_chat_leaves_the_card_once_its_leaf_closes() {
    // Continue in chat adopts the session, then closes the leaf that ran it.
    let mut activities = Activities::default();
    hear(
        &mut activities,
        pane("card_1", "leaf_a"),
        1,
        Doing::Waiting,
        place("leaf_a"),
    )
    .expect("a");
    let told = forget_leaf(&mut activities, "leaf_a").expect("told");
    assert!(told.sessions.is_empty());
    assert_eq!(told.activity, None);
}

#[test]
fn a_background_waiting_and_the_cli_status_agree() {
    use devpit_agentcli::Status;
    // No longer listed by the CLI: gone, whatever was heard last.
    assert_eq!(background_state(None, Some(Doing::Waiting)), Doing::Gone);
    // Still listed: what its hooks said wins over the CLI's coarser word.
    assert_eq!(
        background_state(Some(&Status::Busy), Some(Doing::Waiting)),
        Doing::Waiting
    );
    // Nothing heard yet: the CLI's status, by the plan's table.
    assert_eq!(background_state(Some(&Status::Busy), None), Doing::Working);
    assert_eq!(
        background_state(Some(&Status::Blocked), None),
        Doing::Waiting
    );
    assert_eq!(background_state(Some(&Status::Idle), None), Doing::Open);
    assert_eq!(background_state(Some(&Status::Done), None), Doing::Done);
    assert_eq!(background_state(Some(&Status::Unknown), None), Doing::Open);
}

#[test]
fn a_session_forgotten_leaves_its_card() {
    let mut activities = Activities::default();
    let key = Key {
        card_id: "card_1".to_owned(),
        kind: SessionKind::Background,
        reference: "s-bg".to_owned(),
    };
    hear(
        &mut activities,
        key.clone(),
        1,
        Doing::Waiting,
        Place::default(),
    )
    .expect("heard");
    assert!(forget(&mut activities, &key));
    assert!(activities.happening("card_1").sessions.is_empty());
    assert!(!forget(&mut activities, &key));
}

#[test]
fn a_session_nobody_has_heard_from_does_not_count_on_the_tile() {
    let session = |state| CardSession {
        kind: SessionKind::Pane,
        reference: "leaf".to_owned(),
        state,
        tab_id: None,
        leaf_id: None,
    };
    assert_eq!(
        activity(&[session(None), session(Some(Doing::Done))]),
        Some(Doing::Done)
    );
    assert_eq!(activity(&[session(None)]), None);
}

/// The plan's run table, row by row.
#[test]
fn each_run_state_means_what_the_table_says() {
    for (state, doing) in [
        ("running", Doing::Working),
        ("failed", Doing::Failed),
        ("lost", Doing::Failed),
        ("ok", Doing::Done),
        ("cancelled", Doing::Done),
    ] {
        assert_eq!(state_of_run(state), doing, "{state}");
    }
}

fn said(told: &CardHappening) -> Vec<(SessionKind, &str, Option<Doing>)> {
    told.sessions
        .iter()
        .map(|one| (one.kind, one.reference.as_str(), one.state))
        .collect()
}

/// A run is heard like a hook: its start, its end under the same key, and a
/// new run taking the card's earlier one off it while its panes stay.
#[test]
fn a_run_start_reaches_the_card() {
    let mut activities = Activities::default();
    hear(
        &mut activities,
        pane("card_1", "leaf_a"),
        1,
        Doing::Open,
        place("leaf_a"),
    )
    .expect("pane");

    let started =
        hear_run(&mut activities, "card_1", "s-1", 2, state_of_run("running")).expect("started");
    assert_eq!(started.activity, Some(Doing::Working));
    let failed =
        hear_run(&mut activities, "card_1", "s-1", 3, state_of_run("failed")).expect("ended");
    assert_eq!(failed.activity, Some(Doing::Failed));

    let next = hear_run(
        &mut activities,
        "card_1",
        "run_2",
        4,
        state_of_run("running"),
    )
    .expect("next");
    assert_eq!(
        said(&next),
        [
            (SessionKind::Pane, "leaf_a", Some(Doing::Open)),
            (SessionKind::Run, "run_2", Some(Doing::Working)),
        ]
    );
}

/// A stop is heard as the run's end, and the process it killed finds the run
/// closed, so nothing after it says `failed` or leaves it `working`.
#[test]
fn a_cancelled_run_does_not_stay_working() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    let step = store
        .create_step(&project, "command", "tests", "{}", false)
        .expect("step");
    let run = store.start_run(&card, &step, None).expect("start");
    let reference = run_reference(&store, &run);
    assert_eq!(reference, run, "a run with no agent is heard under its id");

    let mut activities = Activities::default();
    hear_run(&mut activities, &card, &reference, 1, Doing::Working).expect("started");
    assert!(store
        .finish_run(&run, "cancelled", None, None, None, None)
        .expect("stop"));
    hear_run(
        &mut activities,
        &card,
        &reference,
        2,
        state_of_run("cancelled"),
    )
    .expect("stopped");

    // What the run's own thread does once its process is gone.
    assert!(!store
        .finish_run(&run, "failed", None, None, None, None)
        .expect("late"));
    assert_eq!(activities.happening(&card).activity, Some(Doing::Done));
}

/// A card's conversation is heard working while a turn runs and done after it.
#[test]
fn a_chat_turn_is_heard_working_then_done() {
    let mut activities = Activities::default();
    let working = hear(
        &mut activities,
        chat_key("card_1", "conv_1"),
        1,
        Doing::Working,
        Place::default(),
    )
    .expect("working");
    assert_eq!(working.activity, Some(Doing::Working));
    let done = hear(
        &mut activities,
        chat_key("card_1", "conv_1"),
        2,
        Doing::Done,
        Place::default(),
    )
    .expect("done");
    assert_eq!(
        said(&done),
        [(SessionKind::Chat, "conv_1", Some(Doing::Done))]
    );
}
