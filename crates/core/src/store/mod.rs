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

/// Puts the file in WAL, unless it is already there.
///
/// Retried rather than attempted once, and that is the whole of it: SQLite
/// refuses a `journal_mode` change while another connection has the file
/// open, and that refusal is **not** one `busy_timeout` waits out. Measured —
/// a connection losing this race came back in 516µs, having waited for
/// nothing at all.
///
/// Losing is not a failure here. The mode is a property of the file and the
/// connection that won is setting it to the same value, so the only thing to
/// do is look again in a moment. What must not happen is an app that fails to
/// start because two of its own threads opened the store together, which is
/// what startup does every time.
fn set_wal(conn: &Connection) -> Result<(), StoreError> {
    let mode = |conn: &Connection| -> Result<String, StoreError> {
        Ok(conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?)
    };

    // Generous against a cold disk and short against a real fault: whoever
    // holds the file is doing one pragma, not work.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if mode(conn)?.eq_ignore_ascii_case("wal") {
            return Ok(());
        }
        let refusal = match conn.pragma_update(None, "journal_mode", "WAL") {
            Ok(()) => return Ok(()),
            Err(err) => err,
        };
        if std::time::Instant::now() >= deadline {
            // The database's own word, not a timeout of ours: it is the one
            // that says what is actually wrong when this is a real fault.
            return Err(refusal.into());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
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

        // `busy_timeout` comes first, and the order is the whole point:
        // switching a fresh database to WAL takes a lock, and with the timeout
        // still at its default of zero a second connection arriving at the
        // same moment is refused outright rather than waiting. Measured — a
        // test that opens eight at once failed about a quarter of the time
        // with these two lines the other way round.
        conn.pragma_update(None, "busy_timeout", 5_000)?;

        // WAL outlives the process, so it is set once — asked about first and
        // only written when it has to be.
        //
        // Not politeness: SQLite refuses a `journal_mode` change outright
        // while another connection has the file open, and that refusal is not
        // one `busy_timeout` waits out. Two connections opening together is
        // ordinary here (startup alone does it), so a connection that finds
        // WAL already set must not ask for it again.
        set_wal(&conn)?;

        // Per-connection, and so stated every time. foreign_keys defaults to
        // OFF in SQLite; leaving it there means discovering orphans months
        // later, once the data is already wrong.
        conn.pragma_update(None, "foreign_keys", "ON")?;

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
