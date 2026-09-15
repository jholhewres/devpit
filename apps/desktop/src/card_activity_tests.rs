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
    assert_eq!(told.sessions[0].state, Doing::Gone);
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
        state,
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
