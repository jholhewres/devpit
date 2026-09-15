//! Which plugins a project turned on, and the drawings that became the first
//! plugin's files.
//!
//! Plugin ids are plain strings here: the catalogue lives in `devpit-rpc`,
//! which depends on this crate, so the app checks an id before it arrives.

use std::io::ErrorKind;
use std::path::Path;

use rusqlite::Connection;

use crate::data_files;
use crate::home::{plain_data_name, projects_dir, ProjectHome};
use crate::store::{Store, StoreError};

/// Where the rows of the old `drawing` table went. A catalogue test holds
/// these to the Excalidraw manifest.
pub const DRAWINGS_PLUGIN: &str = "excalidraw";
pub const DRAWING_EXTENSION: &str = ".excalidraw";

/// Under the workspace root: where a drawing goes that its project's folder
/// would not take.
const KEPT_DRAWINGS: &str = "drawings";

/// What the bell calls a drawing kept outside its project.
const DRAWING_NOTICE: &str = "drawing";

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or_default()
}

impl Store {
    /// The ids of the plugins a project has installed and turned on.
    ///
    /// Installed is asked too: a synced row may say on for a plugin this
    /// machine never installed.
    pub fn enabled_plugins(&self, project_id: &str) -> Result<Vec<String>, StoreError> {
        self.plugin_ids(project_id, "installed = 1 AND enabled = 1")
    }

    /// The ids of the plugins a project has installed, on or off.
    pub fn installed_plugins(&self, project_id: &str) -> Result<Vec<String>, StoreError> {
        self.plugin_ids(project_id, "installed = 1")
    }

    fn plugin_ids(&self, project_id: &str, filter: &str) -> Result<Vec<String>, StoreError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT plugin_id FROM project_plugin \
             WHERE project_id = ?1 AND {filter} ORDER BY plugin_id"
        ))?;
        let ids = stmt
            .query_map([project_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }

    /// Installs a plugin, on. Installing one already installed changes
    /// nothing, so a second click does not turn back on what was turned off.
    pub fn install_plugin(&self, project_id: &str, plugin_id: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO project_plugin (project_id, plugin_id, installed, enabled, updated_at) \
             VALUES (?1, ?2, 1, 1, ?3) \
             ON CONFLICT (project_id, plugin_id) DO UPDATE SET \
             installed = 1, enabled = 1, updated_at = excluded.updated_at, \
             revision = revision + 1 WHERE installed = 0",
            rusqlite::params![project_id, plugin_id, now()],
        )?;
        Ok(())
    }

    /// Off and uninstalled, as a row rather than a delete so the removal syncs.
    pub fn uninstall_plugin(&self, project_id: &str, plugin_id: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE project_plugin SET installed = 0, enabled = 0, updated_at = ?3, \
             revision = revision + 1 \
             WHERE project_id = ?1 AND plugin_id = ?2 AND installed = 1",
            rusqlite::params![project_id, plugin_id, now()],
        )?;
        Ok(())
    }

    /// Turns an installed plugin on or off; `false` when it is not installed,
    /// which has nothing to turn.
    pub fn set_plugin_enabled(
        &self,
        project_id: &str,
        plugin_id: &str,
        enabled: bool,
    ) -> Result<bool, StoreError> {
        let changed = self.conn.execute(
            "UPDATE project_plugin SET enabled = ?3, \
             updated_at = CASE WHEN enabled != ?3 THEN ?4 ELSE updated_at END, \
             revision = revision + (enabled != ?3) \
             WHERE project_id = ?1 AND plugin_id = ?2 AND installed = 1",
            rusqlite::params![project_id, plugin_id, enabled, now()],
        )?;
        Ok(changed == 1)
    }
}

/// Writes every `drawing` row out as a file before the table is dropped.
///
/// A name that is no file name, or that anything but this same scene already
/// holds, is saved under the row's id instead. A project folder that takes
/// neither sends the drawing to `drawings/` and rings the bell: a file planted
/// there must not keep the app from opening. Only a drawing nothing takes
/// refuses the migration, because dropping the table would lose it.
pub(super) fn export_drawings(conn: &Connection, root: &Path) -> Result<(), StoreError> {
    let mut stmt = conn.prepare(
        "SELECT drawing.id, drawing.project_id, project.folder, drawing.name, drawing.scene \
         FROM drawing JOIN project ON project.id = drawing.project_id ORDER BY drawing.id",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let (id, project_id): (String, String) = (row.get(0)?, row.get(1)?);
        let (name, scene): (String, String) = (row.get(3)?, row.get(4)?);
        let by_id = format!("{id}{DRAWING_EXTENSION}");

        let in_project = ProjectHome::named(root, &project_id, row.get(2)?)
            .ok()
            .and_then(|home| {
                let data = home.plugin_data(DRAWINGS_PLUGIN).ok()?;
                let relative = format!("{}/{data}", home.folder());
                data_files::dir_inside(&projects_dir(root), &relative).ok()
            });
        let named = format!("{name}{DRAWING_EXTENSION}");
        if in_project.is_some_and(|dir| exported(&dir, [named, by_id.clone()], &scene)) {
            continue;
        }

        let kept = data_files::dir_inside(root, KEPT_DRAWINGS)
            .is_ok_and(|dir| exported(&dir, [by_id.clone()], &scene));
        if !kept {
            return Err(StoreError::Export {
                drawing: id,
                reason: format!("neither its project's folder nor {KEPT_DRAWINGS}/ would take it"),
            });
        }
        let detail = format!("Saved as {KEPT_DRAWINGS}/{by_id} in the devpit folder.");
        let title = "A drawing could not be saved in its project's folder";
        Store::add_notice_on(
            conn,
            Some(&project_id),
            DRAWING_NOTICE,
            title,
            Some(&detail),
            None,
        )?;
    }
    Ok(())
}

/// Saves `scene` under the first of `names` that is free or already holds it.
///
/// The name is checked before the disk is asked about it, and whatever else
/// sits at a name — a link, a FIFO, a directory, another file — is left as it
/// is and never opened.
fn exported<const N: usize>(dir: &Path, names: [String; N], scene: &str) -> bool {
    names
        .iter()
        .filter(|file| plain_data_name(file))
        .any(|file| {
            let path = dir.join(file);
            match std::fs::symlink_metadata(&path) {
                Err(err) if err.kind() == ErrorKind::NotFound => {
                    data_files::replace(dir, file, scene.as_bytes()).is_ok()
                }
                Ok(_) => data_files::holds(&path, scene.as_bytes()).unwrap_or(false),
                Err(_) => false,
            }
        })
}

#[cfg(test)]
#[path = "plugins_tests.rs"]
mod tests;
