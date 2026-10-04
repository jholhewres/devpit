//! Review findings the person set aside, by run and place in its review.

use crate::store::{Store, StoreError};

impl Store {
    /// Sets a finding aside, or brings it back.
    pub fn dismiss_finding(
        &self,
        run_id: &str,
        at: u32,
        dismissed: bool,
    ) -> Result<(), StoreError> {
        if dismissed {
            self.conn.execute(
                "INSERT OR IGNORE INTO finding_dismissal (run_id, at, dismissed_at) \
                 VALUES (?1, ?2, CAST(strftime('%s', 'now') AS INTEGER))",
                rusqlite::params![run_id, at],
            )?;
        } else {
            self.conn.execute(
                "DELETE FROM finding_dismissal WHERE run_id = ?1 AND at = ?2",
                rusqlite::params![run_id, at],
            )?;
        }
        Ok(())
    }

    /// The places, in this run's review, of the findings set aside.
    pub fn dismissed_findings(&self, run_id: &str) -> Result<Vec<u32>, StoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT at FROM finding_dismissal WHERE run_id = ?1 ORDER BY at")?;
        let rows = statement.query_map([run_id], |row| row.get::<_, u32>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
#[path = "dismissals_tests.rs"]
mod tests;
