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
    let one = store
        .attach(&card, "/tmp/a.png", "a.png", None)
        .expect("attach");
    let two = store
        .attach(&card, "/tmp/a.png", "a.png", None)
        .expect("again");
    assert_eq!(one, two);
    assert_eq!(store.attachments(&card).expect("read").len(), 1);
}

#[test]
fn a_pin_keeps_the_plugin_that_made_it() {
    let (_dir, store, card) = seeded();
    // Looked up by path: two pins in one second have no order.
    let plugin_of = |path: &str| {
        store
            .attachments(&card)
            .expect("read")
            .into_iter()
            .find(|pin| pin.path == path)
            .expect("pinned")
            .plugin_id
    };
    store
        .attach(&card, "/tmp/a.png", "a", None)
        .expect("by hand");
    store
        .attach(&card, "/tmp/flow.excalidraw", "flow", Some("excalidraw"))
        .expect("by its plugin");
    assert_eq!(plugin_of("/tmp/a.png"), None);
    assert_eq!(
        plugin_of("/tmp/flow.excalidraw").as_deref(),
        Some("excalidraw")
    );

    // Pinned by hand first and by its plugin after: still one pin, and it
    // opens in the plugin; pinning it by hand again does not take that away.
    store
        .attach(&card, "/tmp/b.excalidraw", "b", None)
        .expect("by hand");
    store
        .attach(&card, "/tmp/b.excalidraw", "b", Some("excalidraw"))
        .expect("by its plugin");
    store
        .attach(&card, "/tmp/b.excalidraw", "b", None)
        .expect("by hand again");
    assert_eq!(
        plugin_of("/tmp/b.excalidraw").as_deref(),
        Some("excalidraw")
    );
    assert_eq!(store.attachments(&card).expect("read").len(), 3);
}

#[test]
fn unpinning_leaves_the_others() {
    let (_dir, store, card) = seeded();
    store
        .attach(&card, "/tmp/a.png", "a", None)
        .expect("attach");
    let second = store
        .attach(&card, "/tmp/b.png", "b", None)
        .expect("attach");
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
    store
        .attach(&card, "/tmp/a.png", "a", None)
        .expect("attach");
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
    // for, and a list nobody trims is one day a megabyte. Read ones are what
    // the ordinary ceiling takes; unread ones have their own, higher one, so
    // a focus holding something back still finds it in the store.
    let (_dir, store, _card) = seeded();
    for n in 0..(NOTICES_KEPT + 25) {
        let id = store
            .add_notice(None, "run", &format!("notice {n}"), None, None)
            .expect("notice");
        store.read_notice(&id).expect("read");
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

/// Unread ones are kept, but not for ever.
///
/// Somebody who never opens the bell must not grow the table without end, so
/// the higher ceiling is real and this proves it rather than trusting it.
#[test]
fn even_an_unread_notice_has_a_ceiling() {
    let (_dir, store, _card) = seeded();
    for n in 0..(NOTICES_KEPT_UNREAD + 25) {
        store
            .add_notice(None, "run", &format!("notice {n}"), None, None)
            .expect("notice");
    }
    let kept: i64 = store
        .conn()
        .query_row("SELECT COUNT(*) FROM notice", [], |row| row.get(0))
        .expect("count");
    assert_eq!(kept, NOTICES_KEPT_UNREAD);
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

#[test]
fn deleting_a_card_takes_its_comments_pins_and_runs() {
    let (dir, store, card) = seeded();
    let project = store
        .project_id_of_card(&card)
        .expect("read")
        .expect("there");
    store.add_comment(&card, "you", "a line").expect("comment");
    let file = dir.path().join("project").join("notes.md");
    std::fs::write(&file, "x").expect("write");
    store
        .attach(&card, file.to_str().expect("utf8"), "notes", None)
        .expect("pin");
    let step = store
        .create_step(&project, "command", "tests", "make test", false)
        .expect("step");
    store.start_run(&card, &step, None).expect("run");

    assert!(store.delete_card(&card).expect("delete"));
    assert!(store.card(&card).expect("read").is_none());
    assert!(store.comments(&card).expect("comments").is_empty());
    assert!(store.attachments(&card).expect("pins").is_empty());
    assert!(store.runs(&card).expect("runs").is_empty());
    // The step belongs to the lane, and stays.
    assert_eq!(store.steps(&project).expect("steps").len(), 1);
    assert!(!store.delete_card(&card).expect("again"));
}

fn column_and_project(store: &Store, card: &str) -> (String, String) {
    let row = store.card(card).expect("read").expect("there");
    let project = store
        .project_id_of_card(card)
        .expect("read")
        .expect("there");
    (row.column_id, project)
}

#[test]
fn a_restored_card_comes_back_at_the_end_of_its_lane() {
    let (_dir, store, card) = seeded();
    let (column, project) = column_and_project(&store, &card);
    store.archive_card(&card).expect("archive");
    let other = store
        .create_card(&project, &column, "other", "")
        .expect("card");

    assert!(store.restore_card(&card).expect("restore"));
    let back = store.card(&card).expect("read").expect("there");
    assert_eq!(back.column_id, column);
    let after = store.card(&other).expect("read").expect("there").position;
    assert!(back.position > after);
    assert!(store
        .cards(&project)
        .expect("cards")
        .iter()
        .any(|row| row.id == card));
    // Restoring twice is not restoring anything.
    assert!(!store.restore_card(&card).expect("again"));
}

#[test]
fn a_card_whose_lane_is_gone_comes_back_to_the_first_lane() {
    let (_dir, store, card) = seeded();
    let (_, project) = column_and_project(&store, &card);
    let columns = store.columns(&project).expect("columns");
    store.move_card(&card, &columns[1].id, 0).expect("move");
    store.archive_card(&card).expect("archive");
    // Behind the constraint's back: the lane goes while the card is archived.
    store
        .conn
        .execute_batch(&format!(
            "PRAGMA foreign_keys = OFF; DELETE FROM board_column WHERE id = '{}'; PRAGMA foreign_keys = ON;",
            columns[1].id
        ))
        .expect("drop lane");

    assert!(store.restore_card(&card).expect("restore"));
    let back = store.card(&card).expect("read").expect("there");
    assert_eq!(back.column_id, columns[0].id);
}

#[test]
fn archived_cards_are_listed_newest_first_and_capped() {
    let (_dir, store, card) = seeded();
    let (column, project) = column_and_project(&store, &card);
    let later = store
        .create_card(&project, &column, "later", "")
        .expect("card");
    store.archive_card(&card).expect("archive");
    store
        .conn
        .execute(
            "UPDATE card SET archived_at = archived_at - 10 WHERE id = ?1",
            [&card],
        )
        .expect("older");
    store.archive_card(&later).expect("archive");

    let listed = store.archived_cards(&project, 200).expect("list");
    let ids: Vec<&str> = listed.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, [later.as_str(), card.as_str()]);
    assert_eq!(store.archived_cards(&project, 1).expect("capped").len(), 1);
}

#[test]
fn a_lane_deleted_with_cards_moves_them_in_order_to_the_end_of_another() {
    let (_dir, store, card) = seeded();
    let (column, project) = column_and_project(&store, &card);
    let columns = store.columns(&project).expect("columns");
    let second = store
        .create_card(&project, &column, "second", "")
        .expect("card");
    let archived = store
        .create_card(&project, &column, "archived", "")
        .expect("card");
    store.archive_card(&archived).expect("archive");
    let already = store
        .create_card(&project, &columns[1].id, "already there", "")
        .expect("card");

    store
        .delete_column_moving_cards(&column, &columns[1].id)
        .expect("delete");

    assert!(store
        .columns(&project)
        .expect("columns")
        .iter()
        .all(|lane| lane.id != column));
    let order: Vec<String> = store
        .cards(&project)
        .expect("cards")
        .into_iter()
        .filter(|row| row.column_id == columns[1].id)
        .map(|row| row.id)
        .collect();
    assert_eq!(order, [already, card, second]);
    let moved = store.card(&archived).expect("read").expect("there");
    assert_eq!(moved.column_id, columns[1].id);
}

/// A focus can hold a notice for an afternoon, so the store must still have it.
///
/// The trim took the oldest rows whatever their state, so 200 notices from a
/// busy project threw away what somebody had not looked at on another one —
/// and a queue holding those back to show them on the way out would have been
/// holding rows that no longer existed. Sabotage: drop `read_at IS NOT NULL`
/// from the first delete and the unread one disappears here.

/// Two projects on disk, because a notice's project is a foreign key.
fn two_projects(dir: &std::path::Path, store: &Store) -> (String, String) {
    let mut made = Vec::new();
    for name in ["mine", "far"] {
        let root = dir.join(name);
        std::fs::create_dir_all(&root).expect("create");
        made.push(store.add_project(&root, None).expect("project"));
    }
    (made[0].clone(), made[1].clone())
}

#[test]
fn the_trim_takes_what_was_read_and_leaves_what_was_not() {
    let (dir, store, _card) = seeded();
    let (mine, far) = two_projects(dir.path(), &store);
    let held = store
        .add_notice(Some(&far), "run", "nobody looked at this", None, None)
        .expect("held");

    // Enough traffic to push it well past the ordinary ceiling, all of it read.
    for n in 0..(NOTICES_KEPT + 50) {
        let id = store
            .add_notice(Some(&mine), "run", &format!("notice {n}"), None, None)
            .expect("notice");
        store.read_notice(&id).expect("read");
    }

    let kept: i64 = store
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM notice WHERE id = ?1",
            [&held],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(kept, 1, "the unread notice was trimmed away under a focus");
}

/// The query behind a focus: this project's own, or everything that is not.
#[test]
fn notices_come_back_by_project_and_since_in_pages() {
    let (dir, store, _card) = seeded();
    let (here, there) = two_projects(dir.path(), &store);
    let mine = store
        .add_notice(Some(&here), "run", "mine", None, None)
        .expect("mine");
    let far = store
        .add_notice(Some(&there), "run", "far", None, None)
        .expect("far");
    let nowhere = store
        .add_notice(None, "run", "no project at all", None, None)
        .expect("nowhere");

    let ours = store
        .notices_since(&here, false, 0, None, 50)
        .expect("ours");
    assert_eq!(ours.iter().map(|row| &row.id).collect::<Vec<_>>(), [&mine]);

    // Elsewhere is another project, never an unknown one: a notice with no
    // project is not "from somewhere else", it is one whose project is gone.
    let elsewhere = store
        .notices_since(&here, true, 0, None, 50)
        .expect("elsewhere");
    assert_eq!(
        elsewhere.iter().map(|row| &row.id).collect::<Vec<_>>(),
        [&far]
    );
    assert!(!elsewhere.iter().any(|row| row.id == nowhere));

    // Nothing from before the focus began.
    let later = store
        .notices_since(&here, false, now() + 60, None, 50)
        .expect("later");
    assert!(later.is_empty());
}

/// More than the old ceiling of a hundred, walked one page at a time.
#[test]
fn a_long_focus_reads_every_held_notice_across_pages() {
    let (dir, store, _card) = seeded();
    let (here, there) = two_projects(dir.path(), &store);
    for n in 0..150 {
        store
            .add_notice(Some(&there), "run", &format!("held {n}"), None, None)
            .expect("notice");
    }

    let mut seen = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..20 {
        let page = store
            .notices_since(&here, true, 0, after.as_deref(), 40)
            .expect("page");
        if page.is_empty() {
            break;
        }
        after = Some(page[page.len() - 1].id.clone());
        seen.extend(page.into_iter().map(|row| row.id));
    }

    assert_eq!(seen.len(), 150, "a page was skipped or read twice");
    let mut unique = seen.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 150);
}
