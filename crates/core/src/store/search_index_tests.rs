use super::*;

use crate::Store;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    (dir, store)
}

/// Everything below depends on this. Checked first and on its own, so a
/// SQLite built without FTS5 fails here by name rather than as a confusing
/// "no such module" in the middle of an indexing test.
#[test]
fn the_bundled_sqlite_has_fts5() {
    let conn = rusqlite::Connection::open_in_memory().expect("memory");
    conn.execute_batch("CREATE VIRTUAL TABLE probe USING fts5(text)")
        .expect("FTS5 is compiled into the bundled SQLite");
}

/// Orca's bug #20261: a file rewritten to the size it already had kept its old
/// rows. The mtime is part of the key, so the rewrite is read again.
#[test]
fn a_rewrite_to_the_same_size_is_still_stale() {
    assert!(stale(None, 100, 1));
    assert!(!stale(Some((100, 1)), 100, 1));
    assert!(stale(Some((100, 1)), 100, 2));
    assert!(stale(Some((100, 1)), 120, 1));
}

#[test]
fn a_query_finds_its_session_with_the_words_marked() {
    let (_dir, store) = store();
    let conn = store.conn();
    replace_file(
        conn,
        &TranscriptFile {
            path: "/i/projects/-work-demo/aaa.jsonl",
            session_id: "aaa",
            project: "prj_1",
            installation: "/i",
            size: 10,
            mtime: 1,
        },
        &[
            ("user", "why does the lexer drop tabs"),
            ("assistant", "It trims leading whitespace."),
        ],
    )
    .expect("index");

    let hits = search(conn, "lexer", Some("prj_1"), 10).expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].session_id, "aaa");
    assert!(hits[0].snippet.contains("[lexer]"), "{}", hits[0].snippet);
    // Prefix search while typing.
    assert_eq!(
        search(conn, "lex", Some("prj_1"), 10)
            .expect("search")
            .len(),
        1
    );
    // Another project sees none of it.
    assert!(search(conn, "lexer", Some("prj_2"), 10)
        .expect("search")
        .is_empty());
}

#[test]
fn reindexing_a_file_replaces_its_rows_rather_than_adding() {
    let (_dir, store) = store();
    let conn = store.conn();
    let path = "/i/projects/-work-demo/aaa.jsonl";
    let said = |text: &'static str| vec![("user", text)];
    replace_file(
        conn,
        &TranscriptFile {
            path,
            session_id: "aaa",
            project: "prj_1",
            installation: "/i",
            size: 10,
            mtime: 1,
        },
        &said("parser rewrite"),
    )
    .expect("first");
    replace_file(
        conn,
        &TranscriptFile {
            path,
            session_id: "aaa",
            project: "prj_1",
            installation: "/i",
            size: 12,
            mtime: 2,
        },
        &said("lexer rewrite"),
    )
    .expect("second");
    assert!(search(conn, "parser", None, 10).expect("search").is_empty());
    assert_eq!(search(conn, "lexer", None, 10).expect("search").len(), 1);
    assert_eq!(recorded(conn, path).expect("recorded"), Some((12, 2)));
}

#[test]
fn what_a_person_types_is_never_a_syntax_error() {
    let (_dir, store) = store();
    for typed in [r#"don't"#, "a-b", r#"say "hi"#, "OR", "NEAR(", "*"] {
        assert!(search(store.conn(), typed, None, 10).is_ok(), "{typed}");
    }
    assert_eq!(expression("   "), None);
}

/// A transcript deleted from disk leaves the index with the next refresh, so
/// no hit points at a conversation that is gone.
#[test]
fn a_deleted_transcript_leaves_the_index() {
    let (_dir, store) = store();
    let conn = store.conn();
    for (path, id) in [("/i/p/kept.jsonl", "kept"), ("/i/p/gone.jsonl", "gone")] {
        replace_file(
            conn,
            &TranscriptFile {
                path,
                session_id: id,
                project: "prj_1",
                installation: "/i",
                size: 1,
                mtime: 1,
            },
            &[("user", "the lexer again")],
        )
        .expect("index");
    }

    let dropped = forget_missing(conn, "/i/p", &["/i/p/kept.jsonl".to_owned()]).expect("forget");
    assert_eq!(dropped, 1);
    let hits = search(conn, "lexer", Some("prj_1"), 10).expect("search");
    assert_eq!(
        hits.iter()
            .map(|hit| hit.session_id.as_str())
            .collect::<Vec<_>>(),
        ["kept"]
    );
    assert_eq!(recorded(conn, "/i/p/gone.jsonl").expect("read"), None);
    // Another folder is not touched by a refresh of this one.
    assert_eq!(forget_missing(conn, "/other/p", &[]).expect("forget"), 0);
    assert!(recorded(conn, "/i/p/kept.jsonl").expect("read").is_some());
}
