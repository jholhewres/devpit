use devpit_rpc::ChannelEvent;

use super::{message, Action, Told};

fn waiting() -> Told {
    Told {
        event: ChannelEvent::SessionWaiting,
        line: "deploy-prod waits on you: «Drop the staging database?»".to_owned(),
        bare: "A session waits on you.".to_owned(),
        actions: vec![Action::SendDraft {
            profile: "work".to_owned(),
            session: "deploy-prod".to_owned(),
        }],
        asking: Some("tab_deploy".to_owned()),
        about: None,
    }
}

/// The question's choices, as the screen would give them.
fn read_off_the_screen(_: Option<&str>) -> Vec<Action> {
    vec![Action::ReminderDone {
        card_id: "Yes, drop it".to_owned(),
    }]
}

#[test]
fn at_the_minimal_level_a_notice_names_nothing_and_carries_no_choice() {
    let told = waiting();
    let (text, actions) = message(&[&told], false, read_off_the_screen).expect("a message");
    assert_eq!(text, "A session waits on you.");
    assert!(!text.contains("deploy-prod") && !text.contains("staging"));
    assert_eq!(actions, told.actions, "the choice's words stay here");
}

#[test]
fn with_titles_it_names_the_session_and_offers_the_choices() {
    let told = waiting();
    let (text, actions) = message(&[&told], true, read_off_the_screen).expect("a message");
    assert!(text.contains("deploy-prod"));
    assert_eq!(actions.len(), 2);
}
