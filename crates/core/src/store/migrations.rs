//! Schema migrations, applied once each.
//!
//! Idempotency matters: the app opens the store on every start, and a
//! migration that reruns against real data loses data. The applied version is
//! read from the database itself (`user_version`), not tracked beside it.

use rusqlite::Connection;

use crate::store::StoreError;

#[path = "migrations_list.rs"]
mod list;
use list::MIGRATIONS;

pub(super) struct Migration {
    pub(super) version: i64,
    pub(super) sql: &'static str,
}

/// The newest schema this build knows.
///
/// Read from the list rather than written down beside it: a constant that has
/// to be edited whenever a migration is added is a constant that will one day
/// disagree with the list it describes.
///
/// Only the test asks, like `run_up_to`: the app applies everything and never
/// needs to name the number it reached.
#[cfg(test)]
pub fn latest() -> i64 {
    MIGRATIONS.iter().map(|one| one.version).max().unwrap_or(0)
}

/// Applies whatever has not been applied yet. Called on every start.
pub fn run(conn: &Connection) -> Result<(), StoreError> {
    let applied: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    for migration in MIGRATIONS.iter().filter(|m| m.version > applied) {
        // One transaction per migration: a partial failure must not leave half
        // the tables standing with the version already bumped.
        conn.execute_batch(&format!(
            "BEGIN; {} PRAGMA user_version = {}; COMMIT;",
            migration.sql, migration.version
        ))?;
    }

    Ok(())
}

/// Applies only up to `version`, to build a database of an older shape.
///
/// Only the test asks: it is how "the migration is additive" is proved against
/// rows rather than against an empty file.
#[cfg(test)]
pub fn run_up_to(conn: &Connection, version: i64) -> Result<(), StoreError> {
    for migration in MIGRATIONS.iter().filter(|m| m.version <= version) {
        conn.execute_batch(&format!(
            "BEGIN; {} PRAGMA user_version = {}; COMMIT;",
            migration.sql, migration.version
        ))?;
    }
    Ok(())
}

/// Only the test asks. Same rule as the RPC contract: no caller, no code.
#[cfg(test)]
pub fn latest_version() -> i64 {
    MIGRATIONS.last().map_or(0, |m| m.version)
}
