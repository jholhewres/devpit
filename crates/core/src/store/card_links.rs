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
}

/// A conversation that is the card's.
pub struct ChatLink {
    pub conversation_id: String,
    pub created_at: i64,
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
             ORDER BY started_at DESC, id DESC",
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
                "SELECT short_id, session_id, cwd FROM session_link WHERE card_id = ?1",
                [card_id],
                |row| {
                    Ok(BackgroundLink {
                        short_id: row.get(0)?,
                        session_id: row.get(1)?,
                        cwd: row.get(2)?,
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

#[cfg(test)]
#[path = "card_links_tests.rs"]
mod tests;
