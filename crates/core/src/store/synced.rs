//! The rows sync reads and writes beside the window's own: a preference
//! with when it was set, a project with what makes it the same project on
//! another machine, and a plugin row as another machine left it.

use rusqlite::OptionalExtension;

use crate::store::{Store, StoreError};

/// A project as sync knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedProject {
    pub id: String,
    /// The normalised git origin, hashed: the same on every clone.
    pub origin_hash: Option<String>,
    pub created_at: i64,
    pub name: String,
    pub group: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

/// A project's plugin, installed or removed, on or off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRow {
    pub plugin_id: String,
    pub installed: bool,
    pub enabled: bool,
    pub updated_at: i64,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or_default()
}

impl Store {
    /// A preference and when it was last set, in seconds.
    pub fn preference_stamped(&self, key: &str) -> Result<Option<(String, i64)>, StoreError> {
        Ok(self
            .conn
            .query_row(
                "SELECT value, updated_at FROM preference WHERE key = ?1",
                [key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?)
    }

    /// Every project still listed, oldest first.
    pub fn synced_projects(&self) -> Result<Vec<SyncedProject>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, origin_hash, created_at, name, group_name, icon, color FROM project \
             WHERE archived_at IS NULL ORDER BY created_at, id",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SyncedProject {
                    id: row.get(0)?,
                    origin_hash: row.get(1)?,
                    created_at: row.get(2)?,
                    name: row.get(3)?,
                    group: row.get(4)?,
                    icon: row.get(5)?,
                    color: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Every plugin row of a project, removed ones too.
    pub fn plugin_rows(&self, project_id: &str) -> Result<Vec<PluginRow>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT plugin_id, installed, enabled, updated_at FROM project_plugin \
             WHERE project_id = ?1 ORDER BY plugin_id",
        )?;
        let rows = stmt
            .query_map([project_id], |row| {
                Ok(PluginRow {
                    plugin_id: row.get(0)?,
                    installed: row.get(1)?,
                    enabled: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// A plugin row as another machine left it. The app still checks the id
    /// against its catalogue before it shows the plugin.
    pub fn set_plugin_row(
        &self,
        project_id: &str,
        plugin_id: &str,
        installed: bool,
        enabled: bool,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO project_plugin (project_id, plugin_id, installed, enabled, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT (project_id, plugin_id) DO UPDATE SET \
             installed = excluded.installed, enabled = excluded.enabled, \
             updated_at = excluded.updated_at, revision = revision + 1",
            rusqlite::params![
                project_id,
                plugin_id,
                installed,
                enabled && installed,
                now()
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "synced_tests.rs"]
mod tests;
