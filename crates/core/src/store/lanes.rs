//! Adding a lane, with its place decided by the database.

use super::board::now;
use super::{Store, StoreError};

impl Store {
    /// A new lane after the last one, in one statement: the position is read
    /// and written together, so two lanes added at once cannot both ask for
    /// it. After the greatest position rather than at the count, because a
    /// deleted lane leaves a gap and the count then names one still taken.
    pub fn create_column_at_end(&self, project_id: &str, name: &str) -> Result<String, StoreError> {
        let id = format!("col_{}", ulid::Ulid::generate());
        self.conn.execute(
            "INSERT INTO board_column (id, project_id, name, position, created_at) \
             SELECT ?1, ?2, ?3, COALESCE(MAX(position), -1) + 1, ?4 \
               FROM board_column WHERE project_id = ?2",
            rusqlite::params![id, project_id, name, now()],
        )?;
        Ok(id)
    }
}
