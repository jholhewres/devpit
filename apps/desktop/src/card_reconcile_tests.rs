use devpit_rpc::{LayoutNode, SplitDirection};

use super::*;
use crate::sessions::tab_for_card;

fn front(pane: &str, agent: Option<&str>) -> PaneRunning {
    PaneRunning {
        pane_id: pane.to_owned(),
        command: agent.unwrap_or("zsh").to_owned(),
        busy: agent.is_some(),
        agent: agent.map(str::to_owned),
        label: agent.unwrap_or("zsh").to_owned(),
    }
}

fn leaves() -> HashMap<String, CardPane> {
    HashMap::from([(
        "leaf_a".to_owned(),
        CardPane {
            card_id: "card_1".to_owned(),
            tab_id: "tab_card_card_1".to_owned(),
        },
    )])
}

fn pane(leaf: &str) -> Key {
    Key {
        card_id: "card_1".to_owned(),
        kind: SessionKind::Pane,
        reference: leaf.to_owned(),
    }
}

#[test]
fn a_card_pane_with_an_agent_in_front_and_no_word_is_open() {
    let changes = reconciled(&leaves(), &[front("leaf_a", Some("claude"))], &[]);
    assert_eq!(
        changes,
        [Change::Open {
            key: pane("leaf_a"),
            place: Place {
                tab_id: Some("tab_card_card_1".to_owned()),
                leaf_id: Some("leaf_a".to_owned()),
            },
        }]
    );
    // Already heard from: its own word stands.
    assert!(reconciled(
        &leaves(),
        &[front("leaf_a", Some("claude"))],
        &[pane("leaf_a")]
    )
    .is_empty());
    // Not a card's pane.
    assert!(reconciled(&leaves(), &[front("leaf_z", Some("claude"))], &[]).is_empty());
}

#[test]
fn a_known_pane_tmux_no_longer_lists_leaves_its_card() {
    assert_eq!(
        reconciled(&leaves(), &[], &[pane("leaf_a")]),
        [Change::Gone(pane("leaf_a"))]
    );
    assert!(reconciled(&leaves(), &[front("leaf_a", None)], &[pane("leaf_a")]).is_empty());
    // A pane outside this project's card tabs is another poll's to judge.
    assert!(reconciled(&leaves(), &[], &[pane("leaf_elsewhere")]).is_empty());
}

#[test]
fn a_reconcile_does_not_undo_a_newer_hook() {
    let activities = Mutex::new(Activities::default());
    // Stamped before tmux and `ps` were asked; the hook arrives after.
    let before = 5;
    hear(
        &mut activities.lock().expect("lock"),
        pane("leaf_a"),
        6,
        Doing::Working,
        Place::default(),
    )
    .expect("hook");
    assert!(reconcile_in(&activities, &leaves(), &[], before).is_empty());
    assert!(reconcile_in(
        &activities,
        &leaves(),
        &[front("leaf_a", Some("claude"))],
        before
    )
    .is_empty());
    assert_eq!(
        activities
            .lock()
            .expect("lock")
            .happening("card_1")
            .activity,
        Some(Doing::Working)
    );

    // Heard before the stamp, and gone from tmux since: it leaves.
    let told = reconcile_in(&activities, &leaves(), &[], 7);
    assert_eq!(told.len(), 1);
    assert!(told[0].sessions.is_empty());
}

#[test]
fn after_a_restart_an_agent_in_front_reads_as_open_and_a_shell_as_nothing() {
    let activities = Mutex::new(Activities::default());
    assert!(reconcile_in(&activities, &leaves(), &[front("leaf_a", None)], 1).is_empty());
    assert_eq!(
        activities
            .lock()
            .expect("lock")
            .happening("card_1")
            .activity,
        None
    );

    let told = reconcile_in(
        &activities,
        &leaves(),
        &[front("leaf_a", Some("claude"))],
        2,
    );
    assert_eq!(told.len(), 1);
    assert_eq!(told[0].activity, Some(Doing::Open));
    // Said once: the next poll finds it known and changes nothing.
    assert!(reconcile_in(
        &activities,
        &leaves(),
        &[front("leaf_a", Some("claude"))],
        3
    )
    .is_empty());
}

#[test]
fn a_projects_card_leaves_are_read_in_one_scan() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let card_in = |name: &str| {
        let root = dir.path().join(name);
        std::fs::create_dir_all(&root).expect("root");
        let project = store.add_project(&root, None).expect("project");
        store.ensure_board(&project).expect("board");
        let column = store.columns(&project).expect("columns")[0].id.clone();
        let card = store
            .create_card(&project, &column, name, "")
            .expect("card");
        (project, card)
    };
    let (mine, card) = card_in("mine");
    let (theirs, other) = card_in("theirs");
    let split = LayoutNode::leaf("leaf_a", "s:leaf_a")
        .split_leaf(
            "leaf_a",
            SplitDirection::Vertical,
            LayoutNode::leaf("leaf_b", "s:leaf_b"),
        )
        .expect("split");
    let tree = serde_json::to_string(&split).expect("json");
    store
        .set_pane_layout(&mine, &tab_for_card(&card), &tree, "leaf_a")
        .expect("mine");
    let lone = serde_json::to_string(&LayoutNode::leaf("leaf_c", "s:leaf_c")).expect("json");
    store
        .set_pane_layout(&theirs, &tab_for_card(&other), &lone, "leaf_c")
        .expect("theirs");
    store
        .set_pane_layout(&mine, "tab_plain", &lone, "leaf_c")
        .expect("plain");

    let found = card_leaves(&store, &mine);
    let mut named: Vec<(&str, &str)> = found
        .iter()
        .map(|(leaf, pane)| (leaf.as_str(), pane.card_id.as_str()))
        .collect();
    named.sort();
    assert_eq!(
        named,
        [("leaf_a", card.as_str()), ("leaf_b", card.as_str())]
    );

    store.archive_card(&card).expect("archive");
    assert!(card_leaves(&store, &mine).is_empty());
}

#[test]
fn an_open_from_the_process_table_does_not_swallow_a_hook_already_on_its_way() {
    let activities = Mutex::new(Activities::default());
    // The hook was accepted (stamped 6) before the poll, and is applied after.
    let told = reconcile_in(
        &activities,
        &leaves(),
        &[front("leaf_a", Some("claude"))],
        7,
    );
    assert_eq!(told[0].activity, Some(Doing::Open));
    let waited = hear(
        &mut activities.lock().expect("lock"),
        pane("leaf_a"),
        6,
        Doing::Waiting,
        Place::default(),
    )
    .expect("the waiting still lands");
    assert_eq!(waited.activity, Some(Doing::Waiting));
}

/// The rebuild starts with the project the person was last in.
///
/// `projects()` answers most-recently-opened first, and this is the rule that
/// depends on it: rebuilding every project at startup would ask tmux and the
/// process table once per project, before the window has painted.
#[test]
fn a_restart_rebuilds_the_last_active_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    let older = store
        .add_project(&dir.path().join("older"), None)
        .expect("older");
    let newer = store
        .add_project(&dir.path().join("newer"), None)
        .expect("newer");
    // Written rather than touched: both were added in the same second, and the
    // tie-break is creation order — which is not what this rule is about.
    store
        .conn()
        .execute(
            "UPDATE project SET last_opened_at = CASE id WHEN ?1 THEN 200 ELSE 100 END",
            [&newer],
        )
        .expect("opened at");

    let projects = store.projects().expect("projects");
    assert_eq!(last_opened(&projects).as_deref(), Some(newer.as_str()));
    assert_ne!(last_opened(&projects).as_deref(), Some(older.as_str()));

    // And with nothing registered there is nothing to rebuild.
    assert_eq!(last_opened(&[]), None);
}
