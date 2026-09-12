//! The notes pinned to a project.
//!
//! Apart from the project rows because they are a different table with a
//! different lifetime: a note outlives the thought that made it and nothing
//! about reading a project needs to read them.

use crate::store::{Store, StoreError};

pub struct NoteRow {
    pub id: String,
    pub body: String,
    pub created_at: i64,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

impl Store {
    pub fn notes(&self, project_id: &str) -> Result<Vec<NoteRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, body, created_at FROM scratch \
             WHERE project_id = ?1 AND archived_at IS NULL \
             ORDER BY created_at DESC",
        )?;

        let rows = stmt
            .query_map([project_id], |row| {
                Ok(NoteRow {
                    id: row.get(0)?,
                    body: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn add_note(&self, project_id: &str, body: &str) -> Result<String, StoreError> {
        let id = format!("scr_{}", ulid::Ulid::generate());
        let at = now();
        self.conn.execute(
            "INSERT INTO scratch (id, project_id, body, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?4)",
            rusqlite::params![id, project_id, body, at],
        )?;
        Ok(id)
    }
}
