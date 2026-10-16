use super::*;
use crate::store::NewCard;

/// A store with a project, its board, and an epic in the first lane.
fn seeded() -> (tempfile::TempDir, Store, String, String, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("create");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let epic = store
        .create_card(&project, &column, "[devflow] Brief: search", "")
        .expect("epic");
    (dir, store, project, column, epic)
}

const THREE: [NewCard<'static>; 3] = [
    NewCard {
        title: "[devflow] index the bodies",
        body: "",
        waits_on: &[],
        waits_on_cards: &[],
    },
    NewCard {
        title: "[devflow] search box",
        body: "",
        waits_on: &[0],
        waits_on_cards: &[],
    },
    NewCard {
        title: "[devflow] highlight",
        body: "",
        waits_on: &[1],
        waits_on_cards: &[],
    },
];

#[test]
fn a_waiting_plan_becomes_cards_under_its_epic_once() {
    let (_dir, store, project, column, epic) = seeded();
    // Nothing waits: nothing is created.
    assert_eq!(
        store
            .create_plan(&epic, &column, &THREE, |_| String::new())
            .expect("create"),
        None
    );
    assert!(store.keep_plan(&epic, "[{},{},{}]", false).expect("keep"));
    assert_eq!(
        store.plans_waiting(&project).expect("waiting"),
        [(epic.clone(), 3)]
    );

    let ids = store
        .create_plan(&epic, &column, &THREE, |ids| ids.join(","))
        .expect("create")
        .expect("it was waiting");
    assert_eq!(ids.len(), 3);
    assert_eq!(
        store.plan(&epic).expect("plan").and_then(|row| row.created),
        Some(ids.join(","))
    );
    assert!(store.plans_waiting(&project).expect("waiting").is_empty());

    let edges = store.card_edges(&project).expect("edges");
    assert_eq!(
        edges.parents,
        ids.iter()
            .map(|id| (id.clone(), epic.clone()))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        edges.blockers,
        [
            (ids[1].clone(), ids[0].clone()),
            (ids[2].clone(), ids[1].clone())
        ]
    );

    // A second yes finds it made and makes nothing.
    assert_eq!(
        store
            .create_plan(&epic, &column, &THREE, |_| String::new())
            .expect("create"),
        None
    );
    assert_eq!(store.cards(&project).expect("cards").len(), 4);

    // A new preview waits again.
    store.keep_plan(&epic, "[{}]", true).expect("keep");
    let row = store.plan(&epic).expect("plan").expect("a plan");
    assert!(row.edited && row.created.is_none());
}

#[test]
fn cards_that_would_wait_on_each_other_in_a_circle_are_not_made() {
    let (_dir, store, project, column, epic) = seeded();
    store.keep_plan(&epic, "[{},{}]", false).expect("keep");
    let circle = [
        NewCard {
            title: "a",
            body: "",
            waits_on: &[1],
            waits_on_cards: &[],
        },
        NewCard {
            title: "b",
            body: "",
            waits_on: &[0],
            waits_on_cards: &[],
        },
    ];
    assert!(matches!(
        store.create_plan(&epic, &column, &circle, |_| String::new()),
        Err(StoreError::Circle)
    ));
    // All or none: the first card is not left behind.
    assert_eq!(store.cards(&project).expect("cards").len(), 1);
    assert!(store.plans_waiting(&project).expect("waiting").len() == 1);
}

#[test]
fn a_plan_past_its_ceiling_is_neither_kept_nor_read() {
    let (_dir, store, _project, _column, epic) = seeded();
    assert!(!store
        .keep_plan(&epic, &"x".repeat(MOST_PLAN + 1), false)
        .expect("keep"));
    assert!(store.plan(&epic).expect("plan").is_none());
}

#[test]
fn an_epic_deleted_leaves_its_cards_and_takes_its_plan() {
    let (_dir, store, project, column, epic) = seeded();
    store.keep_plan(&epic, "[{},{},{}]", false).expect("keep");
    let ids = store
        .create_plan(&epic, &column, &THREE, |_| String::new())
        .expect("create")
        .expect("made");
    assert!(store.delete_card(&epic).expect("delete"));
    assert!(store.plan(&epic).expect("plan").is_none());
    let edges = store.card_edges(&project).expect("edges");
    assert!(edges.parents.is_empty());
    assert_eq!(edges.blockers.len(), 2);
    // A card that is deleted no longer blocks anything.
    store.delete_card(&ids[0]).expect("delete");
    assert_eq!(store.card_edges(&project).expect("edges").blockers.len(), 1);
}
