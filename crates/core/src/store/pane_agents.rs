//! The agent devpit started in a pane, and the session it is in.
//!
//! Written when the agent is launched and updated from its own hooks, so a
//! pane tmux lost can start the same agent again on the same conversation.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

#[derive(Debug, Clone, PartialEq)]
pub struct PaneAgent {
    /// The profile or agent id it was started with.
    pub launch: String,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

impl Store {
    /// A new launch replaces whatever the pane ran before, session and all.
    pub fn remember_pane_launch(
        &self,
        project_id: &str,
        leaf_id: &str,
        launch: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO pane_agent (leaf_id, project_id, launch, updated_at) VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(leaf_id) DO UPDATE SET project_id = ?2, launch = ?3, \
             session_id = NULL, transcript_path = NULL, updated_at = ?4",
            rusqlite::params![leaf_id, project_id, launch, now()],
        )?;
        Ok(())
    }

    /// Only for a pane devpit launched an agent in: a hook from an agent
    /// somebody typed by hand creates nothing, since there is no launch to repeat.
    pub fn remember_pane_session(
        &self,
        leaf_id: &str,
        session_id: &str,
        transcript_path: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE pane_agent SET session_id = ?2, transcript_path = ?3, updated_at = ?4 \
             WHERE leaf_id = ?1",
            rusqlite::params![leaf_id, session_id, transcript_path, now()],
        )?;
        Ok(())
    }

    pub fn forget_pane_agent(&self, leaf_id: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM pane_agent WHERE leaf_id = ?1", [leaf_id])?;
        Ok(())
    }

    pub fn pane_agent(&self, leaf_id: &str) -> Result<Option<PaneAgent>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT launch, session_id, transcript_path FROM pane_agent WHERE leaf_id = ?1",
                [leaf_id],
                |row| {
                    Ok(PaneAgent {
                        launch: row.get(0)?,
                        session_id: row.get(1)?,
                        transcript_path: row.get(2)?,
                    })
                },
            )
            .optional()?)
    }
}

#[cfg(test)]
#[path = "pane_agents_tests.rs"]
mod tests;
