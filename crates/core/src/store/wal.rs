//! The store's write-ahead log: putting the file in it, and keeping its syncs
//! out of the locks.

use rusqlite::config::DbConfig;
use rusqlite::Connection;

use crate::store::StoreError;

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
pub(super) fn set_wal(conn: &Connection) -> Result<(), StoreError> {
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

/// Takes the sync out of commits and closes, which hold locks others wait on.
///
/// On Linux a sync waits for the filesystem's journal, seconds behind a build,
/// and whoever queued behind it past `busy_timeout` was told "database is locked".
pub(super) fn sync_outside_locks(conn: &Connection) -> Result<(), StoreError> {
    // A commit synced the WAL under the write lock; the checkpoint does it now.
    // A power cut can lose the last commits, never the file.
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    // Most closes are the last (each command opens its own), and the last one
    // checkpointed under a lock that kept everyone out; the automatic one does not.
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?;
    Ok(())
}
