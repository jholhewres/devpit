//! Where state lives: a SQLite file at `~/.devpit/state.db`, in WAL.
//!
//! WAL because the access pattern is a window drawing while a watcher writes.

mod board;
mod card_links;
mod cards;
mod evidence;
mod folders;
mod lanes;
mod layouts;
mod migrations;
pub mod pane_agents;
mod plugins;
pub mod project_runs;
mod projects;
mod runs;
pub mod search_index;
mod session_links;
pub mod settings;
mod spend;
mod whose_run;

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

    /// Names the drawing, never a path: this message reaches the screen.
    #[error("could not export drawing {drawing}: {reason}")]
    Export { drawing: String, reason: String },

    /// A second run asked for on a card that already has one going. Refused by
    /// the database (`run_one_running`), so two commands at once cannot both
    /// pass a check made before either wrote.
    #[error("a run is already going on this card")]
    AlreadyRunning,
}

pub use board::{CardRow, ColumnRow, StepRow, StepUse, DEFAULT_COLUMNS};
pub use card_links::{BackgroundLink, CardLinks, ChatLink, RunLink, SessionHeld};
pub use cards::{AttachmentRow, CommentRow, NoticeRow};
pub use evidence::{Evidence, EvidenceError, Ran, MOST_EVIDENCE};
pub use layouts::{CardTabLayout, PaneLayoutWrite};
pub use runs::RunRow;
pub use session_links::SessionLink;
pub use whose_run::{Asked, Carried, WhoseRun};
/// The ceilings on anything read from outside, in one place.
pub mod limits {
    pub use crate::store::cards::{LONGEST_COMMENT, LONGEST_LABEL, NOTICES_KEPT};
}
pub use plugins::{DRAWINGS_PLUGIN, DRAWING_EXTENSION};
pub use projects::ProjectRow;
pub use settings::key as preference;

/// The folder an installed devpit keeps everything in, under the home.
pub const RELEASE_ROOT: &str = ".devpit";

/// The folder a devpit being worked on keeps everything in — `make dev`, or
/// any debug build. Beside the installed one's and never inside it.
pub const DEV_ROOT: &str = ".devpit-dev";

/// Which of the two this build is.
///
/// By build profile rather than by a variable somebody sets, because the
/// failure it prevents is silent: a development build on the installed home
/// works perfectly until the two of them are open at once.
pub const ROOT_NAME: &str = if cfg!(debug_assertions) {
    DEV_ROOT
} else {
    RELEASE_ROOT
};

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

        // The workspace root is the store's folder — `default_path` puts it
        // there — and a migration moving rows into project folders needs it.
        migrations::run(&conn, path.parent().unwrap_or(Path::new("")))?;

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
    /// in `apps/desktop/src/sessions.rs`. A `DEVPIT_HOME` long enough to break
    /// that is refused where the socket is made, not here: this function is
    /// read by tests and tools that never open a terminal.
    ///
    /// One machine runs two devpits — the installed one and the one being
    /// worked on — and they must not see each other's projects, conversations
    /// or terminals. **A debug build keeps [`DEV_ROOT`] and a release build
    /// [`RELEASE_ROOT`]**, so that separation holds without anybody having to
    /// remember it. It used to rest on setting `DEVPIT_HOME` by hand, and
    /// `make dev` without it opened the installed devpit's own home: the two
    /// wrote their listener ports over each other's in `hook-endpoint`, so one
    /// of them stopped hearing its agents, and a migration in the build being
    /// worked on migrated data the installed version then could not read.
    ///
    /// Everything devpit keeps derives from here — the store, the tmux socket,
    /// the hook endpoint baked into each agent's `--settings`, the browser
    /// sessions — so this one choice is the whole of it.
    ///
    /// `DEVPIT_HOME` still wins over both, for a home somewhere else entirely.
    /// Empty counts as unset, so `DEVPIT_HOME= devpit` is the usual home rather
    /// than the filesystem root.
    pub fn root() -> Result<PathBuf, StoreError> {
        if let Some(set) = std::env::var_os("DEVPIT_HOME").filter(|set| !set.is_empty()) {
            return Ok(PathBuf::from(set));
        }
        Ok(dirs::home_dir()
            .ok_or(StoreError::NoDataDirectory)?
            .join(ROOT_NAME))
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
