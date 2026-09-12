//! Where state lives: a SQLite file at `~/.devpit/state.db`, in WAL.
//!
//! WAL because the access pattern is a window drawing while a watcher writes.

mod board;
mod cards;
mod layouts;
mod migrations;
mod notes;
mod projects;
mod runs;
pub mod settings;
mod spend;

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

pub use board::{CardRow, ColumnRow, SessionLink, StepRow, DEFAULT_COLUMNS};
pub use cards::{AttachmentRow, CommentRow, NoticeRow};
pub use runs::RunRow;
/// The ceilings on anything read from outside, in one place.
pub mod limits {
    pub use crate::store::cards::{LONGEST_COMMENT, LONGEST_LABEL, NOTICES_KEPT};
}
pub use notes::NoteRow;
pub use projects::ProjectRow;
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
            .join(".devpit"))
    }

    pub fn default_path() -> Result<PathBuf, StoreError> {
        Ok(Self::root()?.join("state.db"))
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
