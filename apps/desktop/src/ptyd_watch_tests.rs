use std::collections::HashMap;

use devpit_ptyd::frame::{Frame, Kind};

use super::*;

fn placed() -> Fronts {
    let mut fronts = Fronts::default();
    fronts.place(HashMap::from([
        (7, ("p1".to_owned(), "leaf_a".to_owned())),
        (8, ("p2".to_owned(), "leaf_b".to_owned())),
    ]));
    fronts
}

#[test]
fn a_pane_says_what_runs_in_it_and_only_a_change_is_news() {
    let mut fronts = placed();
    assert_eq!(fronts.heard(&Frame::new(7, Kind::Program, "zsh")), Some(7));
    assert_eq!(fronts.heard(&Frame::new(7, Kind::Program, "zsh")), None);
    assert_eq!(
        fronts.heard(&Frame::new(7, Kind::Program, "claude")),
        Some(7)
    );
    let running = fronts.running_in("p1");
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].pane_id, "leaf_a");
    assert_eq!(running[0].agent.as_deref(), Some("claude"));
    assert!(fronts.running_in("p2").is_empty());
}

#[test]
fn a_pane_that_ended_leaves_its_project() {
    let mut fronts = placed();
    fronts.heard(&Frame::new(8, Kind::Program, "zsh"));
    assert_eq!(fronts.projects(), vec!["p2"]);
    assert_eq!(
        fronts.heard(&Frame::new(8, Kind::Exited, Vec::new())),
        Some(8)
    );
    assert!(fronts.projects().is_empty());
}

#[test]
fn output_is_not_news() {
    let mut fronts = placed();
    assert_eq!(fronts.heard(&Frame::new(7, Kind::Output, "hi")), None);
}

#[test]
fn a_pane_says_where_it_stands_without_it_being_news() {
    let mut fronts = placed();
    assert_eq!(fronts.heard(&Frame::new(7, Kind::Cwd, "/srv/one")), None);
    assert_eq!(fronts.cwd_of(7), Some("/srv/one"));
    fronts.heard(&Frame::new(7, Kind::Program, "zsh"));
    assert_eq!(fronts.program_of(7), Some("zsh"));
    fronts.heard(&Frame::new(7, Kind::Exited, Vec::new()));
    assert_eq!(fronts.cwd_of(7), None, "an ended pane stands nowhere");
    assert_eq!(fronts.program_of(7), None);
}
