//! Opening the store, and what a migration must never do to existing rows.

use super::*;

/// Checked against the database: a table renamed in the SQL without going
/// through this test is a migration that breaks whoever has data.
const EXPECTED: [&str; 12] = [
    "trust_workspace",
    "project",
    "scratch",
    "drawing",
    "event",
    "pane_layout",
    "preference",
    "board_column",
    "step",
    "card",
    "run",
    "session_link",
];

fn tables(store: &Store) -> Vec<String> {
    let mut stmt = store
        .conn()
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .expect("query the catalog");
    stmt.query_map([], |row| row.get::<_, String>(0))
        .expect("read names")
        .collect::<Result<Vec<_>, _>>()
        .expect("valid names")
}

#[test]
fn migrations_create_the_tables_and_rerun_cleanly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.db");

    let store = Store::open(&path).expect("first open");
    let first = tables(&store);
    for name in EXPECTED {
        assert!(first.contains(&name.to_string()), "missing table {name}");
    }
    drop(store);

    // The second open is the real test: it is what every start after the
    // first one does.
    let store = Store::open(&path).expect("second open");
    assert_eq!(first, tables(&store), "the second open changed the schema");

    let version: i64 = store
        .conn()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("read user_version");
    assert_eq!(version, migrations::latest_version());
}

#[test]
fn syncable_resources_are_born_with_a_revision() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");

    for table in ["project", "scratch", "drawing", "trust_workspace"] {
        let has_revision = store
            .conn()
            .prepare(&format!("SELECT * FROM pragma_table_info('{table}')"))
            .and_then(|mut stmt| {
                stmt.query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .expect("read columns")
            .iter()
            .any(|column| column == "revision");

        assert!(
            has_revision,
            "{table} has no revision — adding it later means a backfill over real data"
        );
    }
}

#[test]
fn foreign_keys_are_enforced() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");

    // A project pointing at a workspace that does not exist must be
    // refused now, not found as an orphan three months later.
    let orphan = store.conn().execute(
        "INSERT INTO project (id, trust_workspace_id, name, root_path, created_at) \
         VALUES ('prj_x', 'tw_missing', 'x', '/tmp/x', 0)",
        [],
    );
    assert!(orphan.is_err(), "foreign_keys is off");
}

/// The migration has to be additive over real rows, not over an empty
/// file: that is the case where a mistake costs someone their data.
#[test]
fn migrating_leaves_older_rows_untouched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("state.db");

    // A database as it stood before the board existed, with rows in it.
    {
        let conn = Connection::open(&path).expect("open raw");
        conn.pragma_update(None, "foreign_keys", "ON").expect("fk");
        migrations::run_up_to(&conn, 3).expect("migrate to 3");
        conn.execute_batch(
            "INSERT INTO trust_workspace                    (id, slug, label, color, vault_namespace, is_default, created_at)                  VALUES ('tw_1', 'p', 'P', '#fff', 'p', 1, 0);                  INSERT INTO project (id, trust_workspace_id, name, root_path, created_at)                  VALUES ('prj_1', 'tw_1', 'demo', '/tmp/demo', 0);                  INSERT INTO pane_layout (project_id, tree, focused_id, updated_at)                  VALUES ('prj_1', '{\"leaf\":1}', 'leaf_1', 0);",
        )
        .expect("seed rows");
    }

    let store = Store::open(&path).expect("migrate to latest");

    let version: i64 = store
        .conn()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("version");
    // Against the list rather than against a number typed here: this
    // asserts "every migration ran", which is the thing being tested,
    // and does not have to be edited to add one.
    assert_eq!(version, migrations::latest());

    let name: String = store
        .conn()
        .query_row("SELECT name FROM project WHERE id = 'prj_1'", [], |row| {
            row.get(0)
        })
        .expect("the project survived");
    assert_eq!(name, "demo");

    // The layout table is rebuilt by migration 5, not altered — SQLite
    // cannot change a primary key in place. So its rows are exactly where
    // a mistake would cost someone an open session.
    let (tab_id, focused): (String, String) = store
        .conn()
        .query_row(
            "SELECT tab_id, focused_id FROM pane_layout WHERE project_id = 'prj_1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("the layout survived being rebuilt");
    assert_eq!(focused, "leaf_1");
    // A tree that predates tabs is adopted by one named after its project,
    // so a session open across the upgrade is still reachable.
    assert_eq!(tab_id, "tab_prj_1");

    // And the board tables arrived alongside them.
    store.ensure_board("prj_1").expect("seed the board");
    assert_eq!(store.columns("prj_1").expect("columns").len(), 6);
}
