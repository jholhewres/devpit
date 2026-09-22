use super::*;

/// The listing recorded after a pane's window was killed from tmux itself.
const LISTED: &str = "devpit_prj_A 0\n\
devpit_prj_A__leaf_1 0\n\
devpit_prj_A__leaf_2 1\n\
devpit_prj_A__leaf_gone 0\n\
devpit_prj_B__leaf_9 0\n";

#[test]
fn a_detached_client_whose_window_is_gone_is_an_orphan() {
    let windows = vec!["leaf_1".to_owned(), "leaf_2".to_owned()];
    assert_eq!(
        orphans(LISTED, "devpit_prj_A", &windows),
        vec!["devpit_prj_A__leaf_gone".to_owned()]
    );
}

#[test]
fn an_attached_client_is_left_alone_even_without_its_window() {
    // Something is looking through it; taking it away blanks a pane.
    assert!(!orphans(LISTED, "devpit_prj_A", &[]).contains(&"devpit_prj_A__leaf_2".to_owned()));
}

#[test]
fn another_projects_clients_are_not_this_ones() {
    assert!(orphans(LISTED, "devpit_prj_A", &[])
        .iter()
        .all(|name| name.starts_with("devpit_prj_A__")));
}
