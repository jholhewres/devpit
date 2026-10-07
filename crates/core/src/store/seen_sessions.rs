//! Sessions an orchestrator's account was seen running, kept past their end.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

/// A session as it was seen running.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSeen<'a> {
    pub session_id: &'a str,
    pub name: &'a str,
    pub cwd: &'a str,
    pub project_id: Option<&'a str>,
    pub card_id: Option<&'a str>,
    pub status: &'a str,
}

/// One kept session, running or ended.
#[derive(Debug, Clone, PartialEq)]
pub struct SeenSessionRow {
    pub session_id: String,
    pub profile_id: String,
    pub name: String,
    pub cwd: String,
    pub project_id: Option<String>,
    pub card_id: Option<String>,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
    pub last_status: Option<String>,
    pub ended_at: Option<i64>,
    /// Who stopped it, when someone did: `orchestrator` or `person`.
    pub ended_by: Option<String>,
}

const COLUMNS: &str = "session_id, profile_id, name, cwd, project_id, card_id, first_seen_at, \
                       last_seen_at, last_status, ended_at, ended_by";

fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SeenSessionRow> {
    Ok(SeenSessionRow {
        session_id: row.get(0)?,
        profile_id: row.get(1)?,
        name: row.get(2)?,
        cwd: row.get(3)?,
        project_id: row.get(4)?,
        card_id: row.get(5)?,
        first_seen_at: row.get(6)?,
        last_seen_at: row.get(7)?,
        last_status: row.get(8)?,
        ended_at: row.get(9)?,
        ended_by: row.get(10)?,
    })
}

impl Store {
    /// Records what `profile` has running at `now`; a session it had that is
    /// no longer among them has ended.
    pub fn saw_sessions(
        &self,
        profile_id: &str,
        running: &[SessionSeen<'_>],
        now: i64,
    ) -> Result<(), StoreError> {
        let tx = self.writing()?;
        for one in running {
            tx.execute(
                "INSERT INTO seen_session (session_id, profile_id, name, cwd, project_id, card_id, \
                 first_seen_at, last_seen_at, last_status) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8) \
                 ON CONFLICT(session_id) DO UPDATE SET name = excluded.name, cwd = excluded.cwd, \
                 project_id = excluded.project_id, card_id = excluded.card_id, \
                 last_seen_at = excluded.last_seen_at, last_status = excluded.last_status, \
                 ended_at = NULL, ended_by = NULL",
                rusqlite::params![
                    one.session_id,
                    profile_id,
                    one.name,
                    one.cwd,
                    one.project_id,
                    one.card_id,
                    now,
                    one.status
                ],
            )?;
        }
        let ids: Vec<&str> = running.iter().map(|one| one.session_id).collect();
        let open: Vec<String> = {
            let mut statement = tx.prepare(
                "SELECT session_id FROM seen_session WHERE profile_id = ?1 AND ended_at IS NULL",
            )?;
            let rows = statement.query_map([profile_id], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<_, _>>()?
        };
        for gone in open.iter().filter(|id| !ids.contains(&id.as_str())) {
            tx.execute(
                "UPDATE seen_session SET ended_at = ?2 WHERE session_id = ?1",
                rusqlite::params![gone, now],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Says who stopped a session, and that it ended.
    pub fn session_stopped(&self, session_id: &str, by: &str, now: i64) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE seen_session SET ended_by = ?2, ended_at = COALESCE(ended_at, ?3) \
             WHERE session_id = ?1",
            rusqlite::params![session_id, by, now],
        )?;
        Ok(())
    }

    /// The sessions of `profile` that ended, the latest first.
    pub fn ended_sessions(
        &self,
        profile_id: &str,
        most: u32,
    ) -> Result<Vec<SeenSessionRow>, StoreError> {
        let mut statement = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM seen_session WHERE profile_id = ?1 AND ended_at IS NOT NULL \
             ORDER BY ended_at DESC, last_seen_at DESC LIMIT ?2"
        ))?;
        let rows = statement.query_map(rusqlite::params![profile_id, most], row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// A kept session of `profile`, by its id or, the latest of them, by its name.
    pub fn seen_session(
        &self,
        profile_id: &str,
        id_or_name: &str,
    ) -> Result<Option<SeenSessionRow>, StoreError> {
        Ok(self
            .conn
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM seen_session WHERE profile_id = ?1 \
                     AND (session_id = ?2 OR name = ?2) \
                     ORDER BY session_id = ?2 DESC, last_seen_at DESC LIMIT 1"
                ),
                rusqlite::params![profile_id, id_or_name],
                row,
            )
            .optional()?)
    }
}

#[cfg(test)]
#[path = "seen_sessions_tests.rs"]
mod tests;
