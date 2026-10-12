use devpit_core::NewCard;

use super::*;

/// A board with an epic whose plan made three cards, each waiting on the one
/// before it.
fn planned() -> (tempfile::TempDir, Store, String, String, Vec<String>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("create");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let first = store.columns(&project).expect("columns")[0].id.clone();
    let epic = store
        .create_card(&project, &first, "[devflow] Brief: search", "")
        .expect("epic");
    store.keep_plan(&epic, "[{},{},{}]", false).expect("plan");
    let cards = [
        NewCard {
            title: "[devflow] index",
            body: "",
            waits_on: &[],
        },
        NewCard {
            title: "[devflow] box",
            body: "",
            waits_on: &[0],
        },
        NewCard {
            title: "[devflow] highlight",
            body: "",
            waits_on: &[1],
        },
    ];
    let ids = store
        .create_plan(&epic, &first, &cards, |_| String::new())
        .expect("create")
        .expect("made");
    (dir, store, project, epic, ids)
}

fn card(id: &str) -> Card {
    serde_json::from_value(serde_json::json!({
        "id": id, "columnId": "", "title": "", "body": "", "position": 0, "worktreePath": null,
        "dueAt": null, "dueTime": false, "costUsd": 0.0, "comments": 0, "pinned": 0, "runs": [],
        "activity": null,
    }))
    .expect("a card")
}

#[test]
fn a_card_says_what_blocks_it_until_that_is_done_and_its_epic_counts_what_is_done() {
    let (_dir, store, project, epic, ids) = planned();
    let ties = Ties::read(&store, &project).expect("ties");
    let mut epic_card = card(&epic);
    ties.fill(&mut epic_card);
    assert_eq!(epic_card.progress, Some(Progress { done: 0, total: 3 }));
    let mut second = card(&ids[1]);
    ties.fill(&mut second);
    assert_eq!(second.parent_id.as_deref(), Some(epic.as_str()));
    assert_eq!(
        second.blocked_by,
        [CardRef {
            id: ids[0].clone(),
            title: "[devflow] index".into(),
            done: false
        }]
    );

    // The default board's `ship` is where done work sits.
    let columns = store.columns(&project).expect("columns");
    let ship = columns
        .iter()
        .find(|column| column.name == "ship")
        .expect("ship");
    store.move_card(&ids[0], &ship.id, 0).expect("move");
    let ties = Ties::read(&store, &project).expect("ties");
    ties.fill(&mut epic_card);
    ties.fill(&mut second);
    assert_eq!(epic_card.progress, Some(Progress { done: 1, total: 3 }));
    assert!(second.blocked_by[0].done);
    assert_eq!(ties.children(&epic).len(), 3);
}

#[test]
fn a_card_with_no_children_has_no_progress() {
    assert_eq!(progress(&[]), None);
    let child = CardRef {
        id: "c".into(),
        title: "c".into(),
        done: true,
    };
    assert_eq!(progress(&[child]), Some(Progress { done: 1, total: 1 }));
}

#[test]
fn a_cards_criteria_are_read_from_its_body() {
    let body = "Build it.\n\n## Acceptance criteria\n- [x] one\n- [ ] two\n";
    assert_eq!(
        criteria_of(body),
        [
            Criterion {
                text: "one".into(),
                done: true
            },
            Criterion {
                text: "two".into(),
                done: false
            }
        ]
    );
}
