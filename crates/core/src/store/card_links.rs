//! What links a card to the sessions working on it.
//!
//! Links only. Whether a session is working, waiting or gone is said by the
//! agent and kept in memory by the app; a copy in a row would be a second
//! answer that goes stale the moment the app closes.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

/// A run whose agent spoke in a session of its own.
pub struct RunLink {
    pub run_id: String,
    pub session_id: String,
    /// Where it ran. `None` for a run recorded before runs kept it.
    pub cwd: Option<String>,
    pub state: String,
    pub started_at: i64,
}

/// The background session a step started for the card.
pub struct BackgroundLink {
    pub short_id: String,
    pub session_id: String,
    pub cwd: Option<String>,
    /// Which profile started it, so asking the CLI about it asks the same
    /// binary that made it. `None` for links made before profiles got here.
    pub profile_id: Option<String>,
}

/// A conversation that is the card's.
pub struct ChatLink {
    pub conversation_id: String,
    pub created_at: i64,
}

/// Where a card holds a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionHeld {
    /// A run's agent spoke in it.
    Run,
    /// A step started it in the background.
    Background,
}

/// Everything durable that ties a card to a session.
pub struct CardLinks {
    /// Newest first.
    pub runs: Vec<RunLink>,
    pub background: Option<BackgroundLink>,
    /// In the order they began.
    pub chats: Vec<ChatLink>,
}

impl Store {
    pub fn card_links(&self, card_id: &str) -> Result<CardLinks, StoreError> {
        let mut runs = self.conn.prepare(
            "SELECT id, session_id, cwd, state, started_at FROM run \
             WHERE card_id = ?1 AND session_id IS NOT NULL \
             ORDER BY started_at DESC, rowid DESC",
        )?;
        let runs = runs
            .query_map([card_id], |row| {
                Ok(RunLink {
                    run_id: row.get(0)?,
                    session_id: row.get(1)?,
                    cwd: row.get(2)?,
                    state: row.get(3)?,
                    started_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let background = self
            .conn
            .query_row(
                "SELECT short_id, session_id, cwd, profile_id FROM session_link \
                 WHERE card_id = ?1",
                [card_id],
                |row| {
                    Ok(BackgroundLink {
                        short_id: row.get(0)?,
                        session_id: row.get(1)?,
                        cwd: row.get(2)?,
                        profile_id: row.get(3)?,
                    })
                },
            )
            .optional()?;

        let mut chats = self.conn.prepare(
            "SELECT conversation_id, created_at FROM card_chat \
             WHERE card_id = ?1 ORDER BY created_at, conversation_id",
        )?;
        let chats = chats
            .query_map([card_id], |row| {
                Ok(ChatLink {
                    conversation_id: row.get(0)?,
                    created_at: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(CardLinks {
            runs,
            background,
            chats,
        })
    }
}

impl Store {
    /// The card still on a board that holds this session, and where it holds it.
    ///
    /// An archived card holds nothing: no board shows it, so nothing it held
    /// has a tile to reach.
    pub fn session_holder(
        &self,
        session_id: &str,
    ) -> Result<Option<(String, SessionHeld)>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT r.card_id, 0 FROM run r JOIN card c ON c.id = r.card_id \
                 WHERE r.session_id = ?1 AND c.archived_at IS NULL \
                 UNION ALL \
                 SELECT l.card_id, 1 FROM session_link l JOIN card c ON c.id = l.card_id \
                 WHERE l.session_id = ?1 AND c.archived_at IS NULL \
                 LIMIT 1",
                [session_id],
                |row| {
                    let held = match row.get::<_, i64>(1)? {
                        0 => SessionHeld::Run,
                        _ => SessionHeld::Background,
                    };
                    Ok((row.get(0)?, held))
                },
            )
            .optional()?)
    }

    /// Files a conversation under a card. Once: a conversation is one card's.
    pub fn link_chat(&self, card_id: &str, conversation_id: &str) -> Result<(), StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs() as i64)
            .unwrap_or_default();
        self.conn.execute(
            "INSERT INTO card_chat (conversation_id, card_id, created_at) VALUES (?1, ?2, ?3) \
             ON CONFLICT(conversation_id) DO NOTHING",
            rusqlite::params![conversation_id, card_id, now],
        )?;
        Ok(())
    }

    /// The card a conversation is filed under; `None` once that card is gone.
    pub fn chat_card(&self, conversation_id: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT card_id FROM card_chat WHERE conversation_id = ?1",
                [conversation_id],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Names the session a run's agent speaks in.
    pub fn set_run_session(&self, run_id: &str, session_id: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE run SET session_id = ?2 WHERE id = ?1",
            [run_id, session_id],
        )?;
        Ok(())
    }

    /// Records the folder a run works in, once it is known.
    pub fn set_run_cwd(&self, run_id: &str, cwd: &str) -> Result<(), StoreError> {
        self.conn
            .execute("UPDATE run SET cwd = ?2 WHERE id = ?1", [run_id, cwd])?;
        Ok(())
    }

    /// The session a run's agent spoke in; `None` for a run with no agent.
    pub fn run_session(&self, run_id: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT session_id FROM run WHERE id = ?1",
                [run_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten())
    }

    /// The project of a card that is still on a board; `None` once archived or gone.
    pub fn live_card_project(&self, card_id: &str) -> Result<Option<String>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT project_id FROM card WHERE id = ?1 AND archived_at IS NULL",
                [card_id],
                |row| row.get(0),
            )
            .optional()?)
    }
}

#[cfg(test)]
#[path = "card_links_tests.rs"]
mod tests;
