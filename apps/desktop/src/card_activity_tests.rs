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
