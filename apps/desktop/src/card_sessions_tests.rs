use devpit_rpc::{Doing, LayoutNode, SplitDirection};

use super::*;

fn seeded() -> (tempfile::TempDir, Store, String, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("root");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    (dir, store, project, card)
}

fn nothing_heard(card: &str) -> CardHappening {
    CardHappening {
        card_id: card.to_owned(),
        activity: None,
        sessions: Vec::new(),
    }
}

#[test]
fn a_cards_panes_are_listed_before_their_agents_say_anything() {
    let (_dir, store, project, card) = seeded();
    let tree = LayoutNode::leaf("leaf_a", "s:leaf_a")
        .split_leaf(
            "leaf_a",
            SplitDirection::Vertical,
            LayoutNode::leaf("leaf_b", "s:leaf_b"),
        )
        .expect("split");
    let tab = tab_for_card(&card);
    store
        .set_pane_layout(
            &project,
            &tab,
            &serde_json::to_string(&tree).expect("json"),
            "leaf_a",
        )
        .expect("layout");
    let heard = CardHappening {
        card_id: card.clone(),
        activity: Some(Doing::Working),
        sessions: vec![CardSession {
            kind: SessionKind::Pane,
            reference: "leaf_a".to_owned(),
            state: Some(Doing::Working),
            tab_id: Some(tab.clone()),
            leaf_id: Some("leaf_a".to_owned()),
        }],
    };

    let sessions = card_sessions(&store, &project, &card, &[], &heard);
    let panes: Vec<(&str, Option<Doing>)> = sessions
        .iter()
        .map(|one| (one.reference.as_str(), one.state))
        .collect();
    assert_eq!(panes, [("leaf_a", Some(Doing::Working)), ("leaf_b", None)]);
    assert!(sessions
        .iter()
        .all(|one| one.kind == SessionKind::Pane && one.tab_id.as_deref() == Some(tab.as_str())));
}

#[test]
fn a_background_session_the_cli_no_longer_lists_is_gone() {
    let (_dir, store, project, card) = seeded();
    store
        .link_session(&card, "a1b2", "s-bg", None, None)
        .expect("link");
    let sessions = card_sessions(&store, &project, &card, &[], &nothing_heard(&card));
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].kind, SessionKind::Background);
    assert_eq!(sessions[0].state, Some(Doing::Gone));
}

#[test]
fn a_card_with_nothing_has_no_sessions() {
    let (_dir, store, project, card) = seeded();
    assert!(card_sessions(&store, &project, &card, &[], &nothing_heard(&card)).is_empty());
}

#[test]
fn a_run_heard_under_its_own_id_is_listed_without_a_link() {
    let (_dir, store, project, card) = seeded();
    let heard = CardHappening {
        card_id: card.clone(),
        activity: Some(Doing::Working),
        sessions: vec![CardSession {
            kind: SessionKind::Run,
            reference: "run_1".to_owned(),
            state: Some(Doing::Working),
            tab_id: None,
            leaf_id: None,
        }],
    };
    assert_eq!(
        card_sessions(&store, &project, &card, &[], &heard),
        heard.sessions
    );
}
