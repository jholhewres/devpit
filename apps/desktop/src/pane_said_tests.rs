use super::*;

#[test]
fn a_pane_whose_agent_works_is_not_read() {
    let mut states = PaneStates::default();
    states.heard("leaf_a", Doing::Working);
    assert!(!states.should_look("leaf_a"));
}

#[test]
fn a_pane_whose_agent_waits_is_read_every_time() {
    let mut states = PaneStates::default();
    states.heard("leaf_a", Doing::Waiting);
    assert!(states.should_look("leaf_a"));
    assert!(states.should_look("leaf_a"));
    states.heard("leaf_a", Doing::Working);
    assert!(!states.should_look("leaf_a"), "answered, so read no more");
}

#[test]
fn a_pane_never_heard_from_is_read_once() {
    let mut states = PaneStates::default();
    assert!(
        states.should_look("leaf_old"),
        "a pane alive from before the app"
    );
    assert!(!states.should_look("leaf_old"));
}
