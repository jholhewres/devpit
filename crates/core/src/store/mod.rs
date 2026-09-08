//! Where state lives: a SQLite file at `~/.quockpit/state.db`, in WAL.
//!
//! WAL because the access pattern is a window drawing while a watcher writes.

mod layouts;
mod migrations;
mod projects;
pub mod settings;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("the database refused the operation: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("could not create {path}: {source}")]
    Directory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("this platform exposes no user data directory")]
    NoDataDirectory,
}

pub use projects::{NoteRow, ProjectRow};
pub use settings::key as preference;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open_default() -> Result<Self, StoreError> {
        Self::open(&Self::default_path()?)
    }

    /// Opens at a given path. Tests enter here, with a tempdir.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| StoreError::Directory {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let conn = Connection::open(path)?;

        // WAL outlives the process, so it is set once. The others are
        // per-connection and must be stated every time.
        //
        // foreign_keys defaults to OFF in SQLite; leaving it there means
        // discovering orphans months later, once the data is already wrong.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 5_000)?;

        migrations::run(&conn)?;

        Ok(Self { conn })
    }

    /// Root of everything this product keeps outside the repository.
    ///
    /// Outside is the point: a project's notes, drawings and wiki are not
    /// commits of it. They sync on their own and the work tree stays clean.
    ///
    /// A directory of our own, never shared with another product: two things
    /// writing into one directory is a careless `rm -rf` away from taking the
    /// wrong one with it.
    ///
    /// The path stays short on purpose. The tmux socket lives under it, and a
    /// Unix socket path is capped at ~108 bytes — see `socket_path_is_short`
    /// in `apps/desktop/src/sessions.rs`.
    pub fn root() -> Result<PathBuf, StoreError> {
        Ok(dirs::home_dir()
            .ok_or(StoreError::NoDataDirectory)?
            .join(".quockpit"))
    }

    pub fn default_path() -> Result<PathBuf, StoreError> {
        Ok(Self::root()?.join("state.db"))
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checked against the database: a table renamed in the SQL without going
    /// through this test is a migration that breaks whoever has data.
    const EXPECTED: [&str; 7] = [
        "trust_workspace",
        "project",
        "scratch",
        "drawing",
        "event",
        "pane_layout",
        "preference",
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
}
