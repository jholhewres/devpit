//! What a card holds a background session by.
//!
//! Its own file rather than a corner of `board.rs`: a link outlives the run
//! that made it, carries the profile that started it, and is read by the
//! attach path, which has nothing to do with columns and cards.

use rusqlite::OptionalExtension;

use super::board::now;
use crate::store::{Store, StoreError};

/// The link between a card and the agent session working on it.
///
/// Only the link. Whether that session is idle or busy, and what it has spent,
/// are answered by the agent CLI and its transcript — a copy here would be a
/// second truth that drifts from the first.
pub struct SessionLink {
    pub card_id: String,
    pub short_id: String,
    pub session_id: String,
    pub transcript_path: Option<String>,
    /// The profile the step started it under. `None` for a link made before
    /// profiles reached this table, which attaches under the default runner.
    pub profile_id: Option<String>,
}

impl Store {
    pub fn link_session(
        &self,
        card_id: &str,
        short_id: &str,
        session_id: &str,
        transcript_path: Option<&str>,
        cwd: Option<&str>,
        profile_id: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO session_link \
             (card_id, short_id, session_id, transcript_path, created_at, cwd, profile_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(card_id) DO UPDATE SET \
               short_id = ?2, session_id = ?3, transcript_path = ?4, cwd = ?6, profile_id = ?7",
            rusqlite::params![
                card_id,
                short_id,
                session_id,
                transcript_path,
                now(),
                cwd,
                profile_id
            ],
        )?;
        Ok(())
    }

    pub fn session_link(&self, card_id: &str) -> Result<Option<SessionLink>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT card_id, short_id, session_id, transcript_path, profile_id \
                 FROM session_link WHERE card_id = ?1",
                [card_id],
                |row| {
                    Ok(SessionLink {
                        card_id: row.get(0)?,
                        short_id: row.get(1)?,
                        session_id: row.get(2)?,
                        transcript_path: row.get(3)?,
                        profile_id: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    /// Every card of a project that has a session behind it.
    /// The distinct profiles this project's background sessions were started
    /// under, `None` among them when any link predates profiles here.
    ///
    /// One row per profile rather than per card: asking the CLI costs a
    /// process, and every session a profile started is listed by one call to
    /// it.
    pub fn session_profiles(&self, project_id: &str) -> Result<Vec<Option<String>>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT l.profile_id FROM session_link l \
             JOIN card c ON c.id = l.card_id \
             WHERE c.project_id = ?1 AND c.archived_at IS NULL",
        )?;
        let found = stmt.query_map([project_id], |row| row.get::<_, Option<String>>(0))?;
        Ok(found.filter_map(Result::ok).collect())
    }

    pub fn session_links(&self, project_id: &str) -> Result<Vec<SessionLink>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT l.card_id, l.short_id, l.session_id, l.transcript_path, l.profile_id \
             FROM session_link l JOIN card c ON c.id = l.card_id \
             WHERE c.project_id = ?1",
        )?;
        let rows = stmt
            .query_map([project_id], |row| {
                Ok(SessionLink {
                    card_id: row.get(0)?,
                    short_id: row.get(1)?,
                    session_id: row.get(2)?,
                    transcript_path: row.get(3)?,
                    profile_id: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}
