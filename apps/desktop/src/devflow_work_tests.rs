use super::*;

fn board() -> (tempfile::TempDir, Store, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("root");
    let project = store.add_project(&root, None).expect("project");
    (dir, store, project)
}

fn new(title: &str, parent: Option<&str>, depends_on: &[&str]) -> New {
    New {
        title: title.to_owned(),
        kind: Kind::Item,
        parent: parent.map(str::to_owned),
        depends_on: depends_on.iter().map(|one| (*one).to_owned()).collect(),
        body: "## Acceptance criteria\n- [ ] it works".to_owned(),
    }
}

/// Sabotage: drop the parent or the blockers in `create`, and the board no
/// longer holds the plan the engine made.
#[test]
fn a_work_item_is_a_card_under_its_epic_waiting_on_its_blockers() {
    let (_dir, store, project) = board();
    let items = Board {
        store: &store,
        project_id: &project,
    };
    let at = SystemTime::now();
    let epic = items
        .create(
            &New {
                kind: Kind::Epic,
                ..new(&brief::epic_title("Rename"), None, &[])
            },
            at,
        )
        .expect("epic");
    assert_eq!(epic.kind, Kind::Epic);
    let first = items
        .create(&new("first", Some(&epic.id), &[]), at)
        .expect("first");
    let second = items
        .create(&new("second", Some(&epic.id), &[&first.id]), at)
        .expect("second");

    let read = items.get(&second.id).expect("read").expect("there");
    assert_eq!(read.parent.as_deref(), Some(epic.id.as_str()));
    assert_eq!(read.depends_on, vec![first.id.clone()]);
    assert_eq!(read.state, State::Open);
    assert_eq!(
        store.card_edges(&project).expect("edges").blockers,
        vec![(second.id.clone(), first.id.clone())]
    );
    let listed: Vec<String> = items
        .list()
        .expect("list")
        .into_iter()
        .map(|one| one.id)
        .collect();
    assert_eq!(listed, vec![epic.id, first.id, second.id]);
}

/// Sabotage: map every state to the first lane, and a finished item never
/// reaches done.
#[test]
fn an_items_state_is_the_role_of_its_cards_lane() {
    let (_dir, store, project) = board();
    let items = Board {
        store: &store,
        project_id: &project,
    };
    let one = items
        .create(&new("one", None, &[]), SystemTime::now())
        .expect("one");
    assert_eq!(
        items.set_state(&one.id, State::Doing).expect("doing").state,
        State::Doing
    );
    assert_eq!(
        items.set_state(&one.id, State::Done).expect("done").state,
        State::Done
    );
    assert_eq!(
        items.set_state(&one.id, State::Open).expect("open").state,
        State::Open
    );
}

#[test]
fn an_item_is_retitled_rewritten_commented_and_closed_on_its_card() {
    let (_dir, store, project) = board();
    let items = Board {
        store: &store,
        project_id: &project,
    };
    let at = SystemTime::now();
    let one = items.create(&new("one", None, &[]), at).expect("one");
    let other = items.create(&new("other", None, &[]), at).expect("other");
    items.retitle(&one.id, "renamed").expect("retitled");
    items.update_body(&one.id, "a new body").expect("rewritten");
    items
        .comment(&one.id, "agent", "the handoff", at)
        .expect("said");
    let card = store.card(&one.id).expect("read").expect("there");
    assert_eq!(
        (card.title.as_str(), card.body.as_str()),
        ("renamed", "a new body")
    );
    let said = items.comments(&one.id).expect("comments");
    assert_eq!(
        (said[0].by.as_str(), said[0].text.as_str()),
        ("agent", "the handoff")
    );

    let closed = items
        .close(&one.id, Closed::Into(other.id.clone()))
        .expect("merged");
    assert_eq!(closed.closed, Some(Closed::Into(other.id.clone())));
    assert!(items
        .close(&other.id, Closed::Into(other.id.clone()))
        .is_err());
}

/// Sabotage: drop the project check in `own`, and one project's agent reads
/// and writes another's cards.
#[test]
fn another_projects_card_is_no_item_of_this_board() {
    let (dir, store, project) = board();
    let root = dir.path().join("other");
    std::fs::create_dir_all(&root).expect("root");
    let other = store.add_project(&root, None).expect("project");
    let theirs = Board {
        store: &store,
        project_id: &other,
    }
    .create(&new("theirs", None, &[]), SystemTime::now())
    .expect("theirs");
    let ours = Board {
        store: &store,
        project_id: &project,
    };
    assert_eq!(ours.get(&theirs.id).expect("read"), None);
    assert!(ours.retitle(&theirs.id, "taken").is_err());
    assert!(ours
        .create(&new("mine", None, &[&theirs.id]), SystemTime::now())
        .is_err());
}
