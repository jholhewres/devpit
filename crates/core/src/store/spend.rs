//! What the work cost, read off the rows the runs wrote.
//!
//! Apart from the board because it is a different question: the board is about
//! where a card is, this is about what getting it there cost. Every number
//! here is measured — nothing estimates.

use crate::store::{Store, StoreError};

impl Store {
    /// What every run in this project has cost, and how many there were.
    ///
    /// Measured, never estimated: the numbers come off the rows the runs
    /// wrote, and a project that has run nothing reports nothing rather than
    /// a zero dressed up as a total.
    pub fn project_spend(&self, project_id: &str) -> Result<(f64, u32), StoreError> {
        Ok(self.conn.query_row(
            "SELECT COALESCE(SUM(r.cost_usd), 0.0), COUNT(*) FROM run r \
             JOIN card c ON c.id = r.card_id WHERE c.project_id = ?1 \
             AND r.cost_usd IS NOT NULL",
            [project_id],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as u32)),
        )?)
    }

    /// The cards that cost anything, most expensive first.
    pub fn dearest_cards(
        &self,
        project_id: &str,
        limit: u32,
    ) -> Result<Vec<(String, f64)>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.title, SUM(r.cost_usd) AS spent FROM run r \
             JOIN card c ON c.id = r.card_id WHERE c.project_id = ?1 \
             AND r.cost_usd IS NOT NULL GROUP BY c.id HAVING spent > 0 \
             ORDER BY spent DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![project_id, limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    /// What a card has cost across every run of it.
    pub fn card_cost(&self, card_id: &str) -> Result<f64, StoreError> {
        Ok(self.conn.query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM run WHERE card_id = ?1",
            [card_id],
            |row| row.get(0),
        )?)
    }
}
