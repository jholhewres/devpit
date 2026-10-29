//! A project's git origin: what makes it the same project on another
//! machine, kept current with its remote and settled on one spelling.

use std::path::Path;

use rusqlite::OptionalExtension;

use crate::origin::{identity, shareable};
use crate::store::{Store, StoreError};

impl Store {
    /// Takes the origin git has now, when it is another repository than the
    /// one kept: a project's identity across machines follows its remote.
    pub fn refresh_origin(&self, id: &str, url: &str) -> Result<bool, StoreError> {
        if self.origin_hash_of(id)? == identity(url) {
            return Ok(false);
        }
        set_origin(&self.conn, id, url)?;
        Ok(true)
    }

    /// Keeps another machine's spelling of the same repository, so both
    /// machines settle on one address. Another repository is refused.
    pub fn respell_origin(&self, id: &str, url: &str) -> Result<bool, StoreError> {
        let kept = self.origin_hash_of(id)?;
        if kept.is_none() || kept != identity(url) {
            return Ok(false);
        }
        self.conn.execute(
            "UPDATE project SET origin_url = ?2 WHERE id = ?1",
            rusqlite::params![id, shareable(url)],
        )?;
        Ok(true)
    }

    fn origin_hash_of(&self, id: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT origin_hash FROM project WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()?
            .flatten())
    }
}

/// An origin kept as another machine may see it, and its identity.
fn set_origin(conn: &rusqlite::Connection, id: &str, url: &str) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE project SET origin_url = ?2, origin_hash = ?3 WHERE id = ?1",
        rusqlite::params![id, shareable(url), identity(url)],
    )?;
    Ok(())
}

/// Migration 032: every origin hashed and kept as this build reads it, so
/// a clone over https with a port or a user joins its ssh twin.
pub(super) fn rehash_origins(conn: &rusqlite::Connection, _root: &Path) -> Result<(), StoreError> {
    let mut stmt =
        conn.prepare("SELECT id, origin_url FROM project WHERE origin_url IS NOT NULL")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, url) in rows {
        set_origin(conn, &id, &url)?;
    }
    Ok(())
}
