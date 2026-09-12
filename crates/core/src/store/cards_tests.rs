//! What a card carries, and what happens to it when the card goes.

use super::*;
use crate::store::DEFAULT_COLUMNS;

fn seeded() -> (tempfile::TempDir, Store, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("create");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    (dir, store, card)
}

#[test]
fn a_card_starts_with_no_deadline_and_can_be_given_one() {
    let (_dir, store, card) = seeded();
    assert_eq!(
        store.card(&card).expect("read").expect("there").due_at,
        None
    );

    assert!(store.set_card_due(&card, Some(1_800_000_000)).expect("set"));
    assert_eq!(
        store.card(&card).expect("read").expect("there").due_at,
        Some(1_800_000_000)
    );
}

#[test]
fn a_deadline_can_be_taken_away_again() {
    // Clearing has to be possible, or the only way out of a date typed by
    // mistake is to delete the card.
    let (_dir, store, card) = seeded();
    store.set_card_due(&card, Some(1_800_000_000)).expect("set");
    assert!(store.set_card_due(&card, None).expect("clear"));
    assert_eq!(
        store.card(&card).expect("read").expect("there").due_at,
        None
    );
}

#[test]
fn comments_come_back_oldest_first() {
    // A conversation read newest-first is a conversation nobody can follow.
    let (_dir, store, card) = seeded();
    store.add_comment(&card, "you", "first").expect("one");
    store.add_comment(&card, "claude", "second").expect("two");

    let said = store.comments(&card).expect("read");
    assert_eq!(said.len(), 2);
    assert_eq!(said[0].body, "first");
    assert_eq!(said[1].author, "claude");
}

#[test]
fn an_edited_comment_says_that_it_was() {
    let (_dir, store, card) = seeded();
    let id = store.add_comment(&card, "you", "before").expect("add");
    assert_eq!(store.comments(&card).expect("read")[0].edited_at, None);

    assert!(store.edit_comment(&id, "after").expect("edit"));
    let said = &store.comments(&card).expect("read")[0];
    assert_eq!(said.body, "after");
    assert!(said.edited_at.is_some(), "the edit left no mark");
}

#[test]
fn editing_something_that_is_not_there_says_so() {
    let (_dir, store, _card) = seeded();
    assert!(!store.edit_comment("cmt_nothing", "x").expect("edit"));
    assert!(!store.delete_comment("cmt_nothing").expect("delete"));
}

#[test]
fn pinning_the_same_file_twice_is_one_attachment() {
    // People drop the same file twice. The second time means "it is already
    // there", not "put it there again".
    let (_dir, store, card) = seeded();
    let one = store.attach(&card, "/tmp/a.png", "a.png").expect("attach");
    let two = store.attach(&card, "/tmp/a.png", "a.png").expect("again");
    assert_eq!(one, two);
    assert_eq!(store.attachments(&card).expect("read").len(), 1);
}

#[test]
fn unpinning_leaves_the_others() {
    let (_dir, store, card) = seeded();
    store.attach(&card, "/tmp/a.png", "a").expect("attach");
    let second = store.attach(&card, "/tmp/b.png", "b").expect("attach");
    assert!(store.detach(&second).expect("detach"));
    let left = store.attachments(&card).expect("read");
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].path, "/tmp/a.png");
}

#[test]
fn what_hangs_off_a_card_goes_when_the_card_does() {
    // The one rule these three tables share, and the reason they arrived in
    // one migration.
    let (_dir, store, card) = seeded();
    store.add_comment(&card, "you", "said").expect("comment");
    store.attach(&card, "/tmp/a.png", "a").expect("attach");
    store
        .add_notice(None, "run", "done", None, Some(&card))
        .expect("notice");

    store
        .conn()
        .execute("DELETE FROM card WHERE id = ?1", [&card])
        .expect("delete the card");

    assert!(store.comments(&card).expect("read").is_empty());
    assert!(store.attachments(&card).expect("read").is_empty());
    assert!(store.notices(50).expect("read").is_empty());
}

#[test]
fn the_bell_counts_only_what_has_not_been_read() {
    let (_dir, store, card) = seeded();
    let one = store
        .add_notice(None, "run", "a run finished", None, Some(&card))
        .expect("notice");
    store
        .add_notice(None, "agent", "waiting on you", None, Some(&card))
        .expect("notice");
    assert_eq!(store.unread_notices().expect("count"), 2);

    assert!(store.read_notice(&one).expect("read"));
    assert_eq!(store.unread_notices().expect("count"), 1);
    // Reading twice is not two reads.
    assert!(!store.read_notice(&one).expect("again"));
    assert_eq!(store.unread_notices().expect("count"), 1);

    assert_eq!(store.read_all_notices().expect("all"), 1);
    assert_eq!(store.unread_notices().expect("count"), 0);
}

#[test]
fn notices_come_back_newest_first() {
    let (_dir, store, _card) = seeded();
    store
        .add_notice(None, "run", "older", None, None)
        .expect("a");
    store
        .add_notice(None, "run", "newer", None, None)
        .expect("b");
    let shown = store.notices(10).expect("read");
    assert_eq!(shown[0].title, "newer");
}

#[test]
fn the_notice_list_does_not_grow_without_bound() {
    // Trimmed on write rather than on read: the read is what a person waits
    // for, and a list nobody trims is one day a megabyte.
    let (_dir, store, _card) = seeded();
    for n in 0..(NOTICES_KEPT + 25) {
        store
            .add_notice(None, "run", &format!("notice {n}"), None, None)
            .expect("notice");
    }
    let kept: i64 = store
        .conn()
        .query_row("SELECT COUNT(*) FROM notice", [], |row| row.get(0))
        .expect("count");
    assert_eq!(kept, NOTICES_KEPT);
    // The newest survived, not the oldest.
    assert_eq!(
        store.notices(1).expect("read")[0].title,
        format!("notice {}", NOTICES_KEPT + 24)
    );
}

#[test]
fn only_cards_past_their_date_are_overdue() {
    let (_dir, store, card) = seeded();
    let column = {
        let project = store
            .project_id_of_card(&card)
            .expect("project")
            .expect("id");
        store.columns(&project).expect("columns")[0].id.clone()
    };
    let project = store
        .project_id_of_card(&card)
        .expect("project")
        .expect("id");
    let later = store
        .create_card(&project, &column, "later", "")
        .expect("card");

    store.set_card_due(&card, Some(1_000)).expect("past");
    store.set_card_due(&later, Some(9_000)).expect("future");

    let past = store.overdue_cards(5_000).expect("overdue");
    assert_eq!(past.len(), 1);
    assert_eq!(past[0].0, card);
}

#[test]
fn the_default_board_is_a_seed_and_not_a_contract() {
    // Restated here because these tests create cards by taking `columns()[0]`,
    // which would quietly depend on the seed's order otherwise.
    assert_eq!(DEFAULT_COLUMNS.len(), 6);
}
