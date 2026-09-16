use std::path::Path;

use devpit_rpc::{LayoutNode, SessionKind, SplitDirection};

use super::*;
use crate::sessions::tab_for_card;

fn seeded() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    (dir, store)
}

fn card_in(store: &Store, dir: &Path, name: &str) -> (String, String) {
    let root = dir.join(name);
    std::fs::create_dir_all(&root).expect("root");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, name, "")
        .expect("card");
    (project, card)
}

fn split_tree() -> String {
    let tree = LayoutNode::leaf("leaf_a", "s:leaf_a")
        .split_leaf(
            "leaf_a",
            SplitDirection::Horizontal,
            LayoutNode::leaf("leaf_b", "s:leaf_b"),
        )
        .expect("split");
    serde_json::to_string(&tree).expect("json")
}

#[test]
fn a_card_tab_is_read_back_the_way_it_was_named() {
    assert_eq!(card_of_tab(&tab_for_card("card_01")), Some("card_01"));
    assert_eq!(card_of_tab("tab_7f3a"), None);
    assert_eq!(card_of_tab("tab_card_../../etc"), None);
}

#[test]
fn a_leaf_in_either_half_of_a_split_card_tab_is_that_cards() {
    let (dir, store) = seeded();
    let (project, card) = card_in(&store, dir.path(), "one");
    store
        .set_pane_layout(&project, &tab_for_card(&card), &split_tree(), "leaf_b")
        .expect("layout");

    assert_eq!(
        card_of_leaf(&store, "leaf_b"),
        Some(Route {
            project_id: project.clone(),
            card_id: card.clone(),
            tab_id: tab_for_card(&card),
        })
    );
    assert_eq!(
        card_of_leaf(&store, "leaf_a").map(|route| route.card_id),
        Some(card)
    );
}

#[test]
fn a_leaf_outside_a_card_tab_belongs_to_no_card() {
    let (dir, store) = seeded();
    let (project, _card) = card_in(&store, dir.path(), "one");
    store
        .set_pane_layout(&project, "tab_plain", &split_tree(), "leaf_a")
        .expect("layout");
    assert_eq!(card_of_leaf(&store, "leaf_a"), None);
    assert_eq!(card_of_leaf(&store, "leaf_nowhere"), None);
}

#[test]
fn a_card_tab_filed_under_another_project_rings_no_card() {
    let (dir, store) = seeded();
    let (mine, _) = card_in(&store, dir.path(), "mine");
    let (_, theirs) = card_in(&store, dir.path(), "theirs");
    store
        .set_pane_layout(&mine, &tab_for_card(&theirs), &split_tree(), "leaf_a")
        .expect("layout");
    assert_eq!(card_of_leaf(&store, "leaf_a"), None);
}

#[test]
fn an_archived_cards_tab_rings_no_card() {
    let (dir, store) = seeded();
    let (project, card) = card_in(&store, dir.path(), "one");
    store
        .set_pane_layout(&project, &tab_for_card(&card), &split_tree(), "leaf_a")
        .expect("layout");
    store.archive_card(&card).expect("archive");
    assert_eq!(card_of_leaf(&store, "leaf_a"), None);
}

#[test]
fn only_waiting_rings_and_a_card_pane_names_its_card() {
    let route = Route {
        project_id: "p1".to_owned(),
        card_id: "card_1".to_owned(),
        tab_id: "tab_card_card_1".to_owned(),
    };
    assert_eq!(
        notice_for(Some(&route), Some("waiting")),
        Some(Ring {
            project_id: Some("p1"),
            card_id: Some("card_1"),
        })
    );
    assert_eq!(
        notice_for(None, Some("waiting")),
        Some(Ring {
            project_id: None,
            card_id: None,
        })
    );
    assert_eq!(notice_for(Some(&route), Some("working")), None);
    assert_eq!(notice_for(Some(&route), None), None);
}

#[test]
fn a_session_is_found_through_the_run_that_spoke_in_it() {
    let (dir, store) = seeded();
    let (project, card) = card_in(&store, dir.path(), "one");
    let step = store
        .create_step(&project, "agent", "review", "{}", false)
        .expect("step");
    let run = store.start_run(&card, &step, None).expect("run");
    store.set_run_session(&run, "s-run").expect("session");
    assert_eq!(
        card_of_session(&store, "s-run"),
        Some((card, SessionKind::Run))
    );
}

#[test]
fn a_session_is_found_through_its_background_link() {
    let (dir, store) = seeded();
    let (_project, card) = card_in(&store, dir.path(), "one");
    store
        .link_session(&card, "a1b2", "s-bg", None, None, None)
        .expect("link");
    assert_eq!(
        card_of_session(&store, "s-bg"),
        Some((card, SessionKind::Background))
    );
}

#[test]
fn an_archived_card_holds_no_session() {
    let (dir, store) = seeded();
    let (_project, card) = card_in(&store, dir.path(), "one");
    store
        .link_session(&card, "a1b2", "s-bg", None, None, None)
        .expect("link");
    store.archive_card(&card).expect("archive");
    assert_eq!(card_of_session(&store, "s-bg"), None);
}

#[test]
fn an_unknown_or_unsafe_session_id_finds_no_card() {
    let (dir, store) = seeded();
    let (_project, card) = card_in(&store, dir.path(), "one");
    // Held under an id a hook must never be trusted with.
    store
        .link_session(&card, "a1b2", "../s-bg", None, None, None)
        .expect("link");
    for id in ["../s-bg", "s-nowhere", ""] {
        assert_eq!(card_of_session(&store, id), None, "{id}");
    }
}
