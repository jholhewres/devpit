use super::*;
use crate::store::migrations;

fn seeded() -> (tempfile::TempDir, Store, String, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("create");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    (dir, store, project, column)
}

#[test]
fn cards_are_numbered_in_the_order_they_are_made_per_project() {
    let (dir, store, project, column) = seeded();
    let first = store.create_card(&project, &column, "a", "").expect("a");
    let second = store.create_card(&project, &column, "b", "").expect("b");
    let other_root = dir.path().join("other");
    std::fs::create_dir_all(&other_root).expect("create");
    let other = store.add_project(&other_root, None).expect("project");
    store.ensure_board(&other).expect("board");
    let other_column = store.columns(&other).expect("columns")[0].id.clone();
    let elsewhere = store
        .create_card(&other, &other_column, "c", "")
        .expect("c");

    assert_eq!(store.card_tally(&first).expect("read").number, Some(1));
    assert_eq!(store.card_tally(&second).expect("read").number, Some(2));
    assert_eq!(store.card_tally(&elsewhere).expect("read").number, Some(1));
    assert_eq!(
        store.card_numbered(&project, 2).expect("read"),
        Some(second)
    );
    assert_eq!(store.card_numbered(&project, 3).expect("read"), None);
}

#[test]
fn a_number_is_never_handed_out_again() {
    let (_dir, store, project, column) = seeded();
    let first = store.create_card(&project, &column, "a", "").expect("a");
    store.archive_card(&first).expect("archive");
    let next = store.create_card(&project, &column, "b", "").expect("b");
    assert_eq!(store.card_tally(&next).expect("read").number, Some(2));
    assert_eq!(store.card_numbered(&project, 1).expect("read"), Some(first));
}

#[test]
fn a_planned_card_takes_the_next_number_too() {
    let (_dir, store, project, column) = seeded();
    let epic = store
        .create_card(&project, &column, "epic", "")
        .expect("epic");
    assert!(store.keep_plan(&epic, "[{}]", false).expect("keep"));
    let one = [crate::NewCard {
        title: "s1",
        body: "",
        waits_on: &[],
    }];
    let planned = store
        .create_plan(&epic, &column, &one, |_| String::new())
        .expect("create")
        .expect("it was waiting");
    assert_eq!(store.card_tally(&planned[0]).expect("read").number, Some(2));
}

#[test]
fn the_migration_numbers_the_cards_already_there_by_age() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Foreign keys left off: the cards are all this needs. Ids sort against
    // their age, so numbering by id would fail.
    let conn = rusqlite::Connection::open(dir.path().join("state.db")).expect("open");
    conn.pragma_update(None, "foreign_keys", "OFF").expect("fk");
    migrations::run_up_to(&conn, 28).expect("to 28");
    conn.execute_batch(
        "INSERT INTO card (id, project_id, column_id, title, position, created_at, updated_at)
           VALUES ('card_a', 'p', 'c', 'newest', 0, 30, 30),
                  ('card_m', 'p', 'c', 'archived', 1, 20, 20),
                  ('card_z', 'p', 'c', 'oldest', 2, 10, 10);
         UPDATE card SET archived_at = 40 WHERE id = 'card_m';",
    )
    .expect("rows");
    migrations::run(&conn, dir.path()).expect("to latest");
    let number = |id: &str| -> i64 {
        conn.query_row("SELECT number FROM card WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .expect("number")
    };
    assert_eq!(
        (number("card_z"), number("card_m"), number("card_a")),
        (1, 2, 3)
    );
}

#[test]
fn a_number_is_read_with_or_without_its_project() {
    assert_eq!(card_number_of("#42"), Some((None, 42)));
    assert_eq!(card_number_of("42"), Some((None, 42)));
    assert_eq!(card_number_of(" devpit#7 "), Some((Some("devpit"), 7)));
    assert_eq!(card_number_of("card_01M4VJA217154VNZPXH6CPF026"), None);
    assert_eq!(card_number_of("#"), None);
    assert_eq!(card_number_of("#0"), None);
    assert_eq!(card_number_of("#-3"), None);
    assert_eq!(card_number_of("42a"), None);
}

#[test]
fn a_tally_counts_what_the_tile_shows_for_each_card_at_once() {
    let (_dir, store, project, column) = seeded();
    let talked = store.create_card(&project, &column, "a", "").expect("a");
    let quiet = store.create_card(&project, &column, "b", "").expect("b");
    store.add_comment(&talked, "you", "one").expect("one");
    store.add_comment(&talked, "you", "two").expect("two");
    let tallies = store.card_tallies(&project).expect("tallies");
    let expected = CardTally {
        comments: 2,
        pinned: 0,
        cost_usd: 0.0,
        number: Some(1),
    };
    assert_eq!(tallies[&talked], expected);
    assert_eq!(tallies[&quiet].comments, 0);
    assert_eq!(store.card_tally(&talked).expect("one"), expected);
    assert_eq!(
        store.card_tally("card_gone").expect("none"),
        CardTally::default()
    );
}

/// A card an older devpit made on this file has no number until the next
/// start gives it one, after the project's highest and never a taken one.
#[test]
fn a_card_made_without_a_number_gets_the_next_one_at_the_start() {
    let (_dir, store, project, column) = seeded();
    let first = store.create_card(&project, &column, "a", "").expect("a");
    let older = store.create_card(&project, &column, "b", "").expect("b");
    let oldest = store.create_card(&project, &column, "c", "").expect("c");
    store
        .conn
        .execute(
            "UPDATE card SET number = NULL, \
             created_at = CASE id WHEN ?1 THEN 100 ELSE 200 END WHERE id IN (?1, ?2)",
            rusqlite::params![older, oldest],
        )
        .expect("as an older devpit left them");

    assert_eq!(store.number_unnumbered().expect("numbered"), 2);
    assert_eq!(store.card_tally(&first).expect("read").number, Some(1));
    assert_eq!(store.card_tally(&older).expect("read").number, Some(2));
    assert_eq!(store.card_tally(&oldest).expect("read").number, Some(3));
    assert_eq!(store.number_unnumbered().expect("nothing left"), 0);
    let numbers = store.card_numbers(&project).expect("numbers");
    assert_eq!(numbers.get(&oldest), Some(&3));
}
