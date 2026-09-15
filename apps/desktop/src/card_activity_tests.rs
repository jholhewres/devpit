use devpit_agentcli::Event;

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
