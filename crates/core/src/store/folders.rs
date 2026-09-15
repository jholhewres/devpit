//! The folder each project's files live in, as the store keeps it.
//!
//! Apart from `projects.rs` because this is the half launch runs once: naming
//! the rows an older build left unnamed, and following the pins into the
//! folder that moved.

use std::collections::HashSet;

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

impl Store {
    /// `None` when there is no such project; `Some(None)` for a row no build
    /// has named yet.
    pub(crate) fn project_folder(&self, id: &str) -> Result<Option<Option<String>>, StoreError> {
        Ok(self
            .conn
            .query_row("SELECT folder FROM project WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .optional()?)
    }

    /// Every named project, with its folder.
    pub(crate) fn project_folders(&self) -> Result<Vec<(String, String)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, folder FROM project WHERE folder IS NOT NULL ORDER BY created_at, id",
        )?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub(crate) fn taken_folders(&self) -> Result<HashSet<String>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT folder FROM project WHERE folder IS NOT NULL")?;
        let taken = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<HashSet<_>, _>>()?;
        Ok(taken)
    }

    /// Names the rows made before `project.folder` existed; answers how many.
    ///
    /// Only `folder IS NULL`, so a second run names nothing, and a folder once
    /// given is never given again.
    pub(crate) fn backfill_folders(&self) -> Result<usize, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM project WHERE folder IS NULL ORDER BY created_at, id")?;
        let unnamed = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut taken = self.taken_folders()?;
        for (id, name) in &unnamed {
            let folder = crate::home::folder_for(name, id, |one| taken.contains(one));
            self.conn.execute(
                "UPDATE project SET folder = ?2 WHERE id = ?1 AND folder IS NULL",
                rusqlite::params![id, folder],
            )?;
            taken.insert(folder);
        }
        Ok(unnamed.len())
    }

    /// Points the pins under one folder at another.
    ///
    /// `substr` rather than `LIKE`: a project id carries `_`, which `LIKE`
    /// reads as a wildcard.
    pub(crate) fn move_pins(&self, from: &str, to: &str) -> Result<usize, StoreError> {
        Ok(self.conn.execute(
            "UPDATE card_attachment SET path = ?2 || substr(path, length(?1) + 1) \
             WHERE path = ?1 OR substr(path, 1, length(?1) + 1) = ?1 || '/'",
            rusqlite::params![from, to],
        )?)
    }
}
