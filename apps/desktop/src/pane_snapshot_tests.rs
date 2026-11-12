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

/// The two-second loop on a machine with no tmux server: no listing, and
/// not even the tmux helper is asked for one.
#[test]
fn with_no_tmux_socket_nothing_is_listed_or_asked() {
    let home = tempfile::tempdir().expect("home");
    let socket = home.path().join("tmux.sock");
    let asked = || -> Vec<Snapped> { panic!("tmux was asked with no socket") };
    assert!(listed_at(Some(&socket), asked).is_empty());
    assert!(listed_at(None, asked).is_empty());

    std::fs::write(&socket, b"").expect("socket");
    assert_eq!(listed_at(Some(&socket), rows).len(), rows().len());
}

/// A listing kept for others is older than a hook heard after it was made:
/// its sequence is the one read before tmux was asked, not when it is reused.
#[test]
fn a_kept_listing_is_older_than_a_hook_heard_after_it_was_made() {
    let shared = Shared::new();
    let made = shared.fresh(rows);
    let hook = crate::card_activity::next_seq();
    let reused = shared.fresh(|| panic!("listed again while fresh"));
    assert_eq!(reused.seq, made.seq);
    assert!(reused.seq < hook);
}

#[test]
fn every_projects_panes_are_named_together_and_handed_back_to_their_project() {
    let named = named_by_project(&rows(), |panes| {
        panes
            .into_iter()
            .map(|one| devpit_rpc::PaneRunning {
                pane_id: one.leaf_id,
                busy: false,
                label: String::new(),
                agent: None,
                command: one.command,
            })
            .collect()
    });
    let leaves = |project: &str| -> Vec<String> {
        let mut leaves: Vec<String> = named[project]
            .iter()
            .map(|one| one.pane_id.clone())
            .collect();
        leaves.sort();
        leaves
    };
    assert_eq!(named.len(), 2);
    assert_eq!(leaves("p1"), ["leaf_a", "leaf_b"]);
    assert_eq!(leaves("p10"), ["leaf_c"]);
}
