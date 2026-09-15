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
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    (dir, store, project, card)
}

#[test]
fn a_card_with_nothing_going_on_has_no_links() {
    let (_dir, store, _project, card) = seeded();
    let links = store.card_links(&card).expect("links");
    assert!(links.runs.is_empty());
    assert!(links.background.is_none());
    assert!(links.chats.is_empty());
}

#[test]
fn a_run_is_linked_once_it_has_a_session_of_its_own() {
    let (_dir, store, project, card) = seeded();
    let step = store
        .create_step(&project, "agent", "review", "{}", false)
        .expect("step");
    let spoke = store.start_run(&card, &step, None).expect("run");
    store
        .start_run(&card, &step, None)
        .expect("a run with no session");
    store.set_run_session(&spoke, "s-1").expect("session");
    store.set_run_cwd(&spoke, "/w/card").expect("cwd");
    assert_eq!(
        store.run_session(&spoke).expect("read").as_deref(),
        Some("s-1")
    );

    let links = store.card_links(&card).expect("links");
    assert_eq!(links.runs.len(), 1);
    assert_eq!(links.runs[0].run_id, spoke);
    assert_eq!(links.runs[0].session_id, "s-1");
    assert_eq!(links.runs[0].cwd.as_deref(), Some("/w/card"));
}

#[test]
fn a_background_session_is_linked_with_the_folder_it_runs_in() {
    let (_dir, store, _project, card) = seeded();
    store
        .link_session(&card, "a0a0", "s-old", None, Some("/w/old"))
        .expect("an earlier run's session");
    // The next run's session takes its place, folder included.
    store
        .link_session(&card, "a1b2", "s-bg", None, Some("/w/card"))
        .expect("link");

    let background = store
        .card_links(&card)
        .expect("links")
        .background
        .expect("linked");
    assert_eq!(background.session_id, "s-bg");
    assert_eq!(background.cwd.as_deref(), Some("/w/card"));
}

#[test]
fn a_cards_chats_are_listed_in_order_and_go_with_the_card() {
    let (_dir, store, _project, card) = seeded();
    store.link_chat(&card, "conv_first").expect("first");
    store.link_chat(&card, "conv_later").expect("later");
    store
        .conn
        .execute(
            "UPDATE card_chat SET created_at = 10 WHERE conversation_id = 'conv_first'",
            [],
        )
        .expect("older");
    assert_eq!(
        store.chat_card("conv_later").expect("read").as_deref(),
        Some(card.as_str())
    );

    let chats: Vec<String> = store
        .card_links(&card)
        .expect("links")
        .chats
        .into_iter()
        .map(|chat| chat.conversation_id)
        .collect();
    assert_eq!(chats, ["conv_first", "conv_later"]);

    store.delete_card(&card).expect("delete");
    assert_eq!(store.chat_card("conv_later").expect("read"), None);
    let left: i64 = store
        .conn
        .query_row("SELECT COUNT(*) FROM card_chat", [], |row| row.get(0))
        .expect("count");
    assert_eq!(left, 0);
}

#[test]
fn migration_14_adds_the_links_beside_what_was_there() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = rusqlite::Connection::open(dir.path().join("state.db")).expect("open");
    conn.pragma_update(None, "foreign_keys", "ON").expect("fk");
    let columns = |table: &str| -> Vec<String> {
        let mut stmt = conn
            .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .expect("info");
        stmt.query_map([], |row| row.get(0))
            .expect("rows")
            .collect::<Result<Vec<String>, _>>()
            .expect("names")
    };

    migrations::run_up_to(&conn, 13).expect("to 13");
    assert!(!columns("run").contains(&"session_id".to_owned()));
    assert!(columns("card_chat").is_empty());

    // The real runner, which starts from the version the file is at.
    migrations::run(&conn, dir.path()).expect("to 14");
    let run = columns("run");
    assert!(run.contains(&"session_id".to_owned()) && run.contains(&"cwd".to_owned()));
    assert!(columns("session_link").contains(&"cwd".to_owned()));
    assert_eq!(
        columns("card_chat"),
        ["conversation_id", "card_id", "created_at"]
    );
}

#[test]
fn only_a_card_still_on_a_board_answers_with_its_project() {
    let (_dir, store, project, card) = seeded();
    assert_eq!(store.live_card_project(&card).expect("read"), Some(project));
    store.archive_card(&card).expect("archive");
    assert_eq!(store.live_card_project(&card).expect("read"), None);
    assert_eq!(store.live_card_project("card_gone").expect("read"), None);
}

#[test]
fn only_tabs_a_card_named_are_card_tabs() {
    let (_dir, store, project, card) = seeded();
    let named = format!("tab_card_{card}");
    store
        .set_pane_layout(&project, &named, "{}", "leaf")
        .expect("card tab");
    store
        .set_pane_layout(&project, "tabXcardYz", "{}", "leaf")
        .expect("look-alike");
    store
        .set_pane_layout(&project, "tab_plain", "{}", "leaf")
        .expect("plain");
    let tabs: Vec<String> = store
        .card_tab_layouts()
        .expect("layouts")
        .into_iter()
        .map(|layout| layout.tab_id)
        .collect();
    assert_eq!(tabs, [named]);
}
