use super::*;

fn row(session: &str, pane: &str, leaf: &str) -> Snapped {
    Snapped {
        session: session.to_owned(),
        pane_id: pane.to_owned(),
        running: Running {
            leaf_id: leaf.to_owned(),
            command: "zsh".to_owned(),
            tty: "/dev/pts/1".to_owned(),
            profile: String::new(),
        },
    }
}

fn rows() -> Vec<Snapped> {
    vec![
        row("devpit_p1", "%1", "leaf_a"),
        row("devpit_p1__leaf_a", "%1", "leaf_a"),
        row("devpit_p1", "%2", "leaf_b"),
        row("devpit_p10", "%3", "leaf_c"),
        row("other", "%4", "w"),
    ]
}

#[test]
fn a_project_has_its_panes_once_and_never_a_neighbours() {
    let leaves: Vec<String> = in_project(&rows(), "p1")
        .into_iter()
        .map(|one| one.leaf_id)
        .collect();
    assert_eq!(leaves, vec!["leaf_a", "leaf_b"], "p10 is not p1");
}

#[test]
fn a_pane_id_names_its_window() {
    assert_eq!(
        windows(&rows()).get("%3").map(String::as_str),
        Some("leaf_c")
    );
}

#[test]
fn the_projects_with_panes_are_read_off_the_session_names() {
    let mut found: Vec<String> = projects(&rows()).into_iter().collect();
    found.sort();
    assert_eq!(found, vec!["p1", "p10"]);
}

fn pane(id: &str, command: &str) -> devpit_rpc::PaneRunning {
    devpit_rpc::PaneRunning {
        pane_id: id.to_owned(),
        busy: false,
        label: command.to_owned(),
        agent: None,
        command: command.to_owned(),
    }
}

#[test]
fn only_a_project_whose_panes_changed_is_told() {
    let mut told = Told::default();
    let first = HashMap::from([("p1".to_owned(), vec![pane("leaf_a", "zsh")])]);
    assert_eq!(told.changed(first.clone()), vec!["p1"]);
    assert!(
        told.changed(first).is_empty(),
        "the same answer is told to nobody"
    );
    let agent = HashMap::from([("p1".to_owned(), vec![pane("leaf_a", "claude")])]);
    assert_eq!(told.changed(agent), vec!["p1"]);
}

#[test]
fn a_project_whose_last_pane_closed_is_told_once() {
    let mut told = Told::default();
    told.changed(HashMap::from([(
        "p1".to_owned(),
        vec![pane("leaf_a", "zsh")],
    )]));
    assert_eq!(told.changed(HashMap::new()), vec!["p1"]);
    assert!(told.changed(HashMap::new()).is_empty());
}
